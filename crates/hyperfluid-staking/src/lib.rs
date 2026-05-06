// === C3 Staking & Validator Manager: Core Types ===
//
// Source: specs/protocol/staking-spec.md Sections 1-2

pub mod graph;

use serde::{Deserialize, Serialize};

pub type Hash32 = [u8; 32];

pub fn sha3_256(data: &[u8]) -> Hash32 {
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Four canonical validator states. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorState {
    Active,
    Paused,
    Unbonding,
    Withdrawn,
}

/// Delegation status. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DelegationStatus {
    Active,
    Unbonding,
    Withdrawn,
}

/// Validator record stored on-chain. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorRecord {
    pub validator_id: Hash32,
    pub state: ValidatorState,
    pub self_bond: u128,
    pub total_delegated: u128,
    pub bonded_stake: u128,
    // SPEC_DEVIATION: delegated_stake_balance computed as self_bond + total_delegated for
    // backward compatibility during transition. After full delegation migration,
    // bonded_stake should equal self_bond + total_delegated.
    pub commission_rate: u8,
    pub bonding_height: u64,
    pub unbonding_height: u64,
    pub jail_until_height: u64,
    // SPEC_DEVIATION: liveness_bitmap is Vec<u8> instead of [u8; 1024]
    pub liveness_bitmap: Vec<u8>,
    pub slash_count: u32,
    pub missed_blocks: u32,
    pub last_renew_height: u64,
}

/// Delegation record stored on-chain. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationRecord {
    pub delegator_id: Hash32,
    pub validator_id: Hash32,
    pub amount: u128,
    pub unbonding_at_height: u64,
    pub status: DelegationStatus,
}

/// Fault classification for slashing. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FaultType {
    Equivocation,
    LivenessFailure,
    Other,
}

/// Slash record for on-chain evidence. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlashRecord {
    pub slash_id: Hash32,
    pub validator_id: Hash32,
    pub fault_type: FaultType,
    pub slash_amount: u128,
    pub slash_height: u64,
    pub evidence_ref: Hash32,
}

/// Governance vote. Source: staking-spec.md Section 2.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteOption {
    Yes,
    No,
}

/// Governance vote transaction. Source: staking-spec.md Section 2.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceVoteTx {
    pub proposal_id: Hash32,
    pub voter_id: Hash32,
    pub vote: VoteOption,
    pub reason_hash: Hash32,
    pub vote_weight: u128,
    pub signature: Vec<u8>,
}

/// Protocol system parameters. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SystemParameters {
    pub epoch_length: u64,
    pub committee_size: u64,
    pub min_self_bond: u128,
    pub min_delegation: u128,
    pub max_commission_rate: u8,
    pub delegation_unbond_delay: u64,
    pub bond_delay: u64,
    pub unbond_delay: u64,
    pub max_governance_proposals: u64,
    pub proposal_deposit: u128,
    pub liveness_window_blocks: u64,
    pub liveness_miss_threshold_pct: u8,
}

impl Default for SystemParameters {
    fn default() -> Self {
        Self {
            epoch_length: 8192,
            committee_size: 100,
            // 1,000 AGX = 1_000_000_000_000_000_000_000 atto-AGX (10^21)
            min_self_bond: 1_000_000_000_000_000_000_000u128,
            // 1 AGX = 10^18 atto-AGX
            min_delegation: 1_000_000_000_000_000_000u128,
            max_commission_rate: 20,
            delegation_unbond_delay: 60_480,
            bond_delay: 8640,
            unbond_delay: 120_960,
            max_governance_proposals: 32,
            // 500 AGX = 500_000_000_000_000_000_000 atto-AGX (5 * 10^20)
            proposal_deposit: 500_000_000_000_000_000_000u128,
            liveness_window_blocks: 8192,
            liveness_miss_threshold_pct: 20,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validator_state_has_four_variants() {
        let states = [
            ValidatorState::Active,
            ValidatorState::Paused,
            ValidatorState::Unbonding,
            ValidatorState::Withdrawn,
        ];
        assert_eq!(states.len(), 4);
    }

    #[test]
    fn validator_record_serde_roundtrip() {
        let r = ValidatorRecord {
            validator_id: [0xAA; 32],
            state: ValidatorState::Active,
            self_bond: 1_000_000_000_000_000_000_000u128,
            total_delegated: 0,
            bonded_stake: 1_000_000_000_000_000_000_000u128,
            commission_rate: 10,
            bonding_height: 0,
            unbonding_height: 0,
            jail_until_height: 0,
            liveness_bitmap: vec![0; 1024],
            slash_count: 0,
            missed_blocks: 0,
            last_renew_height: 0,
        };
        let json = serde_json::to_string(&r).unwrap();
        let r2: ValidatorRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(r, r2);
    }

    #[test]
    fn delegation_record_serde_roundtrip() {
        let d = DelegationRecord {
            delegator_id: [0x11; 32],
            validator_id: [0x22; 32],
            amount: 500_000_000_000_000_000_000u128,
            unbonding_at_height: 1000,
            status: DelegationStatus::Active,
        };
        let json = serde_json::to_string(&d).unwrap();
        let d2: DelegationRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(d, d2);
    }

    #[test]
    fn system_parameters_defaults_match_spec() {
        let p = SystemParameters::default();
        assert_eq!(p.epoch_length, 8192);
        assert_eq!(p.committee_size, 100);
        assert_eq!(p.min_self_bond, 1_000_000_000_000_000_000_000u128);
        assert_eq!(p.proposal_deposit, 500_000_000_000_000_000_000u128);
        assert_eq!(p.unbond_delay, 120_960);
        assert_eq!(p.liveness_miss_threshold_pct, 20);
    }

    #[test]
    fn system_parameters_delegation_defaults() {
        let p = SystemParameters::default();
        assert_eq!(p.min_delegation, 1_000_000_000_000_000_000u128);
        assert_eq!(p.max_commission_rate, 20);
        assert_eq!(p.delegation_unbond_delay, 60_480);
    }

    #[test]
    fn governance_vote_tx_roundtrip() {
        let tx = GovernanceVoteTx {
            proposal_id: [1; 32],
            voter_id: [2; 32],
            vote: VoteOption::Yes,
            reason_hash: [3; 32],
            vote_weight: 1_000_000_000_000_000_000_000u128,
            signature: vec![4; 64],
        };
        let json = serde_json::to_string(&tx).unwrap();
        let tx2: GovernanceVoteTx = serde_json::from_str(&json).unwrap();
        assert_eq!(tx, tx2);
    }
}
