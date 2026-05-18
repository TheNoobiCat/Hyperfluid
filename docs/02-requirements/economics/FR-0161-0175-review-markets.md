## FR-0161: Review Market with Independent Review

**Category:** Economics

**Statement:** The system shall implement a review market where reviewers are protocol-assigned with independence constraints, and accurate reviews earn rewards while bad-faith reviews are penalized.

**Rationale:** Reduces collusion and review capture by author-aligned groups. See `proof-of-work-quality-and-review-markets.md` Section 6, Tradeoff 2.

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 5 (Review market design)
- `proof-of-work-quality-and-review-markets.md` Section 5 (Scoring model)

**Acceptance Criteria:**
- [ ] Reviewers are assigned by protocol, not self-selected.
- [ ] Assignment enforces operator-cluster independence.
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
- [ ] Operator clusters are detected via stake-graph analysis and key correlation heuristics.
- [ ] No reviewer shares a cluster with the worker or another reviewer.
- [ ] If cluster constraints cannot be met, assignment falls back to next eligible reviewer.

**Dependencies:** FR-0099
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
