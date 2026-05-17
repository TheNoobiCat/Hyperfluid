// === C9 PDP: Append-Only Audit Log ===
//
// Source: policy-engine-spec.md §1.2, §1.3

use crate::types::{ActionPlanResponse, ActionType, Hash32, PolicyAuditEntry};
use sha3::{Digest, Sha3_256};
use std::collections::BTreeMap;

/// Append-only, content-addressed audit log of all PDP decisions.
/// Each entry hashes to the previous entry, forming an immutable chain.
pub struct AuditLog {
    entries: Vec<PolicyAuditEntry>,
    by_plan_id: BTreeMap<Hash32, usize>,
    last_entry_id: Option<Hash32>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self { entries: Vec::new(), by_plan_id: BTreeMap::new(), last_entry_id: None }
    }

    /// Record a PDP decision in the audit log.
    pub fn record(
        &mut self,
        response: &ActionPlanResponse,
        action_type: ActionType,
        agent_id: Hash32,
        height: u64,
        evaluator_signature: Vec<u8>,
    ) -> Hash32 {
        let entry_id =
            self.compute_entry_id(response, action_type, agent_id, height, &evaluator_signature);

        let entry = PolicyAuditEntry {
            entry_id,
            plan_id: response.plan_id,
            agent_id,
            action_type,
            decision: response.decision,
            deny_reason: response.deny_reason,
            height,
            evaluator_signature,
        };

        let idx = self.entries.len();
        self.by_plan_id.insert(response.plan_id, idx);
        self.entries.push(entry);
        self.last_entry_id = Some(entry_id);

        entry_id
    }

    fn compute_entry_id(
        &self,
        response: &ActionPlanResponse,
        action_type: ActionType,
        agent_id: Hash32,
        height: u64,
        evaluator_signature: &[u8],
    ) -> Hash32 {
        let mut hasher = Sha3_256::new();

        if let Some(ref prev) = self.last_entry_id {
            hasher.update(prev);
        }

        hasher.update(response.plan_id);
        hasher.update(agent_id);
        let action_discriminant: u8 = action_type_discriminant(action_type);
        hasher.update([action_discriminant]);
        let decision_byte: u8 = match response.decision {
            crate::types::Decision::Approved => 1,
            crate::types::Decision::Denied => 0,
        };
        hasher.update([decision_byte]);
        if let Some(reason) = &response.deny_reason {
            hasher.update([deny_reason_byte(*reason)]);
        }
        hasher.update(height.to_le_bytes());
        hasher.update(evaluator_signature);

        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out
    }

    /// Query an audit entry by plan_id.
    pub fn get_by_plan_id(&self, plan_id: &Hash32) -> Option<&PolicyAuditEntry> {
        self.by_plan_id.get(plan_id).and_then(|&idx| self.entries.get(idx))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> impl Iterator<Item = &PolicyAuditEntry> {
        self.entries.iter()
    }

    pub fn last_entry_id(&self) -> Option<Hash32> {
        self.last_entry_id
    }

    /// Verify the integrity of the entire audit log chain.
    pub fn verify_integrity(&self) -> bool {
        if self.entries.is_empty() {
            return true;
        }

        let mut prev_id: Option<Hash32> = None;
        for entry in &self.entries {
            let mut hasher = Sha3_256::new();

            if let Some(ref pid) = prev_id {
                hasher.update(pid);
            }

            hasher.update(entry.plan_id);
            hasher.update(entry.agent_id);
            let action_discriminant: u8 = action_type_discriminant(entry.action_type);
            hasher.update([action_discriminant]);
            let decision_byte: u8 = match entry.decision {
                crate::types::Decision::Approved => 1,
                crate::types::Decision::Denied => 0,
            };
            hasher.update([decision_byte]);
            if let Some(reason) = &entry.deny_reason {
                hasher.update([deny_reason_byte(*reason)]);
            }
            hasher.update(entry.height.to_le_bytes());
            hasher.update(&entry.evaluator_signature);

            let mut expected = [0u8; 32];
            expected.copy_from_slice(&hasher.finalize());

            if entry.entry_id != expected {
                return false;
            }
            prev_id = Some(entry.entry_id);
        }

        true
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}

fn action_type_discriminant(at: ActionType) -> u8 {
    match at {
        ActionType::PublishTopicMessage => 0,
        ActionType::ClaimTaskLease => 1,
        ActionType::RenewTaskLease => 2,
        ActionType::CreateTask => 3,
        ActionType::SubmitFastPathMerge => 4,
        ActionType::SubmitGovernanceProposal => 5,
        ActionType::CastGovernanceVote => 6,
    }
}

fn deny_reason_byte(r: crate::types::DenyReason) -> u8 {
    match r {
        crate::types::DenyReason::SchemaViolation => 1,
        crate::types::DenyReason::SignatureInvalid => 2,
        crate::types::DenyReason::ReplayDetected => 3,
        crate::types::DenyReason::TTLExpired => 4,
        crate::types::DenyReason::QuotaExhausted => 5,
        crate::types::DenyReason::InsufficientFunds => 6,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{ActionPlanResponse, Decision};

    fn make_response(plan_id: Hash32, decision: Decision) -> ActionPlanResponse {
        ActionPlanResponse {
            plan_id,
            decision,
            deny_reason: None,
            consumed_quota: None,
            approval_height: 100,
            expires_at_height: 200,
        }
    }

    #[test]
    fn audit_log_append_only() {
        let mut log = AuditLog::new();
        assert!(log.is_empty());

        let r1 = make_response([1u8; 32], Decision::Approved);
        log.record(&r1, ActionType::ClaimTaskLease, [0xAA; 32], 100, vec![8u8; 32]);
        assert_eq!(log.len(), 1);

        let r2 = make_response([2u8; 32], Decision::Denied);
        log.record(&r2, ActionType::ClaimTaskLease, [0xAA; 32], 101, vec![8u8; 32]);
        assert_eq!(log.len(), 2);
    }

    #[test]
    fn audit_log_content_addressed() {
        let mut log = AuditLog::new();
        let r1 = make_response([1u8; 32], Decision::Approved);
        let eid1 = log.record(&r1, ActionType::ClaimTaskLease, [0xAA; 32], 100, vec![8u8; 32]);

        let mut log2 = AuditLog::new();
        let eid2 = log2.record(&r1, ActionType::ClaimTaskLease, [0xAA; 32], 100, vec![8u8; 32]);

        assert_eq!(eid1, eid2);
    }

    #[test]
    fn audit_log_chain_immutable() {
        let mut log = AuditLog::new();

        let r1 = make_response([1u8; 32], Decision::Approved);
        log.record(&r1, ActionType::ClaimTaskLease, [0xAA; 32], 100, vec![8u8; 32]);

        let r2 = make_response([2u8; 32], Decision::Denied);
        log.record(&r2, ActionType::SubmitGovernanceProposal, [0xAA; 32], 101, vec![8u8; 32]);

        assert!(log.verify_integrity());
    }

    #[test]
    fn audit_log_query_by_plan_id() {
        let mut log = AuditLog::new();
        let plan_id = [0x42; 32];
        let r = make_response(plan_id, Decision::Approved);
        log.record(&r, ActionType::CreateTask, [0xAA; 32], 100, vec![8u8; 32]);

        let entry = log.get_by_plan_id(&plan_id);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().plan_id, plan_id);
    }

    #[test]
    fn audit_log_verify_integrity_passes_empty() {
        let log = AuditLog::new();
        assert!(log.verify_integrity());
    }

    #[test]
    fn audit_log_integrity_detects_tampering() {
        let mut log = AuditLog::new();
        let r1 = make_response([1u8; 32], Decision::Approved);
        log.record(&r1, ActionType::ClaimTaskLease, [0xAA; 32], 100, vec![8u8; 32]);
        let r2 = make_response([2u8; 32], Decision::Denied);
        log.record(&r2, ActionType::SubmitGovernanceProposal, [0xAA; 32], 101, vec![8u8; 32]);

        // Tamper with an entry
        log.entries[0].height = 999;
        assert!(!log.verify_integrity());
    }
}
