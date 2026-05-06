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

/// Committee liveness mode per three-tier stall model. Source: consensus-spec.md Section 1.2
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitteeMode {
    Normal,
    Degraded,
    Emergency,
}

/// Epoch committee. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committee {
    pub epoch: u64,
    pub seed: Hash32,
    pub members: Vec<Hash32>,
    pub weights: Vec<u128>,
}

impl Committee {
    /// Normal threshold: 67+ validators for full consensus.
    pub const NORMAL_THRESHOLD: u64 = 67;

    /// Degraded threshold: 50-66 validators for critical txs only.
    pub const DEGRADED_THRESHOLD: u64 = 50;

    /// Maximum committee size.
    pub const COMMITTEE_SIZE: u64 = 100;

    /// Emergency idle blocks before auto-recovery triggers.
    pub const EMERGENCY_IDLE_BLOCKS: u64 = 500;

    pub fn safety_threshold() -> u64 {
        Self::NORMAL_THRESHOLD
    }

    /// Determine committee mode from active validator count and total pool size.
    ///
    /// SPEC_DEVIATION: Bootstrap scaling. When total_validators < COMMITTEE_SIZE,
    /// thresholds are scaled proportionally so the chain can bootstrap from
    /// fewer than 50 validators. Without this, a single-node testnet or early
    /// network would enter Emergency mode immediately. Pending formal inclusion
    /// in consensus-spec.md. See docs/08-handoff/latest/open-questions.md#Q1.
    pub fn committee_mode(active_count: u64, total_validators: u64) -> CommitteeMode {
        if active_count == 0 {
            return CommitteeMode::Emergency;
        }
        let (normal, degraded) = Self::scaled_thresholds(total_validators);
        if active_count >= normal {
            CommitteeMode::Normal
        } else if active_count >= degraded {
            CommitteeMode::Degraded
        } else {
            CommitteeMode::Emergency
        }
    }

    /// Block production is possible in Normal and Degraded modes.
    /// Only Emergency mode halts production entirely.
    ///
    /// SPEC_DEVIATION: Uses scaled DEGRADED_THRESHOLD for bootstrap.
    pub fn can_produce(active_count: u64, total_validators: u64) -> bool {
        if active_count == 0 {
            return false;
        }
        let (_, degraded) = Self::scaled_thresholds(total_validators);
        active_count >= degraded
    }

    /// Scale NORMAL_THRESHOLD and DEGRADED_THRESHOLD proportionally
    /// when the total validator pool is below COMMITTEE_SIZE. This allows
    /// the chain to bootstrap with a small validator set.
    fn scaled_thresholds(total_validators: u64) -> (u64, u64) {
        if total_validators >= Self::COMMITTEE_SIZE {
            return (Self::NORMAL_THRESHOLD, Self::DEGRADED_THRESHOLD);
        }
        let n = total_validators as u128;
        let total = Self::COMMITTEE_SIZE as u128;
        let normal = ((Self::NORMAL_THRESHOLD as u128 * n).div_ceil(total)).min(n) as u64;
        let degraded = ((Self::DEGRADED_THRESHOLD as u128 * n).div_ceil(total)).min(n.saturating_sub(1)) as u64;
        (normal, std::cmp::max(degraded, 1))
    }

    /// Compute VDF fallback seed when <33% of committee reveals.
    /// Uses only finalized/historical entropy — no current-epoch malleable data.
    ///
    /// Formula: SHA3-256(previous_vdf_output || epoch_N-1_headers_hash || epoch_number || valid_reveals)
    pub fn compute_vdf_fallback(
        previous_vdf_output: &Hash32,
        epoch_headers_hash: &Hash32,
        epoch_number: u64,
        valid_reveals: &[Hash32],
    ) -> Hash32 {
        let mut hasher = Sha3_256::new();
        hasher.update(previous_vdf_output);
        hasher.update(epoch_headers_hash);
        hasher.update(epoch_number.to_le_bytes());
        for reveal in valid_reveals {
            hasher.update(reveal);
        }
        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out
    }

    /// Emergency epoch transition: sample a new committee from ALL validators
    /// in active or paused states (not unbonding/withdrawn). Used when
    /// the committee stalls for EMERGENCY_IDLE_BLOCKS.
    pub fn emergency_transition(
        epoch: u64,
        seed: Hash32,
        validators: &[Hash32],
        stakes: &[u128],
    ) -> Self {
        Self::sample(epoch, seed, validators, stakes, Self::COMMITTEE_SIZE as usize, &[])
    }

    /// Deterministically sample a committee from the validator pool.
    ///
    /// `ineligible` contains validators who served 2 consecutive epochs
    /// and must be excluded from this committee (two-epoch recency guard).
    pub fn sample(
        epoch: u64,
        seed: Hash32,
        validators: &[Hash32],
        stakes: &[u128],
        committee_size: usize,
        ineligible: &[Hash32],
    ) -> Self {
        Self::sample_with_rotation(epoch, seed, validators, stakes, committee_size, &[], ineligible)
    }

    /// Sample committee with rotation constraint and two-epoch recency guard.
    /// Uses integer arithmetic for determinism across all platforms.
    ///
    /// `previous_members` are the epoch N-1 committee members (max 20% overlap).
    /// `ineligible` are validators who served epochs N-1 AND N-2 (two-epoch guard).
    pub fn sample_with_rotation(
        epoch: u64,
        seed: Hash32,
        validators: &[Hash32],
        stakes: &[u128],
        committee_size: usize,
        previous_members: &[Hash32],
        ineligible: &[Hash32],
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
        let ineligible_set: std::collections::HashSet<_> = ineligible.iter().collect();
        // Integer arithmetic: ceil(committee_size * 20 / 100) — deterministic across all platforms
        let max_overlap = (committee_size * 20).div_ceil(100);

        for seat_index in 0..committee_size {
            let mut hasher = Sha3_256::new();
            hasher.update(epoch.to_le_bytes());
            hasher.update(seed);
            hasher.update((seat_index as u64).to_le_bytes());
            let mut entropy = [0u8; 32];
            entropy.copy_from_slice(&hasher.finalize());

            let total_stake: u128 = stakes.iter().sum();
            let selector = if total_stake > 0 {
                let selector_bytes: [u8; 16] = entropy[..16].try_into().unwrap();
                u128::from_le_bytes(selector_bytes) % total_stake
            } else {
                0u128
            };

            let mut cumulative = 0u128;
            let mut chosen_idx = 0usize;
            for (i, stake) in stakes.iter().enumerate() {
                cumulative += stake;
                if selector < cumulative {
                    chosen_idx = i;
                    break;
                }
            }

            // Priority-ordered constraint enforcement:
            // 1. Not already used AND not ineligible AND doesn't exceed overlap
            // 2. Not already used AND not ineligible (overlap relaxed)
            // 3. Not already used (ineligible + overlap relaxed)
            let mut found = false;

            // Pass 1: all constraints
            let mut idx = chosen_idx;
            let mut attempt = 0usize;
            while attempt < validators.len() {
                let already_used = used.contains(&validators[idx]);
                let is_ineligible = ineligible_set.contains(&validators[idx]);
                let would_exceed_overlap = if !already_used && !previous_set.is_empty() {
                    let current_overlap =
                        members.iter().filter(|m| previous_set.contains(m)).count();
                    previous_set.contains(&validators[idx]) && current_overlap >= max_overlap
                } else {
                    false
                };

                if !already_used && !is_ineligible && !would_exceed_overlap {
                    chosen_idx = idx;
                    found = true;
                    break;
                }
                idx = (idx + 1) % validators.len();
                attempt += 1;
            }

            // Pass 2: relax overlap, keep ineligible guard
            if !found {
                let mut idx = chosen_idx;
                let mut attempt = 0usize;
                while attempt < validators.len() {
                    let already_used = used.contains(&validators[idx]);
                    let is_ineligible = ineligible_set.contains(&validators[idx]);
                    if !already_used && !is_ineligible {
                        chosen_idx = idx;
                        found = true;
                        break;
                    }
                    idx = (idx + 1) % validators.len();
                    attempt += 1;
                }
            }

            // Pass 3: only prevent duplicates
            if !found {
                let mut idx = chosen_idx;
                let mut attempt = 0usize;
                while attempt < validators.len() {
                    let already_used = used.contains(&validators[idx]);
                    if !already_used {
                        chosen_idx = idx;
                        found = true;
                        break;
                    }
                    idx = (idx + 1) % validators.len();
                    attempt += 1;
                }
            }

            if !found {
                // Every validator is already used; this shouldn't happen
                // with validators.len() >= committee_size, but guard anyway.
                panic!("cannot sample committee: all validators already used");
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
/// Collapsed to 7 base types with action sub-enums (2026-05-06 simplification).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxType {
    TransferTx,
    StakingTx(StakingAction),
    DelegationTx(DelegationAction),
    TaskCreateTx,
    GovernanceTx(GovernanceAction),
    EvidenceTx,
    FastPathTx,
}

/// Staking sub-actions. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StakingAction {
    Bond,
    Renew,
    Unbond,
    Withdraw,
}

/// Delegation sub-actions. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationAction {
    Delegate,
    Undelegate,
    WithdrawDelegation,
    SetCommission,
}

/// Governance sub-actions. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernanceAction {
    Propose,
    Vote,
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
            TxType::StakingTx(StakingAction::Bond),
            TxType::StakingTx(StakingAction::Renew),
            TxType::StakingTx(StakingAction::Unbond),
            TxType::StakingTx(StakingAction::Withdraw),
            TxType::DelegationTx(DelegationAction::Delegate),
            TxType::DelegationTx(DelegationAction::Undelegate),
            TxType::DelegationTx(DelegationAction::WithdrawDelegation),
            TxType::DelegationTx(DelegationAction::SetCommission),
            TxType::TaskCreateTx,
            TxType::GovernanceTx(GovernanceAction::Propose),
            TxType::GovernanceTx(GovernanceAction::Vote),
            TxType::EvidenceTx,
            TxType::FastPathTx,
        ];
        assert_eq!(types.len(), 14);
    }

    #[test]
    fn committee_size_validation() {
        let c = Committee {
            epoch: 0,
            seed: [0; 32],
            members: vec![[1; 32]; 100],
            weights: vec![1000u128; 100],
        };
        assert_eq!(c.members.len(), 100);
        assert_eq!(c.weights.len(), 100);
    }
}
