## FR-0161: Quality-Weighted Review Market

**Category:** Economics

**Statement:** The system shall implement a review market where reviewers are protocol-assigned with independence constraints, and accurate reviews earn rewards while bad-faith reviews are penalized.

**Rationale:** Reduces collusion and review capture by author-aligned groups. See `proof-of-work-quality-and-review-markets.md` Section 6, Tradeoff 2.

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 5 (Review market design)
- `proof-of-work-quality-and-review-markets.md` Section 5 (Scoring model)

**Acceptance Criteria:**
- [ ] Reviewers are assigned by protocol, not self-selected.
- [ ] Assignment considers reliability, diversity, and anti-pair-repetition.
- [ ] Reviewer collateral is bonded per review batch.
- [ ] Accurate minority calls that later prove correct earn higher rewards.

**Dependencies:** FR-0099
**Tags:** must-have

---

## FR-0164: Reviewer Collateral

**Category:** Economics

**Statement:** The system shall bond reviewer collateral per review batch.

**Rationale:** Prevents frivolous reviewing without economic stake. See `proof-of-work-quality-and-review-markets.md` Section 5 (Review market design).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 5, lines 110-112

**Acceptance Criteria:**
- [ ] Reviewer must bond collateral before accepting assignment.
- [ ] Collateral is released after challenge window closes.

**Dependencies:** FR-0161
**Tags:** must-have

---

## FR-0165: Reviewer Independence via Operator-Cluster Diversity

**Category:** Economics

**Statement:** The system shall enforce one reviewer independence constraint: no reviewer shares an operator cluster with any other reviewer or the worker (detected via stake-graph analysis, see `stake-graph-analysis-spec.md`).

**Rationale:** Prevents reviewer collusion via same-operator coordination. Operator-cluster analysis is simpler and more robust than temporal/stake/pair-frequency constraints.

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 5, lines 122-131

**Acceptance Criteria:**
- [ ] Protocol tracks pair counts per rolling 10-task window deterministically.
- [ ] Assignment rejected if cap exceeded.
- [ ] Cap enforcement is visible in assignment logs.

**Dependencies:** FR-0099
**Tags:** must-have

---

## FR-0166: Manual Governance Escalation for Collusion

**Category:** Economics

**Statement:** The system shall allow anyone to submit `EvidenceTx` with collusion evidence, triggering standard governance vote on slashing suspected reviewers.

**Rationale:** Statistical rules catch simple cases; sophisticated collusion requires human judgment. See `proof-of-work-quality-and-review-markets.md` Section 5 (Anti-collusion controls).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 5, lines 126-131

**Acceptance Criteria:**
- [ ] Collusion evidence is content-addressed and signed.
- [ ] Governance proposal type `COLLUSION_EVIDENCE` exists.
- [ ] Slashing requires standard governance quorum and majority.
- [ ] Removed: automated statistical correlation penalties (false positive risk).

**Dependencies:** FR-0019, FR-0025
**Tags:** must-have

---

## FR-0167: Reviewer Assignment Fallbacks

**Category:** Economics

**Statement:** The system shall apply deterministic fallbacks when eligible reviewer pool is insufficient: relax pool floor, extend deadline, reduce required reviewer count with proportional reward-cap downgrade.

**Rationale:** Prevents task stalls when domain expertise is scarce. See `proof-of-work-quality-and-review-markets.md` Section 8 (Reviewer assignment parameters).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 8, lines 269-283

**Acceptance Criteria:**
- [ ] Preferred threshold: 50 eligible reviewers.
- [ ] Fallback 1: relax pool floor to current available size.
- [ ] Fallback 2: extend assignment deadline.
- [ ] Fallback 3: reduce reviewer count with proportional reward-cap downgrade.
- [ ] If pool drops below 3 eligible reviewers, task returns to open queue.

**Dependencies:** FR-0161
**Tags:** must-have

---

## FR-0168: Reviewer Load Cap

**Category:** Economics

**Statement:** The system shall cap concurrent review assignments at 5 per reviewer to prevent reviewer monopolies and ensure timely responses.

**Rationale:** Prevents a small set of reviewers from dominating the market. See `proof-of-work-quality-and-review-markets.md` Section 8 (Reviewer assignment parameters).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 8, line 283

**Acceptance Criteria:**
- [ ] Reviewers with >=5 active assignments are excluded from new assignment pool.
- [ ] Load cap is enforced deterministically at assignment time.
- [ ] Review completion frees capacity for next assignment.

**Dependencies:** FR-0161
**Tags:** must-have

---

## FR-0169: Review Assignment Deadline

**Category:** Economics

**Statement:** The system shall enforce review assignment deadlines: 72 hours for standard tasks, 24 hours for urgent tasks.

**Rationale:** Prevents review stalls without requiring manual intervention. See `proof-of-work-quality-and-review-markets.md` Section 8 (Reviewer assignment parameters).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 8, lines 279-281
- `index.md` (Review Timeout Semantics)

**Acceptance Criteria:**
- [ ] Deadline is computed at assignment inclusion height.
- [ ] Missed deadline counts as no-vote (not penalized).
- [ ] Task can be reassigned after deadline with new reviewer set.
- [ ] Distinct from review sandbox timeout (30 minutes, local runtime limit).

**Dependencies:** FR-0161
**Tags:** must-have

---

## FR-0170: Content-Addressed Artifact Reproducibility

**Category:** Economics

**Statement:** The system shall require reviewers to independently fetch artifacts by hash and run reproducibility replay against the same `execution_profile_hash`, publishing signed `ReviewRecord`.

**Rationale:** Ensures review is based on identical inputs, preventing artifact substitution. See `proof-of-work-quality-and-review-markets.md` Section 5 (Verification pipeline).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 5, lines 87-91

**Acceptance Criteria:**
- [ ] Reviewers fetch artifacts by content hash.
- [ ] Reviewers verify hash upon receipt.
- [ ] Reviewers execute against pinned `execution_profile_hash`.
- [ ] `ReviewRecord` includes submission_id, score, verdict, reason_hash, reviewer_sig.

**Dependencies:** FR-0053
**Tags:** must-have

---

## FR-0171: Quality Score Formula

**Category:** Economics

**Statement:** The system shall compute final quality score as weighted combination: `Q = w1*objective + w2*review + w3*durability`, with weights set by governance.

**Rationale:** Multiple quality dimensions reduce gaming of any single metric. See `proof-of-work-quality-and-review-markets.md` Section 5 (Scoring model).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 5, lines 98-104

**Acceptance Criteria:**
- [ ] Weights are governance-adjustable within defined bounds.
- [ ] Each component is independently computable and verifiable.
- [ ] Score is clamped to [0, 1].
- [ ] Score formula is deterministic across all nodes.

**Dependencies:** FR-0163
**Tags:** must-have

---

## FR-0172: Provisional Settlement with Clawback

**Category:** Economics

**Statement:** The system shall support provisional payout upon review completion, with deterministic clawback path if challenge succeeds.

**Rationale:** Preserves speed while keeping fraud correction. See `proof-of-work-quality-and-review-markets.md` Section 6, Tradeoff 3.

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 5 (Challenge and dispute logic)
- `proof-of-work-quality-and-review-markets.md` Section 6, Tradeoff 3

**Acceptance Criteria:**
- [ ] Provisional reward is transferred at review finalization.
- [ ] Successful challenge triggers clawback from worker.
    - [ ] Incorrect reviewers are penalized; challenger is rewarded.
- [ ] Clawback amount is deterministic based on challenge outcome.

**Dependencies:** FR-0148, FR-0149
**Tags:** must-have

---

## FR-0173: Challenge Spam Defense

**Category:** Economics

**Statement:** The system shall defend against challenge spam with challenger collateral, loser-pays penalties, and per-identity challenge quotas.

**Rationale:** Cheap challenges can delay payouts indefinitely. See `proof-of-work-quality-and-review-markets.md` Section 7 (Challenge spam flood).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 7 (Challenge spam flood)

**Acceptance Criteria:**
- [ ] Challenger must post collateral proportional to task value.
- [ ] Failed challenge burns challenger collateral.
- [ ] Per-identity challenge cap prevents bulk griefing.

**Dependencies:** FR-0149
**Tags:** must-have

---

## FR-0174: Domain Expert Bottleneck Fallback

**Category:** Economics

**Statement:** The system shall support hierarchical reviewer tiers and fallback to wider panels with lower confidence weighting when niche domain experts are scarce.

**Rationale:** Prevents indefinite task stalls in specialized domains. See `proof-of-work-quality-and-review-markets.md` Section 7 (Domain expert bottleneck).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 7 (Domain expert bottleneck)

**Acceptance Criteria:**
- [ ] Tier-1 reviewers: domain experts with high reliability.
- [ ] Tier-2 fallback: broader panel with confidence weighting.
- [ ] Weighting adjustment is deterministic and logged.

**Dependencies:** FR-0167
**Tags:** should-have

---

## FR-0175: Replay of Old Evidence Prevention

**Category:** Economics

**Statement:** The system shall bind artifact hash to task scope and freshness nonce to prevent reuse of old artifacts for new bounty claims.

**Rationale:** Prevents artifact recycling fraud. See `proof-of-work-quality-and-review-markets.md` Section 7 (Replay of old evidence).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 7 (Replay of old evidence)

**Acceptance Criteria:**
- [ ] Submission includes task-specific freshness nonce.
- [ ] Artifact hash is bound to task_id + nonce.
- [ ] Replay submission with old nonce is rejected.

**Dependencies:** (none)
**Tags:** must-have
