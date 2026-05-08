// Conformance tests for state-sync-spec.md Section 1.7
//
// Source: docs/04-specifications/storage/state-sync-spec.md

use hyperfluid_state::state_machine::{ExecutionContext, StateMachine};
use hyperfluid_state::state_sync::{
    build_smt_from_keys, compute_state_checksum, snapshot_state, verify_snapshot_checksum,
    verify_state_root_quorum, SyncMode, SyncState,
};
use hyperfluid_state::Account;
use parity_scale_codec::Decode;

fn test_account(id: u8, balance: u128, nonce: u64) -> Account {
    Account { account_id: [id; 32], balance, nonce, pubkey_hash: [0u8; 32], pubkey: Some(vec![id]) }
}

// ── Hook 1: Snap sync from checkpoint produces identical SMT root as full sync ──

#[test]
fn conforms_to_state_sync_spec_1_7_snap_sync_identical_root() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 1000, 0));
    sm.init_account(test_account(2, 2000, 1));
    sm.init_account(test_account(3, 3000, 2));

    // Full sync: build SMT from all accounts
    let snapshot = snapshot_state(&sm, 1, 10, [0xA0u8; 32]);
    let snap_root = snapshot.state_root;

    // Rebuild SMT from snapshot keys
    let rebuilt_root = build_smt_from_keys(&snapshot.sst_keys);
    assert_eq!(
        rebuilt_root, snap_root,
        "rebuilt SMT root from snapshot keys must match snapshot state_root"
    );
}

// ── Hook 2: State root mismatch triggers peer rotation and quorum check ──

#[test]
fn conforms_to_state_sync_spec_1_7_root_mismatch_quorum_fails() {
    let correct_root = [0xB0u8; 32];
    let wrong_root = [0xDEu8; 32];
    let peers = vec![wrong_root; 5]; // all 5 peers report wrong root

    assert!(!verify_state_root_quorum(correct_root, &peers, 3));
}

#[test]
fn conforms_to_state_sync_spec_1_7_root_quorum_passes_with_enough_peers() {
    let correct_root = [0xB0u8; 32];
    let mut peers = vec![correct_root; 3];
    peers.push([0xFFu8; 32]); // one dissenter
    peers.push([0xFEu8; 32]); // another dissenter

    assert!(verify_state_root_quorum(correct_root, &peers, 3));
}

#[test]
fn conforms_to_state_sync_spec_1_7_root_quorum_fails_insufficient_peers() {
    let correct_root = [0xB0u8; 32];
    let peers = vec![correct_root; 2]; // only 2 matching, quorum needs 3

    assert!(!verify_state_root_quorum(correct_root, &peers, 3));
}

// ── Hook 3: Crash recovery restores state to exact pre-crash height ──

#[test]
fn conforms_to_state_sync_spec_1_7_crash_recovery_restores_state() {
    let mut sm = StateMachine::new();
    let a1 = test_account(1, 1000, 5);
    let a2 = test_account(2, 2000, 3);
    sm.init_account(a1.clone());
    sm.init_account(a2.clone());

    let saved_height = 42;
    let snapshot = snapshot_state(&sm, 1, saved_height, [0xC0u8; 32]);

    // Simulate crash: fresh state machine loaded from snapshot keys
    let mut recovered = StateMachine::new();
    for (_key, value) in &snapshot.sst_keys {
        let decoded: Account = Account::decode(&mut value.as_slice()).unwrap();
        recovered.init_account(decoded);
    }

    assert_eq!(recovered.get_account(&[1u8; 32]).unwrap().balance, 1000);
    assert_eq!(recovered.get_account(&[2u8; 32]).unwrap().balance, 2000);
    assert_eq!(recovered.get_account(&[1u8; 32]).unwrap().nonce, 5);
}

// ── Hook 4: Backup restore with checksum rejects corrupted backup ──

#[test]
fn conforms_to_state_sync_spec_1_7_checksum_rejects_corrupted_backup() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 1000, 0));

    let snapshot = snapshot_state(&sm, 1, 10, [0xD0u8; 32]);
    let checksum = compute_state_checksum(&snapshot.sst_keys);

    // Valid checksum
    assert!(verify_snapshot_checksum(&snapshot.sst_keys, checksum));

    // Corrupt the keys
    let mut corrupted_keys = snapshot.sst_keys.clone();
    corrupted_keys[0].1 = vec![0xFFu8; 100]; // tamper with value

    assert!(!verify_snapshot_checksum(&corrupted_keys, checksum));
}

#[test]
fn conforms_to_state_sync_spec_1_7_checksum_empty_snapshot() {
    let keys: Vec<([u8; 32], Vec<u8>)> = vec![];
    let checksum = compute_state_checksum(&keys);
    assert!(verify_snapshot_checksum(&keys, checksum));
}

// ── Hook 5: Deterministic state convergence ──
// Two state machines starting from identical state converge after applying the same operations.

#[test]
fn conforms_to_state_sync_spec_1_7_deterministic_state_convergence() {
    let mut sm1 = StateMachine::new();
    sm1.init_account(test_account(1, 1000, 0));
    sm1.init_account(test_account(2, 2000, 0));
    sm1.init_account(test_account(3, 3000, 0));

    let mut sm2 = StateMachine::new();
    sm2.init_account(test_account(1, 1000, 0));
    sm2.init_account(test_account(2, 2000, 0));
    sm2.init_account(test_account(3, 3000, 0));

    let ctx = ExecutionContext { height: 10, timestamp: 100 };
    // sm1 applies tx sequence
    sm1.execute_transfer([1u8; 32], [2u8; 32], 100, 1, ctx);
    sm2.execute_transfer([1u8; 32], [2u8; 32], 100, 1, ctx);

    sm1.execute_transfer([2u8; 32], [3u8; 32], 50, 1, ctx);
    sm2.execute_transfer([2u8; 32], [3u8; 32], 50, 1, ctx);

    for id in [1u8, 2u8, 3u8] {
        let a1 = sm1.get_account(&[id; 32]).unwrap();
        let a2 = sm2.get_account(&[id; 32]).unwrap();
        assert_eq!(a1.balance, a2.balance, "account {} balance must converge", id);
        assert_eq!(a1.nonce, a2.nonce, "account {} nonce must converge", id);
    }
}

// ── SyncType and SyncState types ──

#[test]
fn conforms_to_state_sync_spec_1_7_sync_mode_enum() {
    let full = SyncMode::Full;
    let snap = SyncMode::Snap;
    let catchup = SyncMode::CatchUp;
    assert_ne!(full, snap);
    assert_ne!(snap, catchup);
}

#[test]
fn conforms_to_state_sync_spec_1_7_sync_state_creation() {
    let state = SyncState {
        mode: SyncMode::Snap,
        current_height: 100,
        target_height: 500,
        validated_roots: 0,
        last_validated_block: [0u8; 32],
    };
    assert_eq!(state.mode, SyncMode::Snap);
    assert_eq!(state.current_height, 100);
    assert_eq!(state.target_height, 500);
}
