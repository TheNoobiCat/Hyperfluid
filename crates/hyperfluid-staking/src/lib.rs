// === C3 Staking & Validator Manager: Core Types ===
//
// Source: specs/protocol/staking-spec.md Sections 1-2

use serde::{Deserialize, Serialize};

pub type Hash32 = [u8; 32];

/// Four canonical validator states. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorState {
    Active,
    Paused,
    Unbonding,
    Withdrawn,
}

/// Validator record stored on-chain. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidatorRecord {
    pub validator_id: Hash32,
    pub state: ValidatorState,
    pub bonded_stake: u128,
    pub bonding_height: u64,
    pub unbonding_height: u64,
    pub jail_until_height: u64,
    // SPEC_DEVIATION: liveness_bitmap is Vec<u8> instead of [u8; 1024]
    // because serde does not natively derive Serialize/Deserialize for arrays > 32 elements.
    // When SCALE encoding is added in Stage 01, this MUST become [u8; 1024] per spec.
    pub liveness_bitmap: Vec<u8>,
    pub slash_count: u32,
    pub missed_blocks: u32,
    pub last_renew_height: u64,
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
    pub min_stake: u128,
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
            min_stake: 1_000_000_000_000_000_000_000u128,
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
            bonded_stake: 1_000_000_000_000_000_000_000u128,
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
    fn system_parameters_defaults_match_spec() {
        let p = SystemParameters::default();
        assert_eq!(p.epoch_length, 8192);
        assert_eq!(p.committee_size, 100);
        assert_eq!(p.min_stake, 1_000_000_000_000_000_000_000u128);
        assert_eq!(p.proposal_deposit, 500_000_000_000_000_000_000u128);
        assert_eq!(p.unbond_delay, 120_960);
        assert_eq!(p.liveness_miss_threshold_pct, 20);
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
