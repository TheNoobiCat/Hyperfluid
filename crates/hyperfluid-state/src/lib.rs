// === C2 State Machine & SMT: Core Types ===
//
// Source: specs/protocol/consensus-spec.md Section 2, state-model.md

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
    CircuitBreakerState = 0x09,
    TrustStage = 0x0A,
    ActionPlan = 0x0B,
    AirdropPool = 0x0C,
    ReplicationLease = 0x0D,
    IncidentRecord = 0x0E,
    ReviewAssignment = 0x0F,
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
            0x09 => Some(KeyPrefix::CircuitBreakerState),
            0x0A => Some(KeyPrefix::TrustStage),
            0x0B => Some(KeyPrefix::ActionPlan),
            0x0C => Some(KeyPrefix::AirdropPool),
            0x0D => Some(KeyPrefix::ReplicationLease),
            0x0E => Some(KeyPrefix::IncidentRecord),
            0x0F => Some(KeyPrefix::ReviewAssignment),
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub account_id: Hash32,
    pub balance: u128,
    pub nonce: u64,
    pub pubkey_hash: Hash32,
    pub pubkey: Option<Vec<u8>>,
}

/// Sparse Merkle Tree node. Source: consensus-spec.md Section 2.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SMTNode {
    pub key: Hash32,
    pub value: Vec<u8>,
    pub hash: Hash32,
}

/// Inclusion proof from leaf to root. Source: consensus-spec.md Section 2.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InclusionProof {
    pub key: Hash32,
    pub value: Vec<u8>,
    pub proof: Vec<Hash32>,
    pub root: Hash32,
    pub height: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_prefix_roundtrip() {
        for b in 0x01u8..=0x0F {
            let kp = KeyPrefix::from_byte(b).unwrap();
            assert_eq!(kp.byte(), b);
        }
    }

    #[test]
    fn key_prefix_invalid_byte() {
        assert!(KeyPrefix::from_byte(0x00).is_none());
        assert!(KeyPrefix::from_byte(0x10).is_none());
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
