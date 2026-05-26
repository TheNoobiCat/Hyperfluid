// === C6 Fast-Path: Types ===
//
// Source: docs/04-specifications/protocol/fastpath-spec.md Section 1.3

use serde::{Deserialize, Serialize};

pub type Hash32 = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewerVote {
    Approve,
    /// Reviewer explicitly denies the proposal. Counted toward quorum but not approval.
    Deny,
    /// Reviewer abstains from voting. Counted toward quorum but not approval.
    Abstain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastPathProposal {
    pub proposal_id: Hash32,
    pub topic_id: Hash32,
    pub proposer_id: Hash32,
    pub base_topic_head: Hash32,
    pub proposed_head: Hash32,
    pub bundle_manifest_hash: Hash32,
    pub expires_at_height: u64,
    pub proposer_signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewerSignature {
    pub reviewer_id: Hash32,
    pub vote: ReviewerVote,
    pub reason_hash: Hash32,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastPathCertificate {
    pub proposal_id: Hash32,
    pub topic_id: Hash32,
    pub base_topic_head: Hash32,
    pub proposed_head: Hash32,
    pub approvals: Vec<ReviewerSignature>,
    pub aggregate_signature: Vec<u8>,
    pub signer_set_hash: Hash32,
    pub issued_at_height: u64,
    pub challenge_until_height: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastPathChallengeTx {
    pub proposal_id: Hash32,
    pub topic_id: Hash32,
    pub challenger_id: Hash32,
    pub evidence_hash: Hash32,
    pub challenger_bond: u128,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastPathRollbackTx {
    pub proposal_id: Hash32,
    pub topic_id: Hash32,
    pub rollback_to_head: Hash32,
    pub arbiter_certificate: Vec<u8>,
    pub signature: Vec<u8>,
}

/// Fast-path protocol parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FastPathParams {
    /// Number of blocks before a certified merge becomes final.
    pub challenge_window_blocks: u64,
    /// Quorum threshold as numerator of 100 (e.g., 67 = 67/100).
    pub quorum_threshold_num: u64,
    /// Maximum fast merges per topic per hour.
    pub max_merges_per_topic_per_hour: u64,
    /// Maximum fast merges per identity per hour.
    pub max_merges_per_identity_per_hour: u64,
    /// Maximum fast merges per identity per epoch (for challenge cap).
    pub max_challenges_per_identity_per_epoch: u64,
}

impl Default for FastPathParams {
    fn default() -> Self {
        Self {
            challenge_window_blocks: 144,
            quorum_threshold_num: 67,
            max_merges_per_topic_per_hour: 20,
            max_merges_per_identity_per_hour: 5,
            max_challenges_per_identity_per_epoch: 3,
        }
    }
}
