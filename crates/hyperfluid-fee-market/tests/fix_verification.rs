// Test names follow fix_F{N}_{short_description} convention per instructions.
#![allow(non_snake_case)]

// === F-58 through F-62: Production-readiness verification ===
//
// Verifies that ALL five fee-market functions previously gated behind
// `#[allow(dead_code)]` are:
//   1. Exercised with positive assertions (nominal behaviour)
//   2. Exercised with negative assertions (edge/failure behaviour)
//   3. Ready for consensus-driver wiring (no signature changes)
//
// Source: specs/protocol/fee-market-spec.md Sections 1-2

use hyperfluid_fee_market::{
    compute_burn_amount, compute_tx_fee, compute_validator_rebate, sender_within_mempool_limit,
    tx_meets_min_fee, FeeConfig, FeeMarketState,
};

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn default_config() -> FeeConfig {
    FeeConfig::default()
}

// ============================================================================
// F-58: compute_tx_fee
// ============================================================================

/// Positive: nominal addition of base fee and priority fee.
#[test]
fn fix_F58_compute_tx_fee_nominal() {
    let total = compute_tx_fee(1_000_000, 500_000);
    assert_eq!(total, 1_500_000, "base_fee + priority_fee should sum");
}

/// Positive: zero priority fee still works (base fee only).
#[test]
fn fix_F58_compute_tx_fee_zero_priority() {
    let total = compute_tx_fee(1_000_000, 0);
    assert_eq!(total, 1_000_000, "zero priority fee should yield base fee alone");
}

/// Positive: zero base fee still works (priority fee only).
#[test]
fn fix_F58_compute_tx_fee_zero_base() {
    let total = compute_tx_fee(0, 500_000);
    assert_eq!(total, 500_000, "zero base fee should yield priority fee alone");
}

/// Negative: saturating addition does not panic on overflow.
#[test]
fn fix_F58_compute_tx_fee_saturating_overflow() {
    let total = compute_tx_fee(u128::MAX, 1);
    assert_eq!(total, u128::MAX, "overflow should saturate to u128::MAX");
}

/// Negative: both fees at max does not panic.
#[test]
fn fix_F58_compute_tx_fee_both_max() {
    let total = compute_tx_fee(u128::MAX, u128::MAX);
    assert_eq!(total, u128::MAX, "two MAX values should saturate to u128::MAX");
}

// ============================================================================
// F-59: tx_meets_min_fee
// ============================================================================

/// Positive: max fee equal to base fee — meets minimum.
#[test]
fn fix_F59_tx_meets_min_fee_equal() {
    assert!(tx_meets_min_fee(1_000_000, 1_000_000), "equal fees should meet minimum");
}

/// Positive: max fee well above base fee — meets minimum.
#[test]
fn fix_F59_tx_meets_min_fee_above() {
    assert!(tx_meets_min_fee(10_000_000, 1_000_000), "higher max fee should meet minimum");
}

/// Positive: zero base fee always passes.
#[test]
fn fix_F59_tx_meets_min_fee_zero_base() {
    assert!(tx_meets_min_fee(0, 0), "zero max with zero base should meet minimum");
}

/// Negative: max fee below base fee — rejected.
#[test]
fn fix_F59_tx_meets_min_fee_below() {
    assert!(!tx_meets_min_fee(500, 1_000), "max fee below base should be rejected");
}

/// Negative: max fee is zero, positive base fee — rejected.
#[test]
fn fix_F59_tx_meets_min_fee_zero_max() {
    assert!(!tx_meets_min_fee(0, 1_000_000), "zero max with positive base should be rejected");
}

// ============================================================================
// F-60: compute_burn_amount
// ============================================================================

/// Positive: single unit of gas.
#[test]
fn fix_F60_compute_burn_amount_single_gas() {
    let burn = compute_burn_amount(1_000_000, 1);
    assert_eq!(burn, 1_000_000, "1 gas at base_fee = base_fee");
}

/// Positive: multiple gas units.
#[test]
fn fix_F60_compute_burn_amount_multiple_gas() {
    let burn = compute_burn_amount(1_000_000, 10);
    assert_eq!(burn, 10_000_000, "10 gas at base_fee = 10 * base_fee");
}

/// Positive: large values.
#[test]
fn fix_F60_compute_burn_amount_large() {
    let burn = compute_burn_amount(1_000_000_000_000_000_000, 1_000_000);
    assert_eq!(
        burn, 1_000_000_000_000_000_000_000_000u128,
        "large base_fee * large gas should compute correctly"
    );
}

/// Negative: zero gas used — zero burn.
#[test]
fn fix_F60_compute_burn_amount_zero_gas() {
    let burn = compute_burn_amount(1_000_000, 0);
    assert_eq!(burn, 0, "zero gas should produce zero burn");
}

/// Negative: zero base fee — zero burn regardless of gas.
#[test]
fn fix_F60_compute_burn_amount_zero_base() {
    let burn = compute_burn_amount(0, 1_000_000);
    assert_eq!(burn, 0, "zero base fee should produce zero burn");
}

/// Negative: overflow saturates.
#[test]
fn fix_F60_compute_burn_amount_overflow() {
    let burn = compute_burn_amount(u128::MAX, 2);
    assert_eq!(burn, u128::MAX, "overflow should saturate to u128::MAX");
}

// ============================================================================
// F-61: compute_validator_rebate
// ============================================================================

/// Positive: proportional to stake — 30% stake gets 30% of fees.
#[test]
fn fix_F61_rebate_proportional() {
    let rebate = compute_validator_rebate(3_000, 10_000, 1_000);
    assert_eq!(rebate, 300, "30% stake should receive 30% of fees");
}

/// Positive: full stake gets all fees.
#[test]
fn fix_F61_rebate_full_stake() {
    let rebate = compute_validator_rebate(10_000, 10_000, 1_000);
    assert_eq!(rebate, 1_000, "100% stake should receive 100% of fees");
}

/// Positive: zero stake share — truncation is fine.
#[test]
fn fix_F61_rebate_small_stake() {
    // 1 / 10_000 * 1_000 = 0 due to integer truncation
    let rebate = compute_validator_rebate(1, 10_000, 1_000);
    assert_eq!(rebate, 0, "tiny stake truncates to zero rebate");
}

/// Negative: zero validator stake.
#[test]
fn fix_F61_rebate_zero_validator_stake() {
    assert_eq!(compute_validator_rebate(0, 10_000, 1_000), 0);
}

/// Negative: zero total bonded stake.
#[test]
fn fix_F61_rebate_zero_total_stake() {
    assert_eq!(compute_validator_rebate(1_000, 0, 1_000), 0);
}

/// Negative: zero priority fees.
#[test]
fn fix_F61_rebate_zero_priority_fees() {
    assert_eq!(compute_validator_rebate(1_000, 10_000, 0), 0);
}

/// Negative: both stake and fees zero.
#[test]
fn fix_F61_rebate_all_zeros() {
    assert_eq!(compute_validator_rebate(0, 0, 0), 0);
}

// ============================================================================
// F-62: sender_within_mempool_limit
// ============================================================================

/// Positive: exactly at the limit boundary.
#[test]
fn fix_F62_sender_at_limit() {
    let config = FeeConfig { max_per_sender_tx: 100, ..default_config() };
    assert!(sender_within_mempool_limit(100, &config), "at limit should be allowed");
}

/// Positive: well below limit.
#[test]
fn fix_F62_sender_below_limit() {
    let config = FeeConfig { max_per_sender_tx: 100, ..default_config() };
    assert!(sender_within_mempool_limit(50, &config), "below limit should be allowed");
}

/// Positive: zero tx count always allowed.
#[test]
fn fix_F62_sender_zero_count() {
    let config = FeeConfig { max_per_sender_tx: 100, ..default_config() };
    assert!(sender_within_mempool_limit(0, &config), "zero tx count should be allowed");
}

/// Negative: one over the limit.
#[test]
fn fix_F62_sender_one_over_limit() {
    let config = FeeConfig { max_per_sender_tx: 100, ..default_config() };
    assert!(!sender_within_mempool_limit(101, &config), "one over limit should be rejected");
}

/// Negative: far over the limit.
#[test]
fn fix_F62_sender_far_over_limit() {
    let config = FeeConfig { max_per_sender_tx: 100, ..default_config() };
    assert!(!sender_within_mempool_limit(999, &config), "far over limit should be rejected");
}

/// Negative: limit of zero (mempool disabled for sender).
#[test]
fn fix_F62_sender_zero_limit() {
    let config = FeeConfig { max_per_sender_tx: 0, ..default_config() };
    assert!(!sender_within_mempool_limit(1, &config), "any tx should be rejected when limit is 0");
}

// ============================================================================
// Integration: FeeMarketState.accumulate_burn + compute_burn_amount
// ============================================================================

/// Positive: integration of burn computation and accumulator.
#[test]
fn fix_F60_burn_accumulate_integration() {
    let mut state = FeeMarketState::default();
    let base_fee = 1_000_000u128;
    let gas_used = 10u64;

    let burn = compute_burn_amount(base_fee, gas_used);
    assert_eq!(burn, 10_000_000, "burn computed correctly");

    state.accumulate_burn(burn);
    assert_eq!(state.fee_burn_accumulator, 10_000_000, "accumulator updated");

    // Second block
    let burn2 = compute_burn_amount(base_fee, 5);
    state.accumulate_burn(burn2);
    assert_eq!(
        state.fee_burn_accumulator,
        10_000_000 + 5_000_000,
        "accumulator should sum across blocks"
    );
}
