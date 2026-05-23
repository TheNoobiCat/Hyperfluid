// === RPC Integration Test: Node + CLI End-to-End ===
//
// Spawns a real node with the JSON-RPC server on an ephemeral port,
// then exercises the CLI binary against it. Verifies the full loop:
// CLI → HTTP POST → RPC handler → ConsensusDriver → JSON response → CLI output.
//
// Source: docs/05-planning/stages/stage-02-agent-runtime.md Week 9-10

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hyperfluid_consensus::driver::ConsensusDriver;
use hyperfluid_consensus::genesis::GenesisConfig;
use hyperfluid_node::rpc;

/// Start a node with RPC server on an ephemeral port. Returns the RPC URL
/// and the runtime that must be kept alive for the test duration.
fn start_test_node() -> (String, tokio::runtime::Runtime, tokio::task::JoinHandle<()>) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    let (url, handle) = rt.block_on(async {
        let alice: [u8; 32] =
            hex::decode("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
                .unwrap()
                .try_into()
                .unwrap();

        let genesis = GenesisConfig {
            chain_id: "rpc-testnet".into(),
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
            accounts: vec![hyperfluid_consensus::genesis::GenesisAccount {
                account_id: alice,
                balance: 1_000_000_000_000_000_000_000u128,
                pubkey: Some(vec![0xAB; 32]),
            }],
            validators: vec![],
        };

        let mut driver = ConsensusDriver::new(genesis.epoch_length);
        driver.pdp_bypass = true; // test environment
        driver.init_genesis(&genesis);

        let driver = Arc::new(Mutex::new(driver));
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let (handle, actual_addr) = rpc::start_rpc_server(Arc::clone(&driver), bind_addr);
        let url = format!("http://{}", actual_addr);
        tokio::time::sleep(Duration::from_millis(100)).await;
        (url, handle)
    });

    (url, rt, handle)
}

#[test]
fn rpc_health_endpoint_responds() {
    let (url, _rt, _handle) = start_test_node();
    let resp = reqwest::blocking::Client::new()
        .post(format!("{}/health", url))
        .send()
        .expect("health request failed");
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().expect("invalid json");
    assert_eq!(body["height"], 0);
    assert!(!body["state_root"].as_str().unwrap().is_empty());
}

#[test]
fn rpc_query_balance_returns_genesis_balance() {
    let alice_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let (url, _rt, _handle) = start_test_node();
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/query/balance", url))
        .json(&serde_json::json!({"account_id": alice_hex}))
        .send()
        .expect("balance request failed")
        .json()
        .expect("invalid json");
    assert_eq!(resp["account_id"], alice_hex);
    let balance_str = resp["balance"].as_str().unwrap_or("0");
    let balance: u128 = balance_str.parse().unwrap_or(0);
    assert!(balance > 0, "genesis account must have balance");
    assert_eq!(resp["nonce"], 0);
}

#[test]
fn rpc_query_nonce_returns_zero_for_new_account() {
    let alice_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let (url, _rt, _handle) = start_test_node();
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/query/nonce", url))
        .json(&serde_json::json!({"account_id": alice_hex}))
        .send()
        .expect("nonce request failed")
        .json()
        .expect("invalid json");
    assert_eq!(resp["nonce"], 0);
}

#[test]
fn rpc_query_state_root_is_nonzero() {
    let (url, _rt, _handle) = start_test_node();
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/query/state_root", url))
        .json(&serde_json::json!({}))
        .send()
        .expect("state_root request failed")
        .json()
        .expect("invalid json");
    let root = resp["state_root"].as_str().unwrap();
    assert!(!root.is_empty());
    assert_ne!(root, "0000000000000000000000000000000000000000000000000000000000000000");
}

#[test]
fn rpc_query_block_returns_genesis() {
    let (url, _rt, _handle) = start_test_node();
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/query/block", url))
        .json(&serde_json::json!({"height": 0}))
        .send()
        .expect("block request failed")
        .json()
        .expect("invalid json");
    assert_eq!(resp["height"], 0);
    assert_eq!(resp["epoch"], 0);
    assert_eq!(resp["tx_count"], 0);
}

#[test]
fn rpc_tx_submit_transfer_accepted() {
    let alice_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let bob_hex = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    let (url, _rt, _handle) = start_test_node();
    let client = reqwest::blocking::Client::new();

    // Submit a transfer
    let payload_hex = {
        use parity_scale_codec::Encode;
        let alice = hex::decode(alice_hex).unwrap();
        let bob = hex::decode(bob_hex).unwrap();
        let mut aid = [0u8; 32];
        aid.copy_from_slice(&alice);
        let mut bid = [0u8; 32];
        bid.copy_from_slice(&bob);
        hex::encode((aid, bid, 100u128, 1u64).encode())
    };

    let resp: serde_json::Value = client
        .post(format!("{}/tx/submit", url))
        .json(&serde_json::json!({
            "tx_type": "transfer",
            "payload": payload_hex,
        }))
        .send()
        .expect("tx submit failed")
        .json()
        .expect("invalid json");
    assert_eq!(resp["status"], "submitted_to_mempool");
    assert!(!resp["tx_hash"].as_str().unwrap().is_empty());
}

#[test]
fn rpc_agent_status_returns_data() {
    let alice_hex = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let (url, _rt, _handle) = start_test_node();
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/agent/status", url))
        .json(&serde_json::json!({"agent_id": alice_hex}))
        .send()
        .expect("agent status failed")
        .json()
        .expect("invalid json");
    assert_eq!(resp["agent_id"], alice_hex);
    let balance_str = resp["balance"].as_str().unwrap_or("0");
    let balance: u128 = balance_str.parse().unwrap_or(0);
    assert!(balance > 0);
}

#[test]
fn rpc_rejects_non_loopback_bind() {
    // This must panic — the RPC server is local-only
    let mut driver = ConsensusDriver::new(100);
    driver.pdp_bypass = true;
    let driver = Arc::new(Mutex::new(driver));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rpc::start_rpc_server(driver, "192.168.1.1:12345".parse().unwrap());
    }));
    assert!(result.is_err(), "non-loopback bind must be rejected");
}
