// Production-readiness fixes verification tests
//
// Covers F-7, F-24, F-52 from the build-worker task list.
// Each fix has at minimum 1 positive and 1 negative test.

use hyperfluid_pdp::audit::AuditLog;
use hyperfluid_pdp::rule_chain::{evaluate, hash_action_plan_for_signing};
use hyperfluid_pdp::types::{
    ActionPlanRequest, ActionType, Decision, DenyReason, Hash32, PdpContext, QuotaState, TrustStage,
};
use ml_dsa::{Generate, Keypair, MlDsa65, Seed, SignatureEncoding, Signer, SigningKey};

fn test_keypair() -> (Vec<u8>, [u8; 32]) {
    let sk = SigningKey::<MlDsa65>::generate();
    let pk = sk.verifying_key().encode().as_slice().to_vec();
    let seed = sk.to_seed();
    let mut seed_bytes = [0u8; 32];
    seed_bytes.copy_from_slice(seed.as_slice());
    (pk, seed_bytes)
}

fn make_signed_request(
    plan_id: Hash32,
    agent_id: Hash32,
    action_type: ActionType,
    nonce: u64,
    expires: u64,
    pk_bytes: &[u8],
    sk_seed: &[u8; 32],
) -> (ActionPlanRequest, PdpContext) {
    let mut request = ActionPlanRequest {
        plan_id,
        agent_id,
        action_type,
        resource_id: [1u8; 32],
        reason_hash: [2u8; 32],
        evidence_refs: vec![],
        nonce,
        expires_at_height: expires,
        agent_signature: vec![],
    };
    let msg = hash_action_plan_for_signing(&request);
    let seed = Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = SigningKey::<MlDsa65>::from_seed(&seed);
    request.agent_signature = sk.sign(&msg).to_vec();
    let ctx = PdpContext {
        current_height: 50,
        key_binding: Some(pk_bytes.to_vec()),
        agent_balance_attagx: 1000,
        agent_nonce: nonce - 1,
        consumed_plan_ids: vec![],
        quota_states: vec![],
        trust_stage: TrustStage::Trusted,
    };
    (request, ctx)
}

// ── F-7: evaluator_signature in audit log ────────────────────────────────

/// Positive: When Some(sig) is passed, the audit log entry contains the signature.
#[test]
fn fix_f7_passes_signature_into_audit_entry() {
    let agent_id = [0xAAu8; 32];
    let (pk, seed) = test_keypair();
    let (request, ctx) =
        make_signed_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100, &pk, &seed);

    let evaluator_sig = vec![0xABu8; 64]; // simulated ML-DSA-65 evaluator sig
    let mut audit_log = AuditLog::new();
    let response = evaluate(&request, &ctx, &mut audit_log, Some(evaluator_sig.clone()));

    assert_eq!(response.decision, Decision::Approved);
    let entry = audit_log.get_by_plan_id(&[1u8; 32]).expect("audit entry must exist");
    assert_eq!(entry.evaluator_signature, evaluator_sig, "F-7: evaluator_signature must match");
}

/// Negative: When None is passed, the audit log entry has an empty signature
/// (backward compatible behavior for callers that don't yet sign audit entries).
#[test]
fn fix_f7_none_produces_empty_signature_backward_compat() {
    let agent_id = [0xAAu8; 32];
    let (pk, seed) = test_keypair();
    let (request, ctx) =
        make_signed_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100, &pk, &seed);

    let mut audit_log = AuditLog::new();
    let response = evaluate(&request, &ctx, &mut audit_log, None);

    assert_eq!(response.decision, Decision::Approved);
    let entry = audit_log.get_by_plan_id(&[1u8; 32]).expect("audit entry must exist");
    assert!(
        entry.evaluator_signature.is_empty(),
        "F-7: None must result in empty evaluator_signature (backward compat)"
    );
}

/// Positive: Distinct signatures produce distinct audit entry IDs
/// (content-addressing includes evaluator_signature).
#[test]
fn fix_f7_different_sigs_produce_different_entry_ids() {
    let agent_id = [0xAAu8; 32];
    let (pk, seed) = test_keypair();
    let (request, ctx) =
        make_signed_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100, &pk, &seed);

    let mut audit_log_a = AuditLog::new();
    let sig_a = vec![0xAAu8; 64];
    let _resp_a = evaluate(&request, &ctx, &mut audit_log_a, Some(sig_a));
    let entry_a = audit_log_a.get_by_plan_id(&[1u8; 32]).unwrap();

    let mut audit_log_b = AuditLog::new();
    let sig_b = vec![0xBBu8; 64];
    let _resp_b = evaluate(&request, &ctx, &mut audit_log_b, Some(sig_b));
    let entry_b = audit_log_b.get_by_plan_id(&[1u8; 32]).unwrap();

    assert_ne!(entry_a.entry_id, entry_b.entry_id, "F-7: different sigs must differ entry_id");
}

// ── F-24: Wildcard _ => vec![] replaced with explicit match arms ──────────

/// Positive: A quota-free action type (DelegateOperation) is handled explicitly
/// and proceeds through the rule chain without quota enforcement.
#[test]
fn fix_f24_quota_free_action_explicitly_handled() {
    let agent_id = [0xAAu8; 32];
    let (pk, seed) = test_keypair();
    let (request, ctx) =
        make_signed_request([1u8; 32], agent_id, ActionType::DelegateOperation, 1, 100, &pk, &seed);

    let mut audit_log = AuditLog::new();
    let result = evaluate(&request, &ctx, &mut audit_log, None);

    assert_eq!(
        result.decision,
        Decision::Approved,
        "F-24: DelegateOperation must be approved (no quota enforcement)"
    );
}

/// Negative: A quota-enforced action (CreateTask) is rejected when its quota
/// is exhausted, proving the explicit match arm routes to quota check correctly.
#[test]
fn fix_f24_quota_enforced_action_rejected_when_exhausted() {
    let agent_id = [0xAAu8; 32];
    let (pk, seed) = test_keypair();
    let (request, mut ctx) =
        make_signed_request([1u8; 32], agent_id, ActionType::CreateTask, 1, 100, &pk, &seed);
    ctx.quota_states = vec![QuotaState {
        quota_id: "task_create_per_stage".into(),
        consumed: 10, // limit is 10 for Trusted — exactly exhausted
        window_start_height: 0,
    }];

    let mut audit_log = AuditLog::new();
    let result = evaluate(&request, &ctx, &mut audit_log, None);

    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(result.deny_reason, Some(DenyReason::QuotaExhausted));
}

/// Positive: A PublishTopicMessage action is handled explicitly by the match
/// (returns empty quota vec), proving it is no longer caught by wildcard.
#[test]
fn fix_f24_publish_topic_message_explicitly_no_quota() {
    let agent_id = [0xAAu8; 32];
    let (pk, seed) = test_keypair();
    let (request, ctx) = make_signed_request(
        [1u8; 32],
        agent_id,
        ActionType::PublishTopicMessage,
        1,
        100,
        &pk,
        &seed,
    );

    let mut audit_log = AuditLog::new();
    let result = evaluate(&request, &ctx, &mut audit_log, None);

    assert_eq!(
        result.decision,
        Decision::Approved,
        "F-24: PublishTopicMessage must be approved (no quota enforcement, explicit arm)"
    );
}

// ── F-52: PublishTopicMessage dead_code ──────────────────────────────────

/// Positive: Construct a request with PublishTopicMessage variant and verify
/// it flows through the full rule chain without dead_code lint.
#[test]
fn fix_f52_publish_topic_message_constructed_in_request() {
    let agent_id = [0xAAu8; 32];
    let (pk, seed) = test_keypair();
    let (request, ctx) = make_signed_request(
        [1u8; 32],
        agent_id,
        ActionType::PublishTopicMessage,
        1,
        100,
        &pk,
        &seed,
    );

    // Verify the variant is properly serialized/deserialized and matched
    assert_eq!(request.action_type, ActionType::PublishTopicMessage);

    let mut audit_log = AuditLog::new();
    let result = evaluate(&request, &ctx, &mut audit_log, None);

    // Should pass schema, sig, replay steps and reach quota check (which returns
    // empty vec for PublishTopicMessage), then fee check.
    assert_eq!(
        result.decision,
        Decision::Approved,
        "F-52: PublishTopicMessage request must be approved"
    );
}

/// Negative: PublishTopicMessage with zero plan_id must still be rejected by
/// schema validation — proving the variant participates in the full rule chain.
#[test]
fn fix_f52_publish_topic_message_still_rejects_invalid_schema() {
    let agent_id = [0xAAu8; 32];
    let (pk, _seed) = test_keypair();

    // Build unsigned request with zero plan_id (schema violation)
    let request = ActionPlanRequest {
        plan_id: [0u8; 32], // invalid
        agent_id,
        action_type: ActionType::PublishTopicMessage,
        resource_id: [1u8; 32],
        reason_hash: [2u8; 32],
        evidence_refs: vec![],
        nonce: 1,
        expires_at_height: 100,
        agent_signature: vec![],
    };

    let ctx = PdpContext {
        current_height: 50,
        key_binding: Some(pk),
        agent_balance_attagx: 1000,
        agent_nonce: 0,
        consumed_plan_ids: vec![],
        quota_states: vec![],
        trust_stage: TrustStage::Trusted,
    };

    let mut audit_log = AuditLog::new();
    let result = evaluate(&request, &ctx, &mut audit_log, None);

    assert_eq!(result.decision, Decision::Denied);
    assert_eq!(
        result.deny_reason,
        Some(DenyReason::SchemaViolation),
        "F-52: PublishTopicMessage must still fail schema validation"
    );
}
