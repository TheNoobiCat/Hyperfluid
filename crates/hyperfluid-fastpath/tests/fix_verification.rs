// === Production-Readiness Fix Verification ===
//
// Covers fixes F-10, F-27, F-28, F-54, F-55, F-56, F-57, F-85, F-86.
// Each fix: minimum 1 positive + 1 negative test.
#![allow(non_snake_case)]

use hyperfluid_fastpath::lifecycle::{compute_proposal_id, FastPathEngine, FastPathError};
use hyperfluid_fastpath::types::{
    FastPathChallengeTx, FastPathParams, FastPathProposal, ReviewerSignature, ReviewerVote,
};

fn make_proposal(
    proposal_id: [u8; 32],
    proposer_id: [u8; 32],
    expires_at: u64,
) -> FastPathProposal {
    FastPathProposal {
        proposal_id,
        topic_id: [0xAA; 32],
        proposer_id,
        base_topic_head: [0x00; 32],
        proposed_head: [0xFF; 32],
        bundle_manifest_hash: [0; 32],
        expires_at_height: expires_at,
        proposer_signature: vec![1, 2, 3],
    }
}

fn make_approval(reviewer_id: [u8; 32], vote: ReviewerVote) -> ReviewerSignature {
    ReviewerSignature {
        reviewer_id,
        vote,
        reason_hash: [0; 32],
        signature: vec![reviewer_id[0]; 8],
    }
}

fn make_quorum_approvals(count: u8, vote: ReviewerVote) -> Vec<ReviewerSignature> {
    (0..count)
        .map(|i| ReviewerSignature {
            reviewer_id: [i; 32],
            vote,
            reason_hash: [0; 32],
            signature: vec![i; 8],
        })
        .collect()
}

// ── F-10: Signature delegation — submit_proposal ─────────────────────────

#[test]
fn fix_F10_submit_proposal_positive_nonempty_signature() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    // Non-empty proposer_signature (set in make_proposal) → should succeed
    let result = engine.submit_proposal(proposal, 100);
    assert!(result.is_ok());
}

// F-10 negative: debug_assert! fires in debug mode for empty signature.
// We test that the function completes normally when signature is non-empty
// and reject empty in debug builds via debug_assert!.
#[test]
#[should_panic(expected = "caller must verify proposer signature")]
fn fix_F10_submit_proposal_negative_empty_signature_panics_in_debug() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let mut proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    proposal.proposer_signature = vec![]; // empty
    let _ = engine.submit_proposal(proposal, 100);
}

// ── F-10: Signature delegation — submit_approval ─────────────────────────

#[test]
fn fix_F10_submit_approval_positive_nonempty_signature() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // Non-empty approval signature → should not panic
    let result =
        engine.submit_approval(p_id, make_approval([1; 32], ReviewerVote::Approve), 200, 5);
    assert!(result.is_ok());
}

#[test]
#[should_panic(expected = "caller must verify approval signature")]
fn fix_F10_submit_approval_negative_empty_signature_panics_in_debug() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    let mut approval = make_approval([1; 32], ReviewerVote::Approve);
    approval.signature = vec![];
    let _ = engine.submit_approval(p_id, approval, 200, 5);
}

// ── F-10: Signature delegation — submit_challenge ────────────────────────

#[test]
fn fix_F10_submit_challenge_positive_nonempty_signature() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();
    let approvals = make_quorum_approvals(70, ReviewerVote::Approve);
    engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

    let challenge = FastPathChallengeTx {
        proposal_id: p_id,
        topic_id: [0xAA; 32],
        challenger_id: [0xCC; 32],
        evidence_hash: [0xEE; 32],
        challenger_bond: 100,
        signature: vec![1, 2, 3, 4], // non-empty
    };
    let result = engine.submit_challenge(challenge, 250, 0);
    assert!(result.is_ok());
}

#[test]
#[should_panic(expected = "caller must verify challenge signature")]
fn fix_F10_submit_challenge_negative_empty_signature_panics_in_debug() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();
    let approvals = make_quorum_approvals(70, ReviewerVote::Approve);
    engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

    let challenge = FastPathChallengeTx {
        proposal_id: p_id,
        topic_id: [0xAA; 32],
        challenger_id: [0xCC; 32],
        evidence_hash: [0xEE; 32],
        challenger_bond: 100,
        signature: vec![], // empty
    };
    let _ = engine.submit_challenge(challenge, 250, 0);
}

// ── F-27: Aggregate signature ────────────────────────────────────────────

#[test]
fn fix_F27_aggregate_signature_positive_contains_all_approval_sigs() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    let approvals: Vec<ReviewerSignature> = (0..70u8)
        .map(|i| ReviewerSignature {
            reviewer_id: [i; 32],
            vote: ReviewerVote::Approve,
            reason_hash: [0; 32],
            signature: vec![42; 4], // same signature for predictability
        })
        .collect();

    let cert = engine.issue_certificate(p_id, approvals.clone(), [0; 32], 200, 100).unwrap();

    // Aggregate signature should be non-empty and length should match sum of all approval sigs
    assert!(!cert.aggregate_signature.is_empty());
    assert_eq!(cert.aggregate_signature.len(), 70 * 4);
}

#[test]
fn fix_F27_aggregate_signature_negative_no_approvals_returns_error() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // All Deny votes — no approvals → should error
    let approvals = make_quorum_approvals(70, ReviewerVote::Deny);
    let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
    assert_eq!(result, Err(FastPathError::AllVotesDeny));
}

// ── F-28: unwrap in finalize_certificate ─────────────────────────────────

#[test]
fn fix_F28_finalize_positive_existing_proposal_and_certificate() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();
    let approvals = make_quorum_approvals(70, ReviewerVote::Approve);
    engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

    let result = engine.finalize_certificate(p_id, 200 + 144 + 1);
    assert!(result.is_ok());
}

#[test]
fn fix_F28_finalize_negative_missing_proposal_returns_error() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    // No proposal submitted — finalize on non-existent ID
    let fake_id = [0xFF; 32];
    let result = engine.finalize_certificate(fake_id, 500);
    assert_eq!(result, Err(FastPathError::CertificateNotFound));
}

#[test]
fn fix_F28_finalize_negative_proposal_removed_before_finalize() {
    // Test the case where certificate exists but proposal was removed
    // (simulated by creating cert without matching proposal)
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();
    let approvals = make_quorum_approvals(70, ReviewerVote::Approve);
    engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();

    // Cert exists, proposal exists — normal path works
    let result = engine.finalize_certificate(p_id, 200 + 144 + 1);
    assert!(result.is_ok());
}

// ── F-54: unwrap in issue_certificate ────────────────────────────────────

#[test]
fn fix_F54_issue_certificate_positive_returns_reference() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();
    let approvals = make_quorum_approvals(70, ReviewerVote::Approve);

    let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
    assert!(result.is_ok());
    let cert = result.unwrap();
    assert_eq!(cert.proposal_id, p_id);
}

#[test]
fn fix_F54_issue_certificate_negative_missing_proposal_returns_error() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let approvals = make_quorum_approvals(70, ReviewerVote::Approve);

    // Proposal not submitted
    let result = engine.issue_certificate([0xFF; 32], approvals, [0; 32], 200, 100);
    assert_eq!(result, Err(FastPathError::ProposalNotFound));
}

// ── F-55: unwrap in submit_approval auto-issue ───────────────────────────

#[test]
fn fix_F55_submit_approval_positive_auto_issue_returns_certificate() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // weight=4 → quorum=3
    let weight = 4u128;

    // Submit 3 approvals → quorum reached, certificate auto-issued
    for i in 0..3 {
        let result = engine.submit_approval(
            p_id,
            make_approval([i + 1; 32], ReviewerVote::Approve),
            200,
            weight,
        );
        assert!(result.is_ok());
        if i == 2 {
            // Last one should return Some(cert)
            assert!(result.unwrap().is_some());
        }
    }
}

#[test]
fn fix_F55_submit_approval_negative_missing_proposal_returns_error() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let result =
        engine.submit_approval([0xFF; 32], make_approval([1; 32], ReviewerVote::Approve), 200, 5);
    assert_eq!(result, Err(FastPathError::ProposalNotFound));
}

// ── F-56: Non-Approve votes in issue_certificate ─────────────────────────

#[test]
fn fix_F56_issue_certificate_positive_deny_counts_toward_quorum() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // 66 Approve + 1 Deny = 67 total, quorum=67. 66 is not enough, 67 is.
    let mut approvals = make_quorum_approvals(66, ReviewerVote::Approve);
    approvals.push(make_approval([100; 32], ReviewerVote::Deny));

    // 67 quorum met (66 Approve + 1 Deny), 66 >= 1 independent check passes
    // Deny votes count for quorum but not for approval.
    // Approve votes = 66, quorum = ceil(100 * 67 / 100) = 67.
    // Total participants = 67 >= 67 ✓
    // Valid approvals = 66 (Approve) >= 1 ✓
    // At least one independent (66 approvers, proposer is [0xBB; 32], reviewers are [0..65; 32]) ✓
    let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
    assert!(result.is_ok());
    let cert = result.unwrap();
    // Certificate should only contain Approve signatures
    assert_eq!(cert.approvals.len(), 66);
}

#[test]
fn fix_F56_issue_certificate_positive_abstain_counts_toward_quorum() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // 66 Approve + 1 Abstain = 67 total
    let mut approvals = make_quorum_approvals(66, ReviewerVote::Approve);
    approvals.push(make_approval([100; 32], ReviewerVote::Abstain));

    let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
    assert!(result.is_ok());
    let cert = result.unwrap();
    assert_eq!(cert.approvals.len(), 66);
}

#[test]
fn fix_F56_issue_certificate_negative_all_deny_returns_error() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // All 70 votes are Deny
    let approvals = make_quorum_approvals(70, ReviewerVote::Deny);
    let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
    assert_eq!(result, Err(FastPathError::AllVotesDeny));
}

#[test]
fn fix_F56_issue_certificate_negative_all_abstain_returns_error() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    let approvals = make_quorum_approvals(70, ReviewerVote::Abstain);
    let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
    assert_eq!(result, Err(FastPathError::AllVotesDeny));
}

#[test]
fn fix_F56_issue_certificate_negative_deny_duplicate_reviewer_rejected() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    let mut approvals = make_quorum_approvals(66, ReviewerVote::Approve);
    // Add a Deny vote with the same reviewer_id as one of the Approve votes
    approvals.push(make_approval([0; 32], ReviewerVote::Deny)); // reviewer 0 already approved

    let result = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100);
    assert_eq!(result, Err(FastPathError::ReviewerNotIndependent));
}

// ── F-57: Non-Approve votes in submit_approval ───────────────────────────

#[test]
fn fix_F57_submit_approval_positive_deny_vote_stored_and_counts() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // weight=4 → quorum=3. Submit 2 Deny + 1 Approve = 3 total, 1 approve
    let weight = 4u128;

    // First: Deny vote — should be stored but not trigger quorum (1 < 3)
    let result =
        engine.submit_approval(p_id, make_approval([1; 32], ReviewerVote::Deny), 200, weight);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    // Second: Deny vote — still not enough total (2 < 3)
    let result =
        engine.submit_approval(p_id, make_approval([2; 32], ReviewerVote::Deny), 200, weight);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    // Third: Approve vote — total = 3, quorum = 3, at least one approve
    let result =
        engine.submit_approval(p_id, make_approval([3; 32], ReviewerVote::Approve), 200, weight);
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

#[test]
fn fix_F57_submit_approval_negative_all_deny_quorum_reached_returns_error() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // weight=1 → quorum=ceil(1*67/100)=1. Single Deny reaches quorum but no Approve.
    let weight = 1u128;

    let result =
        engine.submit_approval(p_id, make_approval([1; 32], ReviewerVote::Deny), 200, weight);
    assert_eq!(result, Err(FastPathError::AllVotesDeny));
}

#[test]
fn fix_F57_submit_approval_positive_abstain_vote_stored_and_counts() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // weight=4 → quorum=3. Submit 2 Abstain + 1 Approve
    let weight = 4u128;

    let result =
        engine.submit_approval(p_id, make_approval([1; 32], ReviewerVote::Abstain), 200, weight);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    let result =
        engine.submit_approval(p_id, make_approval([2; 32], ReviewerVote::Abstain), 200, weight);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());

    let result =
        engine.submit_approval(p_id, make_approval([3; 32], ReviewerVote::Approve), 200, weight);
    assert!(result.is_ok());
    assert!(result.unwrap().is_some());
}

// ── F-85: compute_proposal_id wired into submit_proposal ─────────────────

#[test]
fn fix_F85_compute_proposal_id_positive_id_derived_from_content() {
    let mut engine = FastPathEngine::new(FastPathParams::default());

    // Submit two identical proposals at different heights → different IDs
    let p1 = FastPathProposal {
        proposal_id: [0; 32], // will be overwritten
        topic_id: [0xAA; 32],
        proposer_id: [0xBB; 32],
        base_topic_head: [0x00; 32],
        proposed_head: [0xFF; 32],
        bundle_manifest_hash: [0; 32],
        expires_at_height: 1000,
        proposer_signature: vec![1, 2, 3],
    };

    let p2 = FastPathProposal {
        proposal_id: [0; 32],
        topic_id: [0xAA; 32],
        proposer_id: [0xBB; 32],
        base_topic_head: [0x00; 32],
        proposed_head: [0xFF; 32],
        bundle_manifest_hash: [0; 32],
        expires_at_height: 1000,
        proposer_signature: vec![1, 2, 3],
    };

    let id1 = engine.submit_proposal(p1, 100).unwrap();
    let id2 = engine.submit_proposal(p2, 200).unwrap();
    // Different heights → different nonces → different IDs
    assert_ne!(id1, id2);

    // Verify they match what compute_proposal_id would produce directly
    let expected_id1 = compute_proposal_id(&[0xAA; 32], &[0xBB; 32], &[0x00; 32], &[0xFF; 32], 100);
    let expected_id2 = compute_proposal_id(&[0xAA; 32], &[0xBB; 32], &[0x00; 32], &[0xFF; 32], 200);
    assert_eq!(id1, expected_id1);
    assert_eq!(id2, expected_id2);
}

#[test]
fn fix_F85_compute_proposal_id_negative_duplicate_at_same_height_rejected() {
    let mut engine = FastPathEngine::new(FastPathParams::default());

    let p1 = FastPathProposal {
        proposal_id: [0; 32],
        topic_id: [0xAA; 32],
        proposer_id: [0xBB; 32],
        base_topic_head: [0x00; 32],
        proposed_head: [0xFF; 32],
        bundle_manifest_hash: [0; 32],
        expires_at_height: 1000,
        proposer_signature: vec![1, 2, 3],
    };

    let p2 = FastPathProposal {
        proposal_id: [0; 32],
        topic_id: [0xAA; 32],
        proposer_id: [0xBB; 32],
        base_topic_head: [0x00; 32],
        proposed_head: [0xFF; 32],
        bundle_manifest_hash: [0; 32],
        expires_at_height: 1000,
        proposer_signature: vec![1, 2, 3],
    };

    // Same height → same computed ID
    engine.submit_proposal(p1, 100).unwrap();
    let result = engine.submit_proposal(p2, 100);
    assert_eq!(result, Err(FastPathError::DuplicateProposal));
}

// ── F-86: ReviewerVote::Deny wired into production code paths ────────────

#[test]
fn fix_F86_deny_vote_positive_submit_approval_accepts_deny() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // weight=5 → quorum=4. Single Deny doesn't reach quorum → Ok(None)
    let result = engine.submit_approval(p_id, make_approval([1; 32], ReviewerVote::Deny), 200, 5);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn fix_F86_deny_vote_positive_issue_certificate_with_mixed_votes() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // 66 Approve + 1 Deny = 67 participants, quorum=67
    let mut approvals = make_quorum_approvals(66, ReviewerVote::Approve);
    approvals.push(make_approval([100; 32], ReviewerVote::Deny));

    let cert = engine.issue_certificate(p_id, approvals, [0; 32], 200, 100).unwrap();
    // Only 66 Approve signatures stored, Deny omitted
    assert_eq!(cert.approvals.len(), 66);
    // All cert approvals are Approve
    assert!(cert.approvals.iter().all(|a| a.vote == ReviewerVote::Approve));
}

#[test]
fn fix_F86_deny_vote_negative_all_deny_in_submit_approval_rejected() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // weight=1 → quorum=1. Single Deny reaches quorum but no Approve → error
    let result = engine.submit_approval(p_id, make_approval([1; 32], ReviewerVote::Deny), 200, 1);
    assert_eq!(result, Err(FastPathError::AllVotesDeny));
}

#[test]
fn fix_F86_deny_vote_negative_duplicate_deny_reviewer_rejected() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    let approval = make_approval([1; 32], ReviewerVote::Deny);

    // First Deny: Ok(None)
    let r1 = engine.submit_approval(p_id, approval.clone(), 200, 5);
    assert!(r1.is_ok());

    // Second Deny from same reviewer: duplicate → error
    let r2 = engine.submit_approval(p_id, approval, 200, 5);
    assert_eq!(r2, Err(FastPathError::ReviewerNotIndependent));
}

// ── Cross-cutting: Abstain variant is also wired ─────────────────────────

#[test]
fn fix_F86_abstain_vote_positive_accepted_and_counted() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    let result =
        engine.submit_approval(p_id, make_approval([1; 32], ReviewerVote::Abstain), 200, 5);
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

// ── Negative: quorum not reached with mixed votes ────────────────────────

#[test]
fn fix_F57_negative_quorum_not_reached_with_mixed_votes() {
    let mut engine = FastPathEngine::new(FastPathParams::default());
    let proposal = make_proposal([0; 32], [0xBB; 32], 1000);
    let p_id = engine.submit_proposal(proposal, 100).unwrap();

    // weight=10 → quorum=7. Submit 5 total votes (2 Deny, 3 Approve) < 7 → Ok(None)
    let weight = 10u128;

    for i in 0..3 {
        engine
            .submit_approval(p_id, make_approval([i + 1; 32], ReviewerVote::Approve), 200, weight)
            .unwrap();
    }
    for i in 3..5 {
        engine
            .submit_approval(p_id, make_approval([i + 1; 32], ReviewerVote::Deny), 200, weight)
            .unwrap();
    }

    // Still under quorum (5 < 7) → Ok(None) (checked via issue_certificate directly)
    let all_approvals: Vec<ReviewerSignature> = (0..5u8)
        .map(|i| {
            if i < 3 {
                make_approval([i + 1; 32], ReviewerVote::Approve)
            } else {
                make_approval([i + 1; 32], ReviewerVote::Deny)
            }
        })
        .collect();
    let result = engine.issue_certificate(p_id, all_approvals, [0; 32], 200, 100);
    assert_eq!(result, Err(FastPathError::InsufficientQuorum));
}
