// Conformance tests for policy-engine-spec.md Sections 1.7, 2.7, 3.7
//
// Source: docs/04-specifications/runtime/policy-engine-spec.md

use hyperfluid_pdp::audit::AuditLog;
use hyperfluid_pdp::quota::QuotaManager;
use hyperfluid_pdp::rule_chain::evaluate;
use hyperfluid_pdp::types::{
    ActionPlanRequest, ActionPlanResponse, ActionType, Decision, DenyReason, Hash32, PdpContext,
    QuotaState, TrustStage,
};
use ml_dsa::{Generate, Keypair, MlDsa65, Seed, SignatureEncoding, Signer, SigningKey};
use sha3::{Digest, Sha3_256};

#[allow(dead_code)]
fn sha3_256_bytes(data: &[u8]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[allow(dead_code)]
fn test_keypair() -> (Vec<u8>, [u8; 32]) {
    let sk = SigningKey::<MlDsa65>::generate();
    let pk = sk.verifying_key().encode().as_slice().to_vec();
    let seed = sk.to_seed();
    let mut seed_bytes = [0u8; 32];
    seed_bytes.copy_from_slice(seed.as_slice());
    (pk, seed_bytes)
}

#[allow(dead_code)]
fn sign_request(request: &ActionPlanRequest, sk_seed: &[u8; 32]) -> Vec<u8> {
    let seed = Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = SigningKey::<MlDsa65>::from_seed(&seed);
    let msg = hash_action_plan_for_signing(request);
    let sig = sk.sign(&msg);
    sig.to_vec()
}

#[allow(dead_code)]
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

fn make_ctx(height: u64, balance: u128, nonce: u64) -> PdpContext {
    PdpContext {
        current_height: height,
        key_binding: None,
        agent_balance_attagx: balance,
        agent_nonce: nonce,
        consumed_plan_ids: vec![],
        quota_states: vec![],
        trust_stage: TrustStage::Trusted,
    }
}

// ── Section 1.7: Deterministic Policy Evaluation ────────────────────────

#[test]
fn conforms_to_pdp_spec_1_7_deterministic_evaluation() {
    let request = make_request([1u8; 32], [0xAA; 32], ActionType::ClaimTaskLease, 1, 100);
    let ctx = make_ctx(50, 1000, 0);

    let r1 = evaluate(&request, &ctx);
    let r2 = evaluate(&request, &ctx);
    assert_eq!(r1.decision, r2.decision);
    assert_eq!(r1.deny_reason, r2.deny_reason);
}

#[test]
fn conforms_to_pdp_spec_1_7_schema_violation_rejected() {
    let request = make_request([0u8; 32], [1u8; 32], ActionType::ClaimTaskLease, 1, 100);
    let ctx = make_ctx(0, 1000, 0);
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
}

#[test]
fn conforms_to_pdp_spec_1_7_replay_protection_duplicate_plan_id() {
    let agent_id = [0xAA; 32];
    let plan_id = [0x42; 32];
    let request = make_request(plan_id, agent_id, ActionType::ClaimTaskLease, 1, 100);
    let mut ctx = make_ctx(50, 1000, 0);
    ctx.consumed_plan_ids = vec![plan_id];
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
}

#[test]
fn conforms_to_pdp_spec_1_7_replay_wrong_nonce_rejected() {
    let agent_id = [0xAA; 32];
    let request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 5, 100);
    let ctx = make_ctx(50, 1000, 3);
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
}

#[test]
fn conforms_to_pdp_spec_1_7_ttl_expired_rejected() {
    let agent_id = [0xAA; 32];
    let request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 50);
    let ctx = make_ctx(100, 1000, 0);
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::TTLExpired));
}

#[test]
fn conforms_to_pdp_spec_1_7_quota_exhaustion_rejected() {
    let agent_id = [0xAA; 32];
    let request = make_request([1u8; 32], agent_id, ActionType::SubmitGovernanceProposal, 1, 100);
    let mut ctx = make_ctx(50, 1000, 0);
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
    let request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    let ctx = make_ctx(50, 0, 0);
    let result = evaluate(&request, &ctx);
    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::InsufficientFunds));
}

#[test]
fn conforms_to_pdp_spec_1_7_full_chain_approval() {
    let agent_id = [0xAA; 32];
    let request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
    let ctx = make_ctx(50, 1000, 0);
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
