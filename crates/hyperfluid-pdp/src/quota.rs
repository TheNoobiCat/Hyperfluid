// === C9 PDP: Cross-Layer Quota Matrix ===
//
// Source: policy-engine-spec.md §2

use crate::types::{QuotaEntry, QuotaState, TrustStage};
use std::collections::BTreeMap;

#[derive(Debug, PartialEq, Eq)]
pub enum QuotaError {
    Exhausted,
    NotFound,
}

/// Manages the canonical quota matrix and per-principal quota consumption state.
/// Uses BTreeMap for deterministic iteration in all paths that feed consensus.
pub struct QuotaManager {
    entries: BTreeMap<String, QuotaEntry>,
    states: BTreeMap<String, Vec<QuotaState>>,
}

impl QuotaManager {
    pub fn new() -> Self {
        Self { entries: BTreeMap::new(), states: BTreeMap::new() }
    }

    pub fn with_canonical_entries() -> Self {
        let mut m = Self::new();
        m.load_canonical_entries();
        m
    }

    /// Load all canonical quota entries from the spec §2.4.
    fn load_canonical_entries(&mut self) {
        let entries = vec![
            // P2P ingress
            QuotaEntry {
                quota_id: "p2p_conn_per_identity".into(),
                enforcement_point: "p2p_ingress".into(),
                dimension: "per_identity".into(),
                limit: 50,
                window_blocks: 0,
                stage_multipliers: vec![
                    (TrustStage::Untrusted, (10, 50)),
                    (TrustStage::Trusted, (50, 50)),
                ],
            },
            QuotaEntry {
                quota_id: "p2p_tx_burst".into(),
                enforcement_point: "p2p_ingress".into(),
                dimension: "per_identity".into(),
                limit: 20,
                window_blocks: 360,
                stage_multipliers: vec![],
            },
            // P2P gossip
            QuotaEntry {
                quota_id: "p2p_gossip_budget".into(),
                enforcement_point: "p2p_gossip".into(),
                dimension: "per_sender".into(),
                limit: 100,
                window_blocks: 60,
                stage_multipliers: vec![],
            },
            // Inbox router
            QuotaEntry {
                quota_id: "inbox_msg_per_sender".into(),
                enforcement_point: "inbox_router".into(),
                dimension: "per_sender".into(),
                limit: 60,
                window_blocks: 60,
                stage_multipliers: vec![
                    (TrustStage::Untrusted, (5, 60)),
                    (TrustStage::Trusted, (60, 60)),
                ],
            },
            QuotaEntry {
                quota_id: "inbox_global_per_agent".into(),
                enforcement_point: "inbox_router".into(),
                dimension: "per_agent".into(),
                limit: 2000,
                window_blocks: 3600,
                stage_multipliers: vec![],
            },
            // Topic router
            QuotaEntry {
                quota_id: "topic_msg_global".into(),
                enforcement_point: "topic_router".into(),
                dimension: "per_topic".into(),
                limit: 500,
                window_blocks: 300,
                stage_multipliers: vec![],
            },
            // Fast-path
            QuotaEntry {
                quota_id: "fast_merge_per_topic".into(),
                enforcement_point: "fast_path".into(),
                dimension: "per_topic".into(),
                limit: 20,
                window_blocks: 3600,
                stage_multipliers: vec![],
            },
            QuotaEntry {
                quota_id: "fast_merge_per_identity".into(),
                enforcement_point: "fast_path".into(),
                dimension: "per_identity".into(),
                limit: 5,
                window_blocks: 3600,
                stage_multipliers: vec![],
            },
            // Governance
            QuotaEntry {
                quota_id: "gov_proposals_per_identity".into(),
                enforcement_point: "governance".into(),
                dimension: "per_identity".into(),
                limit: 1,
                window_blocks: 8192,
                stage_multipliers: vec![],
            },
            QuotaEntry {
                quota_id: "gov_open_proposals_global".into(),
                enforcement_point: "governance".into(),
                dimension: "network_wide".into(),
                limit: 32,
                window_blocks: 0,
                stage_multipliers: vec![],
            },
            // Review
            QuotaEntry {
                quota_id: "review_concurrent_per_reviewer".into(),
                enforcement_point: "review_assignment".into(),
                dimension: "per_reviewer".into(),
                limit: 5,
                window_blocks: 0,
                stage_multipliers: vec![],
            },
            // Task board
            QuotaEntry {
                quota_id: "lease_active_per_agent".into(),
                enforcement_point: "task_board".into(),
                dimension: "per_agent".into(),
                limit: 6,
                window_blocks: 0,
                stage_multipliers: vec![
                    (TrustStage::Untrusted, (2, 6)),
                    (TrustStage::Trusted, (6, 6)),
                ],
            },
            // Challenge
            QuotaEntry {
                quota_id: "challenge_per_identity".into(),
                enforcement_point: "challenge".into(),
                dimension: "per_identity".into(),
                limit: 3,
                window_blocks: 8192,
                stage_multipliers: vec![],
            },
            // Task creation (PDP gated)
            QuotaEntry {
                quota_id: "task_create_per_stage".into(),
                enforcement_point: "pdp".into(),
                dimension: "per_agent".into(),
                limit: 10,
                window_blocks: 0,
                stage_multipliers: vec![
                    (TrustStage::Untrusted, (0, 10)),
                    (TrustStage::Trusted, (10, 10)),
                ],
            },
        ];

        for entry in entries {
            self.entries.insert(entry.quota_id.clone(), entry);
        }
    }

    /// Look up a quota entry by ID.
    pub fn get_entry(&self, quota_id: &str) -> Option<&QuotaEntry> {
        self.entries.get(quota_id)
    }

    /// Check whether consuming `amount` of a quota would exceed the limit.
    /// Returns Ok(remaining) if within limit, Err(QuotaError::Exhausted) if exhausted.
    pub fn check_quota(
        &self,
        quota_id: &str,
        principal_id: &str,
        trust_stage: TrustStage,
        amount: u64,
        current_height: u64,
    ) -> Result<u64, QuotaError> {
        let entry = self.entries.get(quota_id).ok_or(QuotaError::NotFound)?;

        // Apply stage multiplier: use (numerator, denominator) rational pair
        let effective_limit = entry
            .stage_multipliers
            .iter()
            .find(|(stage, _)| *stage == trust_stage)
            .map(|(_, (num, den))| {
                if *den == 0 {
                    entry.limit as u128
                } else {
                    (entry.limit as u128 * *num as u128) / *den as u128
                }
            })
            .unwrap_or(entry.limit as u128) as u64;

        let state_key = format!("{principal_id}:{quota_id}");
        let state = self.states.get(&state_key).and_then(|states| {
            states.iter().find(|s| {
                // If window-based, check current window
                if entry.window_blocks > 0 {
                    s.window_start_height + entry.window_blocks > current_height
                        && s.window_start_height <= current_height
                } else {
                    true
                }
            })
        });

        let consumed = state.map(|s| s.consumed).unwrap_or(0);
        let after = consumed.saturating_add(amount);

        if after > effective_limit {
            return Err(QuotaError::Exhausted);
        }

        Ok(effective_limit.saturating_sub(after))
    }

    /// Atomically reserve quota consumption. Returns the QuotaState after
    /// reservation.
    pub fn reserve_quota(
        &mut self,
        quota_id: &str,
        principal_id: &str,
        trust_stage: TrustStage,
        amount: u64,
        current_height: u64,
    ) -> Result<QuotaState, QuotaError> {
        self.check_quota(quota_id, principal_id, trust_stage, amount, current_height)?;

        let state_key = format!("{principal_id}:{quota_id}");
        let states = self.states.entry(state_key).or_default();

        if let Some(existing) = states.iter_mut().find(|s| s.quota_id == *quota_id) {
            existing.consumed = existing.consumed.saturating_add(amount);
            Ok(existing.clone())
        } else {
            let new_state = QuotaState {
                quota_id: quota_id.to_string(),
                consumed: amount,
                window_start_height: current_height,
            };
            states.push(new_state.clone());
            Ok(new_state)
        }
    }

    /// Release reserved quota (rollback on execution failure).
    pub fn release_quota(&mut self, quota_id: &str, principal_id: &str, amount: u64) {
        let state_key = format!("{principal_id}:{quota_id}");
        if let Some(states) = self.states.get_mut(&state_key) {
            if let Some(existing) = states.iter_mut().find(|s| s.quota_id == *quota_id) {
                existing.consumed = existing.consumed.saturating_sub(amount);
            }
        }
    }

    /// Get all entries (returned in sorted order for determinism).
    pub fn entries(&self) -> Vec<&QuotaEntry> {
        self.entries.values().collect()
    }

    /// Get current quota state for a principal.
    pub fn get_state(&self, quota_id: &str, principal_id: &str) -> Option<&QuotaState> {
        let state_key = format!("{principal_id}:{quota_id}");
        self.states
            .get(&state_key)
            .and_then(|states| states.iter().find(|s| s.quota_id == *quota_id))
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::with_canonical_entries()
    }
}

/// Look up a canonical quota entry by ID.
/// This is the single source of truth for the quota matrix (spec §2.4).
pub fn canonical_quota_entry(quota_id: &str) -> Option<QuotaEntry> {
    QuotaManager::with_canonical_entries().get_entry(quota_id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_entries_have_all_quota_ids() {
        let qm = QuotaManager::with_canonical_entries();
        assert_eq!(qm.entries().len(), 14);
    }

    #[test]
    fn check_quota_within_limit() {
        let qm = QuotaManager::with_canonical_entries();
        let result =
            qm.check_quota("gov_proposals_per_identity", "agent1", TrustStage::Trusted, 1, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn check_quota_exhausted_returns_err() {
        let qm = QuotaManager::with_canonical_entries();
        let result =
            qm.check_quota("gov_proposals_per_identity", "agent1", TrustStage::Trusted, 2, 0);
        assert!(result.is_err());
    }

    #[test]
    fn reserve_and_release_quota_atomic() {
        let mut qm = QuotaManager::with_canonical_entries();
        let result =
            qm.reserve_quota("gov_proposals_per_identity", "agent1", TrustStage::Trusted, 1, 0);
        assert!(result.is_ok());

        let result2 =
            qm.reserve_quota("gov_proposals_per_identity", "agent1", TrustStage::Trusted, 1, 0);
        assert!(result2.is_err());

        qm.release_quota("gov_proposals_per_identity", "agent1", 1);
        let result3 =
            qm.reserve_quota("gov_proposals_per_identity", "agent1", TrustStage::Trusted, 1, 0);
        assert!(result3.is_ok());
    }

    #[test]
    fn stage_multiplier_task_create_untrusted_zero() {
        let qm = QuotaManager::with_canonical_entries();
        let entry = qm.get_entry("task_create_per_stage").unwrap();
        let untrusted_mul =
            entry.stage_multipliers.iter().find(|(s, _)| *s == TrustStage::Untrusted);
        assert!(untrusted_mul.is_some());
        assert_eq!(untrusted_mul.unwrap().1, (0, 10));
    }

    #[test]
    fn stage_multiplier_task_create_trusted_ten() {
        let qm = QuotaManager::with_canonical_entries();
        let entry = qm.get_entry("task_create_per_stage").unwrap();
        let trusted_mul = entry.stage_multipliers.iter().find(|(s, _)| *s == TrustStage::Trusted);
        assert!(trusted_mul.is_some());
        assert_eq!(trusted_mul.unwrap().1, (10, 10));
    }
}
