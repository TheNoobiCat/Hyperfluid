// === C4 Governance Engine: Types ===
//
// Source: docs/04-specifications/protocol/governance-spec.md Section 1.3

use serde::{Deserialize, Serialize};

pub type Hash32 = [u8; 32];

/// Governance proposal lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
}

/// On-chain governance proposal for git:head transitions.
/// Source: governance-spec.md §1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceProposal {
    pub proposal_id: Hash32,
    pub proposer_id: Hash32,
    pub proposed_commit: Hash32,
    pub bundle_manifest_hash: Hash32,
    pub current_commit: Hash32,
    pub deposit_amount: u128,
    pub snapshot_height: u64,
    pub vote_start_height: u64,
    pub vote_end_height: u64,
    pub status: ProposalStatus,
    pub yes_weight: u128,
    pub no_weight: u128,
}

/// Bundle manifest describing the proposed code change.
/// Source: governance-spec.md §1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BundleManifest {
    pub manifest_hash: Hash32,
    pub object_ids: Vec<Hash32>,
    pub total_size_bytes: u64,
    pub proposer_fetch_endpoints: Vec<String>,
    pub toolchain_hash: Hash32,
    pub environment_hash: Hash32,
}

/// A vote cast by a validator on a governance proposal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteOption {
    Yes,
    No,
}

/// Governance vote transaction with stake-weighted voting power.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceVote {
    pub proposal_id: Hash32,
    pub voter_id: Hash32,
    pub vote: VoteOption,
    pub reason_hash: Hash32,
    pub vote_weight: u128,
    pub signature: Vec<u8>,
}

/// Canonical governance parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GovernanceParams {
    /// Required quorum as percentage of total snapshot stake (40% = 40).
    pub quorum_required_pct: u64,
    /// Votes within this window count toward quorum.
    pub vote_window_blocks: u64,
    /// Maximum number of open proposals at any time.
    pub max_open_proposals: u64,
    /// Per-identity proposal limit per epoch.
    pub proposals_per_identity_per_epoch: u64,
    /// Minimum deposit required to submit a proposal (in atto-AGX).
    pub proposal_deposit_attagx: u128,
    /// Cooldown epochs after a rejected proposal.
    pub rejected_cooldown_epochs: u64,
}

impl Default for GovernanceParams {
    fn default() -> Self {
        Self {
            quorum_required_pct: 40,
            // 14 epochs * 5040 blocks/epoch = 70,560 blocks
            vote_window_blocks: 70_560,
            max_open_proposals: 32,
            proposals_per_identity_per_epoch: 1,
            // 500 AGX = 500 * 10^18 atto-AGX
            proposal_deposit_attagx: 500_000_000_000_000_000_000u128,
            rejected_cooldown_epochs: 3,
        }
    }
}
