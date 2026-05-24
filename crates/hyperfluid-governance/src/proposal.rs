// === C4 Governance Engine: Proposal Lifecycle ===
//
// Source: governance-spec.md §1.4 State Transitions

use crate::types::{
    GovernanceParams, GovernanceProposal, GovernanceVote, Hash32, ProposalStatus, VoteOption,
};
use sha3::{Digest, Sha3_256};
use std::collections::BTreeMap;

/// Manages the governance proposal lifecycle and vote aggregation.
pub struct GovernanceEngine {
    params: GovernanceParams,
    proposals: BTreeMap<Hash32, GovernanceProposal>,
    votes: BTreeMap<Hash32, Vec<GovernanceVote>>,
    /// Proposer cooldown tracking: (proposer_id, height_until_can_propose_again)
    cooldowns: BTreeMap<Hash32, u64>,
    /// Proposals per epoch per proposer: (proposer_id, epoch, count)
    proposal_counts: BTreeMap<(Hash32, u64), u64>,
}

impl GovernanceEngine {
    pub fn new(params: GovernanceParams) -> Self {
        Self {
            params,
            proposals: BTreeMap::new(),
            votes: BTreeMap::new(),
            cooldowns: BTreeMap::new(),
            proposal_counts: BTreeMap::new(),
        }
    }

    pub fn params(&self) -> &GovernanceParams {
        &self.params
    }

    pub fn proposal_count(&self) -> usize {
        self.proposals.len()
    }

    // ── Proposal Submission ──────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn submit_proposal(
        &mut self,
        proposal_id: Hash32,
        proposer_id: Hash32,
        proposed_commit: Hash32,
        bundle_manifest_hash: Hash32,
        current_commit: Hash32,
        current_height: u64,
        current_epoch: u64,
        _total_snapshot_stake: u128,
    ) -> Result<&GovernanceProposal, ProposalError> {
        // Check max open proposals
        if self.active_proposal_count() >= self.params.max_open_proposals as usize {
            return Err(ProposalError::MaxOpenProposalsReached);
        }

        // Check per-epoch cap
        let count = self.proposal_counts.get(&(proposer_id, current_epoch)).copied().unwrap_or(0);
        if count >= self.params.proposals_per_identity_per_epoch {
            return Err(ProposalError::ProposerLimitExceeded);
        }

        // Check cooldown
        if let Some(&cooldown_until) = self.cooldowns.get(&proposer_id) {
            if current_height < cooldown_until {
                return Err(ProposalError::ProposerInCooldown);
            }
        }

        let proposal = GovernanceProposal {
            proposal_id,
            proposer_id,
            proposed_commit,
            bundle_manifest_hash,
            current_commit,
            deposit_amount: self.params.proposal_deposit_attagx,
            snapshot_height: current_height,
            vote_start_height: current_height,
            vote_end_height: current_height + self.params.vote_window_blocks,
            status: ProposalStatus::Active,
            yes_weight: 0,
            no_weight: 0,
        };

        self.proposals.insert(proposal_id, proposal.clone());

        // Track proposal count per identity per epoch
        *self.proposal_counts.entry((proposer_id, current_epoch)).or_insert(0) += 1;

        // Clear stale cooldown (proposer is active again)
        self.cooldowns.remove(&proposer_id);

        Ok(&self.proposals[&proposal_id])
    }

    fn active_proposal_count(&self) -> usize {
        self.proposals.values().filter(|p| p.status == ProposalStatus::Active).count()
    }

    // ── Vote Casting ─────────────────────────────────────────────────────

    pub fn cast_vote(
        &mut self,
        vote: GovernanceVote,
        current_height: u64,
    ) -> Result<(), ProposalError> {
        let proposal =
            self.proposals.get_mut(&vote.proposal_id).ok_or(ProposalError::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Active {
            return Err(ProposalError::ProposalNotActive);
        }

        if current_height > proposal.vote_end_height {
            return Err(ProposalError::VoteWindowClosed);
        }

        // Prevent double voting
        let votes = self.votes.entry(vote.proposal_id).or_default();
        if votes.iter().any(|v| v.voter_id == vote.voter_id) {
            return Err(ProposalError::AlreadyVoted);
        }

        match vote.vote {
            VoteOption::Yes => proposal.yes_weight += vote.vote_weight,
            VoteOption::No => proposal.no_weight += vote.vote_weight,
        }

        votes.push(vote);
        Ok(())
    }

    // ── Tally Finalization ───────────────────────────────────────────────

    pub fn finalize_proposal(
        &mut self,
        proposal_id: Hash32,
        current_height: u64,
        total_snapshot_stake: u128,
        epoch_length_blocks: u64,
    ) -> Result<ProposalOutcome, ProposalError> {
        let proposal = self.proposals.get(&proposal_id).ok_or(ProposalError::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Active {
            return Err(ProposalError::ProposalNotActive);
        }

        if current_height < proposal.vote_end_height {
            return Err(ProposalError::VoteWindowNotEnded);
        }

        let participated_weight = proposal.yes_weight + proposal.no_weight;
        let participated_pct =
            (participated_weight * 100).checked_div(total_snapshot_stake).unwrap_or(0);

        let yes_pct = (proposal.yes_weight * 100).checked_div(participated_weight).unwrap_or(0);

        let quorum_met = participated_pct >= self.params.quorum_required_pct as u128;
        let majority_yes = yes_pct > 50;

        let proposal = self.proposals.get_mut(&proposal_id).unwrap();

        if quorum_met && majority_yes {
            proposal.status = ProposalStatus::Passed;
            Ok(ProposalOutcome::Passed {
                yes_weight: proposal.yes_weight,
                no_weight: proposal.no_weight,
                participated_weight,
                deposit_returned: true,
            })
        } else {
            proposal.status = ProposalStatus::Rejected;
            self.cooldowns.insert(
                proposal.proposer_id,
                current_height + (self.params.rejected_cooldown_epochs * epoch_length_blocks),
            );
            Ok(ProposalOutcome::Rejected {
                yes_weight: proposal.yes_weight,
                no_weight: proposal.no_weight,
                participated_weight,
                quorum_met,
                deposit_burned: false,
            })
        }
    }

    /// Mark a passed proposal as executed (called at epoch boundary).
    pub fn execute_proposal(&mut self, proposal_id: Hash32) -> Result<Hash32, ProposalError> {
        let proposal =
            self.proposals.get_mut(&proposal_id).ok_or(ProposalError::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Passed {
            return Err(ProposalError::ProposalNotPassed);
        }

        proposal.status = ProposalStatus::Executed;
        Ok(proposal.proposed_commit)
    }

    /// Mark a proposal as invalid and burn the deposit.
    pub fn mark_invalid(
        &mut self,
        proposal_id: Hash32,
        current_height: u64,
        epoch_length_blocks: u64,
    ) -> Result<(), ProposalError> {
        let proposal =
            self.proposals.get_mut(&proposal_id).ok_or(ProposalError::ProposalNotFound)?;

        if proposal.status != ProposalStatus::Active {
            return Err(ProposalError::ProposalNotActive);
        }

        proposal.status = ProposalStatus::Rejected;
        self.cooldowns.insert(
            proposal.proposer_id,
            current_height + (self.params.rejected_cooldown_epochs * epoch_length_blocks),
        );

        Ok(())
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn get_proposal(&self, proposal_id: &Hash32) -> Option<&GovernanceProposal> {
        self.proposals.get(proposal_id)
    }

    pub fn get_votes(&self, proposal_id: &Hash32) -> Option<&Vec<GovernanceVote>> {
        self.votes.get(proposal_id)
    }

    pub fn is_in_cooldown(&self, proposer_id: &Hash32, current_height: u64) -> bool {
        self.cooldowns.get(proposer_id).map(|&until| current_height < until).unwrap_or(false)
    }

    pub fn proposal_ids(&self) -> Vec<Hash32> {
        self.proposals.keys().copied().collect()
    }

    pub fn active_proposal_ids(&self) -> Vec<Hash32> {
        self.proposals
            .iter()
            .filter(|(_, p)| p.status == ProposalStatus::Active)
            .map(|(k, _)| *k)
            .collect()
    }

    pub fn passed_proposal_ids(&self) -> Vec<Hash32> {
        self.proposals
            .iter()
            .filter(|(_, p)| p.status == ProposalStatus::Passed)
            .map(|(k, _)| *k)
            .collect()
    }
}

/// Compute a proposal ID from its content.
pub fn compute_proposal_id(
    proposer_id: &Hash32,
    proposed_commit: &Hash32,
    bundle_manifest_hash: &Hash32,
    current_commit: &Hash32,
    nonce: u64,
) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(proposer_id);
    hasher.update(proposed_commit);
    hasher.update(bundle_manifest_hash);
    hasher.update(current_commit);
    hasher.update(nonce.to_le_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalError {
    MaxOpenProposalsReached,
    ProposerLimitExceeded,
    ProposerInCooldown,
    ProposalNotFound,
    ProposalNotActive,
    ProposalNotPassed,
    VoteWindowClosed,
    VoteWindowNotEnded,
    AlreadyVoted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProposalOutcome {
    Passed {
        yes_weight: u128,
        no_weight: u128,
        participated_weight: u128,
        deposit_returned: bool,
    },
    Rejected {
        yes_weight: u128,
        no_weight: u128,
        participated_weight: u128,
        quorum_met: bool,
        deposit_burned: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_proposer() -> Hash32 {
        [0xAA; 32]
    }

    #[test]
    fn submit_proposal_creates_active_proposal() {
        let mut engine = GovernanceEngine::new(GovernanceParams::default());
        let p_id = compute_proposal_id(&[0xAA; 32], &[0xBB; 32], &[0xCC; 32], &[0xDD; 32], 1);

        let result = engine.submit_proposal(
            p_id,
            [0xAA; 32],
            [0xBB; 32],
            [0xCC; 32],
            [0xDD; 32],
            100,
            0,
            1_000_000_000,
        );
        assert!(result.is_ok());
        let p = result.unwrap();
        assert_eq!(p.status, ProposalStatus::Active);
        assert_eq!(p.proposer_id, [0xAA; 32]);
        assert_eq!(p.deposit_amount, 500_000_000_000_000_000_000u128);
    }

    #[test]
    fn max_32_open_proposals_enforced() {
        let mut engine = GovernanceEngine::new(GovernanceParams::default());
        let proposer = make_proposer();
        for i in 0..32 {
            let p_id = [i as u8; 32];
            engine
                .submit_proposal(
                    p_id,
                    proposer,
                    [i as u8; 32],
                    [0; 32],
                    [0; 32],
                    100 + i as u64,
                    i as u64,
                    1_000_000_000,
                )
                .unwrap();
        }
        // 33rd proposal should be rejected
        let result = engine.submit_proposal(
            [33u8; 32],
            proposer,
            [33u8; 32],
            [0; 32],
            [0; 32],
            200,
            33,
            1_000_000_000,
        );
        assert_eq!(result, Err(ProposalError::MaxOpenProposalsReached));
    }

    #[test]
    fn cast_vote_updates_tally() {
        let mut engine = GovernanceEngine::new(GovernanceParams::default());
        let proposer = make_proposer();
        let p_id = compute_proposal_id(&proposer, &[0xBB; 32], &[0; 32], &[0; 32], 1);

        engine
            .submit_proposal(p_id, proposer, [0xBB; 32], [0; 32], [0; 32], 100, 0, 100_000_000_000)
            .unwrap();

        let vote = GovernanceVote {
            proposal_id: p_id,
            voter_id: [0x11; 32],
            vote: VoteOption::Yes,
            reason_hash: [0; 32],
            vote_weight: 30_000_000_000,
            signature: vec![],
        };
        assert!(engine.cast_vote(vote, 200).is_ok());

        let p = engine.get_proposal(&p_id).unwrap();
        assert_eq!(p.yes_weight, 30_000_000_000);
        assert_eq!(p.no_weight, 0);
    }

    #[test]
    fn double_voting_rejected() {
        let mut engine = GovernanceEngine::new(GovernanceParams::default());
        let proposer = make_proposer();
        let p_id = [0x42; 32];

        engine
            .submit_proposal(p_id, proposer, [0xBB; 32], [0; 32], [0; 32], 100, 0, 100_000)
            .unwrap();

        let vote1 = GovernanceVote {
            proposal_id: p_id,
            voter_id: [0x11; 32],
            vote: VoteOption::Yes,
            reason_hash: [0; 32],
            vote_weight: 50_000,
            signature: vec![],
        };
        engine.cast_vote(vote1, 200).unwrap();

        let vote2 = GovernanceVote {
            proposal_id: p_id,
            voter_id: [0x11; 32],
            vote: VoteOption::No,
            reason_hash: [0; 32],
            vote_weight: 50_000,
            signature: vec![],
        };
        assert_eq!(engine.cast_vote(vote2, 200), Err(ProposalError::AlreadyVoted));
    }

    #[test]
    fn proposal_passes_with_supermajority_and_quorum() {
        let mut engine = GovernanceEngine::new(GovernanceParams {
            vote_window_blocks: 100,
            ..GovernanceParams::default()
        });
        let proposer = make_proposer();
        let total_stake: u128 = 100_000;
        let p_id = [0x01; 32];

        engine
            .submit_proposal(p_id, proposer, [0xBB; 32], [0; 32], [0; 32], 0, 0, total_stake)
            .unwrap();

        // 45% of stake votes yes (> 40% quorum)
        engine
            .cast_vote(
                GovernanceVote {
                    proposal_id: p_id,
                    voter_id: [1; 32],
                    vote: VoteOption::Yes,
                    reason_hash: [0; 32],
                    vote_weight: 45_000,
                    signature: vec![],
                },
                50,
            )
            .unwrap();

        // Finalize after vote window
        let outcome = engine.finalize_proposal(p_id, 200, total_stake, 10).unwrap();
        assert!(matches!(outcome, ProposalOutcome::Passed { .. }));

        let p = engine.get_proposal(&p_id).unwrap();
        assert_eq!(p.status, ProposalStatus::Passed);
    }

    #[test]
    fn proposal_rejected_insufficient_quorum() {
        let mut engine = GovernanceEngine::new(GovernanceParams {
            vote_window_blocks: 100,
            ..GovernanceParams::default()
        });
        let proposer = make_proposer();
        let total_stake: u128 = 100_000;
        let p_id = [0x02; 32];

        engine
            .submit_proposal(p_id, proposer, [0xBB; 32], [0; 32], [0; 32], 0, 0, total_stake)
            .unwrap();

        // Only 10% votes yes (< 40% quorum)
        engine
            .cast_vote(
                GovernanceVote {
                    proposal_id: p_id,
                    voter_id: [1; 32],
                    vote: VoteOption::Yes,
                    reason_hash: [0; 32],
                    vote_weight: 10_000,
                    signature: vec![],
                },
                50,
            )
            .unwrap();

        let outcome = engine.finalize_proposal(p_id, 200, total_stake, 10).unwrap();
        assert!(matches!(outcome, ProposalOutcome::Rejected { quorum_met: false, .. }));

        let p = engine.get_proposal(&p_id).unwrap();
        assert_eq!(p.status, ProposalStatus::Rejected);
    }

    #[test]
    fn proposal_rejected_no_majority() {
        let mut engine = GovernanceEngine::new(GovernanceParams {
            vote_window_blocks: 100,
            ..GovernanceParams::default()
        });
        let proposer = make_proposer();
        let total_stake: u128 = 100_000;
        let p_id = [0x03; 32];

        engine
            .submit_proposal(p_id, proposer, [0xBB; 32], [0; 32], [0; 32], 0, 0, total_stake)
            .unwrap();

        // 25% yes, 25% no — quorum met (50% > 40%), but yes not > 50% of participated
        engine
            .cast_vote(
                GovernanceVote {
                    proposal_id: p_id,
                    voter_id: [1; 32],
                    vote: VoteOption::Yes,
                    reason_hash: [0; 32],
                    vote_weight: 25_000,
                    signature: vec![],
                },
                50,
            )
            .unwrap();
        engine
            .cast_vote(
                GovernanceVote {
                    proposal_id: p_id,
                    voter_id: [2; 32],
                    vote: VoteOption::No,
                    reason_hash: [0; 32],
                    vote_weight: 25_000,
                    signature: vec![],
                },
                50,
            )
            .unwrap();

        let outcome = engine.finalize_proposal(p_id, 200, total_stake, 10).unwrap();
        assert!(matches!(outcome, ProposalOutcome::Rejected { quorum_met: true, .. }));
    }

    #[test]
    fn execute_passed_proposal_transitions_git_head() {
        let mut engine = GovernanceEngine::new(GovernanceParams {
            vote_window_blocks: 100,
            ..GovernanceParams::default()
        });
        let proposer = make_proposer();
        let proposed_commit = [0xFF; 32];
        let p_id = [0x04; 32];

        engine
            .submit_proposal(p_id, proposer, proposed_commit, [0; 32], [0; 32], 0, 0, 100_000)
            .unwrap();

        engine
            .cast_vote(
                GovernanceVote {
                    proposal_id: p_id,
                    voter_id: [1; 32],
                    vote: VoteOption::Yes,
                    reason_hash: [0; 32],
                    vote_weight: 50_000,
                    signature: vec![],
                },
                50,
            )
            .unwrap();

        engine.finalize_proposal(p_id, 200, 100_000, 10).unwrap();

        let new_commit = engine.execute_proposal(p_id).unwrap();
        assert_eq!(new_commit, proposed_commit);

        let p = engine.get_proposal(&p_id).unwrap();
        assert_eq!(p.status, ProposalStatus::Executed);
    }

    #[test]
    fn cooldown_applied_after_rejection() {
        let mut engine = GovernanceEngine::new(GovernanceParams {
            vote_window_blocks: 100,
            ..GovernanceParams::default()
        });
        let proposer = make_proposer();
        let p_id = [0x05; 32];

        engine
            .submit_proposal(p_id, proposer, [0xBB; 32], [0; 32], [0; 32], 0, 0, 100_000)
            .unwrap();

        engine.finalize_proposal(p_id, 200, 100_000, 10).unwrap();

        // Proposer should be in cooldown
        assert!(engine.is_in_cooldown(&proposer, 100));
    }
}
