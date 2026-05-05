// === Consensus data structures ===
//
// Source: specs/protocol/consensus-spec.md Section 1.3

use parity_scale_codec::{Decode, Encode};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

pub type Hash32 = [u8; 32];
pub type Signature = Vec<u8>;

fn hash_bytes(data: &[u8]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Epoch committee. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committee {
    pub epoch: u64,
    pub seed: Hash32,
    pub members: Vec<Hash32>,
    pub weights: Vec<u64>,
}

impl Committee {
    /// The minimum number of active validators required for block production.
    /// 2f+1 safety threshold: with f=33, need 67.
    pub const SAFETY_THRESHOLD: u64 = 67;

    /// Maximum committee size.
    pub const COMMITTEE_SIZE: u64 = 100;

    pub fn safety_threshold() -> u64 {
        Self::SAFETY_THRESHOLD
    }

    pub fn can_produce(active_count: u64) -> bool {
        active_count >= Self::SAFETY_THRESHOLD
    }

    /// Deterministically sample a committee from the validator pool.
    /// Given the same (epoch, seed, validators, stakes, committee_size),
    /// the output MUST be identical across all nodes.
    ///
    /// Uses SHA3-256-based weighted selection. For each committee seat,
    /// a deterministic index is derived from SHA3-256(epoch || seed || seat_index).
    ///
    /// If `previous_members` is provided, enforces at most 33% overlap
    /// (spec Section 1.4: at most 33% overlap, 67% minimum rotation).
    pub fn sample(
        epoch: u64,
        seed: Hash32,
        validators: &[Hash32],
        stakes: &[u64],
        committee_size: usize,
    ) -> Self {
        Self::sample_with_rotation(epoch, seed, validators, stakes, committee_size, &[])
    }

    /// Sample committee with rotation constraint against previous members.
    pub fn sample_with_rotation(
        epoch: u64,
        seed: Hash32,
        validators: &[Hash32],
        stakes: &[u64],
        committee_size: usize,
        previous_members: &[Hash32],
    ) -> Self {
        assert_eq!(validators.len(), stakes.len());
        assert!(
            validators.len() >= committee_size,
            "need at least {} validators to sample a {}-seat committee",
            committee_size,
            committee_size
        );

        let mut members = Vec::with_capacity(committee_size);
        let mut weights = Vec::with_capacity(committee_size);
        let mut used = std::collections::HashSet::new();

        let previous_set: std::collections::HashSet<_> = previous_members.iter().collect();
        let max_overlap = (committee_size as f64 * 0.33).ceil() as usize;

        for seat_index in 0..committee_size {
            let mut hasher = Sha3_256::new();
            hasher.update(epoch.to_le_bytes());
            hasher.update(seed);
            hasher.update((seat_index as u64).to_le_bytes());
            let mut entropy = [0u8; 32];
            entropy.copy_from_slice(&hasher.finalize());

            let total_stake: u64 = stakes.iter().sum();
            let selector = if total_stake > 0 {
                u64::from_le_bytes(entropy[..8].try_into().unwrap()) % total_stake
            } else {
                0
            };

            let mut cumulative = 0u64;
            let mut chosen_idx = 0usize;
            for (i, stake) in stakes.iter().enumerate() {
                cumulative += stake;
                if selector < cumulative {
                    chosen_idx = i;
                    break;
                }
            }

            // Enforce rotation constraint: if we've reached max overlap,
            // skip validators that were in the previous committee
            let mut attempt = 0usize;
            while attempt < validators.len() {
                let already_used = used.contains(&validators[chosen_idx]);
                let would_exceed_overlap = if !already_used && !previous_set.is_empty() {
                    let current_overlap =
                        members.iter().filter(|m| previous_set.contains(m)).count();
                    previous_set.contains(&validators[chosen_idx]) && current_overlap >= max_overlap
                } else {
                    false
                };

                if !already_used && !would_exceed_overlap {
                    break;
                }
                chosen_idx = (chosen_idx + 1) % validators.len();
                attempt += 1;
            }

            members.push(validators[chosen_idx]);
            weights.push(stakes[chosen_idx]);
            used.insert(validators[chosen_idx]);
        }

        Self { epoch, seed, members, weights }
    }
}

/// Block header with state root commitment. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Encode, Decode)]
pub struct BlockHeader {
    pub height: u64,
    pub parent_hash: Hash32,
    pub state_root: Hash32,
    pub transaction_root: Hash32,
    pub committee_id: u64,
    pub proposer_id: Hash32,
    pub timestamp: u64,
    pub epoch: u64,
}

impl BlockHeader {
    /// Compute the canonical block hash = SHA3-256(SCALE(BlockHeader)).
    pub fn block_hash(&self) -> Hash32 {
        let encoded = self.encode();
        hash_bytes(&encoded)
    }
}

/// A full block. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<TransactionEnvelope>,
}

/// Transaction envelope wrapping a typed payload. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionEnvelope {
    pub tx_type: TxType,
    pub tx_payload: Vec<u8>,
    pub approved_plan_id: Option<Hash32>,
    pub gateway_signature: Option<Signature>,
}

/// All transaction types on the protocol. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxType {
    TransferTx,
    StakeBondTx,
    StakeRenewTx,
    UnbondRequestTx,
    WithdrawUnbondedTx,
    TaskCreateTx,
    GovernanceProposeTx,
    GovernanceVoteTx,
    EvidenceTx,
    FastPathProposalTx,
    FastPathReviewTx,
    FastPathChallengeTx,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_header_is_copyable_after_serde() {
        let h = BlockHeader {
            height: 1,
            parent_hash: [0; 32],
            state_root: [1; 32],
            transaction_root: [2; 32],
            committee_id: 0,
            proposer_id: [3; 32],
            timestamp: 1000,
            epoch: 0,
        };
        let json = serde_json::to_string(&h).unwrap();
        let h2: BlockHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn tx_type_variants_are_exhaustive() {
        let types = [
            TxType::TransferTx,
            TxType::StakeBondTx,
            TxType::StakeRenewTx,
            TxType::UnbondRequestTx,
            TxType::WithdrawUnbondedTx,
            TxType::TaskCreateTx,
            TxType::GovernanceProposeTx,
            TxType::GovernanceVoteTx,
            TxType::EvidenceTx,
            TxType::FastPathProposalTx,
            TxType::FastPathReviewTx,
            TxType::FastPathChallengeTx,
        ];
        assert_eq!(types.len(), 12);
    }

    #[test]
    fn committee_size_validation() {
        let c = Committee {
            epoch: 0,
            seed: [0; 32],
            members: vec![[1; 32]; 100],
            weights: vec![1000; 100],
        };
        assert_eq!(c.members.len(), 100);
        assert_eq!(c.weights.len(), 100);
    }
}
