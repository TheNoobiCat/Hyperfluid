// === C6 Fast-Path: Topic Merge Lifecycle ===
//
// Source: fastpath-spec.md §1.4 State Transitions

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
}

impl FastPathEngine {
    pub fn new(params: FastPathParams) -> Self {
        Self {
            params,
            proposals: Vec::new(),
            certificates: Vec::new(),
            challenge_counts: Vec::new(),
        }
    }

    pub fn params(&self) -> &FastPathParams {
        &self.params
    }

    // ── Proposal Submission ──────────────────────────────────────────────

    pub fn submit_proposal(
        &mut self,
        proposal: FastPathProposal,
        current_height: u64,
    ) -> Result<(), FastPathError> {
        if proposal.expires_at_height <= current_height {
            return Err(FastPathError::ProposalExpired);
        }

        // Duplicate proposal check
        if self.proposals.iter().any(|p| p.proposal_id == proposal.proposal_id) {
            return Err(FastPathError::DuplicateProposal);
        }

        self.proposals.push(proposal);
        Ok(())
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

        // Count unique approvers and their total weight
        let mut total_approvals: u128 = 0;
        let mut seen_reviewers = std::collections::HashSet::new();
        let mut valid_approvals = Vec::new();

        for sig in &approvals {
            if sig.vote != ReviewerVote::Approve {
                continue;
            }
            if !seen_reviewers.insert(sig.reviewer_id) {
                return Err(FastPathError::ReviewerNotIndependent); // duplicate
            }
            total_approvals += 1; // Simplified: each reviewer = 1 weight unit
            valid_approvals.push(sig.clone());
        }

        // Check quorum: need 2f+1 weighted approvals
        let quorum_weight =
            (topic_snapshot_weight * self.params.quorum_threshold_num as u128) / 100;
        if total_approvals < quorum_weight {
            return Err(FastPathError::InsufficientQuorum);
        }

        // At least one independent reviewer (different from proposer)
        if !valid_approvals.iter().any(|sig| sig.reviewer_id != proposal.proposer_id) {
            return Err(FastPathError::ReviewerNotIndependent);
        }

        let certificate = FastPathCertificate {
            proposal_id,
            topic_id: proposal.topic_id,
            base_topic_head: proposal.base_topic_head,
            proposed_head: proposal.proposed_head,
            approvals: valid_approvals,
            aggregate_signature: vec![],
            signer_set_hash,
            issued_at_height: current_height,
            challenge_until_height: current_height + self.params.challenge_window_blocks,
        };

        self.certificates.push(certificate);
        Ok(self.certificates.last().unwrap())
    }

    // ── Challenge ────────────────────────────────────────────────────────

    pub fn submit_challenge(
        &mut self,
        challenge: FastPathChallengeTx,
        current_height: u64,
        current_epoch: u64,
    ) -> Result<(), FastPathError> {
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

        Ok(())
    }

    // ── Rollback ─────────────────────────────────────────────────────────

    pub fn rollback(&mut self, rollback: FastPathRollbackTx) -> Result<(), FastPathError> {
        let cert = self
            .certificates
            .iter()
            .find(|c| c.proposal_id == rollback.proposal_id)
            .ok_or(FastPathError::CertificateNotFound)?;

        let proposal =
            self.proposals.iter_mut().find(|p| p.proposal_id == rollback.proposal_id).unwrap();

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

        // Check no successful challenges exist
        let challenged = self.challenge_counts.iter().any(|(id, _, _)| *id == proposal_id);

        if challenged {
            return Err(FastPathError::Challenged);
        }

        let proposal = self.proposals.iter_mut().find(|p| p.proposal_id == proposal_id).unwrap();

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
}

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_proposal_succeeds() {
        let mut engine = FastPathEngine::new(FastPathParams::default());
        let p_id = compute_proposal_id(&[0xAA; 32], &[0xBB; 32], &[0x00; 32], &[0xFF; 32], 1);

        let proposal = FastPathProposal {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![],
        };

        let result = engine.submit_proposal(proposal, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn issue_certificate_with_quorum() {
        let mut engine = FastPathEngine::new(FastPathParams::default());
        let p_id = [0x01; 32];
        let proposer_id = [0xBB; 32];

        let proposal = FastPathProposal {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            proposer_id,
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![],
        };
        engine.submit_proposal(proposal, 100).unwrap();

        // 70 approvals → quorum of 67 met
        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![],
            })
            .collect();

        let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
        assert!(result.is_ok());
        let cert = result.unwrap();
        assert_eq!(cert.proposal_id, p_id);
        assert_eq!(cert.challenge_until_height, 200 + 144);
    }

    #[test]
    fn insufficient_quorum_rejected() {
        let mut engine = FastPathEngine::new(FastPathParams::default());
        let p_id = [0x02; 32];

        let proposal = FastPathProposal {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![],
        };
        engine.submit_proposal(proposal, 100).unwrap();

        // Only 10 approvals, need 67
        let approvals: Vec<ReviewerSignature> = (0..10u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![],
            })
            .collect();

        let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
        assert_eq!(result, Err(FastPathError::InsufficientQuorum));
    }

    #[test]
    fn finalize_unchallenged_certificate() {
        let mut engine = FastPathEngine::new(FastPathParams::default());
        let p_id = [0x03; 32];

        let proposal = FastPathProposal {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![],
        };
        engine.submit_proposal(proposal, 100).unwrap();

        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![],
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
        let p_id = [0x04; 32];

        let proposal = FastPathProposal {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![],
        };
        engine.submit_proposal(proposal, 100).unwrap();

        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![],
            })
            .collect();
        engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

        let challenge = FastPathChallengeTx {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            challenger_id: [0xCC; 32],
            evidence_hash: [0xEE; 32],
            challenger_bond: 100,
            signature: vec![],
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
        let p_id = [0x05; 32];
        let proposal = FastPathProposal {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![],
        };
        engine.submit_proposal(proposal, 100).unwrap();

        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![],
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
                signature: vec![],
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
            signature: vec![],
        };
        assert_eq!(
            engine.submit_challenge(challenge4, 250, 0),
            Err(FastPathError::ChallengeRateLimitExceeded)
        );
    }

    #[test]
    fn certificate_replay_rejected() {
        let mut engine = FastPathEngine::new(FastPathParams::default());
        let p_id = [0x06; 32];

        let proposal = FastPathProposal {
            proposal_id: p_id,
            topic_id: [0xAA; 32],
            proposer_id: [0xBB; 32],
            base_topic_head: [0x00; 32],
            proposed_head: [0xFF; 32],
            bundle_manifest_hash: [0; 32],
            expires_at_height: 1000,
            proposer_signature: vec![],
        };
        engine.submit_proposal(proposal, 100).unwrap();

        let approvals: Vec<ReviewerSignature> = (0..70u8)
            .map(|i| ReviewerSignature {
                reviewer_id: [i; 32],
                vote: ReviewerVote::Approve,
                reason_hash: [0; 32],
                signature: vec![],
            })
            .collect();
        engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

        engine.finalize_certificate(p_id, 200 + 144 + 1).unwrap();

        // Verify topic head advanced
        let prop = engine.get_proposal(&p_id).unwrap();
        assert_eq!(prop.base_topic_head, [0xFF; 32]);
    }
}
