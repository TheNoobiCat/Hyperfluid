// === Inbox Router & Off-Chain Agent Messaging ===
//
// Source: docs/05-planning/stages/stage-02-agent-runtime.md Week 9-10 Task 6
//         docs/04-specifications/runtime/collaboration-spec.md §3
//
// Defines the InboxMessage type and inbox router that validates PDP quotas
// and routes messages through Bloom-filter deduplication.

use std::collections::VecDeque;

use sha3::{Digest, Sha3_256};

pub type Hash32 = [u8; 32];

/// Off-chain agent message routed through gossip.
///
/// Messages carry a sender signature for authentication. The inbox router
/// enforces PDP quotas at the enforcement point before delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxMessage {
    /// SHA3-256 hash of the message body (content-addressed identifier)
    pub message_id: Hash32,
    /// Sender agent ID
    pub sender_id: Hash32,
    /// Recipient agent ID
    pub recipient_id: Hash32,
    /// Topic scope (zero for direct messages)
    pub topic_id: Hash32,
    /// Message body (arbitrary bytes, max 64 KiB)
    pub body_bytes: Vec<u8>,
    /// Per-sender monotonic nonce for replay protection
    pub nonce: u64,
    /// Block height at which this message expires
    pub expires_at_height: u64,
    /// ML-DSA-65 signature over (message_id || sender_id || recipient_id || topic_id || body_hash || nonce || expires_at_height)
    pub signature: Vec<u8>,
}

/// Inbox routing decision after PDP quota enforcement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InboxDecision {
    /// Message accepted and delivered to agent inbox
    Delivered,
    /// Message rejected: sender quota exhausted
    QuotaExhausted,
    /// Message rejected: duplicate (already seen)
    Duplicate,
    /// Message rejected: expired TTL
    Expired,
    /// Message rejected: global inbox budget exceeded
    GlobalBudgetExceeded,
    /// Message rejected: message_id does not match content-addressed computation
    InvalidContentAddressing,
}

/// Compute the content-addressed message_id for an inbox message.
///
/// The message_id is computed as:
/// `SHA3-256(sender_id || recipient_id || topic_id || SHA3-256(body_bytes) || nonce || expires_at_height)`
///
/// Both the sender and the router use this function to ensure the message_id
/// is content-addressed and cannot be forged.
pub fn compute_message_id(
    sender_id: &Hash32,
    recipient_id: &Hash32,
    topic_id: &Hash32,
    body_bytes: &[u8],
    nonce: u64,
    expires_at_height: u64,
) -> Hash32 {
    let body_hash = {
        let mut h = Sha3_256::new();
        h.update(body_bytes);
        let mut out = [0u8; 32];
        out.copy_from_slice(&h.finalize());
        out
    };
    let mut h = Sha3_256::new();
    h.update(sender_id);
    h.update(recipient_id);
    h.update(topic_id);
    h.update(body_hash);
    h.update(nonce.to_le_bytes());
    h.update(expires_at_height.to_le_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&h.finalize());
    out
}

/// Per-sender quota tracking state.
#[derive(Debug, Clone, Default)]
struct SenderQuota {
    /// Messages sent by this sender in the current window
    sent_count: u64,
    /// Height at which the current window started
    window_start_height: u64,
}

/// Inbox quota configuration.
#[derive(Debug, Clone)]
pub struct InboxConfig {
    /// Maximum messages per sender per window (default: 50)
    pub max_per_sender: u64,
    /// Window duration in blocks (default: 20 blocks = ~200 seconds)
    pub sender_window_blocks: u64,
    /// Maximum total inbox messages per agent per hour (default: 2000)
    pub global_per_hour: u64,
    /// Maximum TTL in blocks (default: 10000 = ~28 hours)
    pub max_ttl_blocks: u64,
    /// Maximum message body size in bytes (default: 65536 = 64 KiB)
    pub max_body_bytes: usize,
}

impl Default for InboxConfig {
    fn default() -> Self {
        Self {
            max_per_sender: 50,
            sender_window_blocks: 20,
            global_per_hour: 2000,
            max_ttl_blocks: 10000,
            max_body_bytes: 65536,
        }
    }
}

/// Inbox router that enforces PDP quotas and routes messages to agent inboxes.
pub struct InboxRouter {
    config: InboxConfig,
    /// Per-agent inbox message queues
    inboxes: std::collections::BTreeMap<Hash32, VecDeque<InboxMessage>>,
    /// Per-agent seen message IDs for deduplication
    seen_messages: std::collections::BTreeMap<Hash32, Vec<Hash32>>,
    /// Per-(agent, sender) quota tracking
    sender_quotas: std::collections::BTreeMap<(Hash32, Hash32), SenderQuota>,
    /// Per-agent total messages in current hour window
    global_counters: std::collections::BTreeMap<Hash32, u64>,
    /// Block height at which global counters reset
    global_window_start: u64,
    /// Blocks per hour window
    blocks_per_hour: u64,
    /// Current block height
    current_height: u64,
}

impl InboxRouter {
    /// Create a new inbox router with the given configuration.
    pub fn new(config: InboxConfig, blocks_per_hour: u64) -> Self {
        Self {
            config,
            inboxes: std::collections::BTreeMap::new(),
            seen_messages: std::collections::BTreeMap::new(),
            sender_quotas: std::collections::BTreeMap::new(),
            global_counters: std::collections::BTreeMap::new(),
            global_window_start: 0,
            blocks_per_hour,
            current_height: 0,
        }
    }

    /// Advance the router's internal clock to the given block height.
    /// Rotates quota windows if the height crosses window boundaries.
    pub fn advance_height(&mut self, height: u64) {
        self.current_height = height;

        // Reset global counters if hour window passed
        if height >= self.global_window_start + self.blocks_per_hour {
            self.global_counters.clear();
            self.global_window_start = height;
        }

        // Reset sender quotas whose windows have expired
        let window = self.config.sender_window_blocks;
        let expired: Vec<_> = self
            .sender_quotas
            .iter()
            .filter(|(_, q)| height >= q.window_start_height + window)
            .map(|(k, _)| *k)
            .collect();
        for key in expired {
            self.sender_quotas.remove(&key);
        }
    }

    /// Route an inbox message through quota enforcement and deduplication.
    ///
    /// Returns the routing decision. If `Delivered`, the message is queued
    /// for the recipient agent and can be retrieved via `poll_inbox`.
    pub fn route_message(&mut self, msg: InboxMessage) -> InboxDecision {
        // Signature verification is performed by the caller (node RPC handler)
        // before invoking route_message. We assert here to catch integration bugs.
        if msg.signature.is_empty() {
            eprintln!("[hyperfluid-collaboration] inbox message with empty signature rejected — caller must verify before routing");
            return InboxDecision::InvalidContentAddressing;
        }

        // TTL check
        if msg.expires_at_height <= self.current_height {
            return InboxDecision::Expired;
        }
        if msg.expires_at_height > self.current_height + self.config.max_ttl_blocks {
            return InboxDecision::Expired;
        }

        // Body size check
        if msg.body_bytes.len() > self.config.max_body_bytes {
            return InboxDecision::QuotaExhausted;
        }

        // Content-addressed message_id integrity check (F-65)
        let expected_id = compute_message_id(
            &msg.sender_id,
            &msg.recipient_id,
            &msg.topic_id,
            &msg.body_bytes,
            msg.nonce,
            msg.expires_at_height,
        );
        if msg.message_id != expected_id {
            return InboxDecision::InvalidContentAddressing;
        }

        // Deduplication
        let seen = self.seen_messages.entry(msg.recipient_id).or_default();
        if seen.contains(&msg.message_id) {
            return InboxDecision::Duplicate;
        }

        // Global inbox budget
        let global = self.global_counters.entry(msg.recipient_id).or_insert(0);
        if *global >= self.config.global_per_hour {
            return InboxDecision::GlobalBudgetExceeded;
        }

        // Per-sender quota
        let sender_key = (msg.recipient_id, msg.sender_id);
        let quota = self
            .sender_quotas
            .entry(sender_key)
            .or_insert(SenderQuota { sent_count: 0, window_start_height: self.current_height });
        if quota.sent_count >= self.config.max_per_sender {
            return InboxDecision::QuotaExhausted;
        }

        // Accept: update state
        seen.push(msg.message_id);
        quota.sent_count += 1;
        *global += 1;

        // Limit seen messages per agent
        if seen.len() > 10000 {
            seen.drain(0..seen.len() - 5000);
        }

        // Queue for agent
        self.inboxes.entry(msg.recipient_id).or_default().push_back(msg);

        InboxDecision::Delivered
    }

    /// Poll the next message from an agent's inbox (FIFO).
    /// Returns `None` if the inbox is empty.
    pub fn poll_inbox(&mut self, agent_id: &Hash32) -> Option<InboxMessage> {
        self.inboxes.get_mut(agent_id).and_then(|q| q.pop_front())
    }

    /// Peek at pending messages without removing them.
    pub fn pending_count(&self, agent_id: &Hash32) -> usize {
        self.inboxes.get(agent_id).map(|q| q.len()).unwrap_or(0)
    }

    /// Check if a message has been seen by an agent.
    pub fn is_seen(&self, agent_id: &Hash32, message_id: &Hash32) -> bool {
        self.seen_messages.get(agent_id).map(|v| v.contains(message_id)).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_msg(sender: u8, recipient: u8, body: &[u8], nonce: u64) -> InboxMessage {
        let sender_id = [sender; 32];
        let recipient_id = [recipient; 32];
        let topic_id = [0u8; 32];
        let body_vec = body.to_vec();
        let msg_id =
            compute_message_id(&sender_id, &recipient_id, &topic_id, &body_vec, nonce, 1000);
        InboxMessage {
            message_id: msg_id,
            sender_id,
            recipient_id,
            topic_id,
            body_bytes: body_vec,
            nonce,
            expires_at_height: 1000,
            signature: vec![1u8; 64], // non-empty dummy signature
        }
    }

    #[test]
    fn inbox_router_delivers_message() {
        let mut router = InboxRouter::new(InboxConfig::default(), 600);
        router.advance_height(50);
        let msg = make_msg(1, 2, b"hello", 1);
        let result = router.route_message(msg);
        assert_eq!(result, InboxDecision::Delivered);
        assert_eq!(router.pending_count(&[2u8; 32]), 1);
    }

    #[test]
    fn inbox_router_rejects_duplicate() {
        let mut router = InboxRouter::new(InboxConfig::default(), 600);
        router.advance_height(50);
        let msg = make_msg(1, 2, b"hello", 1);
        let msg2 = msg.clone();
        assert_eq!(router.route_message(msg), InboxDecision::Delivered);
        assert_eq!(router.route_message(msg2), InboxDecision::Duplicate);
    }

    #[test]
    fn inbox_router_rejects_expired() {
        let mut router = InboxRouter::new(InboxConfig::default(), 600);
        router.advance_height(2000);
        let msg = make_msg(1, 2, b"hello", 1); // expires at 1000
        assert_eq!(router.route_message(msg), InboxDecision::Expired);
    }

    #[test]
    fn inbox_router_enforces_per_sender_quota() {
        let config = InboxConfig { max_per_sender: 3, ..Default::default() };
        let mut router = InboxRouter::new(config, 600);
        router.advance_height(50);

        for i in 0..3 {
            assert_eq!(
                router.route_message(make_msg(1, 2, &[i as u8], i as u64)),
                InboxDecision::Delivered
            );
        }
        assert_eq!(
            router.route_message(make_msg(1, 2, b"fourth", 4)),
            InboxDecision::QuotaExhausted
        );
    }

    #[test]
    fn inbox_router_enforces_global_budget() {
        let config = InboxConfig { global_per_hour: 3, ..Default::default() };
        let mut router = InboxRouter::new(config, 600);
        router.advance_height(50);

        for i in 0..3 {
            assert_eq!(
                router.route_message(make_msg(i as u8 + 1, 2, &[i as u8], i as u64)),
                InboxDecision::Delivered
            );
        }
        assert_eq!(
            router.route_message(make_msg(10, 2, b"fourth", 10)),
            InboxDecision::GlobalBudgetExceeded
        );
    }

    #[test]
    fn inbox_router_poll_returns_fifo() {
        let mut router = InboxRouter::new(InboxConfig::default(), 600);
        router.advance_height(50);

        let msg1 = make_msg(1, 2, b"first", 1);
        let msg2 = make_msg(1, 2, b"second", 2);
        router.route_message(msg1.clone());
        router.route_message(msg2.clone());

        let polled1 = router.poll_inbox(&[2u8; 32]).unwrap();
        assert_eq!(polled1.body_bytes, b"first".to_vec());

        let polled2 = router.poll_inbox(&[2u8; 32]).unwrap();
        assert_eq!(polled2.body_bytes, b"second".to_vec());

        assert!(router.poll_inbox(&[2u8; 32]).is_none());
    }

    #[test]
    fn inbox_router_window_rotation_resets_quotas() {
        let config =
            InboxConfig { max_per_sender: 2, sender_window_blocks: 10, ..Default::default() };
        let mut router = InboxRouter::new(config, 600);
        router.advance_height(50);

        assert_eq!(router.route_message(make_msg(1, 2, b"a", 1)), InboxDecision::Delivered);
        assert_eq!(router.route_message(make_msg(1, 2, b"b", 2)), InboxDecision::Delivered);
        assert_eq!(router.route_message(make_msg(1, 2, b"c", 3)), InboxDecision::QuotaExhausted);

        // Advance past window
        router.advance_height(65);
        assert_eq!(router.route_message(make_msg(1, 2, b"d", 4)), InboxDecision::Delivered);
    }

    #[test]
    fn inbox_router_global_window_rotation() {
        let config = InboxConfig { global_per_hour: 2, ..Default::default() };
        let mut router = InboxRouter::new(config, 100); // 100 blocks per hour window
        router.advance_height(50);

        assert_eq!(router.route_message(make_msg(1, 2, b"a", 1)), InboxDecision::Delivered);
        assert_eq!(router.route_message(make_msg(3, 2, b"b", 1)), InboxDecision::Delivered);
        assert_eq!(
            router.route_message(make_msg(5, 2, b"c", 1)),
            InboxDecision::GlobalBudgetExceeded
        );

        // Advance past global window
        router.advance_height(160);
        assert_eq!(router.route_message(make_msg(7, 2, b"d", 1)), InboxDecision::Delivered);
    }

    #[test]
    fn inbox_router_body_size_limit() {
        let config = InboxConfig { max_body_bytes: 100, ..Default::default() };
        let mut router = InboxRouter::new(config, 600);
        router.advance_height(50);

        let large_body = vec![0u8; 200];
        let msg = make_msg(1, 2, &large_body, 1);
        assert_eq!(router.route_message(msg), InboxDecision::QuotaExhausted);
    }

    #[test]
    fn inbox_router_is_seen_tracks_deduplication() {
        let mut router = InboxRouter::new(InboxConfig::default(), 600);
        router.advance_height(50);
        let msg = make_msg(1, 2, b"test", 1);
        let msg_id = msg.message_id;
        router.route_message(msg);
        assert!(router.is_seen(&[2u8; 32], &msg_id));
        assert!(!router.is_seen(&[2u8; 32], &[0xFF; 32]));
    }
}
