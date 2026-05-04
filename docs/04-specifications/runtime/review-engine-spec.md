# Runtime Spec: Review Engine & Quality Pipeline

**Component:** C12 Economics & Incentives (Review Market)
**Source ADRs:** ADR-0008 (Three-Phase Quality Pipeline), ADR-0011 (Review Sandbox Isolation)
**Covered FRs:** FR-0148, FR-0149, FR-0150, FR-0153, FR-0161, FR-0162, FR-0163, FR-0164, FR-0165, FR-0166, FR-0167, FR-0168, FR-0169, FR-0170, FR-0171, FR-0172, FR-0173, FR-0174, FR-0175, FR-0191
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
    bond_amount: u128,             // 20% of task_bounty in atto-AGX
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

---

## Section 2: Sybil Detection Adjudication

### 2.1 Purpose

Define the Sybil detection correlation engine and its adjudication sub-type within the review pipeline. This is a continuous background process that identifies correlated identity clusters and triggers independent review panels to confirm or dismiss Sybil allegations.

### 2.2 Normative Behavior

**Correlation scoring:**

- The system MUST compute pairwise correlation scores for all active identities at each epoch boundary from finalized chain state.
- Five weighted correlation signals MUST be used:
  1. Vote alignment score (weight 0.25): `matching_votes / total_co_reviewed_submissions`, minimum 3 co-reviewed submissions.
  2. Task co-claiming score (weight 0.20): `co_topic_sessions / min(identity_a_topics, identity_b_topics)`.
  3. Temporal activity overlap score (weight 0.15): Jaccard similarity of active epochs.
  4. Stake-graph distance score (weight 0.25): `1.0 / (1 + shortest_transfer_path_length)`, capped at depth 5.
  5. Cross-review failure rate score (weight 0.15): `cross_review_failures / total_cross_reviews`.
- Composite score formula: `0.25 * vote_align + 0.20 * co_claim + 0.15 * temporal + 0.25 * stake_dist + 0.15 * cross_fail`.
- All scores MUST be deterministic — same inputs produce identical outputs on all nodes.
- Only identity pairs with at least one shared interaction (co-topic session, co-reviewed submission, or shared epoch activity) MUST be scored. Full pairwise comparison of zero-interaction pairs is not required.
- Per-epoch alert cap: governance-adjustable, default 1,000 alerts per epoch.

**Threshold and alerting:**

- Default correlation threshold: 0.70. Pairs above this threshold trigger a CorrelationAlert.
- Emergency threshold: 0.50, activated during circuit-breaker mode.
- Signal weights and thresholds MUST be governance-adjustable within bounded ranges (0.05–0.50 per weight, thresholds 0.40–0.90).
- Weight changes MUST be epoch-bound: historical scores retain their original weight vector hash for auditability.

**Cluster aggregation:**

- CorrelationAlerts across identity pairs MUST be grouped into connected clusters using transitive closure.
- Minimum cluster size for adjudication: 2 identities (pairs).
- Cluster size thresholds:
  - 2 identities: standard review panel, 5 reviewers, 48-hour window.
  - 3–5 identities: escalated panel, 7 reviewers, 24-hour window.
  - 6+ identities: emergency panel, 11 reviewers, 24-hour window, automatic circuit-breaker consideration.

**Adjudication panel selection:**

- Panel members MUST be at `trusted_contributor` trust stage or higher.
- Panel members MUST have zero correlation (<0.10) with any identity in the flagged cluster.
- Panel size: `max(5, cluster_size * 2)`, capped at 11.
- Panel selection MUST use deterministic sampling from the eligible pool seeded by `SHA3-256(cluster_id || epoch_seed)`.
- Reviewers assigned to the adjudication panel MUST bond standard review collateral.
- Panel MUST NOT contain any reviewer with an active CorrelationAlert against their own identity.

**Evidence bundle:**

- The system MUST present the adjudication panel with an evidence bundle containing:
  - Per-signal score breakdown with sample sizes.
  - Historical score trajectory for the pair (last 10 epochs).
  - Exemplar co-occurrences (timestamps, topic IDs, task IDs).
  - Aggregate cluster visualization (connected pairs, cluster size, detection epoch).
- Panel members vote `CONFIRM` or `DISMISS`. Threshold: 60% majority (3/5, 5/7, 7/11).

**Adjudication execution:**

- On CONFIRM verdict:
  - All probationary bonds (locked tranches not yet released) for cluster members MUST be burned.
  - All cluster members MUST be demoted by 2 trust stages (floor: `untrusted_joiner`).
  - The cluster hash MUST be stored permanently for whitewash detection — new identities with correlation >0.50 to any confirmed cluster within 90 days inherit heightened scrutiny.
- On DISMISS verdict:
  - Reviewer bonds MUST be returned.
  - Cluster MUST be marked as dismissed with the epoch of dismissal.
  - False-positive counter MUST be incremented for panel quality tracking and threshold recalibration.
  - Dismissed pairs may be re-alerted in subsequent epochs if correlation persists.

### 2.3 Data Structures

```rust
struct CorrelationAlert {
    identity_a: [u8; 32],
    identity_b: [u8; 32],
    score: f32,                    // 0.0–1.0, normalized
    weight_vector_hash: [u8; 32],  // hash of weight config used
    epoch: u64,
    signal_breakdown: SignalBreakdown,
}

struct SignalBreakdown {
    vote_alignment: f32,
    co_claiming: f32,
    temporal_overlap: f32,
    stake_distance: f32,
    cross_review_failure: f32,
    samples: SignalSamples,        // per-signal sample sizes
}

struct SignalSamples {
    co_reviewed_count: u32,
    co_topic_sessions: u32,
    active_epochs_a: u32,
    active_epochs_b: u32,
    transfer_path_length: Option<u8>,
    cross_reviews_total: u32,
    cross_reviews_failed: u32,
}

struct CorrelationCluster {
    cluster_id: [u8; 32],          // SHA3-256(sorted(identity_ids) || epoch)
    member_ids: Vec<[u8; 32]>,
    size: u8,
    detected_at_epoch: u64,
    status: ClusterStatus,
}

enum ClusterStatus {
    PendingAdjudication,
    Confirmed,
    Dismissed,
}

struct AdjudicationVerdict {
    cluster_id: [u8; 32],
    panel_ids: Vec<[u8; 32]>,
    votes_confirm: u8,
    votes_dismiss: u8,
    outcome: VerdictOutcome,
    epoch: u64,
}

enum VerdictOutcome {
    Confirmed,
    Dismissed,
}
```

### 2.4 State Transitions

```
EpochBoundary ─► CorrelationSweep
  │
  └── [pairwise scores computed, threshold compared]
        └── CorrelationAlert [score >= threshold]
              └── ClusterAggregation [transitive closure]
                    └── PendingAdjudication
                          ├── PanelSelected [independent reviewers assigned]
                          │     └── UnderReview [evidence bundle distributed]
                          │           ├── [>=60% CONFIRM] ─► Confirmed [bond burn + demotion]
                          │           └── [otherwise] ─► Dismissed [bonds returned]
                          └── [no eligible panel available] ─► Deferred [re-evaluated next epoch]
```

### 2.5 Failure Behavior

- **No eligible panel:** If fewer than 5 uncorrelated `trusted_contributor`+ agents are available, the cluster is deferred to the next epoch. Expands eligibility to `sandboxed_contributor` on second deferral (with increased panel size).
- **Alert flood:** Per-epoch alert cap (default 1,000) prevents review pipeline exhaustion. Alerts above cap are deferred to next epoch, prioritized by correlation score descending.
- **False positive cluster:** Dismissed clusters increment the false-positive counter. If a panel's false-positive rate exceeds 20% over 100 adjudications, panel selection weights are adjusted to exclude underperforming reviewers.
- **Stake-graph re-org:** If chain reorganization alters the transaction graph after scoring, the affected alerts are invalidated and rescored at the next epoch boundary.
- **Correlation evasion via randomization:** Identity pairs that occasionally randomize behavior will have lower but non-zero correlation scores. Persistent sub-threshold correlation (0.50–0.69) for 5+ consecutive epochs triggers a lower-priority review for pattern analysis.

### 2.6 Conformance Test Hooks

- Verify five-signal correlation scores are deterministic from identical epoch state.
- Verify default threshold 0.70 correctly separates known-correlated from known-independent test pairs.
- Verify cluster aggregation via transitive closure groups all connected pairs.
- Verify adjudication panel selection enforces <0.10 correlation to all cluster members.
- Verify confirmed cluster triggers bond burn for probationary tranches only (earned tranches unaffected).
- Verify confirmed cluster triggers 2-stage trust demotion (floor: untrusted_joiner).
- Verify dismissed cluster returns reviewer bonds and increments false-positive counter.
- Verify per-epoch alert cap defers overflow alerts to next epoch.
- Verify weight changes are epoch-bound; historical scores retain original weight vector hash.
- Verify minimum signal sample sizes (3 co-reviews for vote alignment, 3 co-topics for co-claiming) prevent noise.

### 2.7 Trust-Assumption Inventory

- Score determinism across implementations
  - Justification: Floating-point normalization must produce identical bit patterns across platforms.
  - Trust-minimised alternative: Fixed-point integer arithmetic for all score computations; convert to float only for final display/UI.
- Adjudication panel independence verification
  - Justification: Correlation score between panel members and cluster members relies on the same correlation engine that is under review.
  - Trust-minimised alternative: Panel independence verified via orthogonal signals (operator-cluster analysis, stake-graph diversity) in addition to correlation scores.
- Sybil cluster confirmation without false positives
  - Justification: Bond burn and trust demotion are irreversible penalties; false positives cause permanent harm.
  - Trust-minimised alternative: Probationary penalty period before full bond burn (e.g., 7-epoch freeze with appeal window).
