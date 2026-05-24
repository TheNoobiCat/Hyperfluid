// Conformance tests for review-engine-spec.md Section 1.7
//
// Test naming: conforms_to_<spec>_<section>_<short_description>
// Source: docs/04-specifications/runtime/review-engine-spec.md

use hyperfluid_state::state_machine::{ExecutionContext, ExecutionResult, StateMachine};
use hyperfluid_state::{Account, ReviewVerdict, TaskStatus, TrustStageEnum};

const MIN_COLLATERAL: u128 = 10_000_000_000_000_000_000u128; // 10 AGX in atto-AGX
const BOUNTY: u128 = 100_000_000_000_000_000_000u128; // 100 AGX

fn ctx(h: u64) -> ExecutionContext {
    ExecutionContext { height: h, timestamp: h * 1000 }
}

fn fund_account(sm: &mut StateMachine, id: [u8; 32], balance: u128, nonce: u64) {
    sm.init_account(Account { account_id: id, balance, nonce, pubkey_hash: id, pubkey: None });
}

/// Create a task, claim it, heartbeat it to InProgress, then submit for review.
/// Returns the review task IDs that were created.
/// Nonce sequence: fund_account sets nonce=0, then task_create=1, claim=2, heartbeat=3, submit_completion=4.
fn setup_review_ready_state(
    sm: &mut StateMachine,
    worker_id: [u8; 32],
    task_id: [u8; 32],
    height: u64,
) -> Vec<[u8; 32]> {
    sm.execute_task_create(
        worker_id,
        BOUNTY,
        0,
        task_id,
        1,
        [0xBBu8; 32],
        [0xCCu8; 32],
        [0xDDu8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        height,
        ctx(height),
    );
    sm.execute_claim_task(
        task_id,
        worker_id,
        MIN_COLLATERAL,
        2, // nonce: 0→1 after task_create, so nonce=2
        height,
        TrustStageEnum::Untrusted,
        ctx(height),
    );

    // Heartbeat to transition to InProgress
    let lease = sm.leases_iter().next().expect("lease must exist");
    let hb = hyperfluid_state::HeartbeatPayload {
        lease_id: lease.lease_id,
        artifact_hash: Some([0xEEu8; 32]),
        diff_pointer: None,
        test_result_ref: None,
        signature: vec![1, 2, 3],
    };
    sm.execute_heartbeat(hb, 3, height + 1, ctx(height + 1));

    // Submit completion → InReview + 2 review tasks created
    sm.execute_submit_completion(task_id, worker_id, 4, height + 2, ctx(height + 2));

    // Collect review task IDs
    sm.tasks_iter()
        .filter(|t| t.parent_task_id == task_id && matches!(t.status, TaskStatus::Open))
        .map(|t| t.task_id)
        .collect()
}

// ── Section 1.7: Review-as-Task Pipeline ───────────────────────────

#[test]
fn conforms_to_review_spec_1_7_1_task_enters_inreview_on_completion() {
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);

    sm.execute_task_create(
        worker,
        BOUNTY,
        0,
        task_id,
        1,
        [0xBBu8; 32],
        [0xCCu8; 32],
        [0xDDu8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        10,
        ctx(10),
    );
    // worker nonce after task_create: 0→1, so claim nonce = 2
    sm.execute_claim_task(
        task_id,
        worker,
        MIN_COLLATERAL,
        2,
        10,
        TrustStageEnum::Untrusted,
        ctx(10),
    );

    // Heartbeat to InProgress (worker nonce after claim: 1→2, so heartbeat nonce = 3)
    let lease = sm.leases_iter().next().expect("lease must exist");
    let hb = hyperfluid_state::HeartbeatPayload {
        lease_id: lease.lease_id,
        artifact_hash: Some([0xEEu8; 32]),
        diff_pointer: None,
        test_result_ref: None,
        signature: vec![1, 2, 3],
    };
    sm.execute_heartbeat(hb, 3, 11, ctx(11));

    // Submit completion (worker nonce after heartbeat: 2→3, so completion nonce = 4)
    let r = sm.execute_submit_completion(task_id, worker, 4, 12, ctx(12));
    assert_eq!(r, ExecutionResult::Success);

    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::InReview, "task must enter InReview");

    // 2 review tasks must exist in the pool
    let review_tasks: Vec<_> = sm.tasks_iter().filter(|t| t.parent_task_id == task_id).collect();
    assert_eq!(review_tasks.len(), 2, "must create 2 review tasks");
    for rt in &review_tasks {
        assert_eq!(rt.status, TaskStatus::Open);
    }
}

#[test]
fn conforms_to_review_spec_1_7_1_completion_rejected_wrong_owner() {
    // Negative: non-owner cannot submit completion.
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let attacker = [0xFFu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, attacker, 1_000_000_000_000_000_000_000, 0);

    sm.execute_task_create(
        worker,
        BOUNTY,
        0,
        task_id,
        1,
        [0xBBu8; 32],
        [0xCCu8; 32],
        [0xDDu8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        10,
        ctx(10),
    );
    // worker nonce after task_create: 0→1, so claim nonce = 2
    sm.execute_claim_task(
        task_id,
        worker,
        MIN_COLLATERAL,
        2,
        10,
        TrustStageEnum::Untrusted,
        ctx(10),
    );

    let lease = sm.leases_iter().next().expect("lease must exist");
    let hb = hyperfluid_state::HeartbeatPayload {
        lease_id: lease.lease_id,
        artifact_hash: Some([0xEEu8; 32]),
        diff_pointer: None,
        test_result_ref: None,
        signature: vec![1, 2, 3],
    };
    // worker nonce after claim: 1→2, so heartbeat nonce = 3
    sm.execute_heartbeat(hb, 3, 11, ctx(11));

    // Attacker tries to submit completion (attacker nonce = 0, so nonce = 1)
    let r = sm.execute_submit_completion(task_id, attacker, 1, 12, ctx(12));
    assert_eq!(r, ExecutionResult::Rejected, "non-owner must be rejected");
}

#[test]
fn conforms_to_review_spec_1_7_2_untrusted_rejected_claiming_review_task() {
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let untrusted = [0xBBu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, untrusted, 1_000_000_000_000_000_000_000, 0);

    let review_task_ids = setup_review_ready_state(&mut sm, worker, task_id, 10);
    assert_eq!(review_task_ids.len(), 2);

    // Untrusted agent tries to claim a review task (nonce: untrusted has nonce 0 → claim nonce = 1)
    let r = sm.execute_claim_task(
        review_task_ids[0],
        untrusted,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Untrusted,
        ctx(14),
    );
    assert_eq!(r, ExecutionResult::Rejected, "untrusted agent must not claim review tasks");
}

#[test]
fn conforms_to_review_spec_1_7_2_trusted_can_claim_review_task() {
    // Positive: trusted agent can claim a review task.
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let reviewer = [0xBBu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, reviewer, 1_000_000_000_000_000_000_000, 0);

    let review_task_ids = setup_review_ready_state(&mut sm, worker, task_id, 10);

    // Trusted agent claims a review task (nonce: reviewer has nonce 0 → claim nonce = 1)
    let r = sm.execute_claim_task(
        review_task_ids[0],
        reviewer,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );
    assert_eq!(r, ExecutionResult::Success, "trusted agent must be able to claim review tasks");

    let rt =
        sm.tasks_iter().find(|t| t.task_id == review_task_ids[0]).expect("review task must exist");
    assert_eq!(rt.primary_owner, reviewer);
    assert_eq!(rt.status, TaskStatus::Claimed);
}

#[test]
fn conforms_to_review_spec_1_7_3_accept_majority_payout() {
    // Verify settlement: 2 Accept → 90% worker, 10% split among 2 reviewers.
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let r1 = [0xBBu8; 32];
    let r2 = [0xCCu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, r1, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, r2, 1_000_000_000_000_000_000_000, 0);

    let review_task_ids = setup_review_ready_state(&mut sm, worker, task_id, 10);
    assert_eq!(review_task_ids.len(), 2);

    // Both reviewers (trusted) claim and submit Accept
    // Reviewers start with nonce 0 → claim nonce = 1, submit_review nonce = 2
    sm.execute_claim_task(
        review_task_ids[0],
        r1,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );
    sm.execute_claim_task(
        review_task_ids[1],
        r2,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );

    let r = sm.execute_submit_review(
        review_task_ids[0],
        r1,
        ReviewVerdict::Accept,
        [0x01u8; 32],
        2,
        15,
        ctx(15),
    );
    assert_eq!(r, ExecutionResult::Success);
    let r = sm.execute_submit_review(
        review_task_ids[1],
        r2,
        ReviewVerdict::Accept,
        [0x02u8; 32],
        2,
        15,
        ctx(15),
    );
    assert_eq!(r, ExecutionResult::Success);

    // Task should be Done
    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::Done, "accept majority → Done");

    // Worker should get 90% of bounty (90 AGX)
    let worker_acct = sm.get_account(&worker).expect("worker account must exist");
    // Original balance: 1000 AGX - 100 AGX bounty = 900 AGX. Then +90 AGX payout.
    assert_eq!(
        worker_acct.balance, 990_000_000_000_000_000_000u128,
        "worker must receive 90% of bounty"
    );

    // Each reviewer should get 5% (5 AGX each)
    let r1_acct = sm.get_account(&r1).expect("reviewer 1 account must exist");
    assert_eq!(r1_acct.balance, 1_005_000_000_000_000_000_000u128, "reviewer 1 must receive 5 AGX");
    let r2_acct = sm.get_account(&r2).expect("reviewer 2 account must exist");
    assert_eq!(r2_acct.balance, 1_005_000_000_000_000_000_000u128, "reviewer 2 must receive 5 AGX");
}

#[test]
fn conforms_to_review_spec_1_7_4_reject_majority_returns_to_open() {
    // Verify settlement: 2 Reject → task returns to Open, reviewers still paid.
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let r1 = [0xBBu8; 32];
    let r2 = [0xCCu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, r1, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, r2, 1_000_000_000_000_000_000_000, 0);

    let review_task_ids = setup_review_ready_state(&mut sm, worker, task_id, 10);

    // Both reviewers submit Reject
    // Reviewers start with nonce 0 → claim nonce = 1, submit_review nonce = 2
    sm.execute_claim_task(
        review_task_ids[0],
        r1,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );
    sm.execute_claim_task(
        review_task_ids[1],
        r2,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );

    sm.execute_submit_review(
        review_task_ids[0],
        r1,
        ReviewVerdict::Reject,
        [0x01u8; 32],
        2,
        15,
        ctx(15),
    );
    sm.execute_submit_review(
        review_task_ids[1],
        r2,
        ReviewVerdict::Reject,
        [0x02u8; 32],
        2,
        15,
        ctx(15),
    );

    // Task returns to Open
    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::Open, "reject majority → Open");
    assert_eq!(task.primary_owner, [0u8; 32], "owner must be cleared");

    // Reviewers still paid (5% each = 5 AGX)
    let r1_acct = sm.get_account(&r1).expect("reviewer 1 must exist");
    assert_eq!(r1_acct.balance, 1_005_000_000_000_000_000_000u128);
}

#[test]
fn conforms_to_review_spec_1_7_5_tie_vote_counts_as_reject() {
    // Edge: tie (1 Accept, 1 Reject) → task returns to Open (pro-quality bias).
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let r1 = [0xBBu8; 32];
    let r2 = [0xCCu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, r1, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, r2, 1_000_000_000_000_000_000_000, 0);

    let review_task_ids = setup_review_ready_state(&mut sm, worker, task_id, 10);

    // Reviewers start with nonce 0 → claim nonce = 1, submit_review nonce = 2
    sm.execute_claim_task(
        review_task_ids[0],
        r1,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );
    sm.execute_claim_task(
        review_task_ids[1],
        r2,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );

    // Tie: 1 Accept, 1 Reject
    sm.execute_submit_review(
        review_task_ids[0],
        r1,
        ReviewVerdict::Accept,
        [0x01u8; 32],
        2,
        15,
        ctx(15),
    );
    sm.execute_submit_review(
        review_task_ids[1],
        r2,
        ReviewVerdict::Reject,
        [0x02u8; 32],
        2,
        15,
        ctx(15),
    );

    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::Open, "tie vote → return to Open");

    // Both reviewers still paid
    let r1_acct = sm.get_account(&r1).expect("reviewer 1 must exist");
    assert_eq!(r1_acct.balance, 1_005_000_000_000_000_000_000u128);
    let r2_acct = sm.get_account(&r2).expect("reviewer 2 must exist");
    assert_eq!(r2_acct.balance, 1_005_000_000_000_000_000_000u128);
}

#[test]
fn conforms_to_review_spec_1_7_5_single_verdict_does_not_settle() {
    // Edge: only 1 reviewer submits before timeout — should not settle, task stays InReview.
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let r1 = [0xBBu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, r1, 1_000_000_000_000_000_000_000, 0);

    let review_task_ids = setup_review_ready_state(&mut sm, worker, task_id, 10);

    // Only 1 reviewer submits (r1 nonce: 0 → claim nonce = 1, submit_review nonce = 2)
    sm.execute_claim_task(
        review_task_ids[0],
        r1,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );
    sm.execute_submit_review(
        review_task_ids[0],
        r1,
        ReviewVerdict::Accept,
        [0x01u8; 32],
        2,
        15,
        ctx(15),
    );

    // Task should still be InReview (not yet settled)
    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::InReview, "single verdict must not settle");
}

#[test]
fn conforms_to_review_spec_1_7_6_review_expiry_returns_to_open() {
    // Verify review lease expiry returns work task to Open when reviewers
    // claim a review task but don't submit before TTL.
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let r1 = [0xBBu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, r1, 1_000_000_000_000_000_000_000, 0);

    let review_task_ids = setup_review_ready_state(&mut sm, worker, task_id, 10);

    // Reviewer claims but does NOT submit — lease will expire (r1 nonce: 0 → claim nonce = 1)
    sm.execute_claim_task(
        review_task_ids[0],
        r1,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );

    // Advance to well past lease expiry (14 + 120 = 134 → use 200)
    let expired = sm.run_review_expiry(&task_id, 200);
    assert!(expired, "review expiry must return true when review lease expires");

    // Work task should return to Open
    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::Open, "task must return to Open on review expiry");
    assert_eq!(task.primary_owner, [0u8; 32], "owner must be cleared");
}

#[test]
fn conforms_to_review_spec_1_7_6_review_not_expired_before_ttl() {
    // Negative: review expiry does not fire before TTL period.
    let mut sm = StateMachine::new();
    let worker = [0xAAu8; 32];
    let r1 = [0xBBu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, worker, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, r1, 1_000_000_000_000_000_000_000, 0);

    let review_task_ids = setup_review_ready_state(&mut sm, worker, task_id, 10);
    // r1 nonce: 0 → claim nonce = 1
    sm.execute_claim_task(
        review_task_ids[0],
        r1,
        MIN_COLLATERAL,
        1,
        14,
        TrustStageEnum::Trusted,
        ctx(14),
    );

    // At height 20 (lease expires at 14 + 120 = 134), expiry does not fire
    let expired = sm.run_review_expiry(&task_id, 20);
    assert!(!expired, "review expiry must not fire before TTL");

    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::InReview, "task must still be InReview");
}
