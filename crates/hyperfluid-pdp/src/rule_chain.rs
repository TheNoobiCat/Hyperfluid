// === 5-Step Deterministic PDP Rule Chain ===
//
// Source: policy-engine-spec.md §1.4 State Transitions
//
// The PDP evaluates action plans through an ordered 5-step chain:
//   1. Schema validation
//   2. Signature verification (ML-DSA-65 with key rotation support)
//   3. Replay protection (plan_id dedup, nonce monotonic, TTL)
//   4. Quota check (cross-layer quota matrix, atomic reservation)
//   5. Fee check (sufficient balance for EIP-1559 tx fee)
//
// Early exit on first failure with structured deny reason code.

use crate::error::PdpError;
use crate::types::{
    ActionPlanRequest, ActionPlanResponse, ActionType, Decision, Hash32, PdpContext,
    QuotaConsumption, QuotaEntry,
};
use ml_dsa::{EncodedVerifyingKey, MlDsa65, Verifier, VerifyingKey};
use sha3::{Digest, Sha3_256};

/// Evaluates an action plan through the deterministic 5-step rule chain.
pub fn evaluate(request: &ActionPlanRequest, ctx: &PdpContext) -> ActionPlanResponse {
    let mut response = ActionPlanResponse {
        plan_id: request.plan_id,
        decision: Decision::Denied,
        deny_reason: None,
        consumed_quota: None,
        approval_height: ctx.current_height,
        expires_at_height: request.expires_at_height,
    };

    let result = run_chain(request, ctx);
    match result {
        Ok(consumed_quota) => {
            response.decision = Decision::Approved;
            response.consumed_quota = consumed_quota;
        }
        Err(e) => {
            response.deny_reason = Some(e.deny_reason());
        }
    }

    response
}

fn run_chain(
    request: &ActionPlanRequest,
    ctx: &PdpContext,
) -> Result<Option<Vec<QuotaConsumption>>, PdpError> {
    step1_schema_validation(request)?;
    step2_signature_verification(request, ctx)?;
    step3_replay_protection(request, ctx)?;
    let quota_consumption = step4_quota_check(request, ctx)?;
    step5_fee_check(request, ctx)?;

    Ok(quota_consumption)
}

// ── Step 1: Schema Validation ─────────────────────────────────────────────

fn step1_schema_validation(request: &ActionPlanRequest) -> Result<(), PdpError> {
    if request.plan_id == [0u8; 32] {
        return Err(PdpError::SchemaViolation("plan_id must be non-zero".into()));
    }
    if request.agent_id == [0u8; 32] {
        return Err(PdpError::SchemaViolation("agent_id must be non-zero".into()));
    }
    if request.agent_signature.is_empty() {
        return Err(PdpError::SchemaViolation("agent_signature must not be empty".into()));
    }
    if request.expires_at_height == 0 {
        return Err(PdpError::SchemaViolation("expires_at_height must be non-zero".into()));
    }

    Ok(())
}

// ── Step 2: Signature Verification ────────────────────────────────────────

fn step2_signature_verification(
    request: &ActionPlanRequest,
    ctx: &PdpContext,
) -> Result<(), PdpError> {
    let binding = ctx.key_binding.as_ref().ok_or(PdpError::SignatureVerificationFailed)?;

    let message = hash_action_plan_for_signing(request);

    // During grace window, accept either active or pending key
    if binding.in_grace_window(ctx.current_height) {
        if let Some(ref pending_pk) = binding.pending_pubkey {
            if verify_ml_dsa(&message, &request.agent_signature, pending_pk).is_ok() {
                return Ok(());
            }
        }
    }

    // Default: verify against active key
    verify_ml_dsa(&message, &request.agent_signature, &binding.active_pubkey)
        .map_err(|_| PdpError::SignatureVerificationFailed)
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

fn verify_ml_dsa(message: &Hash32, signature: &[u8], pubkey: &[u8]) -> Result<(), ()> {
    let vk_enc = EncodedVerifyingKey::<MlDsa65>::try_from(pubkey).map_err(|_| ())?;
    let vk = VerifyingKey::<MlDsa65>::decode(&vk_enc);
    let sig = ml_dsa::Signature::<MlDsa65>::try_from(signature).map_err(|_| ())?;
    vk.verify(message, &sig).map_err(|_| ())
}

// ── Step 3: Replay Protection ─────────────────────────────────────────────

fn step3_replay_protection(request: &ActionPlanRequest, ctx: &PdpContext) -> Result<(), PdpError> {
    if ctx.consumed_plan_ids.contains(&request.plan_id) {
        return Err(PdpError::ReplayDetected { plan_id: request.plan_id });
    }

    if request.nonce != ctx.agent_nonce + 1 {
        return Err(PdpError::ReplayDetected { plan_id: request.plan_id });
    }

    if request.expires_at_height <= ctx.current_height {
        return Err(PdpError::TTLExpired {
            expires_at: request.expires_at_height,
            current: ctx.current_height,
        });
    }

    if request.expires_at_height > ctx.current_height + 10000 {
        return Err(PdpError::TTLExpired {
            expires_at: request.expires_at_height,
            current: ctx.current_height,
        });
    }

    Ok(())
}

// ── Step 4: Quota Check ───────────────────────────────────────────────────

fn step4_quota_check(
    request: &ActionPlanRequest,
    ctx: &PdpContext,
) -> Result<Option<Vec<QuotaConsumption>>, PdpError> {
    let relevant_quota_ids = quota_ids_for_action(request.action_type);
    if relevant_quota_ids.is_empty() {
        return Ok(None);
    }

    let mut consumption = Vec::new();

    for quota_id in &relevant_quota_ids {
        let entry = get_quota_entry(quota_id);
        let state = ctx.quota_states.iter().find(|qs| qs.quota_id == *quota_id);

        let consumed = state.map(|s| s.consumed).unwrap_or(0);
        let after = consumed.saturating_add(1);

        if after > entry.limit {
            return Err(PdpError::QuotaExhausted { quota_id: quota_id.clone() });
        }

        consumption.push(QuotaConsumption {
            quota_id: quota_id.clone(),
            amount_consumed: 1,
            remaining: entry.limit.saturating_sub(after),
        });
    }

    Ok(Some(consumption))
}

fn quota_ids_for_action(action_type: ActionType) -> Vec<String> {
    match action_type {
        ActionType::SubmitGovernanceProposal | ActionType::CastGovernanceVote => {
            vec!["gov_proposals_per_identity".to_string(), "gov_open_proposals_global".to_string()]
        }
        ActionType::SubmitFastPathMerge => {
            vec!["fast_merge_per_topic".to_string(), "fast_merge_per_identity".to_string()]
        }
        ActionType::ClaimTaskLease | ActionType::RenewTaskLease => {
            vec!["lease_active_per_agent".to_string()]
        }
        ActionType::CreateTask => vec!["task_create_per_stage".to_string()],
        _ => vec![],
    }
}

/// Returns the canonical quota entry for a given quota ID.
/// Source: policy-engine-spec.md §2.4
fn get_quota_entry(quota_id: &str) -> QuotaEntry {
    use crate::types::TrustStage;
    match quota_id {
        "p2p_conn_per_identity" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "p2p_ingress".into(),
            dimension: "per_identity".into(),
            limit: 50,
            window_blocks: 0,
            stage_multipliers: vec![
                (TrustStage::Untrusted, (10, 50)),
                (TrustStage::Trusted, (50, 50)),
            ],
        },
        "p2p_tx_burst" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "p2p_ingress".into(),
            dimension: "per_identity".into(),
            limit: 20,
            window_blocks: 360,
            stage_multipliers: vec![],
        },
        "p2p_gossip_budget" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "p2p_gossip".into(),
            dimension: "per_sender".into(),
            limit: 100,
            window_blocks: 60,
            stage_multipliers: vec![],
        },
        "inbox_msg_per_sender" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "inbox_router".into(),
            dimension: "per_sender".into(),
            limit: 60,
            window_blocks: 60,
            stage_multipliers: vec![
                (TrustStage::Untrusted, (5, 60)),
                (TrustStage::Trusted, (60, 60)),
            ],
        },
        "inbox_global_per_agent" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "inbox_router".into(),
            dimension: "per_agent".into(),
            limit: 2000,
            window_blocks: 3600,
            stage_multipliers: vec![],
        },
        "topic_msg_global" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "topic_router".into(),
            dimension: "per_topic".into(),
            limit: 500,
            window_blocks: 300,
            stage_multipliers: vec![],
        },
        "fast_merge_per_topic" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "fast_path".into(),
            dimension: "per_topic".into(),
            limit: 20,
            window_blocks: 3600,
            stage_multipliers: vec![],
        },
        "fast_merge_per_identity" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "fast_path".into(),
            dimension: "per_identity".into(),
            limit: 5,
            window_blocks: 3600,
            stage_multipliers: vec![],
        },
        "gov_proposals_per_identity" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "governance".into(),
            dimension: "per_identity".into(),
            limit: 1,
            window_blocks: 8192,
            stage_multipliers: vec![],
        },
        "gov_open_proposals_global" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "governance".into(),
            dimension: "network_wide".into(),
            limit: 32,
            window_blocks: 0,
            stage_multipliers: vec![],
        },
        "review_concurrent_per_reviewer" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "review_assignment".into(),
            dimension: "per_reviewer".into(),
            limit: 5,
            window_blocks: 0,
            stage_multipliers: vec![],
        },
        "lease_active_per_agent" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "task_board".into(),
            dimension: "per_agent".into(),
            limit: 6,
            window_blocks: 0,
            stage_multipliers: vec![(TrustStage::Untrusted, (2, 6)), (TrustStage::Trusted, (6, 6))],
        },
        "challenge_per_identity" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "challenge".into(),
            dimension: "per_identity".into(),
            limit: 3,
            window_blocks: 8192,
            stage_multipliers: vec![],
        },
        "task_create_per_stage" => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "pdp".into(),
            dimension: "per_agent".into(),
            limit: 10,
            window_blocks: 0,
            stage_multipliers: vec![
                (TrustStage::Untrusted, (0, 10)),
                (TrustStage::Trusted, (10, 10)),
            ],
        },
        _ => QuotaEntry {
            quota_id: quota_id.into(),
            enforcement_point: "unknown".into(),
            dimension: "unknown".into(),
            limit: u64::MAX,
            window_blocks: 0,
            stage_multipliers: vec![],
        },
    }
}

// ── Step 5: Fee Check ─────────────────────────────────────────────────────

fn step5_fee_check(_request: &ActionPlanRequest, ctx: &PdpContext) -> Result<(), PdpError> {
    const MIN_TX_FEE_ATTAGX: u128 = 1;
    if ctx.agent_balance_attagx < MIN_TX_FEE_ATTAGX {
        return Err(PdpError::InsufficientFunds {
            balance: ctx.agent_balance_attagx,
            required: MIN_TX_FEE_ATTAGX,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DenyReason, KeyBinding, QuotaState};
    use ml_dsa::{Generate, Keypair, Seed, SignatureEncoding, Signer, SigningKey};

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

    #[test]
    fn step1_rejects_zero_plan_id() {
        let request = make_request([0u8; 32], [1u8; 32], ActionType::ClaimTaskLease, 1, 100);
        let (pk, _sk) = test_keypair();
        let ctx = make_ctx(0, 1000, 0, KeyBinding::stable([1u8; 32], pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    #[test]
    fn step1_rejects_zero_agent_id() {
        let request = make_request([1u8; 32], [0u8; 32], ActionType::ClaimTaskLease, 1, 100);
        let (pk, _sk) = test_keypair();
        let ctx = make_ctx(0, 1000, 0, KeyBinding::stable([0u8; 32], pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    #[test]
    fn step1_rejects_empty_signature() {
        let request = make_request([1u8; 32], [2u8; 32], ActionType::ClaimTaskLease, 1, 100);
        let (pk, _sk) = test_keypair();
        let ctx = make_ctx(0, 1000, 0, KeyBinding::stable([2u8; 32], pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    #[test]
    fn step1_rejects_zero_expires_at_height() {
        let (pk, _sk) = test_keypair();
        let mut request = make_request([1u8; 32], [2u8; 32], ActionType::ClaimTaskLease, 1, 0);
        request.agent_signature = vec![0u8; 32];
        let ctx = make_ctx(0, 1000, 0, KeyBinding::stable([2u8; 32], pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    #[test]
    fn step2_accepts_valid_signature() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        request.agent_signature = sign_request(&request, &sk_seed);
        let ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn step2_rejects_wrong_key_signature() {
        let (pk, _sk) = test_keypair();
        let (_wrong_pk, wrong_sk) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        request.agent_signature = sign_request(&request, &wrong_sk);
        let ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
    }

    #[test]
    fn step2_rejects_tampered_message() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        request.agent_signature = sign_request(&request, &sk_seed);
        request.nonce = 99;
        let ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
    }

    #[test]
    fn step3_rejects_replayed_plan_id() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
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
    fn step3_rejects_wrong_nonce() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 5, 100);
        request.agent_signature = sign_request(&request, &sk_seed);
        let ctx = make_ctx(50, 1000, 3, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
    }

    #[test]
    fn step3_rejects_expired_ttl() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 50);
        request.agent_signature = sign_request(&request, &sk_seed);
        let ctx = make_ctx(100, 1000, 0, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::TTLExpired));
    }

    #[test]
    fn step3_rejects_ttl_too_far_future() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 20000);
        request.agent_signature = sign_request(&request, &sk_seed);
        let ctx = make_ctx(100, 1000, 0, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::TTLExpired));
    }

    #[test]
    fn step3_accepts_valid_nonce_sequence() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 5, 100);
        request.agent_signature = sign_request(&request, &sk_seed);
        let ctx = make_ctx(50, 1000, 4, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn step4_quota_exhausted_blocks() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
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
    fn step4_quota_allows_under_limit() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request =
            make_request([1u8; 32], agent_id, ActionType::SubmitGovernanceProposal, 1, 100);
        request.agent_signature = sign_request(&request, &sk_seed);
        let mut ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
        ctx.quota_states = vec![QuotaState {
            quota_id: "gov_proposals_per_identity".into(),
            consumed: 0,
            window_start_height: 0,
        }];
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn step5_rejects_zero_balance() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        request.agent_signature = sign_request(&request, &sk_seed);
        let ctx = make_ctx(50, 0, 0, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::InsufficientFunds));
    }

    #[test]
    fn step5_allows_sufficient_balance() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        request.agent_signature = sign_request(&request, &sk_seed);
        let ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        request.agent_signature = sign_request(&request, &sk_seed);
        let ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk.clone()));

        let r1 = evaluate(&request, &ctx);
        let r2 = evaluate(&request, &ctx);
        assert_eq!(r1.decision, r2.decision);
        assert_eq!(r1.deny_reason, r2.deny_reason);
    }

    #[test]
    fn key_rotation_accepts_pending_key_in_grace_window() {
        let (old_pk, _old_sk) = test_keypair();
        let (new_pk, new_sk) = test_keypair();
        let agent_id = [0xAAu8; 32];

        let binding = KeyBinding {
            agent_id,
            active_pubkey: old_pk,
            pending_pubkey: Some(new_pk.clone()),
            rotation_height: Some(50),
            grace_end_height: Some(150),
        };

        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        request.agent_signature = sign_request(&request, &new_sk);
        let ctx = make_ctx(50, 1000, 0, binding);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn key_rotation_accepts_active_key_in_grace_window() {
        let (old_pk, old_sk) = test_keypair();
        let (new_pk, _new_sk) = test_keypair();
        let agent_id = [0xAAu8; 32];

        let binding = KeyBinding {
            agent_id,
            active_pubkey: old_pk.clone(),
            pending_pubkey: Some(new_pk),
            rotation_height: Some(50),
            grace_end_height: Some(150),
        };

        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        request.agent_signature = sign_request(&request, &old_sk);
        let ctx = make_ctx(50, 1000, 0, binding);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn key_rotation_rejects_old_key_after_grace() {
        let (_old_pk, old_sk) = test_keypair();
        let (new_pk, _new_sk) = test_keypair();
        let agent_id = [0xAAu8; 32];

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
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
    }

    #[test]
    fn invalid_signature_bytes_rejected() {
        let (pk, _sk) = test_keypair();
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        request.agent_signature = vec![0u8; 10]; // invalid signature bytes
        let ctx = make_ctx(50, 1000, 0, KeyBinding::stable(agent_id, pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
    }
}
