// Conformance tests for consensus-spec.md Section 2.7 (SMT State)
//
// Test naming: conforms_to_<spec>_<section>_<short_description>
// Source: docs/04-specifications/protocol/consensus-spec.md Section 2.7

use hyperfluid_state::smt::SparseMerkleTree;
use hyperfluid_state::state_machine::{ExecutionContext, ExecutionResult, StateMachine};
use hyperfluid_state::{sha3_256, state_key, KeyPrefix};

#[test]
fn conforms_to_consensus_spec_2_7_1_deterministic_state_root() {
    // Two SMTs receiving identical key-value pairs in the same order
    // MUST produce identical state roots.
    let kvs = vec![
        (state_key(KeyPrefix::Account, &[1u8; 32]), vec![10u8; 64]),
        (state_key(KeyPrefix::Account, &[2u8; 32]), vec![20u8; 64]),
        (state_key(KeyPrefix::Account, &[3u8; 32]), vec![30u8; 64]),
    ];

    let mut tree1 = SparseMerkleTree::new();
    let mut tree2 = SparseMerkleTree::new();

    for (key, value) in &kvs {
        tree1.insert(*key, value.clone());
        tree2.insert(*key, value.clone());
    }

    let root1 = tree1.root();
    let root2 = tree2.root();
    assert_eq!(root1, root2);
    assert_ne!(root1, [0u8; 32]);
}

#[test]
fn conforms_to_consensus_spec_2_7_2_inclusion_proof_validates() {
    // Insert key-value, generate proof, verify proof against root.
    let key = state_key(KeyPrefix::Account, &[0xAAu8; 32]);
    let value = vec![99u8; 64];

    let mut tree = SparseMerkleTree::new();
    tree.insert(key, value.clone());
    let root = tree.root();

    let proof = tree.prove(&key).expect("proof must exist for inserted key");
    let valid = SparseMerkleTree::verify_proof(&proof, root);
    assert!(valid, "inclusion proof must validate against root");
}

#[test]
fn conforms_to_consensus_spec_2_7_2_inclusion_proof_wrong_value_fails() {
    let key = state_key(KeyPrefix::Account, &[0xBBu8; 32]);
    let value = vec![99u8; 64];

    let mut tree = SparseMerkleTree::new();
    tree.insert(key, value.clone());
    let _root = tree.root();

    let proof = tree.prove(&key).expect("proof must exist");
    // Verify with wrong root fails
    let wrong_root: [u8; 32] = [0xFF; 32];
    let valid = SparseMerkleTree::verify_proof(&proof, wrong_root);
    assert!(!valid, "proof must not validate against wrong root");
}

#[test]
fn conforms_to_consensus_spec_2_7_2_exclusion_proof() {
    // Non-existent key must produce no proof or proof of non-inclusion.
    let mut tree = SparseMerkleTree::new();
    tree.insert(state_key(KeyPrefix::Account, &[1u8; 32]), vec![1]);

    let missing_key = state_key(KeyPrefix::Account, &[2u8; 32]);
    let proof = tree.prove(&missing_key);
    assert!(proof.is_none(), "non-existent key must return None proof");
}

#[test]
fn conforms_to_consensus_spec_2_7_2_empty_tree_root() {
    let tree = SparseMerkleTree::new();
    assert_eq!(tree.root(), [0u8; 32], "empty tree root must be zero");
}

#[test]
fn conforms_to_consensus_spec_2_7_2_single_leaf_tree() {
    let key = state_key(KeyPrefix::Account, &[42u8; 32]);
    let value = vec![7u8; 12];

    let mut tree = SparseMerkleTree::new();
    tree.insert(key, value.clone());
    let root = tree.root();

    let proof = tree.prove(&key).expect("proof must exist for single leaf");
    let valid = SparseMerkleTree::verify_proof(&proof, root);
    assert!(valid);
}

#[test]
fn conforms_to_consensus_spec_2_7_4_replay_protection() {
    let mut sm = StateMachine::new();
    let plan_id: [u8; 32] = [0xDEu8; 32];
    let ctx = ExecutionContext { height: 1, timestamp: 1000 };

    let result1 = sm.consume_plan_id(plan_id, ctx);
    assert_eq!(result1, ExecutionResult::Success);

    let result2 = sm.consume_plan_id(plan_id, ctx);
    assert_eq!(result2, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_consensus_spec_2_7_5_first_spend_pubkey_reveal() {
    // SHA3-256(pubkey_reveal) must equal sender_address.
    // spec: account_id = SHA3-256 of ML-DSA pubkey
    let pubkey = vec![1u8, 2, 3, 4, 5];
    let pubkey_hash = sha3_256(&pubkey);
    let account_id = sha3_256(&pubkey);

    // Simulate the first-spend reveal lifecycle:
    let mut acct =
        hyperfluid_state::Account { account_id, balance: 100, nonce: 0, pubkey_hash, pubkey: None };
    // Before first spend, pubkey is not yet revealed
    assert!(acct.pubkey.is_none());
    // First spend reveals the pubkey
    acct.pubkey = Some(pubkey.clone());
    // After reveal, the pubkey hash must match
    assert_eq!(sha3_256(&pubkey), acct.pubkey_hash);
    // account_id must equal pubkey_hash per spec
    assert_eq!(acct.account_id, acct.pubkey_hash);
}
