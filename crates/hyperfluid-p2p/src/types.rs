use serde::{Deserialize, Serialize};
use sha3::{
    digest::{ExtendableOutput, Update, XofReader},
    Digest,
};
use sha3::{Sha3_256, Shake256};

pub type Hash32 = [u8; 32];
pub type PeerId = Hash32;

/// Hash bytes via SHA3-256.
pub fn hash_bytes(data: &[u8]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    Digest::update(&mut hasher, data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Bitmask of node capabilities. Source: p2p-wire-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityFlags(pub u8);

impl CapabilityFlags {
    pub const RELAY: u8 = 0b0000_0001;
    pub const VALIDATOR: u8 = 0b0000_0010;
    pub const FULL_NODE: u8 = 0b0000_0100;
    pub const BOOTSTRAP: u8 = 0b0000_1000;
    pub const ARCHIVE: u8 = 0b0001_0000;
    pub const STUN_SERVER: u8 = 0b0010_0000;
    pub const STORAGE_PROVIDER: u8 = 0b0100_0000;
    pub const AGENT_HOST: u8 = 0b1000_0000;

    pub fn new() -> Self {
        Self(0)
    }

    pub fn set(&mut self, flag: u8) {
        self.0 |= flag;
    }

    pub fn clear(&mut self, flag: u8) {
        self.0 &= !flag;
    }

    pub fn has(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    pub fn bits(&self) -> u8 {
        self.0
    }
}

impl Default for CapabilityFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Peer identity and reachability. Source: p2p-wire-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerInfo {
    pub peer_id: Hash32,
    pub endpoints: Vec<String>,
    pub relay_routes: Vec<Hash32>,
    pub last_seen_height: u64,
    pub capabilities: CapabilityFlags,
}

/// Connection tracked per peer. Source: p2p-wire-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionState {
    pub peer_id: Hash32,
    pub state: ConnState,
    pub direct_endpoint: Option<String>,
    pub relay_path: Option<Vec<Hash32>>,
    pub last_probe_height: u64,
    pub consecutive_failures: u32,
}

/// Connection state machine states. Source: p2p-wire-spec.md Section 1.4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnState {
    Unknown,
    DirectProbing,
    DirectActive,
    RelayActive,
    Upgrading,
}

/// Kademlia DHT entry with signed peer record. Source: p2p-wire-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DHTEntry {
    pub key: Hash32,
    pub value: PeerInfo,
    pub ttl_blocks: u64,
    pub signature: Vec<u8>,
}

/// Gossip message envelope. Source: p2p-wire-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GossipMessage {
    pub message_id: Hash32,
    pub ttl: u8,
    pub fanout: u8,
    pub payload: Vec<u8>,
    pub origin_peer_id: Hash32,
    pub timestamp: u64,
    pub version: u8,
}

impl GossipMessage {
    pub const MAX_FANOUT: u8 = 8;
    pub const MAX_TTL: u8 = 16;
}

/// Bloom filter for gossip deduplication. Source: p2p-wire-spec.md Section 1.4
///
/// Parameters: 100,000 expected entries, 1% false positive rate.
/// Precomputed optimal values: bit array ~1,048,576 bits, 7 hash functions.
pub struct GossipBloomFilter {
    bits: Vec<u64>,
    bit_len: usize,
    num_hashes: u32,
    count: u64,
}

/// Precomputed Bloom filter parameters for spec-defined target:
/// n = 100,000, p = 0.01 → m = 958,506, k = 7.
const BLOOM_M: usize = 958_506;
const BLOOM_K: u32 = 7;
const BLOOM_WORDS: usize = BLOOM_M.div_ceil(64);

impl GossipBloomFilter {
    /// Create a Bloom filter with precomputed spec parameters (100k entries, 1% FPR).
    pub fn new() -> Self {
        Self { bits: vec![0u64; BLOOM_WORDS], bit_len: BLOOM_M, num_hashes: BLOOM_K, count: 0 }
    }

    /// Insert an element into the filter.
    pub fn insert(&mut self, data: &[u8]) {
        let hashes = Self::hash_indices(data, self.num_hashes, self.bit_len);
        let mut inserted = false;
        for h in &hashes {
            let (word_idx, bit_mask) = Self::bit_location(*h);
            if (self.bits[word_idx] & bit_mask) == 0 {
                inserted = true;
                self.bits[word_idx] |= bit_mask;
            }
        }
        if inserted {
            self.count += 1;
        }
    }

    /// Check if an element may be present (false positives possible).
    pub fn contains(&self, data: &[u8]) -> bool {
        let hashes = Self::hash_indices(data, self.num_hashes, self.bit_len);
        hashes.iter().all(|h| {
            let (word_idx, bit_mask) = Self::bit_location(*h);
            (self.bits[word_idx] & bit_mask) != 0
        })
    }

    /// Number of unique elements inserted.
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Clear all entries from the filter.
    pub fn clear(&mut self) {
        for word in &mut self.bits {
            *word = 0;
        }
        self.count = 0;
    }

    fn bit_location(index: usize) -> (usize, u64) {
        (index / 64, 1u64 << (index % 64))
    }

    /// Generate k hash indices from data using SHAKE-256 for double hashing.
    fn hash_indices(data: &[u8], k: u32, bit_len: usize) -> Vec<usize> {
        let mut hasher = Shake256::default();
        Update::update(&mut hasher, data);
        let mut reader = hasher.finalize_xof();
        let mut buf = [0u8; 8];
        let mut indices = Vec::with_capacity(k as usize);
        for _ in 0..k {
            reader.read(&mut buf);
            let val = u64::from_le_bytes(buf);
            indices.push((val as usize) % bit_len);
        }
        indices
    }
}

impl Default for GossipBloomFilter {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for the peer discovery subsystem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryConfig {
    pub direct_retry_attempts: u32,
    pub direct_retry_timeout_secs: u64,
    pub dht_k: u32,
    pub dht_refresh_secs: u64,
    pub gossip_fanout: u8,
    pub gossip_ttl: u8,
    pub upgrade_probe_secs: u64,
    pub upgrade_probe_jitter_pct: u8,
    pub min_bootstrap_cache: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            direct_retry_attempts: 3,
            direct_retry_timeout_secs: 5,
            dht_k: 20,
            dht_refresh_secs: 1800,
            gossip_fanout: GossipMessage::MAX_FANOUT,
            gossip_ttl: GossipMessage::MAX_TTL,
            upgrade_probe_secs: 60,
            upgrade_probe_jitter_pct: 20,
            min_bootstrap_cache: 5,
        }
    }
}

/// Bootstrap response from a bootstrap node. Source: p2p-wire-spec.md Section 1.2
pub struct BootstrapResponse {
    pub seed_peers: Vec<PeerInfo>,
    pub relay_list: Vec<PeerId>,
    pub trust_policy: TrustPolicy,
    pub signature: Vec<u8>,
}

/// Trust policy returned by bootstrap nodes. Source: p2p-wire-spec.md Section 1.2
pub struct TrustPolicy {
    pub min_trust_stage: u8,
    pub require_validator: bool,
    pub blocked_peer_ids: Vec<Hash32>,
}
