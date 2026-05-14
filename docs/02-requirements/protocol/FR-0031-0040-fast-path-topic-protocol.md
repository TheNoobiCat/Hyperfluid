## FR-0031: Topic-Scoped Fast-Path Merges

**Category:** Consensus

**Statement:** The system shall allow fast-path topic merges that remain topic-scoped and cannot directly mutate canonical `git:head` or global protocol state.

**Rationale:** Splits collaboration velocity from global sovereignty. See `topic-fastpath-protocol-spec.md` Section 5 (Core Mechanisms).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 5, lines 81-91
- `topic-fastpath-protocol-spec.md` Section 6, Tradeoff 1

**Acceptance Criteria:**
- [ ] Fast-path merge targets only `topic/<id>/main` branches.
- [ ] Topic merge cannot modify canonical main branch directly.
- [ ] Promotion to canonical requires separate governance proposal with promotion bundle.

**Dependencies:** FR-0021
**Tags:** must-have

---

## FR-0032: Fast-Path Quorum Certificate

**Category:** Consensus

**Statement:** The system shall require a quorum certificate with `2f + 1` weighted approvals from the topic snapshot maintainer/reviewer set to finalize a fast-path merge.

**Rationale:** Ensures Byzantine fault tolerance at topic level. See `topic-fastpath-protocol-spec.md` Section 5 (Eligibility and quorum rules).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 5, lines 93-97
- `collaboration-layer-parallel-teams.md` Section 5 (Fast-path merge constraints)

**Acceptance Criteria:**
- [ ] Quorum threshold is `2f + 1` weighted approvals from snapshot set.
- [ ] Certificate includes aggregate signature and signer set hash.
- [ ] Voting set is frozen at topic snapshot epoch.

**Dependencies:** FR-0031
**Tags:** must-have

---

## FR-0033: Fast-Path Independent Reviewer Requirement

**Category:** Consensus

**Statement:** The system shall require at least one independent reviewer outside the primary author cluster for every fast-path merge certificate.

**Rationale:** Prevents collusion in topic-level approvals. See `collaboration-layer-parallel-teams.md` Section 5 (Fast-path merge constraints).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5, lines 117-119
- `proof-of-work-quality-and-review-markets.md` Section 5 (Reviewer assignment constraints)

**Acceptance Criteria:**
- [ ] Certificate validation checks that at least one approver is not in the primary author's operator cluster.
- [ ] Independent reviewer is defined by stake-graph distance or key correlation heuristics.
- [ ] Merge without independent reviewer is rejected.

**Dependencies:** FR-0032
**Tags:** must-have

---

## FR-0034: Fast-Path Merge Throughput Limits

**Category:** Consensus

**Statement:** The system shall enforce per-topic and per-identity merge throughput limits: max 20 fast merges per topic per hour, max 5 per identity per hour.

**Rationale:** Prevents merge flood attacks on topic branches. See `collaboration-layer-parallel-teams.md` Section 5 (Fast-path merge constraints).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5, lines 126-131
- `network-policy-engine-spec.md` Section 5 (Cross-layer quota matrix)

**Acceptance Criteria:**
- [ ] `max_fast_merges_per_topic_per_hour` = 20.
- [ ] `max_fast_merges_per_identity_per_hour` = 5.
- [ ] Burst mode requires additional independent reviewer signatures.

**Dependencies:** FR-0032
**Tags:** must-have

---

## FR-0035: Deterministic Precheck Before Fast-Path Review

**Category:** Consensus

**Statement:** The system shall run deterministic prechecks (object graph, merge reproducibility, topic scope) before opening fast-path review windows; failures reject immediately.

**Rationale:** Eliminates non-deterministic review overhead. See `topic-fastpath-protocol-spec.md` Section 5 (Deterministic review runtime).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 5, lines 102-107
- `topic-fastpath-protocol-spec.md` Section 5 (Deterministic precheck)

**Acceptance Criteria:**
- [ ] Precheck verifies bundle manifest hash, commit reachability, and merge determinism.
- [ ] Failed precheck emits immediate rejection with reason code.
- [ ] Review sandbox starts only after precheck passes.

**Dependencies:** FR-0031
**Tags:** must-have

---

## FR-0036: Fast-Path Challenge Window

**Category:** Consensus

**Statement:** The system shall open a challenge window after fast-path certification, allowing eligible participants to submit fraud evidence before finalization.

**Rationale:** Preserves speed while keeping fraud correction mechanism. See `topic-fastpath-protocol-spec.md` Section 5 (Conflict and rollback policy).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 5, lines 108-114
- `proof-of-work-quality-and-review-markets.md` Section 5 (Challenge and dispute logic)

**Acceptance Criteria:**
- [ ] Challenge window duration is 144 blocks (~24 hours at 10s block time).
- [ ] Valid challenge causes topic rollback and proposer penalties.
- [ ] Challenger collateral is required; loser-pays policy applies.

**Dependencies:** FR-0032, FR-0148
**Tags:** must-have

---

## FR-0037: Fast-Path Rollback Execution

**Category:** Consensus

**Statement:** The system shall execute certified rollbacks reverting topic state to a prior head, scoped to the affected topic only.

**Rationale:** Localizes fault impact without affecting global state. See `topic-fastpath-protocol-spec.md` Section 5 (Conflict and rollback policy).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 5, lines 108-114

**Acceptance Criteria:**
- [ ] Rollback `FastPathRollbackTx` includes `proposal_id`, `rollback_to_head`, and arbiter certificate.
- [ ] Rollback scope is strictly topic-local.
- [ ] Canonical `git:head` remains unchanged during topic rollback.

**Dependencies:** FR-0036
**Tags:** must-have

---

## FR-0038: Deterministic Conflict Tie-Break

**Category:** Consensus

**Statement:** The system shall resolve competing certificates for the same base topic head using deterministic tie-break: higher approval weight, then lower certificate hash.

**Rationale:** Prevents race-condition capture. See `topic-fastpath-protocol-spec.md` Section 5 (Conflict and rollback policy).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 5, lines 109-112
- `topic-fastpath-protocol-spec.md` Section 6, Tradeoff 3

**Acceptance Criteria:**
- [ ] Tie-break rule is applied deterministically by all nodes.
- [ ] Competing certificates are ordered by approval weight descending, then certificate hash ascending.
- [ ] Lower-ranked certificates are rejected, not accepted pending.

**Dependencies:** FR-0032
**Tags:** must-have

---

## FR-0039: Fast-Path Certificate Replay Protection

**Category:** Consensus

**Statement:** The system shall bind certificate validity to `proposal_id` and `base_topic_head`, rejecting replays of old certificates against newer topic heads.

**Rationale:** Prevents replay attacks using superseded certificates. See `topic-fastpath-protocol-spec.md` Section 7 (Replay of old certificate).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 7 (Replay of old certificate)

**Acceptance Criteria:**
- [ ] Certificate validation checks that `base_topic_head` matches current topic head at proposal time.
- [ ] Replayed certificate against different head is rejected.
- [ ] Certificate includes unique `proposal_id` and expiry height.

**Dependencies:** FR-0032, FR-0008
**Tags:** must-have

---

## FR-0040: Promotion Bridge Packaging

**Category:** Consensus

**Statement:** The system shall package topic merge outputs into promotion bundles containing merge certificate and artifact provenance for optional canonical governance proposals.

**Rationale:** Enables topic outputs to be adopted globally through governance. See `topic-fastpath-protocol-spec.md` Section 4 (Architecture).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 4 (Promotion Bridge)
- `collaboration-layer-parallel-teams.md` Section 5 (Layered version control)

**Acceptance Criteria:**
- [ ] Promotion bundle includes topic merge certificate, artifact hash chain, and diff summary.
- [ ] Bundle is content-addressed and signed by the promotion bridge validators.
- [ ] Governance proposal can reference promotion bundle hash for canonical adoption.

**Dependencies:** FR-0031, FR-0021
**Tags:** should-have
