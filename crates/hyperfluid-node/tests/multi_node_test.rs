// === Multi-Node Integration Test Harness ===
//
// Deterministic multi-node state consistency tests.
// Verifies that multiple independent ConsensusDriver instances produce
// identical state given the same inputs — the core property required for
// BFT consensus convergence.
//
// All tests are single-threaded and deterministic: no actual networking,
// no TCP sockets. Each TestNode wraps a ConsensusDriver initialised from
// the same GenesisConfig and processes transactions independently. The
// tests verify that state roots, balances, block hashes, and chain heights
// are identical when the same input sequence is applied.
//
// Source: docs/08-handoff/latest/build-status.md
//         GAP: no-multi-node-harness (HIGH)
//         Integration Gate: Node binary / integration

use hyperfluid_consensus::driver::ConsensusDriver;
use hyperfluid_consensus::genesis::{GenesisAccount, GenesisConfig};
use hyperfluid_consensus::types::{Block, Hash32, TransactionEnvelope, TxType};
use parity_scale_codec::Encode;

// ─── Transfer Payload (mirrors ConsensusDriver's internal TransferPayload) ────

/// SCALE-encoded payload for TransferTx transactions.
/// Must match the wire format expected by `ConsensusDriver::execute_tx`.
#[derive(parity_scale_codec::Encode, parity_scale_codec::Decode)]
struct TransferPayload {
    sender_id: Hash32,
    recipient_id: Hash32,
    amount: u128,
    nonce: u64,
}

// ─── Test Node ───────────────────────────────────────────────────────────────

/// A lightweight test node wrapping a ConsensusDriver.
///
/// Simulates an independent full node for multi-node determinism tests.
/// The `node_id`, `port`, and `chain_id` fields are stored for future
/// networking integration but are not used in the deterministic tests below.
#[allow(dead_code)]
struct TestNode {
    driver: ConsensusDriver,
    node_id: u8,
    port: u16,
    chain_id: String,
}

impl TestNode {
    /// Create a new test node, bootstrapping the consensus driver from the
    /// given genesis configuration.
    fn new(node_id: u8, port: u16, genesis: &GenesisConfig) -> Self {
        let mut driver = ConsensusDriver::new(genesis.epoch_length);
        driver.init_genesis(genesis);
        Self { driver, node_id, port, chain_id: genesis.chain_id.clone() }
    }

    /// Produce a new block containing the given transactions.
    /// Returns the produced block whose `header.state_root` reflects the
    /// post-execution state.
    fn produce_block(&mut self, txs: Vec<TransactionEnvelope>) -> Block {
        // Use next height as timestamp — monotonic and deterministic.
        let timestamp = self.driver.height + 1;
        self.driver.produce_block(txs, timestamp)
    }

    /// Return the current SMT state root.
    fn state_root(&self) -> Hash32 {
        self.driver.state_machine.compute_state_root()
    }

    /// Return the current chain height (latest block number).
    fn height(&self) -> u64 {
        self.driver.height
    }

    /// Query an account balance. Returns `None` if the account does not exist.
    fn balance(&self, account_id: &Hash32) -> Option<u128> {
        self.driver.account_balance(account_id)
    }
}

// ─── Test Helpers ────────────────────────────────────────────────────────────

/// Create a minimal genesis config with the given accounts and sensible
/// system-parameter defaults. No validators are registered (pure state-machine
/// mode for determinism testing).
fn make_test_genesis(accounts: Vec<(Hash32, u128)>) -> GenesisConfig {
    let genesis_accounts: Vec<GenesisAccount> = accounts
        .into_iter()
        .map(|(account_id, balance)| GenesisAccount { account_id, balance, pubkey: None })
        .collect();

    GenesisConfig {
        chain_id: "multi-node-test".into(),
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
        accounts: genesis_accounts,
        validators: vec![],
    }
}

/// Build a `TransferTx` transaction envelope with the given parameters.
fn make_transfer(
    sender: Hash32,
    recipient: Hash32,
    amount: u128,
    nonce: u64,
) -> TransactionEnvelope {
    let payload = TransferPayload { sender_id: sender, recipient_id: recipient, amount, nonce };
    TransactionEnvelope {
        tx_type: TxType::TransferTx,
        tx_payload: payload.encode(),
        approved_plan_id: None,
        gateway_signature: None,
    }
}

// ─── 1. Two Nodes Produce Blocks Independently ───────────────────────────────

/// Create 2 TestNodes with the same genesis config. Each produces 3 empty blocks.
/// Both must reach height 3 and have identical genesis blocks.
#[test]
fn test_two_nodes_produce_blocks_independently() {
    let genesis = make_test_genesis(vec![([1u8; 32], 1_000_000_000_000_000_000_000u128)]);

    let mut node_a = TestNode::new(1, 10001, &genesis);
    let mut node_b = TestNode::new(2, 10002, &genesis);

    // Both produce 3 empty blocks
    for _ in 0..3 {
        node_a.produce_block(vec![]);
        node_b.produce_block(vec![]);
    }

    assert_eq!(node_a.height(), 3);
    assert_eq!(node_b.height(), 3);

    // Genesis blocks (height 0) must be byte-for-byte identical
    let genesis_a = &node_a.driver.block_store[0];
    let genesis_b = &node_b.driver.block_store[0];
    assert_eq!(
        genesis_a.header.block_hash(),
        genesis_b.header.block_hash(),
        "genesis block hashes must be identical across nodes"
    );

    // State roots should still match after identical empty-block sequences
    assert_eq!(node_a.state_root(), node_b.state_root());
}

// ─── 2. Two Nodes Identical State After Same Transactions ────────────────────

/// Create 2 nodes with identical genesis (Alice 1000 AGX, Bob 0 AGX).
/// Both execute the same TransferTx (Alice → Bob, 100 AGX).
/// Verify both nodes have identical state roots and correct balances.
#[test]
fn test_two_nodes_identical_state_after_same_transactions() {
    let alice: Hash32 = [0xAA; 32];
    let bob: Hash32 = [0xBB; 32];
    let alice_initial: u128 = 1_000_000_000_000_000_000_000u128; // 1000 AGX

    let genesis = make_test_genesis(vec![(alice, alice_initial), (bob, 0)]);

    let mut node_a = TestNode::new(1, 10001, &genesis);
    let mut node_b = TestNode::new(2, 10002, &genesis);

    let tx = make_transfer(alice, bob, 100_000_000_000_000_000_000u128, 1); // 100 AGX

    node_a.produce_block(vec![tx.clone()]);
    node_b.produce_block(vec![tx]);

    // State roots must be identical
    assert_eq!(
        node_a.state_root(),
        node_b.state_root(),
        "state roots must be identical after same transactions"
    );

    // Both nodes must see correct post-transfer balances
    assert_eq!(node_a.balance(&alice), Some(900_000_000_000_000_000_000u128));
    assert_eq!(node_b.balance(&alice), Some(900_000_000_000_000_000_000u128));
    assert_eq!(node_a.balance(&bob), Some(100_000_000_000_000_000_000u128));
    assert_eq!(node_b.balance(&bob), Some(100_000_000_000_000_000_000u128));
}

// ─── 3. Two Nodes Diverge With Different Transactions ────────────────────────

/// Create 2 nodes. Node A transfers 100 AGX Alice → Bob.
/// Node B transfers 200 AGX Alice → Bob. Verify state roots DIFFER.
#[test]
fn test_two_nodes_diverge_with_different_transactions() {
    let alice: Hash32 = [0xAA; 32];
    let bob: Hash32 = [0xBB; 32];

    let genesis = make_test_genesis(vec![(alice, 1_000_000_000_000_000_000_000u128), (bob, 0)]);

    let mut node_a = TestNode::new(1, 10001, &genesis);
    let mut node_b = TestNode::new(2, 10002, &genesis);

    let tx_a = make_transfer(alice, bob, 100_000_000_000_000_000_000u128, 1); // 100 AGX
    let tx_b = make_transfer(alice, bob, 200_000_000_000_000_000_000u128, 1); // 200 AGX

    node_a.produce_block(vec![tx_a]);
    node_b.produce_block(vec![tx_b]);

    // State roots must diverge
    assert_ne!(
        node_a.state_root(),
        node_b.state_root(),
        "state roots must diverge after different transfer amounts"
    );

    // Verify the divergence direction
    assert_eq!(node_a.balance(&bob), Some(100_000_000_000_000_000_000u128));
    assert_eq!(node_b.balance(&bob), Some(200_000_000_000_000_000_000u128));
    assert_eq!(node_a.balance(&alice), Some(900_000_000_000_000_000_000u128));
    assert_eq!(node_b.balance(&alice), Some(800_000_000_000_000_000_000u128));
}

// ─── 4. Three Nodes Sequential Block Sync ────────────────────────────────────

/// Create 3 nodes with identical genesis. Node A produces a block with a
/// transfer. Nodes B and C independently execute the same transactions.
/// Verify all 3 converge to identical state roots.
#[test]
fn test_three_nodes_sequential_block_sync() {
    let alice: Hash32 = [0xA1; 32];
    let bob: Hash32 = [0xB1; 32];

    let genesis = make_test_genesis(vec![(alice, 1_000_000_000_000_000_000_000u128), (bob, 0)]);

    let mut node_a = TestNode::new(1, 10001, &genesis);
    let mut node_b = TestNode::new(2, 10002, &genesis);
    let mut node_c = TestNode::new(3, 10003, &genesis);

    // All start with identical genesis state
    assert_eq!(node_a.state_root(), node_b.state_root());
    assert_eq!(node_b.state_root(), node_c.state_root());

    // Node A produces a block with a transfer
    let tx = make_transfer(alice, bob, 50_000_000_000_000_000_000u128, 1); // 50 AGX
    node_a.produce_block(vec![tx.clone()]);

    // Nodes B and C independently execute the same transaction
    node_b.produce_block(vec![tx.clone()]);
    node_c.produce_block(vec![tx]);

    // All three must converge to identical state
    assert_eq!(node_a.state_root(), node_b.state_root(), "node A and B must converge");
    assert_eq!(node_b.state_root(), node_c.state_root(), "node B and C must converge");

    // All have the same height
    assert_eq!(node_a.height(), 1);
    assert_eq!(node_b.height(), 1);
    assert_eq!(node_c.height(), 1);

    // All see the same balances
    for node in &[&node_a, &node_b, &node_c] {
        assert_eq!(node.balance(&alice), Some(950_000_000_000_000_000_000u128));
        assert_eq!(node.balance(&bob), Some(50_000_000_000_000_000_000u128));
    }
}

// ─── 5. Multi-Node Genesis Consistency ───────────────────────────────────────

/// Create 5 nodes with identical genesis configs. Verify all 5 have identical
/// genesis block hashes, initial state roots, and account balances.
#[test]
fn test_multi_node_genesis_consistency() {
    let genesis = make_test_genesis(vec![
        ([0x01; 32], 1_000_000_000_000_000_000_000u128),
        ([0x02; 32], 500_000_000_000_000_000_000u128),
        ([0x03; 32], 100_000_000_000_000_000_000u128),
    ]);

    let nodes: Vec<TestNode> =
        (1..=5).map(|id| TestNode::new(id, 10000 + id as u16, &genesis)).collect();

    // All 5 must have identical genesis block hashes
    let genesis_hash = nodes[0].driver.block_store[0].header.block_hash();
    for node in &nodes[1..] {
        assert_eq!(
            node.driver.block_store[0].header.block_hash(),
            genesis_hash,
            "node {} genesis block hash mismatch",
            node.node_id
        );
    }

    // All 5 must have identical initial state roots
    let state_root = nodes[0].state_root();
    for node in &nodes[1..] {
        assert_eq!(
            node.state_root(),
            state_root,
            "node {} initial state root mismatch",
            node.node_id
        );
    }

    // All accounts must have identical balances across all nodes
    for node in &nodes {
        assert_eq!(node.balance(&[0x01; 32]), Some(1_000_000_000_000_000_000_000u128));
        assert_eq!(node.balance(&[0x02; 32]), Some(500_000_000_000_000_000_000u128));
        assert_eq!(node.balance(&[0x03; 32]), Some(100_000_000_000_000_000_000u128));
    }
}

// ─── 6. Multi-Node State Divergence Detected ─────────────────────────────────

/// Create 2 nodes, diverge their state with different transactions,
/// then verify the state roots no longer match.
#[test]
fn test_multi_node_state_divergence_detected() {
    let alice: Hash32 = [0xDD; 32];
    let bob: Hash32 = [0xEE; 32];

    let genesis = make_test_genesis(vec![(alice, 1_000_000_000_000_000_000_000u128), (bob, 0)]);

    let mut node_a = TestNode::new(1, 10001, &genesis);
    let mut node_b = TestNode::new(2, 10002, &genesis);

    // Initial state must match
    assert_eq!(node_a.state_root(), node_b.state_root(), "initial state roots must be identical");

    // Diverge: Node A transfers 100 AGX, Node B transfers 300 AGX
    let tx_a = make_transfer(alice, bob, 100_000_000_000_000_000_000u128, 1);
    let tx_b = make_transfer(alice, bob, 300_000_000_000_000_000_000u128, 1);

    node_a.produce_block(vec![tx_a]);
    node_b.produce_block(vec![tx_b]);

    // State roots must now diverge
    assert_ne!(
        node_a.state_root(),
        node_b.state_root(),
        "state roots must diverge after different transactions"
    );

    // Verify the divergence is in the expected direction
    assert_eq!(node_a.balance(&bob), Some(100_000_000_000_000_000_000u128));
    assert_eq!(node_b.balance(&bob), Some(300_000_000_000_000_000_000u128));

    // Alice balances reflect the different transfer amounts
    assert_eq!(node_a.balance(&alice), Some(900_000_000_000_000_000_000u128));
    assert_eq!(node_b.balance(&alice), Some(700_000_000_000_000_000_000u128));
}
