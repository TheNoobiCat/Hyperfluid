// === C3 Staking & Validator Manager: Core Types ===
//
// Source: specs/protocol/staking-spec.md Sections 1-2

use serde::{Deserialize, Serialize};

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
}
