// === C5 Fee Market: EIP-1559 Dynamic Fee ===
//
// Source: specs/protocol/fee-market-spec.md Sections 1-2

/// State of the EIP-1559 fee market, updated each block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeMarketState {
    pub base_fee: u128,
    pub fee_burn_accumulator: u128,
    pub min_fee_floor: u128,
}

/// Configuration parameters for the fee market.
/// Governance-adjustable within bounds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeeConfig {
    pub target_utilization_pct: u8,
    /// Max adjustment expressed as per-mil (125 = 12.5%, i.e. 125/1000).
    /// Named explicitly to avoid misconfiguration as a percentage.
    pub max_adjustment_per_mil: u16,
    pub min_base_fee: u128,
    pub max_per_sender_tx: u32,
}

impl Default for FeeConfig {
    fn default() -> Self {
        Self {
            target_utilization_pct: 50,
            max_adjustment_per_mil: 125, // 12.5% expressed as per-mil (125/1000)
            min_base_fee: 1_000_000u128, // [TUNE] 1_000_000 atto-AGX
            max_per_sender_tx: 100,
        }
    }
}

impl Default for FeeMarketState {
    fn default() -> Self {
        Self {
            base_fee: FeeConfig::default().min_base_fee,
            fee_burn_accumulator: 0,
            min_fee_floor: FeeConfig::default().min_base_fee,
        }
    }
}

/// Adjust base fee according to EIP-1559 algorithm.
///
/// Uses integer arithmetic for determinism across all platforms.
/// adjustment_denominator = 8 (smooth adjustment per spec)
pub fn compute_next_base_fee(
    current_base_fee: u128,
    block_utilization_pct: u8,
    config: &FeeConfig,
    adjustment_denominator: u128,
) -> u128 {
    let target = config.target_utilization_pct as u128;
    let utilization = block_utilization_pct as u128;

    if utilization > target {
        let denom = target * adjustment_denominator;
        let delta = current_base_fee
            .checked_mul(utilization.saturating_sub(target))
            .and_then(|v| v.checked_div(denom))
            .unwrap_or(u128::MAX);
        let cap = current_base_fee
            .checked_mul(config.max_adjustment_per_mil as u128)
            .map(|v| v / 1000)
            .unwrap_or(u128::MAX);
        let increased = current_base_fee.saturating_add(delta);
        std::cmp::min(increased, current_base_fee.saturating_add(cap))
    } else if utilization < target {
        let denom = target * adjustment_denominator;
        let delta = current_base_fee
            .checked_mul(target.saturating_sub(utilization))
            .and_then(|v| v.checked_div(denom))
            .unwrap_or(u128::MAX);
        let cap = current_base_fee
            .checked_mul(config.max_adjustment_per_mil as u128)
            .map(|v| v / 1000)
            .unwrap_or(u128::MAX);
        let decreased = current_base_fee.saturating_sub(delta);
        let floor_reduced = current_base_fee.saturating_sub(cap);
        #[allow(clippy::comparison_chain)]
        if decreased > floor_reduced {
            std::cmp::max(decreased, config.min_base_fee)
        } else if floor_reduced > config.min_base_fee {
            floor_reduced
        } else {
            config.min_base_fee
        }
    } else {
        current_base_fee
    }
}

/// Compute total transaction cost: (base_fee + priority_fee)
pub fn compute_tx_fee(base_fee: u128, priority_fee: u128) -> u128 {
    base_fee.saturating_add(priority_fee)
}

/// Determine if a transaction meets the minimum fee requirement.
pub fn tx_meets_min_fee(max_fee: u128, base_fee: u128) -> bool {
    max_fee >= base_fee
}

/// Compute the burn portion of a transaction fee.
pub fn compute_burn_amount(base_fee: u128, gas_used: u64) -> u128 {
    base_fee.saturating_mul(gas_used as u128)
}

impl FeeMarketState {
    pub fn accumulate_burn(&mut self, burn_amount: u128) {
        self.fee_burn_accumulator = self.fee_burn_accumulator.saturating_add(burn_amount);
    }
}

/// Compute validator rebate from total priority fees across epoch.
/// Proportional to validator's stake share of total bonded stake.
pub fn compute_validator_rebate(
    validator_stake: u128,
    total_bonded_stake: u128,
    total_priority_fees: u128,
) -> u128 {
    if total_bonded_stake == 0 || total_priority_fees == 0 {
        return 0;
    }
    total_priority_fees * validator_stake / total_bonded_stake
}

/// Check per-sender mempool transaction limit.
pub fn sender_within_mempool_limit(tx_count: u32, config: &FeeConfig) -> bool {
    tx_count <= config.max_per_sender_tx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> FeeConfig {
        FeeConfig::default()
    }

    #[test]
    fn base_fee_increases_when_above_target() {
        let config = default_config();
        let new_fee = compute_next_base_fee(100_000_000, 75, &config, 8);
        assert!(new_fee > 100_000_000, "fee should increase when above 50% target");
    }

    #[test]
    fn base_fee_decreases_when_below_target() {
        let config = default_config();
        let new_fee = compute_next_base_fee(100_000_000, 25, &config, 8);
        assert!(new_fee < 100_000_000, "fee should decrease when below 50% target");
    }

    #[test]
    fn base_fee_unchanged_at_target() {
        let config = default_config();
        let new_fee = compute_next_base_fee(100_000_000, 50, &config, 8);
        assert_eq!(new_fee, 100_000_000);
    }

    #[test]
    fn base_fee_never_exceeds_12_5_percent() {
        let config = default_config();
        let new_fee = compute_next_base_fee(100_000_000, 100, &config, 8);
        let max_allowed = 100_000_000 + (100_000_000 * 125 / 1000);
        assert!(new_fee <= max_allowed, "increase should be capped at 12.5%");
    }

    #[test]
    fn base_fee_never_drops_below_12_5_percent_max() {
        let config = default_config();
        let new_fee = compute_next_base_fee(100_000_000, 0, &config, 8);
        let min_allowed = 100_000_000 - (100_000_000 * 125 / 1000);
        assert!(new_fee >= min_allowed, "decrease should be capped at 12.5%");
    }

    #[test]
    fn base_fee_respects_min_floor() {
        let config = default_config();
        let new_fee = compute_next_base_fee(1_000_000, 0, &config, 8);
        assert!(new_fee >= config.min_base_fee, "fee should never drop below min floor");
    }

    #[test]
    fn tx_rejected_if_max_fee_below_base() {
        assert!(!tx_meets_min_fee(100, 1000));
        assert!(tx_meets_min_fee(1000, 1000));
        assert!(tx_meets_min_fee(2000, 1000));
    }

    #[test]
    fn burn_computed_correctly() {
        assert_eq!(compute_burn_amount(500, 1), 500);
        assert_eq!(compute_burn_amount(100, 3), 300);
        assert_eq!(compute_burn_amount(500, 0), 0);
    }

    #[test]
    fn burn_accumulator_works() {
        let mut state = FeeMarketState::default();
        assert_eq!(state.fee_burn_accumulator, 0);
        state.accumulate_burn(1000);
        assert_eq!(state.fee_burn_accumulator, 1000);
        state.accumulate_burn(500);
        assert_eq!(state.fee_burn_accumulator, 1500);
    }

    #[test]
    fn rebate_proportional_to_stake() {
        let rebate = compute_validator_rebate(3000, 10000, 1000);
        assert_eq!(rebate, 300);

        let rebate2 = compute_validator_rebate(1000, 10000, 1000);
        assert_eq!(rebate2, 100);
    }

    #[test]
    fn rebate_zero_for_zero_stake() {
        assert_eq!(compute_validator_rebate(0, 10000, 1000), 0);
        assert_eq!(compute_validator_rebate(1000, 0, 1000), 0);
        assert_eq!(compute_validator_rebate(1000, 10000, 0), 0);
    }

    #[test]
    fn sender_limit_enforced() {
        let config = default_config();
        assert!(sender_within_mempool_limit(100, &config));
        assert!(!sender_within_mempool_limit(101, &config));
    }

    #[test]
    fn total_tx_fee_correct() {
        assert_eq!(compute_tx_fee(100, 50), 150);
        assert_eq!(compute_tx_fee(0, 50), 50);
        assert_eq!(compute_tx_fee(100, 0), 100);
    }

    #[test]
    fn fee_adjustment_deterministic() {
        let config = default_config();
        let r1 = compute_next_base_fee(100_000_000, 75, &config, 8);
        let r2 = compute_next_base_fee(100_000_000, 75, &config, 8);
        assert_eq!(r1, r2);
    }

    #[test]
    fn fee_adjustment_changes_with_different_utilization() {
        let config = default_config();
        let r1 = compute_next_base_fee(100_000_000, 75, &config, 8);
        let r2 = compute_next_base_fee(100_000_000, 80, &config, 8);
        assert_ne!(r1, r2);
    }
}
