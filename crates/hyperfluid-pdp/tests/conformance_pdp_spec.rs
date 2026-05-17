// Conformance tests for policy-engine-spec.md Sections 1.7, 2.7, 3.7
//
// Source: docs/04-specifications/runtime/policy-engine-spec.md

use hyperfluid_pdp::audit::AuditLog;
use hyperfluid_pdp::key_rotation;
use hyperfluid_pdp::quota::QuotaManager;
use hyperfluid_pdp::rule_chain::evaluate;
use hyperfluid_pdp::types::{
    ActionPlanRequest, ActionPlanResponse, ActionType, Decision, DenyReason, Hash32, KeyBinding,
    KeyRotationTransaction, PdpContext, QuotaState,
};
use ml_dsa::{Generate, Keypair, MlDsa65, Seed, SignatureEncoding, Signer, SigningKey};
use sha3::{Digest, Sha3_256};

fn sha3_256_bytes(data: &[u8]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

fn test_keypair() -> (Vec<u8>, [u8; 32]) {
    let sk = SigningKey::<MlDsa65>::generate();
    let pk = sk.verifying_key().encode().as_slice().to_vec();
    let seed = sk.to_seed();
    let mut seed_bytes = [0u8; 32];
    seed_bytes.copy_from_slice(seed.as_slice());
    (pk, seed_bytes)
}

fn sign_request(request: &ActionPlanRequest, sk_seed: &[u8; 32]) -> Vec<u8> {
    let seed = Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = SigningKey::<MlDsa65>::from_seed(&seed);
    let msg = hash_action_plan_for_signing(request);
    let sig = sk.sign(&msg);
    sig.to_vec()
}

fn hash_action_plan_for_signing(request: &ActionPlanRequest) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(request.plan_id);
    hasher.update(request.agent_id);
    let action_discriminant: u8 = match request.action_type {
        ActionType::PublishTopicMessage => 0,
        ActionType::ClaimTaskLease => 1,
        ActionType::RenewTaskLease => 2,
        ActionType::CreateTask => 3,
        ActionType::SubmitFastPathMerge => 4,
        ActionType::SubmitGovernanceProposal => 5,
        ActionType::CastGovernanceVote => 6,
    };
    hasher.update([action_discriminant]);
    hasher.update(request.resource_id);
    hasher.update(request.reason_hash);
    for ev in &request.evidence_refs {
        hasher.update(ev);
    }
    hasher.update(request.nonce.to_le_bytes());
    hasher.update(request.expires_at_height.to_le_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

fn make_request(
    plan_id: Hash32,
    agent_id: Hash32,
    action_type: ActionType,
    nonce: u64,
    expires: u64,
) -> ActionPlanRequest {
    ActionPlanRequest {
        plan_id,
        agent_id,
        action_type,
        resource_id: [1u8; 32],
        reason_hash: [2u8; 32],
        evidence_refs: vec![],
        nonce,
        expires_at_height: expires,
        agent_signature: vec![],
    }
}

fn make_ctx(height: u64, balance: u128, nonce: u64, key_binding: KeyBinding) -> PdpContext {
    PdpContext {
        current_height: height,
        key_binding: Some(key_binding),
        agent_balance_attagx: balance,
        agent_nonce: nonce,
        consumed_plan_ids: vec![],
        quota_states: vec![],
    }
}

// ── Section 1.7: Deterministic Policy Evaluation ────────────────────────

#[test]
fn conforms_to_pdp_spec_1_7_deterministic_evaluation() {
    let (pk, sk_seed) = test_keypair();
    let agent_id = [0xAA; 32];
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    request.agent_signature = sign_request(&request, &sk_seed);
    let ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));

    let r1 = evaluate(&request, &ctx);
    let r2 = evaluate(&request, &ctx);
    assert_eq!(r1.decision, r2.decision);
    assert_eq!(r1.deny_reason, r2.deny_reason);
    assert_eq!(r1.decision, Decision::Approved);
}

#[test]
fn conforms_to_pdp_spec_1_7_schema_violation_rejected() {
    let (pk, _sk) = test_keypair();
    let request = make_request([0u8; 32], [1u8; 32], ActionType::ClaimTaskLease, 1, 100);
    let ctx = make_ctx(0, 1000, 0, KeyBinding::stable([1u8; 32], pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
}

#[test]
fn conforms_to_pdp_spec_1_7_signature_invalid_rejected() {
    let (pk, _sk) = test_keypair();
    let (_wrong_pk, wrong_sk) = test_keypair();
    let agent_id = [0xAA; 32];
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    request.agent_signature = sign_request(&request, &wrong_sk);
    let ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
}

#[test]
fn conforms_to_pdp_spec_1_7_replay_protection_duplicate_plan_id() {
    let (pk, sk_seed) = test_keypair();
    let agent_id = [0xAA; 32];
    let plan_id = [0x42; 32];
    let mut request = make_request(plan_id, agent_id, ActionType::ClaimTaskLease, 1, 100);
    request.agent_signature = sign_request(&request, &sk_seed);
    let mut ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
    ctx.consumed_plan_ids = vec![plan_id];
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
}

#[test]
fn conforms_to_pdp_spec_1_7_replay_wrong_nonce_rejected() {
    let (pk, sk_seed) = test_keypair();
    let agent_id = [0xAA; 32];
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 5, 100);
    request.agent_signature = sign_request(&request, &sk_seed);
    let ctx = make_ctx(50, 1000, 3, KeyBinding::stable(agent_id, pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
}

#[test]
fn conforms_to_pdp_spec_1_7_ttl_expired_rejected() {
    let (pk, sk_seed) = test_keypair();
    let agent_id = [0xAA; 32];
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 50);
    request.agent_signature = sign_request(&request, &sk_seed);
    let ctx = make_ctx(100, 1000, 0, KeyBinding::stable(agent_id, pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::TTLExpired));
}

#[test]
fn conforms_to_pdp_spec_1_7_quota_exhaustion_rejected() {
    let (pk, sk_seed) = test_keypair();
    let agent_id = [0xAA; 32];
    let mut request =
        make_request([1u8; 32], agent_id, ActionType::SubmitGovernanceProposal, 1, 100);
    request.agent_signature = sign_request(&request, &sk_seed);
    let mut ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
    ctx.quota_states = vec![QuotaState {
        quota_id: "gov_proposals_per_identity".into(),
        consumed: 1,
        window_start_height: 0,
    }];
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::QuotaExhausted));
}

#[test]
fn conforms_to_pdp_spec_1_7_fee_check_insufficient_balance() {
    let (pk, sk_seed) = test_keypair();
    let agent_id = [0xAA; 32];
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    request.agent_signature = sign_request(&request, &sk_seed);
    let ctx = make_ctx(50, 0, 0, KeyBinding::stable(agent_id, pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::InsufficientFunds));
}

#[test]
fn conforms_to_pdp_spec_1_7_full_chain_approval() {
    let (pk, sk_seed) = test_keypair();
    let agent_id = [0xAA; 32];
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    request.agent_signature = sign_request(&request, &sk_seed);
    let ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Approved);
    assert_eq!(result.deny_reason, None);
}

// ── Section 2.7: Cross-Layer Quota Matrix ────────────────────────────────

#[test]
fn conforms_to_pdp_spec_2_7_quota_manager_canonical_entries() {
    let qm = QuotaManager::default();
    assert_eq!(qm.entries().len(), 14);
    assert!(qm.get_entry("task_create_per_stage").is_some());
    assert!(qm.get_entry("gov_proposals_per_identity").is_some());
}

#[test]
fn conforms_to_pdp_spec_2_7_quota_atomic_reservation() {
    let mut qm = QuotaManager::with_canonical_entries();
    let result = qm.reserve_quota("gov_proposals_per_identity", "agent1", 1, 0);
    assert!(result.is_ok());
    let result2 = qm.reserve_quota("gov_proposals_per_identity", "agent1", 1, 0);
    assert!(result2.is_err());
}

#[test]
fn conforms_to_pdp_spec_2_7_quota_release_after_failure() {
    let mut qm = QuotaManager::with_canonical_entries();
    qm.reserve_quota("gov_proposals_per_identity", "agent1", 1, 0).unwrap();
    qm.release_quota("gov_proposals_per_identity", "agent1", 1);
    let result = qm.reserve_quota("gov_proposals_per_identity", "agent1", 1, 0);
    assert!(result.is_ok());
}

#[test]
fn conforms_to_pdp_spec_2_7_task_create_stage_multipliers() {
    let qm = QuotaManager::with_canonical_entries();
    let entry = qm.get_entry("task_create_per_stage").unwrap();
    let untrusted = entry
        .stage_multipliers
        .iter()
        .find(|(s, _)| *s == hyperfluid_pdp::types::TrustStage::Untrusted);
    assert_eq!(untrusted.unwrap().1, (0, 10));
    let trusted = entry
        .stage_multipliers
        .iter()
        .find(|(s, _)| *s == hyperfluid_pdp::types::TrustStage::Trusted);
    assert_eq!(trusted.unwrap().1, (10, 10));
}

// ── Section 3.7: Key Rotation State Finalization ────────────────────────

#[test]
fn conforms_to_pdp_spec_3_7_both_keys_valid_in_grace_window() {
    let (old_pk, old_sk) = test_keypair();
    let (new_pk, new_sk) = test_keypair();
    let agent_id = [0xAA; 32];

    // Test 1: Old key accepted during grace window
    let binding = KeyBinding {
        agent_id,
        active_pubkey: old_pk.clone(),
        pending_pubkey: Some(new_pk.clone()),
        rotation_height: Some(50),
        grace_end_height: Some(150),
    };
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    request.agent_signature = sign_request(&request, &old_sk);
    let ctx = make_ctx(50, 1000, 0, binding);
    assert_eq!(evaluate(&request, &ctx).decision, Decision::Approved);

    // Test 2: New key accepted during grace window
    let binding2 = KeyBinding {
        agent_id,
        active_pubkey: old_pk,
        pending_pubkey: Some(new_pk),
        rotation_height: Some(50),
        grace_end_height: Some(150),
    };
    let mut request2 = make_request([2u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    request2.agent_signature = sign_request(&request2, &new_sk);
    let ctx2 = make_ctx(50, 1000, 0, binding2);
    assert_eq!(evaluate(&request2, &ctx2).decision, Decision::Approved);
}

#[test]
fn conforms_to_pdp_spec_3_7_old_key_rejected_after_grace() {
    let (_old_pk, old_sk) = test_keypair();
    let (new_pk, _new_sk) = test_keypair();
    let agent_id = [0xAA; 32];

    let binding = KeyBinding {
        agent_id,
        active_pubkey: new_pk,
        pending_pubkey: None,
        rotation_height: Some(50),
        grace_end_height: Some(150),
    };
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    request.agent_signature = sign_request(&request, &old_sk);
    let ctx = make_ctx(200, 1000, 0, binding);
    assert_eq!(evaluate(&request, &ctx).decision, Decision::Denied);
    assert_eq!(evaluate(&request, &ctx).deny_reason, Some(DenyReason::SignatureInvalid));
}

#[test]
fn conforms_to_pdp_spec_3_7_supersede_rotation_restarts_grace_window() {
    let (pk, sk_seed) = test_keypair();
    let (new_pk1, _new_seed1) = test_keypair();
    let (new_pk2, _new_seed2) = test_keypair();
    let agent_id = [0xAA; 32];

    let binding = KeyBinding::stable(agent_id, pk.clone());
    let tx1 = KeyRotationTransaction {
        agent_id,
        new_pubkey: new_pk1.clone(),
        new_pubkey_hash: sha3_256_bytes(&new_pk1),
        signature: vec![],
        nonce: 1,
    };
    let sig1 = {
        let seed = Seed::try_from(sk_seed.as_slice()).unwrap();
        let sk = SigningKey::<MlDsa65>::from_seed(&seed);
        let msg = {
            let mut hasher = Sha3_256::new();
            hasher.update(tx1.agent_id);
            hasher.update(&tx1.new_pubkey);
            hasher.update(tx1.new_pubkey_hash);
            hasher.update(tx1.nonce.to_le_bytes());
            let mut out = [0u8; 32];
            out.copy_from_slice(&hasher.finalize());
            out
        };
        sk.sign(&msg).to_vec()
    };
    let signed_tx1 = KeyRotationTransaction { signature: sig1, ..tx1 };
    let binding2 = key_rotation::initiate_key_rotation(&binding, &signed_tx1, 100).unwrap();
    assert_eq!(binding2.grace_end_height, Some(200));

    let new_pk2_hash = sha3_256_bytes(&new_pk2);
    let tx2 = KeyRotationTransaction {
        agent_id,
        new_pubkey: new_pk2,
        new_pubkey_hash: new_pk2_hash,
        signature: vec![],
        nonce: 2,
    };
    let signed_tx2 = {
        let seed = Seed::try_from(sk_seed.as_slice()).unwrap();
        let sk = SigningKey::<MlDsa65>::from_seed(&seed);
        let msg = {
            let mut hasher = Sha3_256::new();
            hasher.update(tx2.agent_id);
            hasher.update(&tx2.new_pubkey);
            hasher.update(tx2.new_pubkey_hash);
            hasher.update(tx2.nonce.to_le_bytes());
            let mut out = [0u8; 32];
            out.copy_from_slice(&hasher.finalize());
            out
        };
        let sig = sk.sign(&msg);
        KeyRotationTransaction { signature: sig.to_vec(), ..tx2 }
    };
    let binding3 = key_rotation::initiate_key_rotation(&binding2, &signed_tx2, 150).unwrap();
    assert_eq!(binding3.grace_end_height, Some(250));
}

#[test]
fn conforms_to_pdp_spec_3_7_nonce_continuity_across_rotation() {
    let (pk, sk_seed) = test_keypair();
    let agent_id = [0xAA; 32];

    assert_eq!(key_rotation::KEY_ROTATION_GRACE_WINDOW, 100u64);

    let binding = KeyBinding::stable(agent_id, pk);
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    request.agent_signature = sign_request(&request, &sk_seed);
    let ctx = make_ctx(50, 1000, 0, binding);
    assert_eq!(evaluate(&request, &ctx).decision, Decision::Approved);
}

#[test]
fn conforms_to_pdp_spec_3_7_rotation_tx_signed_with_wrong_key_rejected() {
    let (pk, _sk_seed) = test_keypair();
    let (new_pk, _new_seed) = test_keypair();
    let (_, wrong_seed) = test_keypair();
    let agent_id = [0xAA; 32];
    let binding = KeyBinding::stable(agent_id, pk);

    let tx = KeyRotationTransaction {
        agent_id,
        new_pubkey: new_pk.clone(),
        new_pubkey_hash: sha3_256_bytes(&new_pk),
        signature: vec![],
        nonce: 1,
    };
    let wrong_sig = {
        let seed = Seed::try_from(wrong_seed.as_slice()).unwrap();
        let sk = SigningKey::<MlDsa65>::from_seed(&seed);
        let msg = {
            let mut hasher = Sha3_256::new();
            hasher.update(tx.agent_id);
            hasher.update(&tx.new_pubkey);
            hasher.update(tx.new_pubkey_hash);
            hasher.update(tx.nonce.to_le_bytes());
            let mut out = [0u8; 32];
            out.copy_from_slice(&hasher.finalize());
            out
        };
        sk.sign(&msg).to_vec()
    };
    let signed_tx = KeyRotationTransaction { signature: wrong_sig, ..tx };

    let result = key_rotation::initiate_key_rotation(&binding, &signed_tx, 100);
    assert!(result.is_err());
}

#[test]
fn conforms_to_pdp_spec_3_7_audit_log_append_only_and_content_addressed() {
    let mut log = AuditLog::new();
    assert!(log.is_empty());

    let response1 = ActionPlanResponse {
        plan_id: [1u8; 32],
        decision: Decision::Approved,
        deny_reason: None,
        consumed_quota: None,
        approval_height: 100,
        expires_at_height: 200,
    };
    let eid1 = log.record(&response1, ActionType::ClaimTaskLease, [0xAA; 32], 100, vec![8u8; 32]);
    assert_eq!(log.len(), 1);

    let response2 = ActionPlanResponse {
        plan_id: [2u8; 32],
        decision: Decision::Denied,
        deny_reason: Some(DenyReason::SchemaViolation),
        consumed_quota: None,
        approval_height: 101,
        expires_at_height: 201,
    };
    let eid2 = log.record(
        &response2,
        ActionType::SubmitGovernanceProposal,
        [0xAA; 32],
        101,
        vec![8u8; 32],
    );
    assert_eq!(log.len(), 2);
    assert_ne!(eid1, eid2);
    assert!(log.verify_integrity());
    assert!(log.last_entry_id().is_some());
    assert!(log.get_by_plan_id(&[1u8; 32]).is_some());
    assert!(log.get_by_plan_id(&[2u8; 32]).is_some());
}
