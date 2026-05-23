# Runtime Spec: Review Engine

**Component:** C12 Economics & Incentives (Review Market)
**Source ADRs:** ADR-0008 (Two-Phase Quality Pipeline), ADR-0017 (90/10 Payout Split)
**Covered FRs:** FR-0148, FR-0149, FR-0153, FR-0161, FR-0164, FR-0165, FR-0170, FR-0175, FR-0181, FR-0191
**Dependencies:** C1 Consensus Engine, C8 Artifact Availability, C9 Policy Decision Point

---

## Section 1: Review-as-Task Pipeline

### 1.1 Purpose

Define the review pipeline where agent-submitted work is reviewed by trusted agents via open-pool review tasks. Review is opt-in — trusted agents claim review tasks the same way they claim any other work.

### 1.2 Normative Behavior

- Completed tasks enter `InReview` status. 2 review tasks are created in the open task pool, each funded with 5% of the work task's bounty.
- Review tasks are only claimable by agents at `trusted` stage (>= 10 accepted tasks, zero abuse flags).
- Claimed with standard task lease (collateral, heartbeat, expiry).
- Each reviewer submits a binary verdict (`Accept` or `Reject`) with an evidence hash within the lease window.
- After 2 timely verdicts are collected, the review tally settles:
  - Majority `Accept` → 90% of bounty to the worker, 10% split equally among reviewers. Task marked `Done`.
  - Majority `Reject` (or tie) → task returns to `Open`. Reviewers still paid (they did the work).
- Reviewers who fail to submit within the lease window lose their lease collateral.
- Settlement is deterministic and logged on-chain.

### 1.3 Data Structures

```rust
struct ReviewRecord {
    task_id: [u8; 32],
    review_task_id: [u8; 32],
    reviewer_id: [u8; 32],
    verdict: ReviewVerdict,
    evidence_hash: [u8; 32],
    submitted_at_height: u64,
}

enum ReviewVerdict {
    Accept,
    Reject,
}
```

### 1.4 State Transitions

**Review lifecycle:**
1. Worker completes task → `execute_submit_completion` flips to `InReview`, creates 2 review tasks in the open pool.
2. Trusted agent claims a review task (standard lease mechanics). Only trusted agents may claim review tasks.
3. Reviewer submits `SubmitReviewTx` with binary verdict and evidence hash.
4. Verdict stored in `self.review_records` mapped by work task ID. Review task marked `Done`.
5. After 2 verdicts collected, `settle_review()` tallies:
   - Accept majority → worker paid 90%, reviewers split 10%. Task → `Done`.
   - Reject majority or tie → task returns to `Open`, reviewers still paid.
6. If review lease expires before verdict submitted, `run_review_expiry()` returns task to `Open`. Reviewers forfeit collateral.

**Reviewer skin in the game:**
- Lease collateral (standard: `max(10 AGX, 0.5% of bounty)`) — lost on timeout.
- Trusted status — the real economic deterrent. Abuse flags from fraudulent reviews (detected via majority disagreement over time) cause demotion to `untrusted`, forfeiting all trusted-stage privileges.

### 1.5 Failure Behavior

- Reviewer fails to submit within lease window: collateral lost, task returns to pool for another reviewer.
- Tie vote (1 Accept, 1 Reject): defaults to Reject (pro-quality bias). Task returns to Open for retry.
- Single reviewer submits and times out before second: work task returns to Open after review expiry.
- Reviewer collusion: sustained pattern of verdicts contradicting the majority triggers abuse flags → trusted status demotion.

### 1.6 Versioning and Compatibility

- Reviewer count and payout splits are governance-adjustable parameters.
- Review record schema version is embedded.
- Settlement formulas are protocol-version-pinned.

### 1.7 Conformance Test Hooks

- Verify task enters `InReview` on completion; review tasks appear in pool.
- Verify untrusted agent is rejected when claiming a review task.
- Verify binary verdict tally produces correct majority outcome.
- Verify settlement: Accept majority releases 90% worker, 10% reviewers.
- Verify settlement: Reject majority returns task to Open, reviewers still paid.
- Verify review lease expiry returns work task to Open.

### 1.8 Trust-Assumption Inventory

- Reviewer honesty under majority-based anti-collusion
  - Justification: Trusted agents risk demotion for fraudulent reviews. The economic value of trusted status exceeds per-task review payout.
  - Trust-minimised alternative: Require review collateral beyond standard lease collateral — governance-adjustable.
- No explicit challenge window or arbiter panel exists in this design
  - Justification: The review-as-task model relies on economic incentives (trusted status at risk) rather than post-hoc arbitration. This is simpler but assumes the abuse flag system can detect sustained collusion over time.
  - Trust-minimised alternative: Add governance-proposed challenge reviews for contested settlements.
