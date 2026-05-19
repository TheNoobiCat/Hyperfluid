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

// ── Step 2: Signature Verification (stub — ML-DSA integration deferred) ────

fn step2_signature_verification(
    _request: &ActionPlanRequest,
    _ctx: &PdpContext,
) -> Result<(), PdpError> {
    Ok(())
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
        let entry = get_quota_entry(quota_id);
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
        _ => {
            // Unknown quota_id — deny by returning a zero-limit entry.
            // This prevents silent unlimited access for unrecognised quota IDs.
            QuotaEntry {
                quota_id: quota_id.into(),
                enforcement_point: "unknown".into(),
                dimension: "unknown".into(),
                limit: 0,
                window_blocks: 0,
                stage_multipliers: vec![],
            }
        }
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
    use crate::types::{DenyReason, QuotaState, TrustStage};

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
        let ctx = make_ctx(0, 1000, 0);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    #[test]
    fn step1_rejects_zero_agent_id() {
        let request = make_request([1u8; 32], [0u8; 32], ActionType::ClaimTaskLease, 1, 100);
        let ctx = make_ctx(0, 1000, 0);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    #[test]
    fn step1_rejects_zero_expires_at_height() {
        let mut request = make_request([1u8; 32], [2u8; 32], ActionType::ClaimTaskLease, 1, 0);
        request.agent_signature = vec![0u8; 32];
        let ctx = make_ctx(0, 1000, 0);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::SchemaViolation));
    }

    #[test]
    fn step3_rejects_replayed_plan_id() {
        let agent_id = [0xAAu8; 32];
        let plan_id = [0x42; 32];
        let mut request = make_request(plan_id, agent_id, ActionType::ClaimTaskLease, 1, 100);
        let mut ctx = make_ctx(50, 1000, 0);
        ctx.consumed_plan_ids = vec![plan_id];
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
    }

    #[test]
    fn step3_rejects_wrong_nonce() {
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 5, 100);
        let ctx = make_ctx(50, 1000, 3);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::ReplayDetected));
    }

    #[test]
    fn step3_rejects_expired_ttl() {
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 50);
        let ctx = make_ctx(100, 1000, 0);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::TTLExpired));
    }

    #[test]
    fn step3_rejects_ttl_too_far_future() {
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 20000);
        let ctx = make_ctx(100, 1000, 0);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::TTLExpired));
    }

    #[test]
    fn step3_accepts_valid_nonce_sequence() {
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 5, 100);
        let ctx = make_ctx(50, 1000, 4);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn step4_quota_exhausted_blocks() {
        let agent_id = [0xAAu8; 32];
        let mut request =
            make_request([1u8; 32], agent_id, ActionType::SubmitGovernanceProposal, 1, 100);
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
    fn step4_quota_allows_under_limit() {
        let agent_id = [0xAAu8; 32];
        let mut request =
            make_request([1u8; 32], agent_id, ActionType::SubmitGovernanceProposal, 1, 100);
        let mut ctx = make_ctx(50, 1000, 0);
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
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        let ctx = make_ctx(50, 0, 0);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Denied);
        assert_eq!(result.deny_reason, Some(DenyReason::InsufficientFunds));
    }

    #[test]
    fn step5_allows_sufficient_balance() {
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        let ctx = make_ctx(50, 1000, 0);
        let result = evaluate(&request, &ctx);
        assert_eq!(result.decision, Decision::Approved);
    }

    #[test]
    fn deterministic_same_input_same_output() {
        let agent_id = [0xAAu8; 32];
        let mut request = make_request([1u8; 32], agent_id, ActionType::ClaimTaskLease, 1, 100);
        let ctx = make_ctx(50, 1000, 0);

        let r1 = evaluate(&request, &ctx);
        let r2 = evaluate(&request, &ctx);
        assert_eq!(r1.decision, r2.decision);
        assert_eq!(r1.deny_reason, r2.deny_reason);
    }
}
