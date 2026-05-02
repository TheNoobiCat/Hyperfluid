# Runtime Spec: Review Engine & Quality Pipeline

**Component:** C12 Economics & Incentives (Review Market)
**Source ADRs:** ADR-0008 (Three-Phase Quality Pipeline), ADR-0011 (Review Sandbox Isolation)
**Covered FRs:** FR-0161, FR-0162, FR-0163, FR-0164, FR-0165, FR-0166, FR-0167, FR-0168, FR-0169, FR-0170, FR-0171, FR-0172, FR-0173, FR-0174, FR-0175
**Dependencies:** C1 Consensus Engine, C8 Artifact Availability, C9 Policy Decision Point

---

## Section 1: Three-Phase Quality Pipeline

### 1.1 Purpose

Define the review market and three-phase quality verification pipeline for work output evaluation.

### 1.2 Normative Behavior

- The system MUST implement a three-phase quality pipeline: Phase 1 (objective checks) → Phase 2 (independent review) → Phase 3 (challenge finality).
- Reviewers MUST be assigned by protocol, not self-selected.
- Reviewer assignment MUST enforce independence constraints: minimum 2 distinct operator clusters, temporal spread (active within 7 days), stake spread (max 30% from same tier), pair frequency cap (max 1 same reviewer-author pair per 10 tasks).
- The system MUST cap concurrent review assignments at 5 per reviewer.
- Review assignments MUST have deadlines: 72 hours standard, 24 hours urgent.
- Missed deadline MUST count as no-vote (not penalized, does not affect quorum).
- Provisional settlement MUST be immediate on review completion; final settlement MUST wait until challenge window (144 blocks) closes unchallenged.

### 1.3 Data Structures

```rust
struct ObjectiveCheckRecord {
    task_id: [u8; 32],
    artifact_root_hash: [u8; 32],
    checker_bundle_hash: [u8; 32],
    pass_fail_vector: Vec<bool>,
    metrics_hash: [u8; 32],
    verifier_signature: Vec<u8>,
    height: u64,
}

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
    quality_score: u8,            // 0-100 normalized
    reason_hash: [u8; 32],
    objective_check_ref: [u8; 32],
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
    bond_amount: u64,             // 20% of task_bounty
    submitted_at_height: u64,
    outcome: Option<ChallengeOutcome>,
}

enum ChallengeOutcome {
    Upheld,      // challenge successful, work+reviews penalized
    Overturned,  // challenge unsuccessful, challenger bond burned
}

struct QualityScore {
    task_id: [u8; 32],
    objective_score: f64,    // normalized [0, 1]
    review_score: f64,       // weighted reviewer verdicts [0, 1]
    durability_score: f64,   // survived challenge window [0, 1]
    final_score: f64,        // w1*objective + w2*review + w3*durability
    weights: (f64, f64, f64),
}
```

### 1.4 State Transitions

**Three-phase pipeline flow:**

```
Phase 1: OBJECTIVE VERIFICATION
  Work submitted with artifact_root_hash, execution_profile_hash
  → Node pulls artifact chunks by hash, recomputes root
  → Runs deterministic checker set (pinned per epoch by checker_bundle_hash)
  → Produces ObjectiveCheckRecord with pass_fail_vector + metrics_hash
  → All checks pass → proceeds to Phase 2; any failure → rejected immediately

Phase 2: INDEPENDENT REVIEW
  Protocol assigns reviewers with independence constraints
  → Reviewers independently fetch artifacts by hash
  → Reviewers verify hash, run against same execution_profile_hash
  → Reviewers submit signed ReviewRecord with verdict + quality_score
  → Provisional settlement at review completion (2f+1 quorum)
  → Challenge window (144 blocks) opens

Phase 3: CHALLENGE FINALITY
  Any eligible participant may submit ChallengeRecord with bond
  → Commit-reveal: challenger submits commit hash, reveals after 6 blocks
  → Arbiter evaluates evidence against work output and review records
  → If upheld: work rejected, worker penalized, incorrect reviewers penalized, challenger rewarded
  → If overturned: challenge bond burned (partially)
  → Final settlement after challenge window closes unchallenged or challenge resolved
```

**Reviewer assignment algorithm:**
1. Build eligible reviewer pool: trust_stage >= trusted_contributor, <5 active assignments, no recent abuse flags.
2. Apply independence constraints: operator cluster diversity (min 2), temporal spread (active last 7 days), stake spread (max 30% same tier), pair frequency cap (1 in 10).
3. If pool >= 50 eligible: deterministic selection by SHA3-256(task_id || epoch_seed).
4. Fallback 1: relax pool floor to current available size.
5. Fallback 2: extend assignment deadline by 24 hours.
6. Fallback 3: reduce required reviewer count (proportional reward-cap downgrade).
7. If pool < 3 eligible: task returns to open queue (domain expert bottleneck).

### 1.5 Failure Behavior

- **Objective check failure:** Immediate rejection. No review phase. Work submission deposit burned if checks were trivially satisfiable but failed.
- **All reviewers time out:** Task returns to open pool with new reviewer set. No penalties on timed-out reviewers.
- **Reviewer collusion:** Pair-frequency cap and independence constraints limit sustained collusion. EvidenceTx for governance review of suspected colluders.
- **Challenge spam:** Challenger bond (20% of task bounty) is burned on failed challenge. Per-identity challenge cap (3 per epoch).
- **Settlement clawback:** Successful challenge triggers clawback from worker AND incorrect reviewers. Proportional to review influence.
- **Evidence replay:** Artifact hash bound to task_id + freshness nonce prevents old evidence reuse for new reward claims.

### 1.6 Versioning and Compatibility

- Checker bundle hash versioned by epoch; deterministically pinned.
- Quality score weights (w1, w2, w3) are governance-adjustable with default bounds: each weight [0.1, 0.8], sum = 1.0.
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
- Verify quality score formula: Q = w1*objective + w2*review + w3*durability.
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
