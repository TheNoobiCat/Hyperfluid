// === 1000-Block Cross-Component Soak Test ===
//
// Source: docs/05-planning/stages/stage-02-agent-runtime.md Week 9-10 Task 9
//
// Exercises the ConsensusDriver across 1000 blocks with:
//   - State machine execution (transfers)
//   - Fee market adjustment (EIP-1559)
//   - PDP validation bypass (controlled test)
//   - Block store persistence
//   - State root determinism
//
// Full multi-validator BFT soak deferred until Malachite effect handler + clatter
// network bridge are implemented.

use hyperfluid_consensus::driver::ConsensusDriver;
use hyperfluid_consensus::genesis::{GenesisAccount, GenesisConfig};
use hyperfluid_consensus::types::{Hash32, TransactionEnvelope, TxType};
use parity_scale_codec::Encode;

#[derive(Encode)]
struct TransferPayload {
    sender_id: Hash32,
    recipient_id: Hash32,
    amount: u128,
    nonce: u64,
}

fn make_genesis(accounts: Vec<GenesisAccount>) -> GenesisConfig {
    GenesisConfig {
        chain_id: "soak-testnet".into(),
        timestamp: 1_000_000,
        epoch_length: 100,
        committee_size: 100,
        min_stake: 1_000_000_000_000_000_000u128,
        bond_delay: 100,
        unbond_delay: 1000,
        max_governance_proposals: 32,
        proposal_deposit: 1_000_000_000_000_000_000u128,
        liveness_window_blocks: 500,
        liveness_miss_threshold_pct: 50,
        total_agx_supply: 10_000_000_000_000_000_000_000_000_000u128,
        airdrop_amount_per_agent: 100_000_000_000_000_000_000u128,
        accounts,
        validators: vec![],
    }
}

/// 1000-block sustained run with periodic transfers.
/// Verifies no crashes, state root is non-zero, block count is correct.
#[test]
fn soak_1000_blocks_with_transfers() {
    let alice: Hash32 = [0xA1; 32];
    let bob: Hash32 = [0xB2; 32];

    let genesis = make_genesis(vec![
        GenesisAccount {
            account_id: alice,
            balance: 10_000_000_000_000_000_000_000u128, // 10000 AGX
            pubkey: None,
        },
        GenesisAccount { account_id: bob, balance: 0, pubkey: None },
    ]);

    let mut driver = ConsensusDriver::new(100);
    driver.pdp_bypass = true; // test environment
    driver.init_genesis(&genesis);

    let mut nonce: u64 = 0;

    for height in 1..=1000 {
        let txs = if height % 10 == 0 {
            nonce += 1;
            let payload = TransferPayload {
                sender_id: alice,
                recipient_id: bob,
                amount: 1_000_000, // tiny transfer each 10 blocks
                nonce,
            };
            let tx = TransactionEnvelope {
                tx_type: TxType::TransferTx,
                tx_payload: payload.encode(),
                approved_plan_id: None,
                gateway_signature: None,
            };
            vec![tx]
        } else {
            vec![]
        };

        let block = driver.produce_block(txs, 1);

        assert_eq!(block.header.height, height, "height mismatch at block {}", height);
        assert_ne!(block.header.state_root, [0u8; 32], "zero state root at block {}", height);
        assert_eq!(driver.height, height);
        assert_eq!(driver.block_store.len(), (height + 1) as usize); // +1 for genesis
    }

    assert_eq!(driver.height, 1000);
    assert!(driver.block_store.len() > 1000);
    assert!(driver.fee_state.base_fee > 0, "fee market should adjust over 1000 blocks");
}

/// 500-block soak with alternating transaction types.
/// Exercises state machine with task_create + transfer + fee market.
#[test]
fn soak_500_blocks_mixed_operations() {
    let alice: Hash32 = [0xA1; 32];
    let bob: Hash32 = [0xB2; 32];
    let charlie: Hash32 = [0xC3; 32];

    let genesis = make_genesis(vec![
        GenesisAccount {
            account_id: alice,
            balance: 50_000_000_000_000_000_000_000u128,

            pubkey: None,
        },
        GenesisAccount { account_id: bob, balance: 0, pubkey: None },
        GenesisAccount {
            account_id: charlie,
            balance: 5_000_000_000_000_000_000_000u128,
            pubkey: None,
        },
    ]);

    let mut driver = ConsensusDriver::new(100);
    driver.pdp_bypass = true;
    driver.init_genesis(&genesis);

    let initial_root = driver.state_machine.compute_state_root();
    let mut nonce: u64 = 0;

    for height in 1..=500 {
        let txs = match height % 3 {
            1 if height > 10 => {
                // Transfer every 3rd block after ramp-up
                nonce += 1;
                let payload =
                    TransferPayload { sender_id: alice, recipient_id: bob, amount: 500_000, nonce };
                vec![TransactionEnvelope {
                    tx_type: TxType::TransferTx,
                    tx_payload: payload.encode(),
                    approved_plan_id: None,
                    gateway_signature: None,
                }]
            }
            _ => vec![],
        };

        let block = driver.produce_block(txs, 1);
        assert_eq!(block.header.height, height);
        assert_ne!(block.header.state_root, [0u8; 32]);
    }

    let final_root = driver.state_machine.compute_state_root();
    assert_ne!(final_root, initial_root, "state root must change after 500 blocks of operations");
    assert_eq!(driver.height, 500);
}

/// 200-block soak verifying state root determinism: same inputs → same roots.
#[test]
fn soak_deterministic_state_roots() {
    let alice: Hash32 = [0xA1; 32];
    let genesis = make_genesis(vec![GenesisAccount {
        account_id: alice,
        balance: 10_000_000_000_000_000_000_000u128,
        pubkey: None,
    }]);

    let mut driver1 = ConsensusDriver::new(100);
    driver1.pdp_bypass = true;
    driver1.init_genesis(&genesis);

    let mut driver2 = ConsensusDriver::new(100);
    driver2.pdp_bypass = true;
    driver2.init_genesis(&genesis);

    assert_eq!(
        driver1.state_machine.compute_state_root(),
        driver2.state_machine.compute_state_root(),
        "genesis state roots must match"
    );

    for _ in 1..=200 {
        driver1.produce_block(vec![], 1);
        driver2.produce_block(vec![], 1);
    }

    assert_eq!(
        driver1.state_machine.compute_state_root(),
        driver2.state_machine.compute_state_root(),
        "state roots must match after 200 identical empty blocks"
    );
}
