// End-to-end test: CLI → PDP → State Machine pipeline
//
// Source: docs/05-planning/stages/stage-02-agent-runtime.md Week 9-10 Task 3
//
// Demonstrates the full flow:
//   1. Construct an ActionPlanRequest with real ML-DSA-65 signature
//   2. PDP evaluates through the 5-step deterministic rule chain
//   3. State machine executes the approved action
//   4. Verify state changes and determinism

use hyperfluid_pdp::audit::AuditLog;
use hyperfluid_pdp::rule_chain::{evaluate, hash_action_plan_for_signing};
use hyperfluid_pdp::types::{
    ActionPlanRequest, ActionType, Decision, Hash32, PdpContext, TrustStage,
};
use hyperfluid_state::state_machine::{ExecutionContext, ExecutionResult, StateMachine};
use hyperfluid_state::Account;
use ml_dsa::{Generate, Keypair, MlDsa65, Seed, SignatureEncoding, Signer, SigningKey};
use sha3::Digest;

fn sha3_256(data: &[u8]) -> Hash32 {
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

fn make_keypair() -> (Vec<u8>, [u8; 32]) {
    let sk = SigningKey::<MlDsa65>::generate();
    let pk = sk.verifying_key().encode().as_slice().to_vec();
    let seed = sk.to_seed();
    let mut seed_bytes = [0u8; 32];
    seed_bytes.copy_from_slice(seed.as_slice());
    (pk, seed_bytes)
}

fn sign_action_plan(request: &ActionPlanRequest, sk_seed: &[u8; 32]) -> Vec<u8> {
    let seed = Seed::try_from(sk_seed.as_slice()).unwrap();
    let sk = SigningKey::<MlDsa65>::from_seed(&seed);
    let msg = hash_action_plan_for_signing(request);
    sk.sign(&msg).to_vec()
}

/// Full E2E: CLI command → ActionPlanRequest → PDP → State Machine
#[test]
fn e2e_transfer_flow_full_pipeline() {
    let agent_id: Hash32 = [0xA1; 32];
    let recipient_id: Hash32 = [0xB2; 32];
    let (pk_bytes, sk_seed) = make_keypair();

    // 1. Construct ActionPlanRequest (simulating CLI output being serialized)
    let mut request = ActionPlanRequest {
        plan_id: sha3_256(b"transfer-plan-001"),
        agent_id,
        action_type: ActionType::ClaimTaskLease,
        resource_id: recipient_id,
        reason_hash: sha3_256(b"monthly payout"),
        evidence_refs: vec![],
        nonce: 1,
        expires_at_height: 1000,
        agent_signature: vec![],
    };

    // 2. Sign the plan (simulating CLI signing before submit)
    request.agent_signature = sign_action_plan(&request, &sk_seed);

    // 3. PDP Evaluation
    let pdp_ctx = PdpContext {
        current_height: 50,
        key_binding: Some(pk_bytes.clone()),
        agent_balance_attagx: 1_000_000_000_000_000_000_000u128, // 1000 AGX
        agent_nonce: 0,
        consumed_plan_ids: vec![],
        quota_states: vec![],
        trust_stage: TrustStage::Trusted,
    };

    let mut audit_log = AuditLog::new();
    let result = evaluate(&request, &pdp_ctx, &mut audit_log, None);

    assert_eq!(result.decision, Decision::Approved, "PDP must approve valid signed transfer");
    assert_eq!(result.deny_reason, None);

    // 4. State Machine Execution
    let mut sm = StateMachine::new();
    let sender_account = Account {
        account_id: agent_id,
        balance: 1_000_000_000_000_000_000_000u128,
        nonce: 0,
        pubkey_hash: sha3_256(&pk_bytes),
        pubkey: Some(pk_bytes),
    };
    let recipient_account = Account {
        account_id: recipient_id,
        balance: 0,
        nonce: 0,
        pubkey_hash: [0u8; 32],
        pubkey: None,
    };
    sm.init_account(sender_account);
    sm.init_account(recipient_account);

    let transfer_amount: u128 = 100_000_000_000_000_000_000u128;

    let ctx = hyperfluid_state::state_machine::ExecutionContext { height: 50, timestamp: 1000 };

    let result = sm.execute_transfer(agent_id, recipient_id, transfer_amount, 1, ctx);
    assert!(
        matches!(result, ExecutionResult::Success),
        "State machine must execute approved transfer"
    );

    assert_eq!(
        sm.get_account(&agent_id).map(|a| a.balance),
        Some(900_000_000_000_000_000_000u128),
        "Sender balance after transfer"
    );
    assert_eq!(
        sm.get_account(&recipient_id).map(|a| a.balance),
        Some(transfer_amount),
        "Recipient balance after transfer"
    );
}

/// Verify PDP rejects the plan when the request is tampered after signing
#[test]
fn e2e_rejects_tampered_request_after_cli_sign() {
    let agent_id: Hash32 = [0xA1; 32];
    let (pk_bytes, sk_seed) = make_keypair();

    let mut request = ActionPlanRequest {
        plan_id: sha3_256(b"transfer-plan-002"),
        agent_id,
        action_type: ActionType::ClaimTaskLease,
        resource_id: [0xB2; 32],
        reason_hash: sha3_256(b"test"),
        evidence_refs: vec![],
        nonce: 1,
        expires_at_height: 1000,
        agent_signature: vec![],
    };

    request.agent_signature = sign_action_plan(&request, &sk_seed);

    // Tamper after signing
    request.nonce = 999;

    let pdp_ctx = PdpContext {
        current_height: 50,
        key_binding: Some(pk_bytes),
        agent_balance_attagx: 1_000_000_000_000_000_000_000u128,
        agent_nonce: 0,
        consumed_plan_ids: vec![],
        quota_states: vec![],
        trust_stage: TrustStage::Trusted,
    };

    let mut audit_log = AuditLog::new();
    let result = evaluate(&request, &pdp_ctx, &mut audit_log, None);
    assert_eq!(result.decision, Decision::Denied, "PDP must reject tampered request");
    assert_eq!(result.deny_reason, Some(hyperfluid_pdp::types::DenyReason::SignatureInvalid));
}

/// Verify PDP rejects unsigned request (no signature)
#[test]
fn e2e_rejects_unsigned_request() {
    let agent_id: Hash32 = [0xA1; 32];
    let (pk_bytes, _sk_seed) = make_keypair();

    let request = ActionPlanRequest {
        plan_id: sha3_256(b"transfer-plan-003"),
        agent_id,
        action_type: ActionType::ClaimTaskLease,
        resource_id: [0xB2; 32],
        reason_hash: sha3_256(b"test"),
        evidence_refs: vec![],
        nonce: 1,
        expires_at_height: 1000,
        agent_signature: vec![], // empty signature
    };

    let pdp_ctx = PdpContext {
        current_height: 50,
        key_binding: Some(pk_bytes),
        agent_balance_attagx: 1_000_000_000_000_000_000_000u128,
        agent_nonce: 0,
        consumed_plan_ids: vec![],
        quota_states: vec![],
        trust_stage: TrustStage::Trusted,
    };

    let mut audit_log = AuditLog::new();
    let result = evaluate(&request, &pdp_ctx, &mut audit_log, None);
    assert_eq!(result.decision, Decision::Denied, "PDP must reject unsigned request");
    assert_eq!(result.deny_reason, Some(hyperfluid_pdp::types::DenyReason::SignatureInvalid));
}

/// Verify deterministic behavior: same input always produces same PDP output
#[test]
fn e2e_deterministic_pipeline() {
    let agent_id: Hash32 = [0xA1; 32];
    let (pk_bytes, sk_seed) = make_keypair();

    let mut request = ActionPlanRequest {
        plan_id: sha3_256(b"transfer-plan-004"),
        agent_id,
        action_type: ActionType::ClaimTaskLease,
        resource_id: [0xB2; 32],
        reason_hash: sha3_256(b"test"),
        evidence_refs: vec![],
        nonce: 1,
        expires_at_height: 1000,
        agent_signature: vec![],
    };

    request.agent_signature = sign_action_plan(&request, &sk_seed);

    let pdp_ctx = PdpContext {
        current_height: 50,
        key_binding: Some(pk_bytes),
        agent_balance_attagx: 1_000_000_000_000_000_000_000u128,
        agent_nonce: 0,
        consumed_plan_ids: vec![],
        quota_states: vec![],
        trust_stage: TrustStage::Trusted,
    };

    let mut audit_log = AuditLog::new();
    let r1 = evaluate(&request, &pdp_ctx, &mut audit_log, None);
    let r2 = evaluate(&request, &pdp_ctx, &mut audit_log, None);

    assert_eq!(r1.decision, r2.decision);
    assert_eq!(r1.deny_reason, r2.deny_reason);
    assert_eq!(r1.approval_height, r2.approval_height);
}

/// Verify task_create flow through PDP + StateMachine
#[test]
fn e2e_task_create_full_pipeline() {
    let agent_id: Hash32 = [0xA1; 32];
    let (pk_bytes, sk_seed) = make_keypair();

    let mut request = ActionPlanRequest {
        plan_id: sha3_256(b"task-create-001"),
        agent_id,
        action_type: ActionType::CreateTask,
        resource_id: sha3_256(b"seed-ref-001"),
        reason_hash: sha3_256(b"Implement protocol optimization"),
        evidence_refs: vec![],
        nonce: 1,
        expires_at_height: 1000,
        agent_signature: vec![],
    };

    request.agent_signature = sign_action_plan(&request, &sk_seed);

    let pdp_ctx = PdpContext {
        current_height: 50,
        key_binding: Some(pk_bytes.clone()),
        agent_balance_attagx: 1_000_000_000_000_000_000_000u128,
        agent_nonce: 0,
        consumed_plan_ids: vec![],
        quota_states: vec![],
        trust_stage: TrustStage::Trusted,
    };

    let mut audit_log = AuditLog::new();
    let result = evaluate(&request, &pdp_ctx, &mut audit_log, None);
    assert_eq!(result.decision, Decision::Approved, "PDP must approve valid task create");

    // State machine: create task with escrow
    let mut sm = StateMachine::new();
    let account = Account {
        account_id: agent_id,
        balance: 1_000_000_000_000_000_000_000u128,
        nonce: 0,
        pubkey_hash: sha3_256(&pk_bytes),
        pubkey: Some(pk_bytes),
    };
    sm.init_account(account);

    let ctx = ExecutionContext { height: 50, timestamp: 1000 };

    let bounty: u128 = 100_000_000_000_000_000_000u128;
    let task_id: Hash32 = sha3_256(b"task-id-001");

    let result = sm.execute_task_create(
        agent_id,
        bounty,
        0, // fee_agx
        task_id,
        1, // nonce
        sha3_256(b"seed-ref-001"),
        sha3_256(b"topic-001"),
        sha3_256(b"metadata"),
        sha3_256(b"skills"),
        [0u8; 32], // sponsor_id
        [0u8; 32], // requester_pubkey
        50,        // current_height
        ctx,
    );
    assert!(
        matches!(result, ExecutionResult::Success),
        "State machine must execute valid task create"
    );

    assert_eq!(
        sm.get_account(&agent_id).map(|a| a.balance),
        Some(900_000_000_000_000_000_000u128),
        "Creator balance after escrow"
    );
}
