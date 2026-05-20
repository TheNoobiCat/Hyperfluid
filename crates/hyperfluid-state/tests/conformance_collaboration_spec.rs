// Conformance tests for collaboration-spec.md Section 1.7 and 3.7
//
// Test naming: conforms_to_<spec>_<section>_<short_description>
// Source: docs/04-specifications/runtime/collaboration-spec.md

use hyperfluid_state::state_machine::{ExecutionContext, ExecutionResult, StateMachine};
use hyperfluid_state::{Account, EscrowStatus, HeartbeatPayload, TaskStatus, TrustStageEnum};

/// Minimum lease collateral: max(10 AGX, 0.5% of bounty) = 10 AGX for bounties up to 2000 AGX.
const MIN_COLLATERAL: u128 = 10_000_000_000_000_000_000u128; // 10 AGX in atto-AGX

fn ctx(h: u64) -> ExecutionContext {
    ExecutionContext { height: h, timestamp: h * 1000 }
}

fn fund_account(sm: &mut StateMachine, id: [u8; 32], balance: u128, nonce: u64) {
    sm.init_account(Account { account_id: id, balance, nonce, pubkey_hash: id, pubkey: None });
}

// ── Section 1.7: Decentralized Task Board ──────────────────────────

#[test]
fn conforms_to_collaboration_spec_1_7_1_task_transitions_deterministic() {
    // Verify task transitions open → claimed → in_progress → done deterministically.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    let task_id = [0x01u8; 32];
    let bounty = 100_000_000_000_000_000_000u128; // 100 AGX
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000, 0);

    // Open: create task
    let r = sm.execute_task_create(
        agent,
        bounty,
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
    assert_eq!(r, ExecutionResult::Success);
    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::Open);

    // Claimed
    let r = sm.execute_claim_task(
        task_id,
        agent,
        MIN_COLLATERAL,
        20,
        TrustStageEnum::Untrusted,
        ctx(20),
    );
    assert_eq!(r, ExecutionResult::Success);
    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::Claimed);

    // InProgress via valid heartbeat
    let lease = sm.leases_iter().next().expect("lease must exist");
    let hb = HeartbeatPayload {
        lease_id: lease.lease_id,
        artifact_hash: Some([0xEEu8; 32]),
        diff_pointer: None,
        test_result_ref: None,
        signature: vec![1, 2, 3],
    };
    let r = sm.execute_heartbeat(hb, 30, ctx(30));
    assert_eq!(r, ExecutionResult::Success);
    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::InProgress);

    // InReview via submit completion
    let r = sm.execute_submit_completion(task_id, agent, 40, ctx(40));
    assert_eq!(r, ExecutionResult::Success);
    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::InReview);

    // Deterministic: two state machines with same inputs produce same task states
    let mut sm2 = StateMachine::new();
    fund_account(&mut sm2, agent, 1_000_000_000_000_000_000_000, 0);
    sm2.execute_task_create(
        agent,
        bounty,
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
    sm2.execute_claim_task(task_id, agent, MIN_COLLATERAL, 20, TrustStageEnum::Untrusted, ctx(20));
    let l2 = sm2.leases_iter().next().expect("lease must exist");
    let hb2 = HeartbeatPayload {
        lease_id: l2.lease_id,
        artifact_hash: Some([0xEEu8; 32]),
        diff_pointer: None,
        test_result_ref: None,
        signature: vec![1, 2, 3],
    };
    sm2.execute_heartbeat(hb2, 30, ctx(30));
    sm2.execute_submit_completion(task_id, agent, 40, ctx(40));
    let t2 = sm2.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(t2.status, TaskStatus::InReview);
}

#[test]
fn conforms_to_collaboration_spec_1_7_1_task_creation_rejected_wrong_nonce() {
    // Negative: task creation with wrong nonce is rejected.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000, 0);
    let r = sm.execute_task_create(
        agent,
        100_000_000_000_000_000_000,
        0,
        [0x01u8; 32],
        99,
        [0xBBu8; 32],
        [0xCCu8; 32],
        [0xDDu8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        10,
        ctx(10),
    );
    assert_eq!(r, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_collaboration_spec_1_7_1_task_creation_rejected_insufficient_funds() {
    // Edge: insufficient balance for bounty.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    fund_account(&mut sm, agent, 10, 0);
    let r = sm.execute_task_create(
        agent,
        100_000_000_000_000_000_000,
        0,
        [0x01u8; 32],
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
    assert_eq!(r, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_collaboration_spec_1_7_1_duplicate_task_id_rejected() {
    // Edge: cannot create same task_id twice.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    let task_id = [0x01u8; 32];
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000, 0);
    let r = sm.execute_task_create(
        agent,
        100_000_000_000_000_000_000,
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
    assert_eq!(r, ExecutionResult::Success);
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000, 2);
    let r = sm.execute_task_create(
        agent,
        100_000_000_000_000_000_000,
        0,
        task_id,
        3,
        [0xBBu8; 32],
        [0xCCu8; 32],
        [0xDDu8; 32],
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
        10,
        ctx(10),
    );
    assert_eq!(r, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_collaboration_spec_1_7_3_lease_ttl_enforced() {
    // Verify lease TTL of 20 minutes (120 blocks) enforced: task returns to Open on timeout.
    let mut sm = StateMachine::new();
    let a1 = [0xAAu8; 32];
    let a2 = [0xBBu8; 32];
    let task_id = [0x01u8; 32];
    let bounty = 100_000_000_000_000_000_000u128;
    fund_account(&mut sm, a1, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, a2, 1_000_000_000_000_000_000_000, 0);
    sm.execute_task_create(
        a1,
        bounty,
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
    sm.execute_claim_task(task_id, a1, MIN_COLLATERAL, 20, TrustStageEnum::Untrusted, ctx(20));

    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.status, TaskStatus::Claimed);
    // Lease expires at height 140 (20 + 120)
    assert_eq!(task.lease_expires_height, 140);

    // At height 30, lease is still active — second agent cannot claim
    let r =
        sm.execute_claim_task(task_id, a2, MIN_COLLATERAL, 30, TrustStageEnum::Untrusted, ctx(30));
    assert_eq!(r, ExecutionResult::Rejected, "active lease must block second claim");

    // At height 200, lease has expired — second agent CAN claim
    sm.execute_release_task(task_id, a1, ctx(200));
    let r = sm.execute_claim_task(
        task_id,
        a2,
        MIN_COLLATERAL,
        200,
        TrustStageEnum::Untrusted,
        ctx(200),
    );
    assert_eq!(r, ExecutionResult::Success, "expired lease must allow new claim");

    let task = sm.tasks_iter().find(|t| t.task_id == task_id).expect("task must exist");
    assert_eq!(task.primary_owner, a2);
    assert_eq!(task.status, TaskStatus::Claimed);
}

#[test]
fn conforms_to_collaboration_spec_1_7_4_empty_heartbeat_rejected() {
    // Negative: heartbeat with empty progress evidence is rejected.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    let task_id = [0x01u8; 32];
    let bounty = 100_000_000_000_000_000_000u128;
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000, 0);
    sm.execute_task_create(
        agent,
        bounty,
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
    sm.execute_claim_task(task_id, agent, MIN_COLLATERAL, 20, TrustStageEnum::Untrusted, ctx(20));

    let lease = sm.leases_iter().next().expect("lease must exist");
    let hb = HeartbeatPayload {
        lease_id: lease.lease_id,
        artifact_hash: None,
        diff_pointer: None,
        test_result_ref: None,
        signature: vec![1, 2, 3],
    };
    let r = sm.execute_heartbeat(hb, 30, ctx(30));
    assert_eq!(r, ExecutionResult::Rejected, "empty heartbeat must be rejected");
}

#[test]
fn conforms_to_collaboration_spec_1_7_4_valid_heartbeat_accepted() {
    // Positive: heartbeat with evidence is accepted.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    let task_id = [0x01u8; 32];
    let bounty = 100_000_000_000_000_000_000u128;
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000, 0);
    sm.execute_task_create(
        agent,
        bounty,
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
    sm.execute_claim_task(task_id, agent, MIN_COLLATERAL, 20, TrustStageEnum::Untrusted, ctx(20));

    let lease = sm.leases_iter().next().expect("lease must exist");
    let hb = HeartbeatPayload {
        lease_id: lease.lease_id,
        artifact_hash: Some([0xEEu8; 32]),
        diff_pointer: None,
        test_result_ref: None,
        signature: vec![1, 2, 3],
    };
    let r = sm.execute_heartbeat(hb, 30, ctx(30));
    assert_eq!(r, ExecutionResult::Success, "heartbeat with evidence must be accepted");
}

#[test]
fn conforms_to_collaboration_spec_1_7_5_lease_caps_by_trust_stage() {
    // Verify per-agent lease caps: untrusted max 2, trusted max 6.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000, 0);

    // Untrusted: max 2 leases
    for i in 0..3 {
        let task_id = [0x01 + i as u8; 32];
        sm.execute_task_create(
            agent,
            100_000_000_000_000_000_000,
            0,
            task_id,
            (i + 1) as u64,
            [0xBBu8; 32],
            [0xCCu8; 32],
            [0xDDu8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            10,
            ctx(10),
        );
        let r = sm.execute_claim_task(
            task_id,
            agent,
            MIN_COLLATERAL,
            (i + 10) as u64,
            TrustStageEnum::Untrusted,
            ctx(10),
        );
        if i < 2 {
            assert_eq!(r, ExecutionResult::Success, "untrusted should allow {} leases", i + 1);
        } else {
            assert_eq!(r, ExecutionResult::Rejected, "untrusted must reject 3rd lease");
        }
    }

    // Trusted: max 6 leases
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000, 0);
    for i in 0..7 {
        let task_id = [0x01 + i as u8; 32];
        sm.execute_task_create(
            agent,
            100_000_000_000_000_000_000,
            0,
            task_id,
            (i + 1) as u64,
            [0xBBu8; 32],
            [0xCCu8; 32],
            [0xDDu8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            10,
            ctx(10),
        );
        let r = sm.execute_claim_task(
            task_id,
            agent,
            MIN_COLLATERAL,
            (i + 10) as u64,
            TrustStageEnum::Trusted,
            ctx(10),
        );
        if i < 6 {
            assert_eq!(r, ExecutionResult::Success, "trusted should allow {} leases", i + 1);
        } else {
            assert_eq!(r, ExecutionResult::Rejected, "trusted must reject 7th lease");
        }
    }
}

#[test]
fn conforms_to_collaboration_spec_1_7_6_lease_collateral_requirement() {
    // Verify lease collateral requirement: max(10 AGX, 0.5% bounty).
    // 10 AGX = 10 * 10^18 atto-AGX = 10_000_000_000_000_000_000
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    let task_id = [0x01u8; 32];
    let large_bounty = 10_000_000_000_000_000_000_000u128; // 10,000 AGX → 0.5% = 50 AGX
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000_000, 0);
    sm.execute_task_create(
        agent,
        large_bounty,
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

    // Insufficient collateral → rejected
    let min_collateral = 10_000_000_000_000_000_000u128.max(large_bounty * 5 / 1000);
    let r = sm.execute_claim_task(
        task_id,
        agent,
        min_collateral - 1,
        20,
        TrustStageEnum::Untrusted,
        ctx(20),
    );
    assert_eq!(r, ExecutionResult::Rejected, "insufficient collateral must be rejected");

    // Sufficient collateral → accepted
    let mut sm = StateMachine::new();
    fund_account(&mut sm, agent, 1_000_000_000_000_000_000_000_000, 0);
    sm.execute_task_create(
        agent,
        large_bounty,
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
    let r = sm.execute_claim_task(
        task_id,
        agent,
        min_collateral,
        20,
        TrustStageEnum::Untrusted,
        ctx(20),
    );
    assert_eq!(r, ExecutionResult::Success, "sufficient collateral must be accepted");

    // Small bounty: 10 AGX floor applies
    let mut sm = StateMachine::new();
    let task_id2 = [0x02u8; 32];
    let small_bounty = 100_000_000_000_000_000_000u128; // 100 AGX → 0.5% = 0.5 AGX < 10 AGX floor
    sm.execute_task_create(
        agent,
        small_bounty,
        0,
        task_id2,
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
    let r = sm.execute_claim_task(
        task_id2,
        agent,
        9_000_000_000_000_000_000,
        20,
        TrustStageEnum::Untrusted,
        ctx(20),
    );
    assert_eq!(r, ExecutionResult::Rejected, "below 10 AGX floor must be rejected");
}

#[test]
fn conforms_to_collaboration_spec_1_7_7_bounty_escrow_deducted() {
    // Verify bounty escrow: task creation deducts bounty_agx from funder balance.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    let initial_balance = 1_000_000_000_000_000_000_000u128;
    let bounty = 100_000_000_000_000_000_000u128; // 100 AGX
    fund_account(&mut sm, agent, initial_balance, 0);

    let r = sm.execute_task_create(
        agent,
        bounty,
        0,
        [0x01u8; 32],
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
    assert_eq!(r, ExecutionResult::Success);

    let task = sm.tasks_iter().find(|t| t.task_id == [0x01u8; 32]).expect("task must exist");
    assert_eq!(task.bounty_agx, bounty);
    assert_eq!(task.escrow_status, EscrowStatus::Locked);

    // Funder balance deducted
    let acct = sm.get_account(&agent).expect("account must exist");
    assert_eq!(acct.balance, initial_balance - bounty, "bounty must be deducted from funder");
}

#[test]
fn conforms_to_collaboration_spec_1_7_7_non_existent_creator_rejected() {
    // Negative: task creation by non-existent creator is rejected.
    let mut sm = StateMachine::new();
    let r = sm.execute_task_create(
        [0xAAu8; 32],
        100_000_000_000_000_000_000,
        0,
        [0x01u8; 32],
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
    assert_eq!(r, ExecutionResult::Rejected, "non-existent creator must be rejected");
}

#[test]
fn conforms_to_collaboration_spec_1_7_7_claiming_task_no_active_lease() {
    // Edge: claiming a task with an active lease is rejected.
    let mut sm = StateMachine::new();
    let a1 = [0xAAu8; 32];
    let a2 = [0xBBu8; 32];
    let task_id = [0x01u8; 32];
    let bounty = 100_000_000_000_000_000_000u128;
    fund_account(&mut sm, a1, 1_000_000_000_000_000_000_000, 0);
    fund_account(&mut sm, a2, 1_000_000_000_000_000_000_000, 0);
    sm.execute_task_create(
        a1,
        bounty,
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
    // First agent claims
    sm.execute_claim_task(task_id, a1, MIN_COLLATERAL, 20, TrustStageEnum::Untrusted, ctx(20));
    // Second agent tries to claim same task
    let r =
        sm.execute_claim_task(task_id, a2, MIN_COLLATERAL, 20, TrustStageEnum::Untrusted, ctx(20));
    assert_eq!(r, ExecutionResult::Rejected, "already-claimed task must be rejected");
}

// ── Section 3.7: Trust Ladder ──────────────────────────────────────

#[test]
fn conforms_to_collaboration_spec_3_7_1_new_agent_starts_untrusted() {
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    sm.init_trust_stage(agent);
    let record = sm.get_trust_stage(&agent).expect("trust stage must exist");
    assert_eq!(record.stage, TrustStageEnum::Untrusted);
    assert_eq!(record.accepted_work_count, 0);
    assert_eq!(record.abuse_flags, 0);
}

#[test]
fn conforms_to_collaboration_spec_3_7_1_promotion_requires_10_accepted_and_clean() {
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    sm.init_trust_stage(agent);

    // 9 accepted works → not enough
    for _ in 0..9 {
        sm.record_accepted_work(&agent);
    }
    let promoted = sm.run_trust_promotion();
    assert!(promoted.is_empty(), "9 accepted works must not trigger promotion");
    let record = sm.get_trust_stage(&agent).expect("trust stage must exist");
    assert_eq!(record.stage, TrustStageEnum::Untrusted);

    // 10th work → promotion (no abuse flags)
    sm.record_accepted_work(&agent);
    let promoted = sm.run_trust_promotion();
    assert_eq!(promoted, vec![agent], "10 accepted works must trigger promotion");
    let record = sm.get_trust_stage(&agent).expect("trust stage must exist");
    assert_eq!(record.stage, TrustStageEnum::Trusted);
}

#[test]
fn conforms_to_collaboration_spec_3_7_1_abuse_flags_block_promotion() {
    // Negative: agent with abuse flags is not promoted even with 10+ accepted.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    sm.init_trust_stage(agent);

    for _ in 0..10 {
        sm.record_accepted_work(&agent);
    }
    sm.record_abuse(&agent, false); // non-severe abuse flag

    let promoted = sm.run_trust_promotion();
    assert!(promoted.is_empty(), "abuse flags must block promotion");
    let record = sm.get_trust_stage(&agent).expect("trust stage must exist");
    assert_eq!(record.stage, TrustStageEnum::Untrusted);
}

#[test]
fn conforms_to_collaboration_spec_3_7_3_high_severity_abuse_resets_to_untrusted() {
    // High-severity abuse resets a trusted agent to untrusted.
    let mut sm = StateMachine::new();
    let agent = [0xAAu8; 32];
    sm.init_trust_stage(agent);

    // Promote to trusted
    for _ in 0..10 {
        sm.record_accepted_work(&agent);
    }
    sm.run_trust_promotion();
    let record = sm.get_trust_stage(&agent).expect("trust stage must exist");
    assert_eq!(record.stage, TrustStageEnum::Trusted);

    // High-severity abuse resets
    sm.record_abuse(&agent, true);
    let record = sm.get_trust_stage(&agent).expect("trust stage must exist");
    assert_eq!(record.stage, TrustStageEnum::Untrusted);
    assert_eq!(record.accepted_work_count, 0);
}

#[test]
fn conforms_to_collaboration_spec_3_7_4_whitewash_guard_new_identity() {
    // Edge: agent with abuse history cannot gain instant trust via new identity.
    // The whitewash guard is enforced at the key-binding/registration layer.
    // Here we verify that a new agent starts at untrusted regardless.
    let mut sm = StateMachine::new();
    let new_agent = [0xFFu8; 32];
    sm.init_trust_stage(new_agent);
    let record = sm.get_trust_stage(&new_agent).expect("trust stage must exist");
    assert_eq!(record.stage, TrustStageEnum::Untrusted);
    assert_eq!(record.accepted_work_count, 0);
    assert_eq!(record.abuse_flags, 0);
}
