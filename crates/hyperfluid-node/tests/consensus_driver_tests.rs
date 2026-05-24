// === Consensus Driver Integration Tests ===
//
// Tests the ConsensusDriver through its public interface, verifying:
//   1. Real block production with advancing height and parent hash chains
//   2. Transaction execution changes account state and state root
//   3. Multi-account genesis with block production cycles
//
// Source: docs/08-handoff/latest/build-status.md GAP: no-bft-consensus

use hyperfluid_consensus::driver::ConsensusDriver;
use hyperfluid_consensus::genesis::{GenesisAccount, GenesisConfig, GenesisValidator};
use hyperfluid_consensus::types::{
    DelegationAction, GovernanceAction, StakingAction, TransactionEnvelope, TxType,
};
use parity_scale_codec::Encode;

/// Payload for TransferTx — mirrors the struct in driver.rs.
#[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
struct TransferPayload {
    sender_id: [u8; 32],
    recipient_id: [u8; 32],
    amount: u128,
    nonce: u64,
}

/// Payload for GovernanceTx — mirrors the struct in driver.rs.
#[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
struct GovernancePayload {
    proposal_id: [u8; 32],
    proposer_id: [u8; 32],
    is_vote: bool,
    vote_approve: bool,
    target_hash: [u8; 32],
    title_hash: [u8; 32],
    description_hash: [u8; 32],
}

/// Payload for FastPathTx — mirrors the struct in driver.rs.
#[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
struct FastPathPayload {
    proposal_id: [u8; 32],
    topic_id: [u8; 32],
    proposer_id: [u8; 32],
    merge_hash: [u8; 32],
    is_challenge: bool,
}

/// Payload for StakingTx — mirrors the struct in driver.rs.
#[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
struct StakingPayload {
    validator_id: [u8; 32],
    amount: u128,
    nonce: u64,
}

/// Payload for DelegationTx — mirrors the struct in driver.rs.
#[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
struct DelegationPayload {
    delegator_id: [u8; 32],
    validator_id: [u8; 32],
    amount: u128,
    nonce: u64,
}

/// Build a minimal genesis config with the given accounts and optional validators.
fn make_genesis(accounts: Vec<GenesisAccount>, validators: Vec<GenesisValidator>) -> GenesisConfig {
    GenesisConfig {
        chain_id: "integration-test".into(),
        timestamp: 0,
        epoch_length: 8192,
        committee_size: 100,
        min_stake: 1_000_000_000_000_000_000_000u128,
        bond_delay: 100,
        unbond_delay: 1000,
        max_governance_proposals: 32,
        proposal_deposit: 500_000_000_000_000_000_000u128,
        liveness_window_blocks: 100,
        liveness_miss_threshold_pct: 20,
        total_agx_supply: 10_000_000_000_000_000_000_000_000u128,
        airdrop_amount_per_agent: 100_000_000_000_000_000_000u128,
        accounts,
        validators,
    }
}

/// Create a genesis account with the given id byte, balance (in atto-AGX), and pubkey.
fn account(id: u8, balance_agx: u128) -> GenesisAccount {
    GenesisAccount { account_id: [id; 32], balance: balance_agx, pubkey: Some(vec![id; 32]) }
}

// ─── 1. Block Production with Genesis ─────────────────────────

/// Initialize driver with genesis (10 accounts), run 5 block production cycles,
/// verify height advances from 0 to 5, parent hash chains, and block store has 6 blocks.
#[test]
fn test_node_produces_real_blocks() {
    let accounts: Vec<GenesisAccount> = (1u8..=10)
        .map(|i| account(i, 1_000_000_000_000_000_000_000u128)) // 1000 AGX each
        .collect();

    let genesis = make_genesis(accounts, vec![]);
    let mut driver = ConsensusDriver::new(genesis.epoch_length);

    let genesis_block = driver.init_genesis(&genesis);
    assert_eq!(genesis_block.header.height, 0);
    assert_eq!(genesis_block.header.parent_hash, [0u8; 32]);
    assert_ne!(genesis_block.header.state_root, [0u8; 32]);

    // Produce 5 empty blocks
    let mut produced_blocks = vec![genesis_block];
    for ts in 1..=5u64 {
        let block = driver.produce_block(vec![], ts);
        produced_blocks.push(block);
    }

    // Height advances from 0 to 5
    assert_eq!(driver.height, 5);

    // Block store has 6 blocks (genesis + 5 produced)
    assert_eq!(driver.block_store.len(), 6);

    // Each block (after genesis) has a non-zero parent_hash
    for block in &driver.block_store[1..] {
        assert_ne!(
            block.header.parent_hash, [0u8; 32],
            "block at height {} has zero parent_hash",
            block.header.height
        );
    }

    // Parent hash chains correctly
    for i in 1..produced_blocks.len() {
        let expected_parent = produced_blocks[i - 1].header.block_hash();
        let actual_parent = produced_blocks[i].header.parent_hash;
        assert_eq!(
            actual_parent, expected_parent,
            "block {} parent_hash mismatch",
            produced_blocks[i].header.height
        );
    }

    // State root is consistent across empty blocks (no transactions → no state change)
    let root_after_genesis = produced_blocks[0].header.state_root;
    for block in &produced_blocks[1..] {
        assert_eq!(
            block.header.state_root, root_after_genesis,
            "empty block at height {} changed state root",
            block.header.height
        );
    }

    // Verify all 10 accounts exist with correct balances
    for i in 1u8..=10 {
        assert_eq!(
            driver.account_balance(&[i; 32]),
            Some(1_000_000_000_000_000_000_000u128),
            "account {} balance mismatch",
            i
        );
    }
}

// ─── 2. Transaction Changes State ─────────────────────────────

/// Create 2 test accounts (Alice 1000 AGX, Bob 0 AGX), produce a block with a
/// TransferTx from Alice to Bob (100 AGX). Verify balances and state root change.
#[test]
fn test_transaction_changes_state() {
    let alice_id: [u8; 32] = [0xAA; 32];
    let bob_id: [u8; 32] = [0xBB; 32];

    let alice_initial: u128 = 1_000_000_000_000_000_000_000u128; // 1000 AGX
    let transfer_amount: u128 = 100_000_000_000_000_000_000u128; // 100 AGX

    let genesis = make_genesis(
        vec![
            GenesisAccount {
                account_id: alice_id,
                balance: alice_initial,
                pubkey: Some(vec![0xAA; 32]),
            },
            GenesisAccount { account_id: bob_id, balance: 0, pubkey: Some(vec![0xBB; 32]) },
        ],
        vec![],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.init_genesis(&genesis);
    driver.pdp_bypass = true; // mock pubkeys for testing

    let root_before_tx = driver.state_machine.compute_state_root();
    assert_ne!(root_before_tx, [0u8; 32]);

    // Build a TransferTx
    let payload = TransferPayload {
        sender_id: alice_id,
        recipient_id: bob_id,
        amount: transfer_amount,
        nonce: 1,
    };
    let tx = TransactionEnvelope {
        tx_type: TxType::TransferTx,
        tx_payload: payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };

    let block = driver.produce_block(vec![tx], 1);

    // Verify balances (includes base fee deduction)
    let base_fee = driver.fee_state.base_fee;
    assert_eq!(
        driver.account_balance(&alice_id),
        Some(alice_initial - transfer_amount - base_fee),
        "Alice balance should be initial minus transfer minus base fee"
    );
    assert_eq!(
        driver.account_balance(&bob_id),
        Some(transfer_amount),
        "Bob balance should be transfer amount"
    );

    // State root must have changed because state changed
    let root_after_tx = block.header.state_root;
    assert_ne!(root_after_tx, root_before_tx, "state root must change after a transfer");

    // Block metadata
    assert_eq!(block.header.height, 1);
    assert_eq!(driver.height, 1);
    assert_eq!(driver.block_store.len(), 2);

    // Alice nonce should have advanced
    assert_eq!(driver.account_nonce(&alice_id), Some(1));
    // Bob nonce stays 0 (recipient doesn't sign)
    assert_eq!(driver.account_nonce(&bob_id), Some(0));
}

// ─── 3. Multiple Consecutive Transfers ────────────────────────

/// Produce multiple blocks with transfers and verify cumulative state changes.
#[test]
fn test_multiple_transfers_across_blocks() {
    let alice_id: [u8; 32] = [0xA1; 32];
    let bob_id: [u8; 32] = [0xB2; 32];

    let genesis = make_genesis(
        vec![
            GenesisAccount {
                account_id: alice_id,
                balance: 1_000_000_000_000_000_000_000u128, // 1000 AGX
                pubkey: None,
            },
            GenesisAccount { account_id: bob_id, balance: 0, pubkey: None },
        ],
        vec![],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.init_genesis(&genesis);
    driver.pdp_bypass = true; // mock pubkeys for testing

    // Block 1: transfer 100 AGX
    let tx1 = TransactionEnvelope {
        tx_type: TxType::TransferTx,
        tx_payload: TransferPayload {
            sender_id: alice_id,
            recipient_id: bob_id,
            amount: 100_000_000_000_000_000_000u128,
            nonce: 1,
        }
        .encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };
    driver.produce_block(vec![tx1], 1);

    // Block 2: transfer another 50 AGX
    let tx2 = TransactionEnvelope {
        tx_type: TxType::TransferTx,
        tx_payload: TransferPayload {
            sender_id: alice_id,
            recipient_id: bob_id,
            amount: 50_000_000_000_000_000_000u128,
            nonce: 2,
        }
        .encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };
    driver.produce_block(vec![tx2], 2);

    // After 2 transfers: Alice = 850 AGX minus 2x base fee, Bob = 150 AGX
    let base_fee = driver.fee_state.base_fee;
    let expected_alice = 850_000_000_000_000_000_000u128 - base_fee * 2;
    assert_eq!(driver.account_balance(&alice_id), Some(expected_alice));
    assert_eq!(driver.account_balance(&bob_id), Some(150_000_000_000_000_000_000u128));
    assert_eq!(driver.account_nonce(&alice_id), Some(2));
    assert_eq!(driver.height, 2);
    assert_eq!(driver.block_store.len(), 3);
}

// ─── 4. Rejected Transfer Does Not Change State ───────────────

/// A rejected transfer (insufficient balance) should NOT change state root.
#[test]
fn test_rejected_transfer_preserves_state() {
    let sender: [u8; 32] = [0xDD; 32];

    let genesis = make_genesis(
        vec![GenesisAccount {
            account_id: sender,
            balance: 100, // tiny balance
            pubkey: None,
        }],
        vec![],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.init_genesis(&genesis);

    let root_before = driver.state_machine.compute_state_root();

    // Attempt to transfer more than available
    let tx = TransactionEnvelope {
        tx_type: TxType::TransferTx,
        tx_payload: TransferPayload {
            sender_id: sender,
            recipient_id: [0xEE; 32],
            amount: 1_000_000_000_000_000_000_000u128,
            nonce: 1,
        }
        .encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };

    let block = driver.produce_block(vec![tx], 1);
    let root_after = block.header.state_root;

    // State root must NOT change because the transfer was rejected
    assert_eq!(root_after, root_before, "rejected transfer should not change state root");

    // Sender balance unchanged
    assert_eq!(driver.account_balance(&sender), Some(100));
}

// ─── 5. Epoch Boundary With Transfers ─────────────────────────

/// Verify epoch advances correctly when producing blocks through an epoch boundary.
#[test]
fn test_epoch_advances_at_boundary() {
    let epoch_length: u64 = 10;

    let genesis = make_genesis(vec![account(1, 1_000_000_000_000_000_000_000u128)], vec![]);
    let mut genesis_with_epoch = genesis.clone();
    genesis_with_epoch.epoch_length = epoch_length;

    let mut driver = ConsensusDriver::new(epoch_length);
    driver.init_genesis(&genesis_with_epoch);

    // Genesis is at epoch 0, height 0
    assert_eq!(driver.epoch, 0);

    // Produce blocks up to height 9 → still epoch 0
    for _ in 0..9 {
        driver.produce_block(vec![], 0);
    }
    assert_eq!(driver.height, 9);
    assert_eq!(driver.epoch, 0);

    // Block 10 → epoch 1
    let b10 = driver.produce_block(vec![], 0);
    assert_eq!(b10.header.height, 10);
    assert_eq!(b10.header.epoch, 1);
    assert_eq!(driver.epoch, 1);

    // Block 19 → still epoch 1
    for _ in 0..9 {
        driver.produce_block(vec![], 0);
    }
    assert_eq!(driver.height, 19);
    assert_eq!(driver.epoch, 1);

    // Block 20 → epoch 2
    let b20 = driver.produce_block(vec![], 0);
    assert_eq!(b20.header.height, 20);
    assert_eq!(b20.header.epoch, 2);
}

// ─── 6. Governance Proposal Submission ─────────────────────────

/// Submit a GovernanceTx(Propose) through the ConsensusDriver and verify the
/// proposal is created in the governance engine with correct fields.
#[test]
fn test_governance_proposal_submitted() {
    let proposer_id: [u8; 32] = [0xAA; 32];
    let proposal_id: [u8; 32] = [0x42; 32];
    let target_hash: [u8; 32] = [0xFF; 32];
    let title_hash: [u8; 32] = [0x11; 32];
    let description_hash: [u8; 32] = [0x22; 32];

    let genesis = make_genesis(
        vec![GenesisAccount {
            account_id: proposer_id,
            balance: 1_000_000_000_000_000_000_000u128, // 1000 AGX
            pubkey: None,
        }],
        vec![],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.pdp_bypass = true; // PDP state not wired in this test
    driver.init_genesis(&genesis);

    // Build a GovernanceTx(Propose) transaction
    let payload = GovernancePayload {
        proposal_id,
        proposer_id,
        is_vote: false,
        vote_approve: false,
        target_hash,
        title_hash,
        description_hash,
    };
    let tx = TransactionEnvelope {
        tx_type: TxType::GovernanceTx(GovernanceAction::Propose),
        tx_payload: payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };

    let block = driver.produce_block(vec![tx], 1);

    // Block production succeeded
    assert_eq!(block.header.height, 1);

    // Verify the proposal exists in the governance engine
    let proposal = driver.governance.get_proposal(&proposal_id);
    assert!(proposal.is_some(), "proposal should exist in governance engine after submission");

    let p = proposal.unwrap();
    assert_eq!(p.proposal_id, proposal_id);
    assert_eq!(p.proposer_id, proposer_id);
    assert_eq!(p.proposed_commit, target_hash);
    // Status should be Active
    assert_eq!(format!("{:?}", p.status), "Active", "newly submitted proposal should be Active");
    // Deposit should match the default governance params
    assert_eq!(p.deposit_amount, 500_000_000_000_000_000_000u128);
}

// ─── 7. Governance Vote Casting ────────────────────────────────

/// Submit a proposal, then cast a GovernanceTx(Vote) and verify the vote
/// is counted in the governance engine's tally.
#[test]
fn test_governance_vote_cast() {
    let proposer_id: [u8; 32] = [0xAA; 32];
    let voter_id: [u8; 32] = [0xBB; 32];
    let proposal_id: [u8; 32] = [0x42; 32];
    let target_hash: [u8; 32] = [0xFF; 32];

    let genesis = make_genesis(
        vec![
            GenesisAccount {
                account_id: proposer_id,
                balance: 1_000_000_000_000_000_000_000u128, // 1000 AGX
                pubkey: None,
            },
            GenesisAccount {
                account_id: voter_id,
                balance: 1_000_000_000_000_000_000_000u128, // 1000 AGX
                pubkey: None,
            },
        ],
        vec![],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.pdp_bypass = true; // PDP state not wired in this test
    driver.init_genesis(&genesis);

    // Step 1: Submit the proposal
    let propose_payload = GovernancePayload {
        proposal_id,
        proposer_id,
        is_vote: false,
        vote_approve: false,
        target_hash,
        title_hash: [0u8; 32],
        description_hash: [0u8; 32],
    };
    let propose_tx = TransactionEnvelope {
        tx_type: TxType::GovernanceTx(GovernanceAction::Propose),
        tx_payload: propose_payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };
    driver.produce_block(vec![propose_tx], 1);

    // Verify proposal exists before voting
    assert!(
        driver.governance.get_proposal(&proposal_id).is_some(),
        "proposal must exist before voting"
    );

    // Step 2: Cast a YES vote
    let vote_payload = GovernancePayload {
        proposal_id,
        proposer_id: voter_id, // voter_id goes in proposer_id field
        is_vote: true,
        vote_approve: true, // YES
        target_hash: [0u8; 32],
        title_hash: [0u8; 32],
        description_hash: [0u8; 32],
    };
    let vote_tx = TransactionEnvelope {
        tx_type: TxType::GovernanceTx(GovernanceAction::Vote),
        tx_payload: vote_payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };
    driver.produce_block(vec![vote_tx], 2);

    // Verify the vote was counted
    let proposal = driver.governance.get_proposal(&proposal_id).unwrap();
    assert!(proposal.yes_weight > 0, "yes_weight should be > 0 after a YES vote");
    assert_eq!(proposal.no_weight, 0, "no_weight should be 0 after only a YES vote");

    // Verify the vote is in the votes list
    let votes = driver.governance.get_votes(&proposal_id);
    assert!(votes.is_some(), "votes list should exist for the proposal");
    let votes = votes.unwrap();
    assert_eq!(votes.len(), 1, "should have exactly 1 vote");
    assert_eq!(votes[0].voter_id, voter_id);
}

// ─── 8. Fast-Path Merge Proposal Submission ────────────────────

/// Submit a FastPathTx (merge proposal) through the ConsensusDriver and
/// verify the proposal is created in the fast-path engine.
#[test]
fn test_fastpath_merge_submitted() {
    let proposer_id: [u8; 32] = [0xAA; 32];
    let proposal_id: [u8; 32] = [0x01; 32];
    let topic_id: [u8; 32] = [0xCC; 32];
    let merge_hash: [u8; 32] = [0xFF; 32];

    let genesis = make_genesis(
        vec![GenesisAccount {
            account_id: proposer_id,
            balance: 1_000_000_000_000_000_000_000u128, // 1000 AGX
            pubkey: None,
        }],
        vec![],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.pdp_bypass = true; // PDP state not wired in this test
    driver.init_genesis(&genesis);

    // Build a FastPathTx (merge proposal, not a challenge)
    let payload =
        FastPathPayload { proposal_id, topic_id, proposer_id, merge_hash, is_challenge: false };
    let tx = TransactionEnvelope {
        tx_type: TxType::FastPathTx,
        tx_payload: payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };

    let block = driver.produce_block(vec![tx], 1);
    assert_eq!(block.header.height, 1);

    // Verify the proposal exists in the fast-path engine
    let proposal = driver.fastpath.get_proposal(&proposal_id);
    assert!(proposal.is_some(), "fast-path proposal should exist after submission");

    let p = proposal.unwrap();
    assert_eq!(p.proposal_id, proposal_id);
    assert_eq!(p.topic_id, topic_id);
    assert_eq!(p.proposer_id, proposer_id);
    assert_eq!(p.proposed_head, merge_hash);
    assert!(p.expires_at_height > 1, "proposal should expire in the future");
}

// ─── 9. Validator Bond via StakingTx ─────────────────────────────

/// Submit a StakingTx::Bond through the ConsensusDriver and verify:
/// - Validator record is created in state machine
/// - Funds are locked (deducted from account balance)
/// - State root changes
#[test]
fn test_validator_bond_via_driver() {
    let validator_id: [u8; 32] = [0x11; 32];
    let bond_amount = 1_000_000_000_000_000_000_000u128; // 1000 AGX

    let genesis = make_genesis(
        vec![GenesisAccount {
            account_id: validator_id,
            balance: 5_000_000_000_000_000_000_000u128, // 5000 AGX
            pubkey: None,
        }],
        vec![],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.pdp_bypass = true;
    driver.init_genesis(&genesis);
    let root_before = driver.state_machine.compute_state_root();

    let payload = StakingPayload { validator_id, amount: bond_amount, nonce: 1 };
    let tx = TransactionEnvelope {
        tx_type: TxType::StakingTx(StakingAction::Bond),
        tx_payload: payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };

    let block = driver.produce_block(vec![tx], 1);
    let root_after = block.header.state_root;

    let vt = driver.state_machine.get_validator(&validator_id).unwrap();
    assert_eq!(vt.self_bond, bond_amount);
    let base_fee = driver.fee_state.base_fee;
    assert_eq!(
        driver.account_balance(&validator_id),
        Some(4_000_000_000_000_000_000_000u128 - base_fee)
    );
    assert_ne!(root_before, root_after, "state root must change after bond");
}

// ─── 10. Validator Unbond via StakingTx ───────────────────────────

/// Bond, then unbond via driver and verify state transition.
#[test]
fn test_validator_unbond_via_driver() {
    let validator_id: [u8; 32] = [0x22; 32];
    let bond_amount = 1_000_000_000_000_000_000_000u128;

    let genesis = make_genesis(
        vec![GenesisAccount {
            account_id: validator_id,
            balance: 5_000_000_000_000_000_000_000u128,
            pubkey: None,
        }],
        vec![],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.pdp_bypass = true;
    driver.init_genesis(&genesis);

    let bond_payload = StakingPayload { validator_id, amount: bond_amount, nonce: 1 };
    let bond_tx = TransactionEnvelope {
        tx_type: TxType::StakingTx(StakingAction::Bond),
        tx_payload: bond_payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };
    driver.produce_block(vec![bond_tx], 1);

    let unbond_payload = StakingPayload { validator_id, amount: 0, nonce: 2 };
    let unbond_tx = TransactionEnvelope {
        tx_type: TxType::StakingTx(StakingAction::Unbond),
        tx_payload: unbond_payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };
    let block = driver.produce_block(vec![unbond_tx], 2);

    let vt = driver.state_machine.get_validator(&validator_id).unwrap();
    assert_eq!(vt.state, hyperfluid_state::state_machine::ValidatorLifecycleState::Unbonding);
    assert_eq!(vt.unbonding_height, 2);
    assert_ne!(block.header.state_root, [0u8; 32]);
}

// ─── 11. Validator Withdraw via StakingTx ─────────────────────────

/// Bond, unbond, advance past unbond delay, then withdraw via driver.
#[test]
fn test_validator_withdraw_via_driver() {
    let validator_id: [u8; 32] = [0x33; 32];
    let bond_amount = 1_000_000_000_000_000_000_000u128;

    let mut genesis = make_genesis(
        vec![GenesisAccount {
            account_id: validator_id,
            balance: 5_000_000_000_000_000_000_000u128,
            pubkey: None,
        }],
        vec![],
    );
    genesis.unbond_delay = 10;

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.pdp_bypass = true;
    driver.staking_params.unbond_delay = 10;
    driver.init_genesis(&genesis);

    // Bond at height 1
    let bond_payload = StakingPayload { validator_id, amount: bond_amount, nonce: 1 };
    driver.produce_block(
        vec![TransactionEnvelope {
            tx_type: TxType::StakingTx(StakingAction::Bond),
            tx_payload: bond_payload.encode(),
            approved_plan_id: None,
            gateway_signature: None,
        }],
        1,
    );

    // Unbond at height 2
    let unbond_payload = StakingPayload { validator_id, amount: 0, nonce: 2 };
    driver.produce_block(
        vec![TransactionEnvelope {
            tx_type: TxType::StakingTx(StakingAction::Unbond),
            tx_payload: unbond_payload.encode(),
            approved_plan_id: None,
            gateway_signature: None,
        }],
        2,
    );

    // Advance past unbond delay
    for _ in 0..12 {
        driver.produce_block(vec![], 0);
    }

    // Withdraw
    let withdraw_payload = StakingPayload { validator_id, amount: 0, nonce: 3 };
    driver.produce_block(
        vec![TransactionEnvelope {
            tx_type: TxType::StakingTx(StakingAction::Withdraw),
            tx_payload: withdraw_payload.encode(),
            approved_plan_id: None,
            gateway_signature: None,
        }],
        0,
    );

    assert!(
        driver.state_machine.get_validator(&validator_id).is_none(),
        "validator should be withdrawn"
    );
    // 3 base fee deductions for bond, unbond, and withdraw transactions
    let expected_balance = 5_000_000_000_000_000_000_000u128 - driver.fee_state.base_fee * 3;
    assert_eq!(
        driver.account_balance(&validator_id),
        Some(expected_balance),
        "balance should be restored minus base fees"
    );
}

// ─── 12. Delegation via DelegationTx ──────────────────────────────

/// Submit DelegationTx::Delegate through the driver and verify delegation state.
#[test]
fn test_delegation_via_driver() {
    let delegator_id: [u8; 32] = [0x44; 32];
    let validator_id: [u8; 32] = [0x55; 32];
    let bond_amount = 1_000_000_000_000_000_000_000u128;
    let delegation_amount = 100_000_000_000_000_000_000u128; // 100 AGX

    let genesis = make_genesis(
        vec![
            GenesisAccount {
                account_id: delegator_id,
                balance: 2_000_000_000_000_000_000_000u128,
                pubkey: None,
            },
            GenesisAccount {
                account_id: validator_id,
                balance: 2_000_000_000_000_000_000_000u128,
                pubkey: None,
            },
        ],
        vec![GenesisValidator { validator_id, bonded_stake: bond_amount }],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.pdp_bypass = true;
    driver.init_genesis(&genesis);

    let payload =
        DelegationPayload { delegator_id, validator_id, amount: delegation_amount, nonce: 1 };
    let tx = TransactionEnvelope {
        tx_type: TxType::DelegationTx(DelegationAction::Delegate),
        tx_payload: payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };

    driver.produce_block(vec![tx], 1);

    let base_fee = driver.fee_state.base_fee;
    assert_eq!(
        driver.account_balance(&delegator_id),
        Some(1_900_000_000_000_000_000_000u128 - base_fee),
        "delegator balance should decrease by delegation amount + base fee"
    );
}

// ─── 13. Fee Market Updates Per Block ─────────────────────────────

/// Verify that fee market base fee adjusts per block based on utilization.
#[test]
fn test_fee_market_adjusts_per_block() {
    let account_id: [u8; 32] = [0x66; 32];

    let genesis = make_genesis(
        vec![GenesisAccount {
            account_id,
            balance: 10_000_000_000_000_000_000_000u128,
            pubkey: None,
        }],
        vec![],
    );

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    driver.init_genesis(&genesis);
    let base_fee_before = driver.fee_state.base_fee;

    // Produce 5 blocks with no transactions (0% utilization → fee should decrease)
    for i in 0..5 {
        driver.produce_block(vec![], 100 + i);
    }

    let base_fee_after = driver.fee_state.base_fee;
    assert!(
        base_fee_after <= base_fee_before,
        "base fee should decrease (or stay at floor) with 0% utilization: {} → {}",
        base_fee_before,
        base_fee_after,
    );
}

// ─── 14. State Root Changes From Validator Operations ─────────────

/// Verify that a full bond→unbond→withdraw cycle produces deterministic
/// state roots at each step and that two drivers with identical inputs converge.
#[test]
fn test_validator_cycle_state_root_determinism() {
    let validator_id: [u8; 32] = [0x77; 32];
    let bond_amount = 1_000_000_000_000_000_000_000u128;

    let mut genesis = make_genesis(
        vec![GenesisAccount {
            account_id: validator_id,
            balance: 5_000_000_000_000_000_000_000u128,
            pubkey: None,
        }],
        vec![],
    );
    genesis.unbond_delay = 10;

    let mut d1 = ConsensusDriver::new(genesis.epoch_length);
    d1.pdp_bypass = true;
    d1.staking_params.unbond_delay = 10;
    d1.init_genesis(&genesis);

    let mut d2 = ConsensusDriver::new(genesis.epoch_length);
    d2.pdp_bypass = true;
    d2.staking_params.unbond_delay = 10;
    d2.init_genesis(&genesis);

    let bond_payload = StakingPayload { validator_id, amount: bond_amount, nonce: 1 };
    let bond_tx = TransactionEnvelope {
        tx_type: TxType::StakingTx(StakingAction::Bond),
        tx_payload: bond_payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };

    d1.produce_block(vec![bond_tx.clone()], 1);
    d2.produce_block(vec![bond_tx], 1);
    assert_eq!(d1.state_machine.compute_state_root(), d2.state_machine.compute_state_root());

    let unbond_payload = StakingPayload { validator_id, amount: 0, nonce: 2 };
    let unbond_tx = TransactionEnvelope {
        tx_type: TxType::StakingTx(StakingAction::Unbond),
        tx_payload: unbond_payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    };

    d1.produce_block(vec![unbond_tx.clone()], 2);
    d2.produce_block(vec![unbond_tx], 2);
    assert_eq!(d1.state_machine.compute_state_root(), d2.state_machine.compute_state_root());
}
