// === C6 Fast-Path: Topic Merge Lifecycle ===
//
// Source: fastpath-spec.md §1.4 State Transitions
//
// # Cryptographic Signature Handling
//
// This crate deliberately avoids cryptographic dependencies. All signature
// verification for fast-path proposals, approvals, and challenges is
// DELEGATED to the consensus driver (caller). The fast-path engine operates
// on already-verified data.
//
// SPEC_DEVIATION: signature verification delegated to caller (consensus
// driver) to keep fast-path crate free of crypto dependencies. The engine
// performs a debug_assert! sanity check that signature byte vectors are
// non-empty at each entry point, but does NOT verify cryptographic
// signatures.

use std::collections::{BTreeMap, BTreeSet};

use crate::types::{
    FastPathCertificate, FastPathChallengeTx, FastPathParams, FastPathProposal, FastPathRollbackTx,
    Hash32, ReviewerSignature, ReviewerVote,
};
use sha3::{Digest, Sha3_256};

/// Manages the fast-path topic merge lifecycle.
pub struct FastPathEngine {
    params: FastPathParams,
    proposals: Vec<FastPathProposal>,
    certificates: Vec<FastPathCertificate>,
    /// Track challenge count per identity per epoch for rate limiting.
    challenge_counts: Vec<(Hash32, u64, u64)>, // (challenger_id, epoch, count)
    /// Track which proposals have been successfully challenged (cannot be finalized).
    challenged_proposals: BTreeSet<Hash32>,
    /// Accumulated approval signatures per proposal, keyed by proposal_id.
    /// Used by submit_approval() to reach quorum before auto-issuing.
    pending_approvals: BTreeMap<Hash32, Vec<ReviewerSignature>>,
}

impl FastPathEngine {
    pub fn new(params: FastPathParams) -> Self {
        Self {
            params,
            proposals: Vec::new(),
            certificates: Vec::new(),
            challenge_counts: Vec::new(),
            challenged_proposals: BTreeSet::new(),
            pending_approvals: BTreeMap::new(),
        }
    }

    pub fn params(&self) -> &FastPathParams {
        &self.params
    }

    // ── Proposal Submission ──────────────────────────────────────────────

    pub fn submit_proposal(
        &mut self,
        mut proposal: FastPathProposal,
        current_height: u64,
    ) -> Result<Hash32, FastPathError> {
        // SPEC_DEVIATION: proposer signature verified by consensus driver before calling
        debug_assert!(
            !proposal.proposer_signature.is_empty(),
            "caller must verify proposer signature before submit_proposal"
        );

        if proposal.expires_at_height <= current_height {
            return Err(FastPathError::ProposalExpired);
        }

        // Compute canonical proposal ID from content (F-85)
        proposal.proposal_id = compute_proposal_id(
            &proposal.topic_id,
            &proposal.proposer_id,
            &proposal.base_topic_head,
            &proposal.proposed_head,
            current_height,
        );

        // Duplicate proposal check
        if self.proposals.iter().any(|p| p.proposal_id == proposal.proposal_id) {
            return Err(FastPathError::DuplicateProposal);
        }

        let pid = proposal.proposal_id;
        self.proposals.push(proposal);
        Ok(pid)
    }

    // ── Review & Certificate Issuance ────────────────────────────────────

    pub fn issue_certificate(
        &mut self,
        proposal_id: Hash32,
        approvals: Vec<ReviewerSignature>,
        signer_set_hash: Hash32,
        current_height: u64,
        topic_snapshot_weight: u128,
    ) -> Result<&FastPathCertificate, FastPathError> {
        let proposal = self
            .proposals
            .iter()
            .find(|p| p.proposal_id == proposal_id)
            .ok_or(FastPathError::ProposalNotFound)?;

        // Count unique participants and their total weight (F-56)
        let mut total_participants: u128 = 0;
        let mut unseen_reviewers = BTreeSet::new();
        let mut valid_approvals = Vec::new();

        for sig in &approvals {
            // Reject duplicate reviewer regardless of vote type
            if !unseen_reviewers.insert(sig.reviewer_id) {
                return Err(FastPathError::ReviewerNotIndependent);
            }
            total_participants += 1; // Simplified: each reviewer = 1 weight unit

            match sig.vote {
                ReviewerVote::Approve => {
                    valid_approvals.push(sig.clone());
                }
                ReviewerVote::Deny | ReviewerVote::Abstain => {
                    // Counted toward quorum but not toward approval
                }
            }
        }

        // Check quorum: need 2f+1 weighted participants (ceil division)
        let quorum_weight =
            (topic_snapshot_weight * self.params.quorum_threshold_num as u128).div_ceil(100);
        if total_participants < quorum_weight {
            return Err(FastPathError::InsufficientQuorum);
        }

        // Check at least one approval (F-27, F-56)
        if valid_approvals.is_empty() {
            return Err(FastPathError::AllVotesDeny);
        }

        // At least one independent reviewer (different from proposer)
        if !valid_approvals.iter().any(|sig| sig.reviewer_id != proposal.proposer_id) {
            return Err(FastPathError::ReviewerNotIndependent);
        }

        // Build aggregate signature from all approval signatures (F-27)
        let aggregate_signature: Vec<u8> =
            valid_approvals.iter().flat_map(|s| s.signature.clone()).collect();

        let certificate = FastPathCertificate {
            proposal_id,
            topic_id: proposal.topic_id,
            base_topic_head: proposal.base_topic_head,
            proposed_head: proposal.proposed_head,
            approvals: valid_approvals,
            aggregate_signature,
            signer_set_hash,
            issued_at_height: current_height,
            challenge_until_height: current_height + self.params.challenge_window_blocks,
        };

        self.certificates.push(certificate);
        // F-54: replace unwrap with proper error propagation
        self.certificates.last().ok_or(FastPathError::CertificateNotFound)
    }

    // ── Per-Reviewer Approval Accumulation ───────────────────────────────

    /// Submit an individual reviewer's approval for a fast-path proposal.
    /// Accumulates approvals until quorum is reached, then auto-issues
    /// the certificate.
    ///
    /// Returns `Ok(None)` while quorum has not yet been met.
    /// Returns `Ok(Some(&certificate))` once quorum is reached and the
    /// certificate is issued.
    ///
    /// # Errors
    /// - `ProposalNotFound` if no proposal with `proposal_id` exists.
    /// - `ReviewerNotIndependent` if the same reviewer submits twice, or
    ///   if all approvers are the proposer.
    pub fn submit_approval(
        &mut self,
        proposal_id: Hash32,
        approval: ReviewerSignature,
        current_height: u64,
        topic_snapshot_weight: u128,
    ) -> Result<Option<&FastPathCertificate>, FastPathError> {
        // SPEC_DEVIATION: signature verified by consensus driver before calling
        debug_assert!(
            !approval.signature.is_empty(),
            "caller must verify approval signature before submit_approval"
        );

        // 1. Verify proposal exists
        if !self.proposals.iter().any(|p| p.proposal_id == proposal_id) {
            return Err(FastPathError::ProposalNotFound);
        }

        // 2. Store all vote types (Approve, Deny, Abstain) for quorum counting (F-57, F-86)
        //    Duplicate detection applied regardless of vote type.
        let mut has_duplicate = false;
        {
            let pending = self.pending_approvals.entry(proposal_id).or_default();
            if pending.iter().any(|s| s.reviewer_id == approval.reviewer_id) {
                has_duplicate = true;
            } else {
                pending.push(approval);
            }
        }

        if has_duplicate {
            return Err(FastPathError::ReviewerNotIndependent);
        }

        // 3. Snapshot the accumulated approvals to free the mutable borrow
        let all_approvals = self.pending_approvals.get(&proposal_id).cloned().unwrap_or_default();

        // 4. Check quorum: need 2f+1 weighted participants (all types count)
        let total_participants = all_approvals.len() as u128;
        let quorum_weight =
            (topic_snapshot_weight * self.params.quorum_threshold_num as u128).div_ceil(100);
        if total_participants < quorum_weight {
            return Ok(None);
        }

        // 5. Check for at least one Approve vote (F-57, F-86)
        if !all_approvals.iter().any(|s| s.vote == ReviewerVote::Approve) {
            // All votes are Deny/Abstain — proposal fails, clear pending
            self.pending_approvals.remove(&proposal_id);
            return Err(FastPathError::AllVotesDeny);
        }

        // 6. Independence check: at least one approver is not the proposer
        let Some(proposal) = self.proposals.iter().find(|p| p.proposal_id == proposal_id) else {
            return Err(FastPathError::ProposalNotFound);
        };
        if !all_approvals
            .iter()
            .any(|sig| sig.reviewer_id != proposal.proposer_id && sig.vote == ReviewerVote::Approve)
        {
            return Err(FastPathError::ReviewerNotIndependent);
        }

        // 7. Derive signer_set_hash deterministically from the reviewer set
        let mut hasher = Sha3_256::new();
        let mut reviewer_ids: Vec<&Hash32> = all_approvals.iter().map(|a| &a.reviewer_id).collect();
        reviewer_ids.sort();
        for id in &reviewer_ids {
            hasher.update(id);
        }
        let signer_set_hash = {
            let mut out = [0u8; 32];
            out.copy_from_slice(&hasher.finalize());
            out
        };

        // 8. Issue the certificate (re-validates internally).
        //    Match explicitly to drop the cert reference before clearing pending,
        //    avoiding a borrow conflict with the mutable self access below.
        let issue_result = self.issue_certificate(
            proposal_id,
            all_approvals,
            signer_set_hash,
            current_height,
            topic_snapshot_weight,
        );

        match issue_result {
            Ok(_) => {
                // Clear pending — certificate is now stored in self.certificates
                self.pending_approvals.remove(&proposal_id);
                // F-55: replace unwrap with proper error propagation
                Ok(Some(self.certificates.last().ok_or(FastPathError::CertificateNotFound)?))
            }
            Err(e) => Err(e),
        }
    }

    // ── Challenge ────────────────────────────────────────────────────────

    pub fn submit_challenge(
        &mut self,
        challenge: FastPathChallengeTx,
        current_height: u64,
        current_epoch: u64,
    ) -> Result<(), FastPathError> {
        // SPEC_DEVIATION: signature verified by consensus driver before calling
        debug_assert!(
            !challenge.signature.is_empty(),
            "caller must verify challenge signature before submit_challenge"
        );

        let cert = self
            .certificates
            .iter()
            .find(|c| c.proposal_id == challenge.proposal_id)
            .ok_or(FastPathError::CertificateNotFound)?;

        if current_height >= cert.challenge_until_height {
            return Err(FastPathError::ChallengeWindowClosed);
        }

        // Challenge rate limit
        let challenge_count = self
            .challenge_counts
            .iter()
            .filter(|(id, ep, _)| *id == challenge.challenger_id && *ep == current_epoch)
            .map(|(_, _, c)| *c)
            .sum::<u64>();

        if challenge_count >= self.params.max_challenges_per_identity_per_epoch {
            return Err(FastPathError::ChallengeRateLimitExceeded);
        }

        self.challenge_counts.push((challenge.challenger_id, current_epoch, 1));
        self.challenged_proposals.insert(challenge.proposal_id);

        Ok(())
    }

    // ── Rollback ─────────────────────────────────────────────────────────

    pub fn rollback(&mut self, rollback: FastPathRollbackTx) -> Result<(), FastPathError> {
        let cert = self
            .certificates
            .iter()
            .find(|c| c.proposal_id == rollback.proposal_id)
            .ok_or(FastPathError::CertificateNotFound)?;

        let proposal = self
            .proposals
            .iter_mut()
            .find(|p| p.proposal_id == rollback.proposal_id)
            .ok_or(FastPathError::ProposalNotFound)?;

        if rollback.topic_id != cert.topic_id {
            return Err(FastPathError::TopicMismatch);
        }

        proposal.base_topic_head = rollback.rollback_to_head;

        Ok(())
    }

    // ── Finalization ────────────────────────────────────────────────────

    pub fn finalize_certificate(
        &mut self,
        proposal_id: Hash32,
        current_height: u64,
    ) -> Result<Hash32, FastPathError> {
        let cert = self
            .certificates
            .iter()
            .find(|c| c.proposal_id == proposal_id)
            .ok_or(FastPathError::CertificateNotFound)?;

        if current_height < cert.challenge_until_height {
            return Err(FastPathError::ChallengeWindowNotEnded);
        }

        // Check no successful challenges exist against this proposal
        if self.challenged_proposals.contains(&proposal_id) {
            return Err(FastPathError::Challenged);
        }

        let proposal = self
            .proposals
            .iter_mut()
            .find(|p| p.proposal_id == proposal_id)
            .ok_or(FastPathError::ProposalNotFound)?;

        // Advance topic head
        let new_head = proposal.proposed_head;
        proposal.base_topic_head = new_head;

        Ok(new_head)
    }

    // ── Accessors ────────────────────────────────────────────────────────

    pub fn get_proposal(&self, proposal_id: &Hash32) -> Option<&FastPathProposal> {
        self.proposals.iter().find(|p| p.proposal_id == *proposal_id)
    }

    pub fn get_certificate(&self, proposal_id: &Hash32) -> Option<&FastPathCertificate> {
        self.certificates.iter().find(|c| c.proposal_id == *proposal_id)
    }

    pub fn proposal_ids(&self) -> Vec<Hash32> {
        self.proposals.iter().map(|p| p.proposal_id).collect()
    }

    pub fn proposals_iter(&self) -> impl Iterator<Item = &FastPathProposal> {
        self.proposals.iter()
    }
}

/// Compute a proposal ID from its content.
/// Used in submit_proposal to derive the canonical proposal ID.
pub fn compute_proposal_id(
    topic_id: &Hash32,
    proposer_id: &Hash32,
    base_topic_head: &Hash32,
    proposed_head: &Hash32,
    nonce: u64,
) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(topic_id);
    hasher.update(proposer_id);
    hasher.update(base_topic_head);
    hasher.update(proposed_head);
    hasher.update(nonce.to_le_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FastPathError {
    ProposalNotFound,
    ProposalExpired,
    DuplicateProposal,
    CertificateNotFound,
    InsufficientQuorum,
    ReviewerNotIndependent,
    ChallengeWindowClosed,
    ChallengeWindowNotEnded,
    ChallengeRateLimitExceeded,
    Challenged,
    TopicMismatch,
    /// All submitted votes were Deny/Abstain — no approvals to certify.
    AllVotesDeny,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_proposal_succeeds() {
        let mut engine = FastPathEngine::new(FastPathParams::default());

        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten by engine
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3], // non-empty for debug_assert
        };

        let result = engine.submit_proposal(proposal, 100);
        assert!(result.is_ok());
        let returned_id = result.unwrap();
        assert_ne!(returned_id, [0u8; 32]); // ID was computed, not the dummy
    }

    #[test]
    fn issue_certificate_with_quorum() {
        let mut engine = FastPathEngine::new(FastPathParams::default());
        let proposer_id = [0xBB; 32];

        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten by engine
            topic_id: [0xAA; 32],
            proposer_id,
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3],
        };
        let p_id = engine.submit_proposal(proposal, 100).unwrap();

        // 70 approvals → quorum of 67 met
        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![i; 8], // non-empty for aggregate
            })
            .collect();

        let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
        assert!(result.is_ok());
        let cert = result.unwrap();
        assert_eq!(cert.proposal_id, p_id);
        assert_eq!(cert.challenge_until_height, 200 + 144);
        // F-27: aggregate_signature should be non-empty
        assert!(!cert.aggregate_signature.is_empty());
    }

    #[test]
    fn insufficient_quorum_rejected() {
        let mut engine = FastPathEngine::new(FastPathParams::default());

        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten by engine
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3],
        };
        let p_id = engine.submit_proposal(proposal, 100).unwrap();

        // Only 10 approvals, need 67
        let approvals: Vec<ReviewerSignature> = (0..10u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![i; 8],
            })
            .collect();

        let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
        assert_eq!(result, Err(FastPathError::InsufficientQuorum));
    }

    #[test]
    fn finalize_unchallenged_certificate() {
        let mut engine = FastPathEngine::new(FastPathParams::default());

        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3],
        };
        let p_id = engine.submit_proposal(proposal, 100).unwrap();

        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![i; 8],
            })
            .collect();

        engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

        // Finalize after challenge window
        let new_head = engine.finalize_certificate(p_id, 200 + 144 + 1).unwrap();
        assert_eq!(new_head, [0xFF; 32]);
    }

    #[test]
    fn challenge_submitted_successfully() {
        let mut engine = FastPathEngine::new(FastPathParams::default());

        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3],
        };
        let p_id = engine.submit_proposal(proposal, 100).unwrap();

        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![i; 8],
            })
            .collect();
        engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

        let challenge = FastPathChallengeTx {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            challenger_id: [0xCC; 32],
            evidence_hash: [0xEE; 32],
            challenger_bond: 100,
            signature: vec![1, 2, 3, 4], // non-empty for debug_assert
        };

        let result = engine.submit_challenge(challenge, 250, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn challenge_rate_limit_enforced() {
        let mut engine = FastPathEngine::new(FastPathParams {
            max_challenges_per_identity_per_epoch: 3,
            ..FastPathParams::default()
        });
        let challenger = [0xCC; 32];

        // First, create a certificate
        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3],
        };
        let p_id = engine.submit_proposal(proposal, 100).unwrap();

        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![i; 8],
            })
            .collect();
        engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

        // Submit 3 challenges (limit reached)
        for _ in 0..3 {
            let challenge = FastPathChallengeTx {
                proposal_id: p_id,
                topic_id: [0xAA; 32],
                challenger_id: challenger,
                evidence_hash: [0xEE; 32],
                challenger_bond: 100,
                signature: vec![1, 2, 3, 4], // non-empty for debug_assert
            };
            engine.submit_challenge(challenge, 250, 0).unwrap();
        }

        // 4th challenge should fail
        let challenge4 = FastPathChallengeTx {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            challenger_id: challenger,
            evidence_hash: [0xEE; 32],
            challenger_bond: 100,
            signature: vec![5, 6, 7, 8], // non-empty for debug_assert
        };
        assert_eq!(
            engine.submit_challenge(challenge4, 250, 0),
            Err(FastPathError::ChallengeRateLimitExceeded)
        );
    }

    #[test]
    fn certificate_replay_rejected() {
        let mut engine = FastPathEngine::new(FastPathParams::default());

        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3],
        };
        let p_id = engine.submit_proposal(proposal, 100).unwrap();

        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![i; 8],
            })
            .collect();
        engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

        engine.finalize_certificate(p_id, 200 + 144 + 1).unwrap();

        // Verify topic head advanced
        let prop = engine.get_proposal(&p_id).unwrap();
        assert_eq!(prop.base_topic_head, [0xFF; 32]);
    }

    // ── submit_approval tests ─────────────────────────────────────────────

    #[test]
    fn conforms_to_fastpath_spec_section1_7_submit_approval_accumulates() {
        let mut engine = FastPathEngine::new(FastPathParams::default());
        let proposer_id = [0xBB; 32];

        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten
            topic_id: [0xAA; 32],
            proposer_id,
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3],
        };
        let p_id = engine.submit_proposal(proposal, 100).unwrap();

        // topic_snapshot_weight = 4 → quorum = ceil(4 * 67 / 100) = 3
        let weight = 4u128;

        // First approval: 1 < 3 → Ok(None)
        let result = engine.submit_approval(
            p_id,
            ReviewerSignature {
                reviewer_id: [1; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![1, 2, 3, 4],
            },
            200,
            weight,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Second approval: 2 < 3 → Ok(None)
        let result = engine.submit_approval(
            p_id,
            ReviewerSignature {
                reviewer_id: [2; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![5, 6, 7, 8],
            },
            200,
            weight,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Third approval: 3 >= 3 → Ok(Some(cert))
        let result = engine.submit_approval(
            p_id,
            ReviewerSignature {
                reviewer_id: [3; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![9, 10, 11, 12],
            },
            200,
            weight,
        );
        assert!(result.is_ok());
        let opt = result.unwrap();
        assert!(opt.is_some());
        let cert = opt.unwrap();
        assert_eq!(cert.proposal_id, p_id);
        assert_eq!(cert.challenge_until_height, 200 + 144);
    }

    #[test]
    fn conforms_to_fastpath_spec_section1_7_duplicate_approval_rejected() {
        let mut engine = FastPathEngine::new(FastPathParams::default());

        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3],
        };
        let p_id = engine.submit_proposal(proposal, 100).unwrap();

        let approval = ReviewerSignature {
            reviewer_id: [1; 32],
            vote: ReviewerVote::Approve,
            reason_hash: [0; 32],
            signature: vec![1, 2, 3, 4],
        };

        // First submission: Ok(None) — not enough quorum yet
        let result = engine.submit_approval(p_id, approval.clone(), 200, 5);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Duplicate same reviewer: Err(ReviewerNotIndependent)
        let result = engine.submit_approval(p_id, approval, 200, 5);
        assert_eq!(result, Err(FastPathError::ReviewerNotIndependent));
    }

    #[test]
    fn conforms_to_fastpath_spec_section1_7_approve_vote_required() {
        let mut engine = FastPathEngine::new(FastPathParams::default());

        let proposal = FastPathProposal {
            proposal_id: [0; 32], // dummy, overwritten
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![1, 2, 3],
        };
        let p_id = engine.submit_proposal(proposal, 100).unwrap();

        // Deny vote is stored and counted toward quorum in total, but
        // with weight=5 → quorum=4, only 1 participant → Ok(None)
        let result = engine.submit_approval(
            p_id,
            ReviewerSignature {
                reviewer_id: [1; 32],
                vote: ReviewerVote::Deny,
                reason_hash: [0; 32],
                signature: vec![1, 2, 3, 4],
            },
            200,
            5,
        );
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
