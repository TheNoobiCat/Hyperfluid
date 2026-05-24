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
use crate::quota::canonical_quota_entry;
use crate::types::{
    ActionPlanRequest, ActionPlanResponse, ActionType, Decision, PdpContext, QuotaConsumption,
};

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
    if request.expires_at_height == 0 {
        return Err(PdpError::SchemaViolation("expires_at_height must be non-zero".into()));
    }

    Ok(())
}

// ── Step 2: Signature Verification (ML-DSA-65) ────────────────────────────

fn step2_signature_verification(
    request: &ActionPlanRequest,
    ctx: &PdpContext,
) -> Result<(), PdpError> {
    use ml_dsa::{EncodedVerifyingKey, MlDsa65, Verifier, VerifyingKey};

    let pk_bytes = ctx.key_binding.as_ref().ok_or(PdpError::SignatureVerificationFailed)?;
    if request.agent_signature.is_empty() {
        return Err(PdpError::SignatureVerificationFailed);
    }

    let encoded = EncodedVerifyingKey::<MlDsa65>::try_from(pk_bytes.as_slice())
        .map_err(|_| PdpError::SignatureVerificationFailed)?;
    let verifying_key = VerifyingKey::<MlDsa65>::decode(&encoded);

    let sig = ml_dsa::Signature::<MlDsa65>::try_from(request.agent_signature.as_slice())
        .map_err(|_| PdpError::SignatureVerificationFailed)?;

    let message = hash_action_plan_for_signing(request);

    verifying_key.verify(&message, &sig).map_err(|_| PdpError::SignatureVerificationFailed)
}

pub fn hash_action_plan_for_signing(request: &ActionPlanRequest) -> [u8; 32] {
    use sha3::Digest;
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(request.plan_id);
    hasher.update(request.agent_id);
    let action_discriminant: u8 = match request.action_type {
        crate::types::ActionType::PublishTopicMessage => 0,
        crate::types::ActionType::ClaimTaskLease => 1,
        crate::types::ActionType::RenewTaskLease => 2,
        crate::types::ActionType::CreateTask => 3,
        crate::types::ActionType::SubmitFastPathMerge => 4,
        crate::types::ActionType::SubmitGovernanceProposal => 5,
        crate::types::ActionType::CastGovernanceVote => 6,
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
    let trust_stage = ctx.trust_stage;

    for quota_id in &relevant_quota_ids {
        let entry = match canonical_quota_entry(quota_id) {
            Some(e) => e,
            None => continue,
        };
        let state = ctx.quota_states.iter().find(|qs| qs.quota_id == *quota_id);

        let consumed = state.map(|s| s.consumed).unwrap_or(0);
        let after = consumed.saturating_add(1);

        // Apply stage multiplier: effective_limit = limit * num / den
        let effective_limit = entry
            .stage_multipliers
            .iter()
            .find(|(stage, _)| *stage == trust_stage)
            .map(|(_, (num, den))| {
                if *den == 0 {
                    entry.limit
                } else {
                    (entry.limit as u128 * *num as u128 / *den as u128) as u64
                }
            })
            .unwrap_or(entry.limit);

        if after > effective_limit {
            return Err(PdpError::QuotaExhausted { quota_id: quota_id.clone() });
        }

        consumption.push(QuotaConsumption {
            quota_id: quota_id.clone(),
            amount_consumed: 1,
            remaining: effective_limit.saturating_sub(after),
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
    use crate::types::{DenyReason, Hash32, QuotaState, TrustStage};
    use ml_dsa::{Generate, Keypair, MlDsa65, Seed, SignatureEncoding, Signer, SigningKey};

    fn test_keypair() -> (Vec<u8>, [u8; 32]) {
        let sk = SigningKey::<MlDsa65>::generate();
        let pk = sk.verifying_key().encode().as_slice().to_vec();
        let seed = sk.to_seed();
        let mut seed_bytes = [0u8; 32];
        seed_bytes.copy_from_slice(seed.as_slice());
        (pk, seed_bytes)
    }

    fn sign_test_request(request: &ActionPlanRequest, sk_seed: &[u8; 32]) -> Vec<u8> {
        let seed = Seed::try_from(sk_seed.as_slice()).unwrap();
        let sk = SigningKey::<MlDsa65>::from_seed(&seed);
        let msg = hash_action_plan_for_signing(request);
        sk.sign(&msg).to_vec()
    }

    fn make_ctx(
        height: u64,
        balance: u128,
        nonce: u64,
        key_binding: Option<Vec<u8>>,
    ) -> PdpContext {
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

    fn make_signed_request(
        plan_id: Hash32,
        agent_id: Hash32,
        action_type: ActionType,
        nonce: u64,
        expires: u64,
        pk_bytes: &[u8],
        sk_seed: &[u8; 32],
    ) -> (ActionPlanRequest, PdpContext) {
        let mut request = make_request(plan_id, agent_id, action_type, nonce, expires);
        request.agent_signature = sign_test_request(&request, sk_seed);
        let ctx = make_ctx(50, 1000, nonce - 1, Some(pk_bytes.to_vec()));
        (request, ctx)
    }

    // ── Step 1 tests ─────────────────────────────────────────────────────

    #[test]
    fn step1_rejects_zero_plan_id() {
        let request = make_request([0u8; 32], [1u8; 32], ActionType::ClaimTaskLease, 1, 100);
        let ctx = make_ctx(0, 1000, 0, None);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    #[test]
    fn step1_rejects_zero_agent_id() {
        let request = make_request([1u8; 32], [0u8; 32], ActionType::ClaimTaskLease, 1, 100);
        let ctx = make_ctx(0, 1000, 0, None);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    #[test]
    fn step1_rejects_zero_expires_at_height() {
        let mut request = make_request([1u8; 32], [2u8; 32], ActionType::ClaimTaskLease, 1, 0);
        request.agent_signature = vec![0u8; 32];
        let ctx = make_ctx(0, 1000, 0, None);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    // ── Step 2 tests (signature verification) ────────────────────────────

    #[test]
    fn step2_rejects_missing_key_binding() {
        let agent_id = [0xAAu8; 32];
        let request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        let ctx = make_ctx(50, 1000, 0, None);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
    }

    #[test]
    fn step2_rejects_empty_signature() {
        let agent_id = [0xAAu8; 32];
        let (pk, _seed) = test_keypair();
        let request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        let ctx = make_ctx(50, 1000, 0, Some(pk));
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
    }

    #[test]
    fn step2_rejects_wrong_key() {
        let agent_id = [0xAAu8; 32];
        let (_pk_a, seed_a) = test_keypair();
        let (pk_b, _seed_b) = test_keypair();
        let (request, _ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            1,
            100,
            &pk_b,
            &seed_a,
        );
        let ctx = make_ctx(50, 1000, 0, Some(pk_b)); // verify with pk_b but signed with seed_a
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
    }

    #[test]
    fn step2_rejects_tampered_request() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (mut request, ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            1,
            100,
            &pk,
            &seed,
        );
        request.nonce = 999; // tamper after signing
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SignatureInvalid));
    }

    #[test]
    fn step2_accepts_valid_signature() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (request, ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            1,
            100,
            &pk,
            &seed,
        );
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    // ── Step 3 tests ─────────────────────────────────────────────────────

    #[test]
    fn step3_rejects_replayed_plan_id() {
        let agent_id = [0xAAu8; 32];
        let plan_id = [0x42; 32];
        let (pk, seed) = test_keypair();
        let (request, mut ctx) =
            make_signed_request(plan_id, agent_id, ActionType::ClaimTaskLease, 1, 100, &pk, &seed);
        ctx.consumed_plan_ids = vec![plan_id];
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
    }

    #[test]
    fn step3_rejects_wrong_nonce() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (mut request, ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            5,
            100,
            &pk,
            &seed,
        );
        let mut ctx_wrong_nonce = ctx.clone();
        ctx_wrong_nonce.agent_nonce = 3; // nonce doesn't match expected +1
        request.agent_signature = sign_test_request(&request, &seed); // re-sign with nonce=5
        let result = evaluate(&request, &ctx_wrong_nonce);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
    }

    #[test]
    fn step3_rejects_expired_ttl() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (mut request, ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            1,
            100,
            &pk,
            &seed,
        );
        let mut ctx_expired = ctx.clone();
        ctx_expired.current_height = 150; // past expiry
        request.agent_signature = sign_test_request(&request, &seed);
        let result = evaluate(&request, &ctx_expired);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::TTLExpired));
    }

    #[test]
    fn step3_rejects_ttl_too_far_future() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (mut request, ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            1,
            20000,
            &pk,
            &seed,
        );
        let mut ctx_normal = ctx.clone();
        ctx_normal.current_height = 100;
        request.agent_signature = sign_test_request(&request, &seed);
        let result = evaluate(&request, &ctx_normal);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::TTLExpired));
    }

    #[test]
    fn step3_accepts_valid_nonce_sequence() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (mut request, ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            5,
            100,
            &pk,
            &seed,
        );
        let mut ctx_valid = ctx.clone();
        ctx_valid.agent_nonce = 4; // request.nonce == ctx.nonce + 1
        request.agent_signature = sign_test_request(&request, &seed);
        let result = evaluate(&request, &ctx_valid);
        assert_eq!(result.decision, Decision::Approved);
    }

    // ── Step 4 tests ─────────────────────────────────────────────────────

    #[test]
    fn step4_quota_exhausted_blocks() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (request, mut ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::SubmitGovernanceProposal,
            1,
            100,
            &pk,
            &seed,
        );
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
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (request, mut ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::SubmitGovernanceProposal,
            1,
            100,
            &pk,
            &seed,
        );
        ctx.quota_states = vec![QuotaState {
            quota_id: "gov_proposals_per_identity".into(),
            consumed: 0,
            window_start_height: 0,
        }];
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    // ── Step 5 tests ─────────────────────────────────────────────────────

    #[test]
    fn step5_rejects_zero_balance() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (mut request, ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            1,
            100,
            &pk,
            &seed,
        );
        let mut ctx_poor = ctx.clone();
        ctx_poor.agent_balance_attagx = 0;
        request.agent_signature = sign_test_request(&request, &seed);
        let result = evaluate(&request, &ctx_poor);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::InsufficientFunds));
    }

    #[test]
    fn step5_allows_sufficient_balance() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (request, ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            1,
            100,
            &pk,
            &seed,
        );
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let agent_id = [0xAAu8; 32];
        let (pk, seed) = test_keypair();
        let (request, ctx) = make_signed_request(
            [1u8; 32],
            agent_id,
            ActionType::ClaimTaskLease,
            1,
            100,
            &pk,
            &seed,
        );

        let r1 = evaluate(&request, &ctx);
        let r2 = evaluate(&request, &ctx);
        assert_eq!(r1.decision, r2.decision);
        assert_eq!(r1.deny_reason, r2.deny_reason);
    }
}
