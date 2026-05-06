# Runtime Spec: Review Engine

**Component:** C12 Economics & Incentives (Review Market)
**Source ADRs:** ADR-0008 (Two-Phase Quality Pipeline)
**Covered FRs:** FR-0148, FR-0149, FR-0150, FR-0161, FR-0162, FR-0163, FR-0164, FR-0165, FR-0168, FR-0169, FR-0170, FR-0171, FR-0191
**Dependencies:** C1 Consensus Engine, C8 Artifact Availability, C9 Policy Decision Point

---

## Section 1: Two-Phase Quality Pipeline

### 1.1 Purpose

Define the two-phase quality pipeline for task output verification: independent review followed by challenge window. No objective verification phase exists — reviewers verify the work directly.

### 1.2 Normative Behavior

- The system MUST implement a two-phase quality pipeline: Phase 1 (independent review) → Phase 2 (challenge finality).
- Reviewers MUST be assigned by protocol, not self-selected.
- Reviewer assignment MUST enforce one independence constraint: no reviewer shares an operator cluster with any other reviewer or the worker (detected via stake-graph cluster analysis, see `stake-graph-analysis-spec.md`).
- Minimum 3 reviewers per task. If insufficient eligible reviewers, the task remains in the pool until enough become available.
- The system MUST cap concurrent review assignments at 5 per reviewer.
- Review assignments MUST have deadlines: 72 hours standard, 24 hours urgent.
- Missed deadline MUST count as no-vote (not penalized, does not affect quorum).
- Payout is fixed: if majority approves, bounty is split equally among approving reviewers. Worker receives a completion reward from the task bounty. If majority denies, worker forfeits claim and task returns to pool.

### 1.3 Data Structures

```rust
struct ReviewAssignment {
    assignment_id: [u8; 32],       // SHA3-256(task_id || reviewer_id || epoch)
    task_id: [u8; 32],
    reviewer_id: [u8; 32],
    assigned_at_height: u64,
    deadline_height: u64,          // assigned_at + (72h or 24h)
    status: AssignmentStatus,
}

enum AssignmentStatus {
    Pending,
    InProgress,
    Completed,
    TimedOut,
}

struct ReviewRecord {
    assignment_id: [u8; 32],
    task_id: [u8; 32],
    reviewer_id: [u8; 32],
    verdict: Verdict,
    reason_hash: [u8; 32],
    reviewer_signature: Vec<u8>,
    submitted_at_height: u64,
}

enum Verdict {
    Approve,
    Deny,
}

struct ChallengeRecord {
    challenge_id: [u8; 32],
    task_id: [u8; 32],
    challenger_id: [u8; 32],
    evidence_hash: [u8; 32],
    bond_amount: u128,             // 20% of task_bounty in atto-AGX
    submitted_at_height: u64,
    outcome: Option<ChallengeOutcome>,
}

enum ChallengeOutcome {
    Upheld,      // challenge successful, work+reviews penalized
    Overturned,  // challenge unsuccessful, challenger bond burned
}

```

### 1.4 State Transitions

**Two-phase pipeline flow:**

```
Phase 1: INDEPENDENT REVIEW
  Protocol assigns reviewers with independence constraints
  → Reviewers independently fetch artifacts by hash
  → Reviewers verify hash, run against same execution_profile_hash
  → Reviewers submit signed ReviewRecord with binary verdict
  → Provisional settlement at review completion (2f+1 quorum)
  → Challenge window (144 blocks) opens

Phase 2: CHALLENGE FINALITY
  Any eligible participant may submit ChallengeRecord with bond
  → Commit-reveal: challenger submits commit hash, reveals after 6 blocks
  → Arbiter evaluates evidence against work output and review records
  → If upheld: work rejected, worker penalized, incorrect reviewers penalized, challenger rewarded
  → If overturned: challenge bond burned (partially)
  → Final settlement after challenge window closes unchallenged or challenge resolved
```

**Reviewer assignment algorithm:**
1. Build eligible reviewer pool: trust_stage >= trusted, <5 active assignments, no recent abuse flags.
2. Apply independence constraints: operator cluster diversity (min 2), temporal spread (active last 7 days), stake spread (max 30% same tier), pair frequency cap (1 in 10).
3. If pool >= 50 eligible: deterministic selection by SHA3-256(task_id || epoch_seed).
4. Fallback 1: relax pool floor to current available size.
5. Fallback 2: extend assignment deadline by 24 hours.
6. Fallback 3: reduce required reviewer count (proportional reward-cap downgrade).
7. If pool < 3 eligible: task returns to open queue (domain expert bottleneck).

### 1.5 Failure Behavior

- **All reviewers time out:** Task returns to open pool with new reviewer set. No penalties on timed-out reviewers.
- **Reviewer collusion:** Pair-frequency cap and independence constraints limit sustained collusion. EvidenceTx for governance review of suspected colluders.
- **Challenge spam:** Challenger bond (20% of task bounty) is burned on failed challenge. Per-identity challenge cap (3 per epoch).
- **Settlement clawback:** Successful challenge triggers clawback from worker AND incorrect reviewers. Proportional to review influence.
- **Evidence replay:** Artifact hash bound to task_id + freshness nonce prevents old evidence reuse for new reward claims.

### 1.6 Versioning and Compatibility

- Checker bundle hash versioned by epoch; deterministically pinned.

- Assignment algorithm version tracked in policy bundle.

### 1.7 Conformance Test Hooks

- Verify Phase 1 objective checks produce deterministic pass_fail_vector from same inputs.
- Verify protocol-assigned reviewers (not self-selected).
- Verify reviewer independence constraints enforced at assignment time.
- Verify pair-frequency cap: same pair max 1 per 10-task window.
- Verify concurrent review cap at 5 per reviewer.
- Verify 72-hour standard deadline, 24-hour urgent.
- Verify provisional settlement immediate, final settlement after challenge window.
- Verify commit-reveal for challenges with 6-block delay.
- Verify loser-pays: failed challenge burns bond.
- Verify accurate minority rewards: dissenting reviewer proven correct earns bonus.
- Verify fixed payout: majority approves → bounty split equally among approving reviewers.
- Verify replay prevention: old artifact with stale nonce rejected.

### 1.8 Trust-Assumption Inventory

- Objective checker correctness
  - Justification: Phase 1 checks are deterministic but checkers may have bugs or gaps.
  - Trust-minimised alternative: Multi-implementation checker comparison (different languages/implementations produce same vector).
- Reviewer independence constraints effectiveness
  - Justification: Stake-graph analysis and key correlation may not detect all collusion relationships.
  - Trust-minimised alternative: Economic incentives that make collusion more expensive than honesty (requires calibration).
- Challenge arbiter fairness
  - Justification: Challenge outcomes affect real economic penalties; arbiter must be auditable.
  - Trust-minimised alternative: Multi-sig arbiter panel from diverse validators with governance-enforced penalties on incorrect arbitration.

---

## Section 2: Sybil Detection

Sybil detection is handled by the trust ladder's abuse-flag mechanism (see collaboration-spec.md §3). No separate protocol-level correlation engine exists at this layer.
