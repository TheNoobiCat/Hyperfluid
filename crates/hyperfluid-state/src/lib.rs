// === C2 State Machine & SMT: Core Types ===
//
// Source: specs/protocol/consensus-spec.md Section 2, state-model.md

pub mod smt;
pub mod state_machine;
pub mod state_sync;

use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyPrefix {
    Account = 0x01,
    Validator = 0x02,
    GovernanceProposal = 0x03,
    Committee = 0x04,
    ArtifactManifest = 0x05,
    Task = 0x06,
    TelemetryEnvelope = 0x07,
    SystemParams = 0x08,
    TrustStage = 0x09,
    ActionPlan = 0x0A,
    AirdropPool = 0x0B,
    ReplicationLease = 0x0C,
    ReviewAssignment = 0x0D,
    Delegation = 0x0E,
    TaskLease = 0x0F,
    Topic = 0x10,
    ConsumedNonce = 0x11,
}

impl KeyPrefix {
    pub fn byte(&self) -> u8 {
        *self as u8
    }

    pub const fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(KeyPrefix::Account),
            0x02 => Some(KeyPrefix::Validator),
            0x03 => Some(KeyPrefix::GovernanceProposal),
            0x04 => Some(KeyPrefix::Committee),
            0x05 => Some(KeyPrefix::ArtifactManifest),
            0x06 => Some(KeyPrefix::Task),
            0x07 => Some(KeyPrefix::TelemetryEnvelope),
            0x08 => Some(KeyPrefix::SystemParams),
            0x09 => Some(KeyPrefix::TrustStage),
            0x0A => Some(KeyPrefix::ActionPlan),
            0x0B => Some(KeyPrefix::AirdropPool),
            0x0C => Some(KeyPrefix::ReplicationLease),
            0x0D => Some(KeyPrefix::ReviewAssignment),
            0x0E => Some(KeyPrefix::Delegation),
            0x0F => Some(KeyPrefix::TaskLease),
            0x10 => Some(KeyPrefix::Topic),
            0x11 => Some(KeyPrefix::ConsumedNonce),
            _ => None,
        }
    }
}

pub type Hash32 = [u8; 32];

pub fn sha3_256(data: &[u8]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

pub fn state_key(prefix: KeyPrefix, id_bytes: &[u8]) -> Hash32 {
    let mut preimage = Vec::with_capacity(1 + id_bytes.len());
    preimage.push(prefix.byte());
    preimage.extend_from_slice(id_bytes);
    sha3_256(&preimage)
}

/// Account entity. Source: consensus-spec.md Section 2.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Account {
    pub account_id: Hash32,
    pub balance: u128,
    pub nonce: u64,
    pub pubkey_hash: Hash32,
    pub pubkey: Option<Vec<u8>>,
}

/// Task entity (on-chain state). Source: collaboration-spec.md §1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct Task {
    pub task_id: Hash32,
    pub topic_id: Hash32,
    pub seed_ref: Hash32,
    pub parent_task_id: Hash32,
    pub depends_on: Vec<Hash32>,
    pub funder: Hash32,
    pub primary_owner: Hash32,
    pub status: TaskStatus,
    pub bounty_agx: u128,
    pub created_at_height: u64,
    pub lease_expires_height: u64,
    pub required_skills_hash: Hash32,
    pub metadata_hash: Hash32,
    pub sponsor_id: Hash32,
    pub requester_pubkey: Hash32,
    pub escrow_status: EscrowStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum TaskStatus {
    Open,
    Claimed,
    InProgress,
    InReview,
    Done,
    Decomposed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum EscrowStatus {
    Locked,
    BountyRedistributed,
    Released,
    Refunded,
}

/// Task lease (on-chain state). Source: collaboration-spec.md §1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct TaskLease {
    pub lease_id: Hash32,
    pub task_id: Hash32,
    pub owner_id: Hash32,
    pub collateral: u128,
    pub started_at_height: u64,
    pub expires_at_height: u64,
    pub last_heartbeat_height: u64,
    pub heartbeats_received: u32,
}

/// Heartbeat payload (submitted via consensus tx). Source: collaboration-spec.md §1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct HeartbeatPayload {
    pub lease_id: Hash32,
    pub artifact_hash: Option<Hash32>,
    pub diff_pointer: Option<Hash32>,
    pub test_result_ref: Option<Hash32>,
    pub signature: Vec<u8>,
}

/// Binary verdict from a reviewer on a completed task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum ReviewVerdict {
    Accept,
    Reject,
}

/// Review submission record tracked in the state machine.
/// Stored temporarily until majority verdict reached, then settled.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct ReviewRecord {
    pub task_id: Hash32,
    pub review_task_id: Hash32, // the task that pays the reviewer
    pub reviewer_id: Hash32,
    pub verdict: ReviewVerdict,
    pub evidence_hash: Hash32,
    pub submitted_at_height: u64,
}

/// Trust stage record (on-chain state under prefix 0x09)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct TrustStageRecord {
    pub agent_id: Hash32,
    pub stage: TrustStageEnum,
    pub accepted_work_count: u32,
    pub abuse_flags: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum TrustStageEnum {
    Untrusted,
    Trusted,
}

/// Topic lifecycle record (on-chain state under prefix 0x11)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct TopicRecord {
    pub topic_id: Hash32,
    pub seed_ref: Hash32,
    pub status: TopicStatus,
    pub created_at_height: u64,
    pub last_activity_height: u64,
    pub message_count: u64,
    pub decay_score: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub enum TopicStatus {
    New,
    Active,
    Stale,
    Archived,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_prefix_roundtrip() {
        for b in 0x01u8..=0x11 {
            let kp = KeyPrefix::from_byte(b).unwrap();
            assert_eq!(kp.byte(), b);
        }
    }

    #[test]
    fn key_prefix_invalid_byte() {
        assert!(KeyPrefix::from_byte(0x00).is_none());
        assert!(KeyPrefix::from_byte(0x13).is_none());
    }

    #[test]
    fn state_key_is_deterministic() {
        let id = [0xAAu8; 32];
        let k1 = state_key(KeyPrefix::Account, &id);
        let k2 = state_key(KeyPrefix::Account, &id);
        assert_eq!(k1, k2);
    }

    #[test]
    fn state_key_different_prefix_different_key() {
        let id = [0xAAu8; 32];
        let k1 = state_key(KeyPrefix::Account, &id);
        let k2 = state_key(KeyPrefix::Validator, &id);
        assert_ne!(k1, k2);
    }

    #[test]
    fn state_key_different_id_different_key() {
        let id1 = [0xAAu8; 32];
        let id2 = [0xBBu8; 32];
        let k1 = state_key(KeyPrefix::Account, &id1);
        let k2 = state_key(KeyPrefix::Account, &id2);
        assert_ne!(k1, k2);
    }

    #[test]
    fn sha3_256_is_deterministic() {
        let h1 = sha3_256(b"hello");
        let h2 = sha3_256(b"hello");
        assert_eq!(h1, h2);
        let expected = "3338be69";
        assert_eq!(&hex::encode(&h1[..4]), expected);
    }

    #[test]
    fn account_reveals_pubkey_on_first_spend() {
        let mut acct = Account {
            account_id: [0; 32],
            balance: 0,
            nonce: 0,
            pubkey_hash: sha3_256(&[1, 2, 3]),
            pubkey: None,
        };
        assert!(acct.pubkey.is_none());
        acct.pubkey = Some(vec![1, 2, 3]);
        assert_eq!(sha3_256(&[1, 2, 3]), acct.pubkey_hash);
    }
}
