# Runtime Spec: Review Engine

**Component:** C12 Economics & Incentives (Review Market)
**Source ADRs:** ADR-0008 (Two-Phase Quality Pipeline), ADR-0017 (90/10 Payout Split)
**Covered FRs:** FR-0148, FR-0149, FR-0161, FR-0164, FR-0165, FR-0170, FR-0175, FR-0191
**Dependencies:** C1 Consensus Engine, C8 Artifact Availability, C9 Policy Decision Point

---

## Section 1: Two-Phase Quality Pipeline

### 1.1 Purpose

Define the review pipeline that validates agent-submitted work through independent reviewer attestation, challenge window, and deterministic settlement.

### 1.2 Normative Behavior

- Completed tasks enter a review phase with 3 protocol-assigned independent reviewers.
- Reviewers are assigned deterministically using operator-cluster diversity constraints (see FR-0191).
- Each reviewer submits a binary verdict (accept/reject) within the review window.
- Majority approval triggers provisional settlement; majority rejection returns the task to the worker for revision.
- After review finalization, a challenge window opens. If unchallenged, escrow settles: 90% to worker, 10% split among timely reviewers.
- If challenged, an independent arbiter panel evaluates the challenge and either confirms the settlement or claws back funds.
- Settlement is deterministic and logged on-chain.

### 1.3 Data Structures

```rust
struct ReviewAssignment {
    assignment_id: [u8; 32],
    task_id: [u8; 32],
    reviewer_id: [u8; 32],
    assigned_at_height: u64,
    deadline_height: u64,
    status: ReviewStatus,
}

enum ReviewStatus { Assigned, Submitted, TimedOut }

struct ReviewRecord {
    assignment_id: [u8; 32],
    verdict: ReviewVerdict,
    evidence_hash: [u8; 32],
    reviewer_signature: Vec<u8>,
}

enum ReviewVerdict { Accept, Reject }

struct SettlementRecord {
    task_id: [u8; 32],
    worker_id: [u8; 32],
    reviewers: Vec<[u8; 32]>,
    worker_payout: u128,
    reviewer_payout: u128,
    settled_at_height: u64,
}
```

### 1.4 State Transitions

**Review lifecycle:**
1. Task completed → enters review phase.
2. 3 reviewers assigned deterministically from eligible pool.
3. Each reviewer submits `ReviewRecord` with binary verdict.
4. After all submitted or deadline: count votes. Majority rules.
5. If majority accept: escrow → 90% worker, 10% reviewers. Funds computed but not released.
6. Challenge window opens (144 blocks).
7. If unchallenged: funds released atomically.
8. If challenged: arbiter panel evaluates, either confirms settlement or reverses.

### 1.5 Failure Behavior

- Reviewer fails to submit within deadline: counted as no-vote (not penalized).
- Tie vote (1 accept, 1 reject, 1 no-vote): defaults to accept (pro-worker bias).
- Challenge timeout: challenge rejected, settlement confirmed.
- Arbiter panel fails to reach majority: settlement holds.

### 1.6 Versioning and Compatibility

- Reviewer count, deadline windows, and payout splits are governance-adjustable parameters.
- Review record schema version is embedded.
- Settlement formulas are protocol-version-pinned.

### 1.7 Conformance Test Hooks

- Verify 3 reviewers are assigned with operator-cluster diversity constraint.
- Verify binary verdict counting produces correct majority outcome.
- Verify challenge window opens after review finalization.
- Verify settlement splits 90/10 correctly.
- Verify arbiter panel independence constraint.

### 1.8 Trust-Assumption Inventory

- Reviewer independence constraints effectiveness
  - Justification: Stake-graph analysis and key correlation may not detect all collusion relationships.
  - Trust-minimised alternative: Economic incentives that make collusion more expensive than honesty (requires calibration).
- Challenge arbiter fairness
  - Justification: Challenge outcomes affect real economic penalties; arbiter must be auditable.
  - Trust-minimised alternative: Multi-sig arbiter panel from diverse validators with governance-enforced penalties on incorrect arbitration.

---

## Section 2: Sybil Detection

Sybil detection is handled by operator-cluster diversity analysis via stake-graph funding-edge tracking (see collaboration-spec.md §3 and FR-0191). No multi-signal correlation engine exists at this layer.
