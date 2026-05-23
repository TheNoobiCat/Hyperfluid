// Conformance tests for policy-engine-spec.md Sections 1.7, 2.7, 3.7
//
// Source: docs/04-specifications/runtime/policy-engine-spec.md

use hyperfluid_pdp::audit::AuditLog;
use hyperfluid_pdp::quota::QuotaManager;
use hyperfluid_pdp::rule_chain::{evaluate, hash_action_plan_for_signing};
use hyperfluid_pdp::types::{
    ActionPlanRequest, ActionPlanResponse, ActionType, Decision, DenyReason, Hash32, PdpContext,
    QuotaState, TrustStage,
};
use ml_dsa::{Generate, Keypair, MlDsa65, SignatureEncoding, Signer, SigningKey};

fn test_keypair() -> (Vec<u8>, [u8; 32]) {
    let sk = SigningKey::<MlDsa65>::generate();
    let pk = sk.verifying_key().encode().as_slice().to_vec();
    let seed = sk.to_seed();
    let mut seed_bytes = [0u8; 32];
    seed_bytes.copy_from_slice(seed.as_slice());
    (pk, seed_bytes)
}

fn make_ctx(height: u64, balance: u128, nonce: u64, key_binding: Option<Vec<u8>>) -> PdpContext {
    PdpContext {
        current_height: height,
        key_binding,
        agent_balance_attagx: balance,
        agent_nonce: nonce,
        consumed_plan_ids: vec![],
        quota_states: vec![],
        trust_stage: TrustStage::Trusted,
    }
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

// ── Section 1.7: Deterministic Policy Evaluation ────────────────────────

#[test]
fn conforms_to_pdp_spec_1_7_deterministic_evaluation() {
    let request = make_request([1u8; 32], [0xAA; 32], ActionType::ClaimTaskLease, 1, 100);
    let ctx = make_ctx(50, 1000, 0, None);

    let r1 = evaluate(&request, &ctx);
    let r2 = evaluate(&request, &ctx);
    assert_eq!(r1.decision, r2.decision);
    assert_eq!(r1.deny_reason, r2.deny_reason);
}

#[test]
fn conforms_to_pdp_spec_1_7_schema_violation_rejected() {
    let request = make_request([0u8; 32], [1u8; 32], ActionType::ClaimTaskLease, 1, 100);
    let ctx = make_ctx(0, 1000, 0, None);
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
}

#[test]
fn conforms_to_pdp_spec_1_7_signature_verification_valid() {
    let agent_id = [0xAA; 32];
    let (pk, sk_seed) = test_keypair();
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    let msg = hash_action_plan_for_signing(&request);
    let seed = ml_dsa::Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    let ctx = make_ctx(50, 1000, 0, Some(pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.deny_reason, None);
}

#[test]
fn conforms_to_pdp_spec_1_7_signature_invalid_wrong_key() {
    let agent_id = [0xAA; 32];
    let (_pk_a, sk_seed_a) = test_keypair();
    let (pk_b, _sk_seed_b) = test_keypair();
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    let msg = hash_action_plan_for_signing(&request);
    let seed = ml_dsa::Seed::try_from(sk_seed_a.as_slice()).unwrap();
    let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    let ctx = make_ctx(50, 1000, 0, Some(pk_b));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
}

#[test]
fn conforms_to_pdp_spec_1_7_signature_invalid_tampered() {
    let agent_id = [0xAA; 32];
    let (pk, sk_seed) = test_keypair();
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    let msg = hash_action_plan_for_signing(&request);
    let seed = ml_dsa::Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    request.nonce = 999;
    let ctx = make_ctx(50, 1000, 0, Some(pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
}

#[test]
fn conforms_to_pdp_spec_1_7_signature_rejected_no_key_binding() {
    let agent_id = [0xAA; 32];
    let request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    let ctx = make_ctx(50, 1000, 0, None);
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
}

#[test]
fn conforms_to_pdp_spec_1_7_replay_protection_duplicate_plan_id() {
    let agent_id = [0xAA; 32];
    let plan_id = [0x42; 32];
    let (pk, sk_seed) = test_keypair();
    let mut request = make_request(plan_id, agent_id, ActionType::ClaimTaskLease, 1, 100);
    let msg = hash_action_plan_for_signing(&request);
    let seed = ml_dsa::Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    let mut ctx = make_ctx(50, 1000, 0, Some(pk));
    ctx.consumed_plan_ids = vec![plan_id];
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
}

#[test]
fn conforms_to_pdp_spec_1_7_replay_wrong_nonce_rejected() {
    let agent_id = [0xAA; 32];
    let (pk, sk_seed) = test_keypair();
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 5, 100);
    let msg = hash_action_plan_for_signing(&request);
    let seed = ml_dsa::Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    let ctx = make_ctx(50, 1000, 3, Some(pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
}

#[test]
fn conforms_to_pdp_spec_1_7_ttl_expired_rejected() {
    let agent_id = [0xAA; 32];
    let (pk, sk_seed) = test_keypair();
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 50);
    let msg = hash_action_plan_for_signing(&request);
    let seed = ml_dsa::Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    let ctx = make_ctx(100, 1000, 0, Some(pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::TTLExpired));
}

#[test]
fn conforms_to_pdp_spec_1_7_quota_exhaustion_rejected() {
    let agent_id = [0xAA; 32];
    let (pk, sk_seed) = test_keypair();
    let mut request =
        make_request([1u8; 32], agent_id, ActionType::SubmitGovernanceProposal, 1, 100);
    let msg = hash_action_plan_for_signing(&request);
    let seed = ml_dsa::Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    let mut ctx = make_ctx(50, 1000, 0, Some(pk));
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
    let agent_id = [0xAA; 32];
    let (pk, sk_seed) = test_keypair();
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    let msg = hash_action_plan_for_signing(&request);
    let seed = ml_dsa::Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    let ctx = make_ctx(50, 0, 0, Some(pk));
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::InsufficientFunds));
}

#[test]
fn conforms_to_pdp_spec_1_7_full_chain_approval() {
    let agent_id = [0xAA; 32];
    let (pk, sk_seed) = test_keypair();
    let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    let msg = hash_action_plan_for_signing(&request);
    let seed = ml_dsa::Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = ml_dsa::SigningKey::<ml_dsa::MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    let ctx = make_ctx(50, 1000, 0, Some(pk));
    let result = evaluate(&request, &ctx);
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
    let result =
        qm.reserve_quota("gov_proposals_per_identity", "agent1", TrustStage::Trusted, 1, 0);
    assert!(result.is_ok());
    let result2 =
        qm.reserve_quota("gov_proposals_per_identity", "agent1", TrustStage::Trusted, 1, 0);
    assert!(result2.is_err());
}

#[test]
fn conforms_to_pdp_spec_2_7_quota_release_after_failure() {
    let mut qm = QuotaManager::with_canonical_entries();
    qm.reserve_quota("gov_proposals_per_identity", "agent1", TrustStage::Trusted, 1, 0).unwrap();
    qm.release_quota("gov_proposals_per_identity", "agent1", 1);
    let result =
        qm.reserve_quota("gov_proposals_per_identity", "agent1", TrustStage::Trusted, 1, 0);
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
