// Stage 01 Week 7-8: End-to-End Integration Test
//
// Tests the full protocol core lifecycle:
//   1. Genesis bootstrap (accounts, validators, system state)
//   2. Delegation (delegate → wait → withdraw)
//   3. Transfers (account-to-account value movement)
//   4. Fee market adjustment (EIP-1559 compute_next_base_fee)
//   5. Task creation with bounties
//   6. State root determinism across the lifecycle
//   7. Undelegate + withdraw full cycle
//   8. Commission rate enforcement
//
// Source: docs/05-planning/stages/stage-01-protocol-core.md Week 7-8

use hyperfluid_consensus::types::Hash32;
use hyperfluid_fee_market::{compute_next_base_fee, compute_tx_fee, FeeConfig, FeeMarketState};
use hyperfluid_state::state_machine::{ExecutionContext, ExecutionResult, StateMachine};
use hyperfluid_state::Account;

fn new_account(id: u8, balance: u128, nonce: u64) -> Account {
    use hyperfluid_state::sha3_256;
    Account {
        account_id: [id; 32],
        balance,
        nonce,
        pubkey_hash: sha3_256(&[id]),
        pubkey: Some(vec![id; 64]),
    }
}

fn ctx(height: u64) -> ExecutionContext {
    ExecutionContext { height, timestamp: height * 10_000 }
}

// ─── 1. Genesis Bootstrap ───────────────────────────────────

/// Boot: create genesis accounts and verify initial state.
#[test]
fn e2e_01_genesis_bootstrap() {
    let mut sm = StateMachine::new();

    // Create genesis accounts: node operator, 5 validators, 3 users
    sm.init_account(new_account(1, 1_000_000_000_000_000_000_000, 0));
    for v in 2u8..=6 {
        sm.init_account(new_account(v, 100_000_000_000_000_000_000, 0));
    }
    for u in 7u8..=9 {
        sm.init_account(new_account(u, 10_000_000_000_000_000_000, 0));
    }

    assert_eq!(sm.account_count(), 9);
    assert_eq!(sm.get_account(&[1u8; 32]).unwrap().balance, 1_000_000_000_000_000_000_000);

    let root = sm.compute_state_root();
    assert_ne!(root, [0u8; 32], "genesis state root must be non-zero");
}

// ─── 2. Delegation Full Lifecycle ───────────────────────────

/// Delegate → verify balance deduction → undelegate → wait delay → withdraw.
#[test]
fn e2e_02_delegation_full_lifecycle() {
    let mut sm = StateMachine::new();
    let delegator = [7u8; 32];
    let validator = [3u8; 32];
    let min_delegation: u128 = 1_000_000_000_000_000_000; // 1 AGX

    sm.init_account(new_account(7, 10_000_000_000_000_000_000, 0));
    sm.init_account(new_account(3, 100_000_000_000_000_000_000, 0));
    sm.init_validator(validator, 1_000_000_000_000_000_000_000, 0);

    // Delegate 5 AGX
    let r = sm.execute_delegate(
        delegator,
        validator,
        5_000_000_000_000_000_000,
        1,
        min_delegation,
        ctx(1),
    );
    assert_eq!(r, ExecutionResult::Success);
    assert_eq!(sm.get_account(&delegator).unwrap().balance, 5_000_000_000_000_000_000);

    // Undelegate at height 100
    let r = sm.execute_undelegate(delegator, validator, 2, 100, ctx(100));
    assert_eq!(r, ExecutionResult::Success);

    // Withdraw before unbonding delay (7 days = 60480 blocks at 10s)
    let unbond_delay: u64 = 60480;
    let r = sm.execute_withdraw_delegation(delegator, validator, 3, 101, unbond_delay, ctx(101));
    assert_eq!(r, ExecutionResult::Rejected, "must reject withdraw before delay expires");

    // Withdraw after delay
    let r = sm.execute_withdraw_delegation(
        delegator,
        validator,
        3,
        100 + unbond_delay + 1,
        unbond_delay,
        ctx(100 + unbond_delay + 1),
    );
    assert_eq!(r, ExecutionResult::Success);
    assert_eq!(sm.get_account(&delegator).unwrap().balance, 10_000_000_000_000_000_000);
}

/// Delegate below minimum is rejected.
#[test]
fn e2e_03_delegate_below_minimum() {
    let mut sm = StateMachine::new();
    sm.init_account(new_account(7, 10_000_000_000_000_000_000, 0));
    sm.init_account(new_account(3, 100_000_000_000_000_000_000, 0));

    let min: u128 = 1_000_000_000_000_000_000; // 1 AGX min
    let r = sm.execute_delegate([7u8; 32], [3u8; 32], 500_000_000_000_000_000, 1, min, ctx(1));
    assert_eq!(r, ExecutionResult::Rejected);
}

// ─── 3. Transfers Between Accounts ───────────────────────────

/// Transfer → verify balances → verify nonces increment.
#[test]
fn e2e_04_transfer_flow() {
    let mut sm = StateMachine::new();
    sm.init_account(new_account(8, 10_000_000_000_000_000_000, 0));
    sm.init_account(new_account(9, 1_000_000_000_000_000_000, 0));

    // Transfer 3 AGX
    let r = sm.execute_transfer([8u8; 32], [9u8; 32], 3_000_000_000_000_000_000, 1, ctx(10));
    assert_eq!(r, ExecutionResult::Success);
    assert_eq!(sm.get_account(&[8u8; 32]).unwrap().balance, 7_000_000_000_000_000_000);
    assert_eq!(sm.get_account(&[9u8; 32]).unwrap().balance, 4_000_000_000_000_000_000);
    assert_eq!(sm.get_account(&[8u8; 32]).unwrap().nonce, 1);

    // Second transfer (nonce = 2)
    let r = sm.execute_transfer([8u8; 32], [9u8; 32], 1_000_000_000_000_000_000, 2, ctx(11));
    assert_eq!(r, ExecutionResult::Success);
    assert_eq!(sm.get_account(&[8u8; 32]).unwrap().balance, 6_000_000_000_000_000_000);
    assert_eq!(sm.get_account(&[9u8; 32]).unwrap().balance, 5_000_000_000_000_000_000);
}

/// Zero amount transfer rejected.
#[test]
fn e2e_05_transfer_zero_rejected() {
    let mut sm = StateMachine::new();
    sm.init_account(new_account(8, 1_000_000_000, 0));
    let r = sm.execute_transfer([8u8; 32], [9u8; 32], 0, 1, ctx(1));
    assert_eq!(r, ExecutionResult::Rejected);
}

/// Insufficient balance rejected.
#[test]
fn e2e_06_transfer_insufficient_balance() {
    let mut sm = StateMachine::new();
    sm.init_account(new_account(8, 100, 0));
    let r = sm.execute_transfer([8u8; 32], [9u8; 32], 101, 1, ctx(1));
    assert_eq!(r, ExecutionResult::Rejected);
}

// ─── 4. Fee Market Adjustment ────────────────────────────────

/// EIP-1559 base fee adjusts correctly across block sequence.
#[test]
fn e2e_07_fee_market_adjustment() {
    let config = FeeConfig::default();
    let mut state = FeeMarketState::default();

    // Sequence of blocks with varying utilization
    let utilizations = [80, 70, 60, 50, 40, 30, 20, 90, 90, 90];
    let mut prev_fee = state.base_fee;

    for util in &utilizations {
        let next = compute_next_base_fee(prev_fee, *util, &config, 8);
        if *util > config.target_utilization_pct {
            assert!(next >= prev_fee, "fee must increase or stay same when above target");
        } else if *util < config.target_utilization_pct {
            assert!(next <= prev_fee, "fee must decrease or stay same when below target");
        }
        prev_fee = next;
    }

    state.base_fee = prev_fee;
    assert!(state.base_fee >= config.min_base_fee);
}

/// Compute TX fee = base_fee + priority_fee.
#[test]
fn e2e_08_total_tx_fee_computation() {
    let fee = compute_tx_fee(100_000_000, 25_000_000);
    assert_eq!(fee, 125_000_000);
}

// ─── 5. Task Creation ────────────────────────────────────────

/// Create a task with bounty and fee, verify creator is debited.
#[test]
fn e2e_09_task_create_flow() {
    let mut sm = StateMachine::new();
    sm.init_account(new_account(8, 10_000_000_000_000_000_000, 0));

    let task_id: Hash32 = [0xAA; 32];
    let seed_ref: Hash32 = [0xBB; 32];

    let r = sm.execute_task_create(
        [8u8; 32],
        2_000_000_000_000_000_000,
        500_000_000_000_000_000,
        task_id,
        1,
        seed_ref,
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        20,
        ctx(20),
    );
    assert_eq!(r, ExecutionResult::Success);
    assert_eq!(sm.get_account(&[8u8; 32]).unwrap().balance, 7_500_000_000_000_000_000);
}

/// Duplicate task creation rejected.
#[test]
fn e2e_10_task_create_duplicate_rejected() {
    let mut sm = StateMachine::new();
    sm.init_account(new_account(8, 10_000_000_000_000_000_000, 0));

    let task_id: Hash32 = [0xCC; 32];
    let seed: Hash32 = [0xDD; 32];

    let r1 = sm.execute_task_create(
        [8u8; 32],
        1_000_000,
        100_000,
        task_id,
        1,
        seed,
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        1,
        ctx(1),
    );
    assert_eq!(r1, ExecutionResult::Success);

    sm.init_account(new_account(8, 10_000_000_000_000_000_000, 1));
    let r2 = sm.execute_task_create(
        [8u8; 32],
        1_000_000,
        100_000,
        task_id,
        2,
        seed,
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        2,
        ctx(2),
    );
    assert_eq!(r2, ExecutionResult::Rejected);
}

// ─── 6. State Root Determinism Across Lifecycle ──────────────

/// State root must be deterministic after identical operations.
#[test]
fn e2e_11_state_root_deterministic() {
    let mut sm1 = StateMachine::new();
    let mut sm2 = StateMachine::new();

    sm1.init_account(new_account(1, 1_000_000_000, 0));
    sm2.init_account(new_account(1, 1_000_000_000, 0));

    assert_eq!(sm1.compute_state_root(), sm2.compute_state_root());

    let r1 = sm1.execute_transfer([1u8; 32], [2u8; 32], 500_000_000, 1, ctx(10));
    let r2 = sm2.execute_transfer([1u8; 32], [2u8; 32], 500_000_000, 1, ctx(10));

    assert_eq!(r1, ExecutionResult::Success);
    assert_eq!(r2, ExecutionResult::Success);
    assert_eq!(sm1.compute_state_root(), sm2.compute_state_root());
}

// ─── 7. Undelegate + Withdraw Full Cycle ─────────────────────

/// Undelegate marks delegation inactive, withdraw after delay restores funds.
#[test]
fn e2e_12_undelegate_withdraw_cycle() {
    let mut sm = StateMachine::new();
    let delegator = [7u8; 32];
    let validator = [3u8; 32];
    let min: u128 = 1_000_000;

    sm.init_account(new_account(7, 10_000_000_000, 0));
    sm.init_account(new_account(3, 100_000_000_000, 0));
    sm.init_validator(validator, 1_000_000_000_000_000_000_000, 0);

    let initial_balance = sm.get_account(&delegator).unwrap().balance;

    let r = sm.execute_delegate(delegator, validator, 5_000_000_000, 1, min, ctx(10));
    assert_eq!(r, ExecutionResult::Success);

    // Undelegate
    let r = sm.execute_undelegate(delegator, validator, 2, 100, ctx(100));
    assert_eq!(r, ExecutionResult::Success);

    // Undelegate again (should reject — already inactive)
    let r = sm.execute_undelegate(delegator, validator, 3, 101, ctx(101));
    assert_eq!(r, ExecutionResult::Rejected);

    // Withdraw after delay
    let delay: u64 = 60480;
    let r = sm.execute_withdraw_delegation(
        delegator,
        validator,
        3,
        100 + delay + 1,
        delay,
        ctx(100 + delay + 1),
    );
    assert_eq!(r, ExecutionResult::Success);
    assert_eq!(sm.get_account(&delegator).unwrap().balance, initial_balance);
}

// ─── 8. Commission Rate ──────────────────────────────────────

/// Set commission within range succeeds, exceeding max fails.
#[test]
fn e2e_13_commission_rate_constraints() {
    let mut sm = StateMachine::new();
    let v = [3u8; 32];
    sm.init_account(new_account(3, 100_000_000_000, 0));
    sm.init_validator(v, 100_000_000_000_000_000_000u128, 1);

    let max: u8 = 20;
    let r = sm.execute_set_commission(v, 10, 1, max, ctx(1));
    assert_eq!(r, ExecutionResult::Success);

    let r = sm.execute_set_commission(v, 25, 2, max, ctx(2));
    assert_eq!(r, ExecutionResult::Rejected);
}

// ─── 9. Multi-Account Transfer → State Root Consistency ────

/// Execute multiple operations and verify state root consistency.
#[test]
fn e2e_15_multi_op_state_consistency() {
    let mut sm = StateMachine::new();

    // Genesis
    sm.init_account(new_account(1, 1_000_000_000_000_000_000_000, 0));
    sm.init_account(new_account(2, 500_000_000_000_000_000_000, 0));
    sm.init_account(new_account(3, 500_000_000_000_000_000_000, 0));
    sm.init_account(new_account(4, 10_000_000_000_000_000_000, 0));
    sm.init_account(new_account(5, 10_000_000_000_000_000_000, 0));
    sm.init_validator([3u8; 32], 1_000_000_000_000_000_000_000, 0);

    let root_genesis = sm.compute_state_root();
    assert_ne!(root_genesis, [0u8; 32]);
    assert_eq!(sm.account_count(), 5);

    // Transfer 1 → 4 — root must change
    sm.execute_transfer([1u8; 32], [4u8; 32], 50_000_000_000_000_000_000, 1, ctx(1));
    let root_after_transfer = sm.compute_state_root();
    assert_ne!(root_after_transfer, root_genesis);

    // Task create — root must change
    sm.execute_task_create(
        [2u8; 32],
        10_000_000_000_000_000_000,
        1_000_000_000_000_000_000,
        [0xA1; 32],
        1,
        [0xB1; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        2,
        ctx(2),
    );
    let root_after_task = sm.compute_state_root();
    assert_ne!(root_after_task, root_after_transfer);

    // Delegate — must succeed (account 4 nonce was 0 after being recipient)
    let r =
        sm.execute_delegate([4u8; 32], [3u8; 32], 10_000_000_000_000_000_000, 1, 1_000_000, ctx(3));
    assert_eq!(r, ExecutionResult::Success);
    assert!(sm.get_account(&[4u8; 32]).unwrap().balance > 0);

    let root_final = sm.compute_state_root();
    assert_ne!(root_final, [0u8; 32]);
}

// ─── 11. Replay Protection ───────────────────────────────────

/// Action plan replay protection: duplicate plan_id rejected.
#[test]
fn e2e_16_replay_protection() {
    let mut sm = StateMachine::new();
    let plan_id: Hash32 = [0xFE; 32];

    assert_eq!(sm.consume_plan_id(plan_id, ctx(1)), ExecutionResult::Success);
    assert_eq!(sm.consume_plan_id(plan_id, ctx(2)), ExecutionResult::Rejected);
}

// ─── 12. Nonce Enforcement Across Operations ─────────────────

/// Nonce must monotonically increase.
#[test]
fn e2e_17_nonce_enforcement() {
    let mut sm = StateMachine::new();
    sm.init_account(new_account(5, 1_000_000_000, 0));

    // Correct nonce = 1
    assert_eq!(sm.execute_transfer([5u8; 32], [1u8; 32], 100, 1, ctx(1)), ExecutionResult::Success);
    // Now nonce is 1, so next valid is 2
    assert_eq!(
        sm.execute_transfer([5u8; 32], [1u8; 32], 100, 3, ctx(2)),
        ExecutionResult::Rejected
    );
    assert_eq!(sm.execute_transfer([5u8; 32], [1u8; 32], 100, 2, ctx(2)), ExecutionResult::Success);
}
