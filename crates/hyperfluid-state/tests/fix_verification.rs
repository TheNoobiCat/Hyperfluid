// Verification tests for production-readiness fixes (F-22 through F-82).
// Each fix has at minimum 1 positive + 1 negative test.
// Source: crates/hyperfluid-state/src/state_machine.rs, state_sync.rs

use hyperfluid_state::state_machine::{ExecutionContext, ExecutionResult, StateMachine};
use hyperfluid_state::state_sync::{
    build_smt_from_keys, compute_state_checksum, snapshot_state, verify_snapshot_checksum,
};
use hyperfluid_state::{sha3_256, Account};

fn ctx(h: u64) -> ExecutionContext {
    ExecutionContext { height: h, timestamp: 0 }
}

fn test_account_with_pubkey(id: u8, balance: u128, nonce: u64) -> Account {
    Account {
        account_id: [id; 32],
        balance,
        nonce,
        pubkey_hash: sha3_256(&[id; 64]),
        pubkey: Some(vec![id; 64]),
    }
}

fn test_account_no_pubkey(id: u8, balance: u128, nonce: u64, pubkey_bytes: &[u8]) -> Account {
    Account {
        account_id: [id; 32],
        balance,
        nonce,
        pubkey_hash: sha3_256(pubkey_bytes),
        pubkey: None,
    }
}

// ─── F-22 / F-47: First-spend pubkey reveal ─────────────────────────────

#[test]
fn fix_F22_F47_pubkey_reveal_positive() {
    let mut sm = StateMachine::new();
    let pk_bytes = vec![0xABu8; 64];
    let alice = test_account_no_pubkey(1, 1000, 0, &pk_bytes);
    let bob = test_account_with_pubkey(2, 0, 0);
    sm.init_account(alice);
    sm.init_account(bob);

    // Reveal pubkey with correct bytes
    assert_eq!(sm.reveal_pubkey([1u8; 32], pk_bytes.clone()), ExecutionResult::Success);

    // Now the transfer should succeed (pubkey is no longer None)
    let r = sm.execute_transfer([1u8; 32], [2u8; 32], 500, 1, ctx(1));
    assert_eq!(r, ExecutionResult::Success);
    assert_eq!(sm.get_account(&[1u8; 32]).unwrap().balance, 500);
}

#[test]
fn fix_F22_F47_pubkey_reveal_negative_wrong_bytes() {
    let mut sm = StateMachine::new();
    let pk_bytes = vec![0xABu8; 64];
    let alice = test_account_no_pubkey(1, 1000, 0, &pk_bytes);
    sm.init_account(alice);

    // Wrong pubkey bytes (don't match pubkey_hash)
    assert_eq!(sm.reveal_pubkey([1u8; 32], vec![0xFFu8; 64]), ExecutionResult::Rejected);
}

#[test]
fn fix_F22_F47_pubkey_reveal_negative_already_revealed() {
    let mut sm = StateMachine::new();
    let pk_bytes = vec![0xABu8; 64];
    let alice = test_account_no_pubkey(1, 1000, 0, &pk_bytes);
    sm.init_account(alice);

    // First reveal succeeds
    assert_eq!(sm.reveal_pubkey([1u8; 32], pk_bytes.clone()), ExecutionResult::Success);

    // Second reveal must be rejected
    assert_eq!(sm.reveal_pubkey([1u8; 32], pk_bytes), ExecutionResult::Rejected);
}

#[test]
fn fix_F22_F47_pubkey_reveal_negative_nonexistent_account() {
    let mut sm = StateMachine::new();
    assert_eq!(sm.reveal_pubkey([99u8; 32], vec![0xABu8; 64]), ExecutionResult::Rejected);
}

#[test]
fn fix_F22_F47_transfer_rejects_if_pubkey_not_revealed() {
    let mut sm = StateMachine::new();
    let pk_bytes = vec![0xABu8; 64];
    let alice = test_account_no_pubkey(1, 1000, 0, &pk_bytes);
    let bob = test_account_with_pubkey(2, 0, 0);
    sm.init_account(alice);
    sm.init_account(bob);

    // Transfer without revealing pubkey first must be rejected
    let r = sm.execute_transfer([1u8; 32], [2u8; 32], 500, 1, ctx(1));
    assert_eq!(r, ExecutionResult::Rejected);
}

// ─── F-23: Remove unwrap on validator lookup in slashing ─────────────────

#[test]
fn fix_F23_slash_unwrap_positive() {
    let mut sm = StateMachine::new();
    let v = [1u8; 32];
    let bond = 100_000_000_000_000_000_000u128; // 100 AGX
    sm.init_account(test_account_with_pubkey(1, 500_000_000_000_000_000_000, 0));
    sm.execute_bond(v, bond, 1, bond, 10, ctx(10));

    // Slash succeeds without panicking
    let r = sm.execute_slash_equivocation(v, 100, 5000, 100);
    assert_eq!(r, ExecutionResult::Success);
    assert_eq!(sm.get_validator(&v).unwrap().self_bond, bond * 9 / 10);
    assert_eq!(
        sm.get_validator(&v).unwrap().state,
        hyperfluid_state::state_machine::ValidatorLifecycleState::Paused
    );
}

#[test]
fn fix_F23_slash_unwrap_negative_nonexistent() {
    let mut sm = StateMachine::new();
    // Non-existent validator → Rejected (no panic from unwrap)
    let r = sm.execute_slash_equivocation([99u8; 32], 100, 5000, 100);
    assert_eq!(r, ExecutionResult::Rejected);
}

#[test]
fn fix_F23_slash_unwrap_negative_not_active() {
    let mut sm = StateMachine::new();
    let v = [1u8; 32];
    let bond = 100_000_000_000_000_000_000u128;
    sm.init_account(test_account_with_pubkey(1, 500_000_000_000_000_000_000, 0));
    sm.execute_bond(v, bond, 1, bond, 10, ctx(10));
    // Unbond to make it not Active
    sm.execute_unbond(v, 2, 50, ctx(50));

    // Not active → Rejected (no panic)
    let r = sm.execute_slash_equivocation(v, 100, 5000, 100);
    assert_eq!(r, ExecutionResult::Rejected);
}

// ─── F-46: pubkey_hash on auto-created recipient accounts ────────────────

#[test]
fn fix_F46_autocreate_pubkey_hash_positive() {
    let mut sm = StateMachine::new();
    let pk_bytes = vec![0xABu8; 64];
    let alice = test_account_no_pubkey(1, 1000, 0, &pk_bytes);
    sm.init_account(alice);

    // Reveal pubkey first
    assert_eq!(sm.reveal_pubkey([1u8; 32], pk_bytes), ExecutionResult::Success);

    // Transfer to recipient [2u8; 32] — auto-creates the account
    let r = sm.execute_transfer([1u8; 32], [2u8; 32], 500, 1, ctx(1));
    assert_eq!(r, ExecutionResult::Success);

    // Recipient's pubkey_hash should equal recipient_id (not zero)
    let recipient = sm.get_account(&[2u8; 32]).unwrap();
    assert_ne!(recipient.pubkey_hash, [0u8; 32], "pubkey_hash must not be zero");
    assert_eq!(recipient.pubkey_hash, [2u8; 32], "pubkey_hash must equal recipient_id");
}

#[test]
fn fix_F46_autocreate_pubkey_hash_negative_not_zero() {
    let mut sm = StateMachine::new();
    let pk_bytes = vec![0xABu8; 64];
    let alice = test_account_no_pubkey(1, 1000, 0, &pk_bytes);
    sm.init_account(alice);

    assert_eq!(sm.reveal_pubkey([1u8; 32], pk_bytes), ExecutionResult::Success);

    let r = sm.execute_transfer([1u8; 32], [0xDEu8; 32], 500, 1, ctx(1));
    assert_eq!(r, ExecutionResult::Success);

    let recipient = sm.get_account(&[0xDEu8; 32]).unwrap();
    // Negative assertion: pubkey_hash is NOT the old zero value
    assert_eq!(
        recipient.pubkey_hash, [0xDEu8; 32],
        "pubkey_hash must be the recipient_id, not zero"
    );
}

// ─── F-77: consume_plan_id wired via execute_action_plan ──────────────────

#[test]
fn fix_F77_consume_plan_id_positive() {
    let mut sm = StateMachine::new();
    let plan_id = [0xDEu8; 32];
    let r = sm.execute_action_plan(plan_id, ctx(1));
    assert_eq!(r, ExecutionResult::Success);
}

#[test]
fn fix_F77_consume_plan_id_replay_rejected() {
    let mut sm = StateMachine::new();
    let plan_id = [0xDEu8; 32];
    assert_eq!(sm.execute_action_plan(plan_id, ctx(1)), ExecutionResult::Success);
    // Second execution of same plan must be rejected (replay protection)
    assert_eq!(sm.execute_action_plan(plan_id, ctx(2)), ExecutionResult::Rejected);
}

#[test]
fn fix_F77_consume_plan_id_different_plans_independent() {
    let mut sm = StateMachine::new();
    let plan_a = [0xAAu8; 32];
    let plan_b = [0xBBu8; 32];
    assert_eq!(sm.execute_action_plan(plan_a, ctx(1)), ExecutionResult::Success);
    assert_eq!(sm.execute_action_plan(plan_b, ctx(2)), ExecutionResult::Success);
    assert_eq!(sm.execute_action_plan(plan_a, ctx(3)), ExecutionResult::Rejected);
    assert_eq!(sm.execute_action_plan(plan_b, ctx(4)), ExecutionResult::Rejected);
}

// ─── F-78: consume_freshness_nonce wired via execute_consume_freshness_nonce ──

#[test]
fn fix_F78_consume_freshness_nonce_positive() {
    let mut sm = StateMachine::new();
    let task_id = [0xAAu8; 32];
    let nonce = [0xBBu8; 32];
    let r = sm.execute_consume_freshness_nonce(task_id, nonce, ctx(1));
    assert_eq!(r, ExecutionResult::Success);
}

#[test]
fn fix_F78_consume_freshness_nonce_replay_rejected() {
    let mut sm = StateMachine::new();
    let task_id = [0xAAu8; 32];
    let nonce = [0xBBu8; 32];
    assert_eq!(
        sm.execute_consume_freshness_nonce(task_id, nonce, ctx(1)),
        ExecutionResult::Success
    );
    // Same (task_id, nonce) pair must be rejected on second attempt
    assert_eq!(
        sm.execute_consume_freshness_nonce(task_id, nonce, ctx(2)),
        ExecutionResult::Rejected
    );
}

#[test]
fn fix_F78_consume_freshness_nonce_different_pairs_independent() {
    let mut sm = StateMachine::new();
    let t1 = [0xAAu8; 32];
    let n1 = [0xBBu8; 32];
    let t2 = [0xCCu8; 32];
    let n2 = [0xDDu8; 32];
    assert_eq!(sm.execute_consume_freshness_nonce(t1, n1, ctx(1)), ExecutionResult::Success);
    assert_eq!(sm.execute_consume_freshness_nonce(t2, n2, ctx(2)), ExecutionResult::Success);
    assert_eq!(sm.execute_consume_freshness_nonce(t1, n1, ctx(3)), ExecutionResult::Rejected);
    assert_eq!(sm.execute_consume_freshness_nonce(t2, n2, ctx(4)), ExecutionResult::Rejected);
}

// ─── F-79: snapshot_state wired via get_snapshot ─────────────────────────

#[test]
fn fix_F79_snapshot_state_positive() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account_with_pubkey(1, 1000, 0));
    sm.init_account(test_account_with_pubkey(2, 2000, 0));

    let snapshot = sm.get_snapshot(1, 100, [0xCAu8; 32]);
    assert_eq!(snapshot.epoch, 1);
    assert_eq!(snapshot.height, 100);
    assert_eq!(snapshot.block_hash, [0xCAu8; 32]);
    assert!(!snapshot.sst_keys.is_empty(), "snapshot must contain keys");
    assert_ne!(snapshot.state_root, [0u8; 32], "state root must not be zero");
}

#[test]
fn fix_F79_snapshot_state_empty() {
    let sm = StateMachine::new();
    let snapshot = sm.get_snapshot(0, 0, [0u8; 32]);
    // Empty state machine: only the fee_burn_accumulator key is present
    assert_eq!(snapshot.sst_keys.len(), 1, "only fee_burn_accumulator key should exist");
    assert_ne!(snapshot.state_root, [0u8; 32], "state root should be non-zero (fee_burn included)");
}

// ─── F-81: compute_state_checksum wired via get_state_checksum ───────────

#[test]
fn fix_F81_state_checksum_positive() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account_with_pubkey(1, 1000, 0));
    sm.init_account(test_account_with_pubkey(2, 2000, 0));

    let checksum = sm.get_state_checksum();
    assert_ne!(checksum, [0u8; 32], "checksum must not be zero for non-empty state");

    // Same state → same checksum (deterministic)
    let checksum2 = sm.get_state_checksum();
    assert_eq!(checksum, checksum2);
}

#[test]
fn fix_F81_state_checksum_negative_different_state() {
    let mut sm1 = StateMachine::new();
    let mut sm2 = StateMachine::new();
    sm1.init_account(test_account_with_pubkey(1, 1000, 0));
    sm2.init_account(test_account_with_pubkey(1, 2000, 0)); // different balance

    let cs1 = sm1.get_state_checksum();
    let cs2 = sm2.get_state_checksum();
    assert_ne!(cs1, cs2, "different state must produce different checksum");
}

#[test]
fn fix_F81_state_checksum_empty_state() {
    let sm = StateMachine::new();
    let checksum = sm.get_state_checksum();
    // Empty state checksum should be deterministic
    assert_eq!(sm.get_state_checksum(), checksum);
}

// ─── F-80: build_smt_from_keys annotation (staged, but still testable) ───

#[test]
fn fix_F80_build_smt_from_keys_positive() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account_with_pubkey(1, 1000, 0));
    let snapshot = snapshot_state(&sm, 1, 10, [0u8; 32]);

    let rebuilt_root = build_smt_from_keys(&snapshot.sst_keys);
    assert_eq!(rebuilt_root, snapshot.state_root);
}

#[test]
fn fix_F80_build_smt_from_keys_negative_empty() {
    let keys = Vec::new();
    let root = build_smt_from_keys(&keys);
    assert_eq!(root, [0u8; 32]);
}

// ─── F-82: verify_snapshot_checksum annotation (staged, but still testable) ─

#[test]
fn fix_F82_verify_snapshot_checksum_positive() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account_with_pubkey(1, 1000, 0));
    let snapshot = snapshot_state(&sm, 1, 10, [0u8; 32]);
    let checksum = compute_state_checksum(&snapshot.sst_keys);

    assert!(verify_snapshot_checksum(&snapshot.sst_keys, checksum));
}

#[test]
fn fix_F82_verify_snapshot_checksum_negative_corrupted() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account_with_pubkey(1, 1000, 0));
    let snapshot = snapshot_state(&sm, 1, 10, [0u8; 32]);
    let checksum = compute_state_checksum(&snapshot.sst_keys);

    // Corrupt a key value
    let mut corrupted = snapshot.sst_keys.clone();
    if let Some(entry) = corrupted.get_mut(0) {
        entry.1 = vec![0xFFu8; 100];
    }

    assert!(!verify_snapshot_checksum(&corrupted, checksum));
}
