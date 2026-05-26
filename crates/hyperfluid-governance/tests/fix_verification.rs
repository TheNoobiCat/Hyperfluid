//! Integration tests for production-readiness fixes.
//!
//! Each fix has at least 1 positive + 1 negative test:
//!   F-26: GovernanceVote.signature length validation (3309 bytes for ML-DSA-65)
//!   F-9:  cast_vote delegates crypto verification to caller (consensus driver)
#![allow(non_snake_case)]
//!   F-53: compute_proposal_id wired into proposal submission
//!   F-84: Safe BTreeMap access in submit_proposal (no index operator panic)

use hyperfluid_governance::proposal::{compute_proposal_id, GovernanceEngine, ProposalError};
use hyperfluid_governance::types::{
    GovernanceParams, GovernanceVote, Hash32, ProposalStatus, VoteOption,
};

fn make_proposer() -> Hash32 {
    [0xAA; 32]
}

fn make_proposal(engine: &mut GovernanceEngine, nonce: u64) -> Hash32 {
    let result = engine
        .submit_proposal(
            make_proposer(),
            [0xBB; 32],
            [0xCC; 32],
            [0xDD; 32],
            100,
            0,
            1_000_000_000,
            nonce,
        )
        .unwrap();
    result.proposal_id
}

const VALID_SIGNATURE_LEN: usize = 3309;

// ── F-26: GovernanceVote.signature length validation ──────────────────

#[test]
fn fix_F26_signature_length_valid_positive() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let p_id = make_proposal(&mut engine, 1);

    let vote = GovernanceVote {
        proposal_id: p_id,
        voter_id: [0x11; 32],
        vote: VoteOption::Yes,
        reason_hash: [0; 32],
        vote_weight: 10_000,
        signature: vec![0u8; VALID_SIGNATURE_LEN],
    };
    let result = engine.cast_vote(vote, 200);
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
}

#[test]
fn fix_F26_signature_length_too_short_negative() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let p_id = make_proposal(&mut engine, 1);

    let vote = GovernanceVote {
        proposal_id: p_id,
        voter_id: [0x11; 32],
        vote: VoteOption::Yes,
        reason_hash: [0; 32],
        vote_weight: 10_000,
        signature: vec![0u8; 100], // only 100 bytes — far too short
    };
    let result = engine.cast_vote(vote, 200);
    assert_eq!(result, Err(ProposalError::InvalidSignature));
}

#[test]
fn fix_F26_signature_length_too_long_negative() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let p_id = make_proposal(&mut engine, 1);

    let vote = GovernanceVote {
        proposal_id: p_id,
        voter_id: [0x11; 32],
        vote: VoteOption::Yes,
        reason_hash: [0; 32],
        vote_weight: 10_000,
        signature: vec![0u8; 5000], // 5000 bytes — too long
    };
    let result = engine.cast_vote(vote, 200);
    assert_eq!(result, Err(ProposalError::InvalidSignature));
}

// ── F-9: Signature verification delegated to caller ──────────────────

#[test]
fn fix_F9_vote_accepted_with_valid_length_signature_positive() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let p_id = make_proposal(&mut engine, 1);

    let vote = GovernanceVote {
        proposal_id: p_id,
        voter_id: [0x22; 32],
        vote: VoteOption::Yes,
        reason_hash: [0; 32],
        vote_weight: 25_000,
        signature: vec![0u8; VALID_SIGNATURE_LEN],
    };
    assert!(engine.cast_vote(vote, 200).is_ok());

    // Verify the proposal state was updated despite no crypto verification
    let p = engine.get_proposal(&p_id).unwrap();
    assert_eq!(p.yes_weight, 25_000);
}

#[test]
fn fix_F9_no_internal_crypto_verification_negative() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let p_id = make_proposal(&mut engine, 1);

    // Vote with valid-length but garbage signature (all 0xFF) — no crypto
    // verification happens inside governance crate, which is expected per
    // the SPEC_DEVIATION comment. The caller (consensus driver) handles it.
    let vote = GovernanceVote {
        proposal_id: p_id,
        voter_id: [0x33; 32],
        vote: VoteOption::No,
        reason_hash: [0; 32],
        vote_weight: 10_000,
        signature: vec![0xFF; VALID_SIGNATURE_LEN], // garbage but correct length
    };
    // Should not panic or error on cryptographic grounds
    assert!(engine.cast_vote(vote, 200).is_ok());

    // Verify the garbage-signed vote was still counted (caller's responsibility)
    let p = engine.get_proposal(&p_id).unwrap();
    assert_eq!(p.no_weight, 10_000);
}

#[test]
fn fix_F9_delegated_verification_does_not_mask_other_errors_negative() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let _p_id = make_proposal(&mut engine, 1);

    // Vote for a non-existent proposal — should fail with ProposalNotFound,
    // not something crypto-related
    let vote = GovernanceVote {
        proposal_id: [0xFF; 32], // non-existent proposal
        voter_id: [0x44; 32],
        vote: VoteOption::Yes,
        reason_hash: [0; 32],
        vote_weight: 10_000,
        signature: vec![0u8; VALID_SIGNATURE_LEN],
    };
    assert_eq!(engine.cast_vote(vote, 200), Err(ProposalError::ProposalNotFound));
}

// ── F-53: compute_proposal_id wired into submission ──────────────────

#[test]
fn fix_F53_computed_id_matches_external_call_positive() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let proposer = make_proposer();
    let nonce = 42;

    let expected_id = compute_proposal_id(&proposer, &[0xBB; 32], &[0xCC; 32], &[0xDD; 32], nonce);

    let result = engine
        .submit_proposal(proposer, [0xBB; 32], [0xCC; 32], [0xDD; 32], 100, 0, 1_000_000_000, nonce)
        .unwrap();

    assert_eq!(
        result.proposal_id, expected_id,
        "internally computed proposal_id must match external compute_proposal_id"
    );
}

#[test]
fn fix_F53_different_nonces_produce_different_ids_negative() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let proposer = make_proposer();

    let id1 = engine
        .submit_proposal(proposer, [0xBB; 32], [0xCC; 32], [0xDD; 32], 100, 0, 1_000_000_000, 1)
        .unwrap()
        .proposal_id;
    let id2 = engine
        .submit_proposal(proposer, [0xBB; 32], [0xCC; 32], [0xDD; 32], 200, 1, 1_000_000_000, 2)
        .unwrap()
        .proposal_id;

    assert_ne!(id1, id2, "different nonces must produce different proposal IDs");
}

// ── F-84: Safe BTreeMap access in submit_proposal ────────────────────

#[test]
fn fix_F84_submit_proposal_succeeds_positive() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let result = engine.submit_proposal(
        make_proposer(),
        [0xBB; 32],
        [0xCC; 32],
        [0xDD; 32],
        100,
        0,
        1_000_000_000,
        1,
    );
    assert!(result.is_ok(), "expected Ok, got {:?}", result);
    let p = result.unwrap();
    assert_eq!(p.status, ProposalStatus::Active);
}

#[test]
fn fix_F84_submit_proposal_returns_error_on_limit_negative() {
    let mut engine = GovernanceEngine::new(GovernanceParams::default());
    let proposer = make_proposer();

    // Fill up to max open proposals (default: 32)
    for i in 0..32 {
        engine
            .submit_proposal(
                proposer,
                [i as u8; 32],
                [0; 32],
                [0; 32],
                100 + i as u64,
                i as u64,
                1_000_000_000,
                i as u64,
            )
            .unwrap();
    }

    // 33rd should return Err (not panic due to index operator)
    let result =
        engine.submit_proposal(proposer, [33u8; 32], [0; 32], [0; 32], 200, 33, 1_000_000_000, 33);
    assert_eq!(result, Err(ProposalError::MaxOpenProposalsReached));
}

#[test]
fn fix_F84_proposal_not_found_negative() {
    // Verify that the error path works correctly (no index operator panic)
    let mut engine = GovernanceEngine::new(GovernanceParams::default());

    // Try to finalize a non-existent proposal
    let result = engine.finalize_proposal([0xDE; 32], 100, 1_000_000, 10);
    assert_eq!(result, Err(ProposalError::ProposalNotFound));

    // Try to vote on a non-existent proposal
    let vote = GovernanceVote {
        proposal_id: [0xAD; 32],
        voter_id: [0x11; 32],
        vote: VoteOption::Yes,
        reason_hash: [0; 32],
        vote_weight: 10_000,
        signature: vec![0u8; VALID_SIGNATURE_LEN],
    };
    assert_eq!(engine.cast_vote(vote, 200), Err(ProposalError::ProposalNotFound));
}
