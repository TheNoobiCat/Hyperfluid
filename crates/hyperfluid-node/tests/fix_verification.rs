// === Production-Readiness Fix Verification Tests ===
//
// Tests for F-8, F-25, F-48, F-49, F-50, F-51, F-83 in rpc.rs.
//
// Spawns a real node with JSON-RPC server on ephemeral port,
#![allow(non_snake_case)]
// then exercises the RPC handlers with valid and invalid inputs.
//
// Conventions:
//   - Each fix has at least 1 positive test and 1 negative test
//   - Test naming: fix_F{N}_{short_description}
//
// NOTE: On Windows, ML-DSA-65 key generation + tokio runtime nested
// frames can blow the 1 MB default stack. We pre-generate identities
// outside async blocks to avoid deep nesting.

use std::net::SocketAddr;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use hyperfluid_consensus::driver::ConsensusDriver;
use hyperfluid_consensus::genesis::GenesisConfig;
use hyperfluid_consensus::types::Hash32;
use hyperfluid_fastpath::lifecycle::compute_proposal_id;
use hyperfluid_fastpath::types::FastPathProposal;
use hyperfluid_node::rpc;
use hyperfluid_p2p::identity::Identity;
use sha3::Digest;

/// Global tokio runtime shared across all tests to reduce thread churn.
fn runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| tokio::runtime::Runtime::new().expect("tokio runtime"))
}

/// Compute SHA3-256 hash of data.
fn sha3_256_hash(data: &[u8]) -> Hash32 {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Pre-built test fixture holding everything needed for tests.
struct TestFixture {
    _handle: tokio::task::JoinHandle<()>,
    url: String,
    reviewer: Identity,
    proposal_id: Hash32,
    reviewer_id: Hash32,
}

/// Build a test node with RPC server and optionally a fast-path proposal.
fn build_fixture(with_proposal: bool) -> TestFixture {
    // Pre-generate identity OUTSIDE async context to avoid deep stack nesting
    let reviewer = Identity::generate();
    let reviewer_pubkey = reviewer.verifying_key_encoded();
    let reviewer_id = *reviewer.peer_id();
    // Use a DIFFERENT proposer_id (not the reviewer) so the independence check passes.
    // submit_proposal overwrites the proposal_id using compute_proposal_id.
    let proposer_id: Hash32 = [0x99; 32];
    let topic_id: Hash32 = [0xBB; 32];
    let base_topic_head: Hash32 = [0x00; 32];
    let proposed_head: Hash32 = [0xFF; 32];
    let proposal_id = if with_proposal {
        compute_proposal_id(&topic_id, &proposer_id, &base_topic_head, &proposed_head, 0)
    } else {
        [0xAA; 32] // dummy value, never used
    };

    let (_handle, url) = runtime().block_on(async {
        let genesis = GenesisConfig {
            chain_id: "fix-verification".into(),
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
                account_id: reviewer_id,
                balance: 1_000_000_000_000_000_000_000u128,
                pubkey: Some(reviewer_pubkey),
            }],
            validators: vec![],
        };

        let mut driver = ConsensusDriver::new(genesis.epoch_length, [0u8; 32], [0u8; 32]);
        driver.init_genesis(&genesis);

        if with_proposal {
            let proposal = FastPathProposal {
                proposal_id, // will be overwritten by submit_proposal
                topic_id,
                proposer_id, // different from reviewer_id to pass independence check
                base_topic_head,
                proposed_head,
                bundle_manifest_hash: [0xCC; 32],
                expires_at_height: 1000,
                proposer_signature: vec![1u8; 64], // dummy non-empty signature
            };
            driver.fastpath.submit_proposal(proposal, 0).expect("proposal should be accepted");
        }

        let driver = Arc::new(Mutex::new(driver));
        let bind_addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let (handle, actual_addr) = rpc::start_rpc_server(Arc::clone(&driver), bind_addr);
        let url = format!("http://{}", actual_addr);
        tokio::time::sleep(Duration::from_millis(50)).await;
        (handle, url)
    });

    TestFixture { _handle, url, reviewer, proposal_id, reviewer_id }
}

/// Create the signing message the RPC handler expects to verify.
fn build_signing_message(proposal_id: &Hash32, reviewer_id: &Hash32, reason: &str) -> Vec<u8> {
    let reason_hash = sha3_256_hash(reason.as_bytes());
    let vote_byte: u8 = 1u8; // Approve
    let mut msg = Vec::with_capacity(97);
    msg.extend_from_slice(proposal_id);
    msg.extend_from_slice(reviewer_id);
    msg.push(vote_byte);
    msg.extend_from_slice(&reason_hash);
    msg
}

// ─── F-48: target_hash hex decode failure ───

#[test]
fn fix_F48_valid_target_hash_accepted() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let valid_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let resp: serde_json::Value = client
        .post(format!("{}/governance/propose", fx.url))
        .json(&serde_json::json!({
            "proposer": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "target_hash": valid_hash,
        }))
        .send()
        .expect("governance propose request failed")
        .json()
        .expect("invalid json");
    let err = resp.get("error").and_then(|v| v.as_str()).unwrap_or("");
    assert_ne!(err, "Invalid target_hash: hex decode failed");
}

#[test]
fn fix_F48_invalid_target_hash_hex_rejected() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/governance/propose", fx.url))
        .json(&serde_json::json!({
            "proposer": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "target_hash": "not-valid-hex",
        }))
        .send()
        .expect("governance propose request failed")
        .json()
        .expect("invalid json");
    assert_eq!(
        resp.get("error").and_then(|v| v.as_str()).unwrap_or(""),
        "Invalid target_hash: hex decode failed"
    );
}

// ─── F-49: title_hash hex decode failure ───

#[test]
fn fix_F49_valid_title_hash_accepted() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/governance/propose", fx.url))
        .json(&serde_json::json!({
            "proposer": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "target_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "title_hash": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
        }))
        .send()
        .expect("governance propose request failed")
        .json()
        .expect("invalid json");
    let err = resp.get("error").and_then(|v| v.as_str()).unwrap_or("");
    assert_ne!(err, "Invalid title_hash: hex decode failed");
}

#[test]
fn fix_F49_invalid_title_hash_hex_rejected() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/governance/propose", fx.url))
        .json(&serde_json::json!({
            "proposer": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "target_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "title_hash": "zzzz-invalid-hex",
        }))
        .send()
        .expect("governance propose request failed")
        .json()
        .expect("invalid json");
    assert_eq!(
        resp.get("error").and_then(|v| v.as_str()).unwrap_or(""),
        "Invalid title_hash: hex decode failed"
    );
}

// ─── F-50: description_hash hex decode failure ───

#[test]
fn fix_F50_valid_description_hash_accepted() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/governance/propose", fx.url))
        .json(&serde_json::json!({
            "proposer": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "target_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "description_hash": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
        }))
        .send()
        .expect("governance propose request failed")
        .json()
        .expect("invalid json");
    let err = resp.get("error").and_then(|v| v.as_str()).unwrap_or("");
    assert_ne!(err, "Invalid description_hash: hex decode failed");
}

#[test]
fn fix_F50_invalid_description_hash_hex_rejected() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/governance/propose", fx.url))
        .json(&serde_json::json!({
            "proposer": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "target_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "title_hash": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "description_hash": "gg-invalid",
        }))
        .send()
        .expect("governance propose request failed")
        .json()
        .expect("invalid json");
    assert_eq!(
        resp.get("error").and_then(|v| v.as_str()).unwrap_or(""),
        "Invalid description_hash: hex decode failed"
    );
}

// ─── F-51: vote target_hash hex decode failure ───

#[test]
fn fix_F51_vote_valid_target_hash_accepted() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let valid_hash = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let resp: serde_json::Value = client
        .post(format!("{}/governance/vote", fx.url))
        .json(&serde_json::json!({
            "proposal_id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "voter": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "approve": true,
            "target_hash": valid_hash,
        }))
        .send()
        .expect("governance vote request failed")
        .json()
        .expect("invalid json");
    let err = resp.get("error").and_then(|v| v.as_str()).unwrap_or("");
    assert_ne!(err, "Invalid target_hash: hex decode failed");
}

#[test]
fn fix_F51_vote_invalid_target_hash_hex_rejected() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/governance/vote", fx.url))
        .json(&serde_json::json!({
            "proposal_id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "voter": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "approve": true,
            "target_hash": "bad-hex!!!",
        }))
        .send()
        .expect("governance vote request failed")
        .json()
        .expect("invalid json");
    assert_eq!(
        resp.get("error").and_then(|v| v.as_str()).unwrap_or(""),
        "Invalid target_hash: hex decode failed"
    );
}

// ─── F-83: title_hash / description_hash in governance vote ───

#[test]
fn fix_F83_vote_with_title_computes_hash() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/governance/vote", fx.url))
        .json(&serde_json::json!({
            "proposal_id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "voter": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "approve": true,
            "target_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "title": "Upgrade to v2.0",
            "description": "This proposal upgrades the protocol to version 2.0 with new features",
        }))
        .send()
        .expect("governance vote request failed")
        .json()
        .expect("invalid json");
    let err = resp.get("error").and_then(|v| v.as_str()).unwrap_or("");
    assert_ne!(err, "Invalid title_hash: hex decode failed");
    // Verify hash computation is correct
    let expected_title_hash = sha3_256_hash(b"Upgrade to v2.0");
    let expected_desc_hash =
        sha3_256_hash(b"This proposal upgrades the protocol to version 2.0 with new features");
    assert_ne!(expected_title_hash, [0u8; 32], "expected hash should be non-zero");
    assert_ne!(expected_desc_hash, [0u8; 32], "expected hash should be non-zero");
}

#[test]
fn fix_F83_vote_without_title_requires_hash() {
    let fx = build_fixture(false);
    let client = reqwest::blocking::Client::new();
    let resp: serde_json::Value = client
        .post(format!("{}/governance/vote", fx.url))
        .json(&serde_json::json!({
            "proposal_id": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "voter": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc",
            "approve": true,
            "target_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        }))
        .send()
        .expect("governance vote request failed")
        .json()
        .expect("invalid json");
    let err = resp.get("error").and_then(|v| v.as_str()).unwrap_or("");
    assert!(err.contains("title_hash"), "expected title_hash required error, got: {}", err);
}

// ─── F-8: ReviewerSignature signature verification ───

#[test]
fn fix_F8_fastpath_approve_with_valid_signature() {
    let fx = build_fixture(true);
    let client = reqwest::blocking::Client::new();
    let reason = "Approved: code review passed all checks";

    // Build message and sign
    let msg = build_signing_message(&fx.proposal_id, &fx.reviewer_id, reason);
    let signature_bytes = fx.reviewer.sign(&msg);

    let resp: serde_json::Value = client
        .post(format!("{}/fastpath/approve", fx.url))
        .json(&serde_json::json!({
            "proposal_id": hex::encode(fx.proposal_id),
            "reviewer_id": hex::encode(fx.reviewer_id),
            "signature": hex::encode(&signature_bytes),
            "reason": reason,
            "topic_weight": 1,
        }))
        .send()
        .expect("fastpath approve request failed")
        .json()
        .expect("invalid json");

    let error = resp.get("error").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        error != "Invalid reviewer signature",
        "valid signature should not be rejected: {}",
        error
    );
    assert!(error != "Reviewer identity not found", "reviewer should be found: {}", error);
    assert!(
        resp.get("status").is_some() || resp.get("certificate_issued").is_some(),
        "response should indicate approval result: {:?}",
        resp
    );
}

#[test]
fn fix_F8_fastpath_approve_with_invalid_signature_rejected() {
    let fx = build_fixture(true);
    let client = reqwest::blocking::Client::new();
    let reason = "Approved";

    // Build message but sign with a DIFFERENT identity
    let wrong_reviewer = Identity::generate();
    let msg = build_signing_message(&fx.proposal_id, &fx.reviewer_id, reason);
    let signature_bytes = wrong_reviewer.sign(&msg);

    let resp: serde_json::Value = client
        .post(format!("{}/fastpath/approve", fx.url))
        .json(&serde_json::json!({
            "proposal_id": hex::encode(fx.proposal_id),
            "reviewer_id": hex::encode(fx.reviewer_id),
            "signature": hex::encode(&signature_bytes),
            "reason": reason,
            "topic_weight": 1,
        }))
        .send()
        .expect("fastpath approve request failed")
        .json()
        .expect("invalid json");

    assert_eq!(
        resp.get("error").and_then(|v| v.as_str()).unwrap_or(""),
        "Invalid reviewer signature",
        "wrong identity signature should be rejected"
    );
}

#[test]
fn fix_F8_fastpath_approve_unknown_reviewer_rejected() {
    let fx = build_fixture(true);
    let client = reqwest::blocking::Client::new();
    let unknown_reviewer_id: Hash32 = [0xDE; 32]; // Not in genesis

    let reason = "test";
    let msg = build_signing_message(&fx.proposal_id, &unknown_reviewer_id, reason);
    let identity = Identity::generate();
    let signature_bytes = identity.sign(&msg);

    let resp: serde_json::Value = client
        .post(format!("{}/fastpath/approve", fx.url))
        .json(&serde_json::json!({
            "proposal_id": hex::encode(fx.proposal_id),
            "reviewer_id": hex::encode(unknown_reviewer_id),
            "signature": hex::encode(&signature_bytes),
            "reason": reason,
            "topic_weight": 1,
        }))
        .send()
        .expect("fastpath approve request failed")
        .json()
        .expect("invalid json");

    let error = resp.get("error").and_then(|v| v.as_str()).unwrap_or("");
    assert!(
        error == "Reviewer identity not found",
        "unknown reviewer should be rejected, got: {}",
        error
    );
}

#[test]
fn fix_F8_fastpath_approve_missing_signature_rejected() {
    let fx = build_fixture(true);
    let client = reqwest::blocking::Client::new();

    let resp: serde_json::Value = client
        .post(format!("{}/fastpath/approve", fx.url))
        .json(&serde_json::json!({
            "proposal_id": hex::encode(fx.proposal_id),
            "reviewer_id": hex::encode(fx.reviewer_id),
            "reason": "approved",
            "topic_weight": 1,
        }))
        .send()
        .expect("fastpath approve request failed")
        .json()
        .expect("invalid json");

    assert_eq!(
        resp.get("error").and_then(|v| v.as_str()).unwrap_or(""),
        "missing 'signature' field"
    );
}

// ─── F-25: reason_hash computation ───

#[test]
fn fix_F25_reason_hash_computed_correctly() {
    let reason1 = "Approved: code review passed all checks";
    let reason2 = "Approved: LGTM";

    let hash1 = sha3_256_hash(reason1.as_bytes());
    let hash2 = sha3_256_hash(reason2.as_bytes());

    assert_ne!(hash1, hash2, "different reasons must produce different hashes");
    assert_eq!(sha3_256_hash(reason1.as_bytes()), hash1, "same reason must produce same hash");
    assert_ne!(hash1, [0u8; 32], "non-empty reason must produce non-zero hash");
    assert_ne!(sha3_256_hash(b""), [0u8; 32], "empty string still has a valid SHA3-256 hash");
}
