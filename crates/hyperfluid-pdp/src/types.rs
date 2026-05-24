// === C9 PDP: Core Types ===
//
// Source: docs/04-specifications/runtime/policy-engine-spec.md Sections 1.3, 2.3, 3.3

use serde::{Deserialize, Serialize};

pub type Hash32 = [u8; 32];

/// Trust stage for an agent identity (2-stage model: untrusted or trusted).
/// Source: collaboration-spec.md §3
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustStage {
    Untrusted = 0,
    Trusted = 1,
}

impl TrustStage {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(TrustStage::Untrusted),
            1 => Some(TrustStage::Trusted),
            _ => None,
        }
    }
}

/// Action plan request submitted by an agent for PDP evaluation.
/// Source: policy-engine-spec.md §1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPlanRequest {
    pub plan_id: Hash32,
    pub agent_id: Hash32,
    pub action_type: ActionType,
    pub resource_id: Hash32,
    pub reason_hash: Hash32,
    pub evidence_refs: Vec<Hash32>,
    pub nonce: u64,
    pub expires_at_height: u64,
    pub agent_signature: Vec<u8>,
}

/// Network-mutating action types subject to PDP evaluation.
/// Source: policy-engine-spec.md §1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionType {
    // New variants in alphabetical order before PublishTopicMessage
    DelegateOperation,
    ReleaseTask,
    StakeOperation,
    SubmitEvidence,
    SubmitReview,
    SubmitTaskCompletion,
    Transfer,
    // Original variants
    PublishTopicMessage,
    ClaimTaskLease,
    RenewTaskLease,
    CreateTask,
    SubmitFastPathMerge,
    SubmitGovernanceProposal,
    CastGovernanceVote,
}

/// PDP evaluation result returned to the caller.
/// Source: policy-engine-spec.md §1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActionPlanResponse {
    pub plan_id: Hash32,
    pub decision: Decision,
    pub deny_reason: Option<DenyReason>,
    pub consumed_quota: Option<Vec<QuotaConsumption>>,
    pub approval_height: u64,
    pub expires_at_height: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    Approved,
    Denied,
}

/// Structured deny reason codes produced by the PDP rule chain.
/// Source: policy-engine-spec.md §1.3
///
/// SPEC_DEVIATION: Added InsufficientFunds — spec §1.4 Step 5 (fee check) says
/// "Failure → DENIED (insufficient funds)" but the DenyReason enum in §1.3 does
/// not list InsufficientFunds. Added here as the 6th variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenyReason {
    SchemaViolation,
    SignatureInvalid,
    ReplayDetected,
    TTLExpired,
    QuotaExhausted,
    InsufficientFunds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaConsumption {
    pub quota_id: String,
    pub amount_consumed: u64,
    pub remaining: u64,
}

/// Append-only, content-addressed audit log entry.
/// Source: policy-engine-spec.md §1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PolicyAuditEntry {
    pub entry_id: Hash32,
    pub plan_id: Hash32,
    pub agent_id: Hash32,
    pub action_type: ActionType,
    pub decision: Decision,
    pub deny_reason: Option<DenyReason>,
    pub height: u64,
    pub evaluator_signature: Vec<u8>,
}

/// Canonical quota entry from the cross-layer quota matrix.
/// Source: policy-engine-spec.md §2.3
///
/// stage_multipliers uses rational pairs (numerator, denominator) per trust stage
/// to avoid floating-point in deterministic paths.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaEntry {
    pub quota_id: String,
    pub enforcement_point: String,
    pub dimension: String,
    pub limit: u64,
    pub window_blocks: u64,
    pub stage_multipliers: Vec<(TrustStage, (u64, u64))>,
}

/// Live quota consumption state for a given principal.
/// Source: policy-engine-spec.md §2.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuotaState {
    pub quota_id: String,
    pub consumed: u64,
    pub window_start_height: u64,
}

/// Context provided to the PDP for each evaluation.
/// Contains all external state the PDP needs to make a deterministic decision.
#[derive(Debug, Clone)]
pub struct PdpContext {
    pub current_height: u64,
    pub key_binding: Option<Vec<u8>>,
    pub agent_balance_attagx: u128,
    pub agent_nonce: u64,
    pub consumed_plan_ids: Vec<Hash32>,
    pub quota_states: Vec<QuotaState>,
    pub trust_stage: TrustStage,
}
