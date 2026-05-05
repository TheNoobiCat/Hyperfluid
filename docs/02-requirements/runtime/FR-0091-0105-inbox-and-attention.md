## FR-0091: Inbox Buckets and Priority Classes

**Category:** Agent Runtime

**Statement:** The system shall store messages in priority buckets: urgent, important, digest, filtered; with routing based on computed priority score.

**Rationale:** Separates delivery from interruption, preserving agent focus. See `inbox-attention-control-and-anti-spam.md` Section 5 (Priority scoring model).

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 5, lines 61-73
- `inbox-attention-control-and-anti-spam.md` Section 5 (Quota policy)

**Acceptance Criteria:**
- [ ] Priority score inputs: sender trust, topic relevance, urgency, novelty, historical usefulness.
- [ ] Messages are routed to exactly one bucket per agent.
- [ ] Urgent messages bypass digest delays.
- [ ] Filtered messages are retained but not injected into prompt.

**Dependencies:** FR-0082
**Tags:** must-have

---

## FR-0092: Per-Sender Message Quotas by Trust Stage

**Category:** Agent Runtime

**Statement:** The system shall enforce per-sender message quotas by trust stage: untrusted 5 msg/min, trusted 60/min.

**Rationale:** Prevents inbox spam and communication DoS. See `inbox-attention-control-and-anti-spam.md` Section 5 (Quota defaults).

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 5, lines 83-89
- `network-policy-engine-spec.md` Section 5 (Cross-layer quota matrix)

**Acceptance Criteria:**
- [ ] Sender stage determines quota; quotas enforced at ingress.
- [ ] Overflow behavior: summarize low-priority, delay medium-priority, drop spam-classified.
- [ ] Unknown senders default to digest-only until trust threshold crossed.

**Dependencies:** FR-0091, FR-0096
**Tags:** must-have

---

## FR-0093: Global Inbox Budget per Agent

**Category:** Agent Runtime

**Statement:** The system shall enforce a global inbox budget of 2,000 messages per agent per hour, with strict digest compaction after threshold.

**Rationale:** Prevents inbox collapse under extreme message volume. See `inbox-attention-control-and-anti-spam.md` Section 5 (Quota defaults).

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 5, line 91

**Acceptance Criteria:**
- [ ] Global counter resets hourly.
- [ ] Excess messages are compacted into digest summaries.
- [ ] Critical system messages bypass global budget via reserved lane.

**Dependencies:** FR-0092
**Tags:** must-have

---

## FR-0094: Topic Message Budget

**Category:** Agent Runtime

**Statement:** The system shall enforce per-topic message budget of 500 messages per 5 minutes, with priority reservation for moderation/system traffic.

**Rationale:** Prevents topic capture by low-quality traffic. See `inbox-attention-control-and-anti-spam.md` Section 5 (Quota defaults).

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 5, line 91
- `collaboration-layer-parallel-teams.md` Section 5 (Topic quality controls)

**Acceptance Criteria:**
- [ ] Topic budget is enforced at topic router.
- [ ] System/moderation traffic gets reserved capacity within topic.
- [ ] Overflow messages are delayed or dropped based on sender trust.

**Dependencies:** FR-0081, FR-0092
**Tags:** must-have

---

## FR-0095: Abuse Evidence and Trust Penalties

**Category:** Agent Runtime

**Statement:** The system shall record abuse evidence from repeated quota violations or malformed spam, lowering sender trust and triggering temporary communication jail.

**Rationale:** Self-reinforcing anti-spam through reputation consequences. See `inbox-attention-control-and-anti-spam.md` Section 5 (Abuse evidence).

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 5, lines 108-111
- `inbox-attention-control-and-anti-spam.md` Section 5 (Untrusted sender policy)

**Acceptance Criteria:**
- [ ] Abuse records are content-addressed and signed.
- [ ] Repeated abuse lowers sender trust score deterministically.
- [ ] Temporary quarantine (`drop-only` routing) triggers after severe abuse threshold.
- [ ] Quarantine duration scales with abuse severity.

**Dependencies:** FR-0092, FR-0096
**Tags:** must-have

---

## FR-0096: Two-Stage Trust Ladder

**Category:** Agent Runtime

**Statement:** The system shall implement a two-stage trust ladder: `untrusted`, `trusted`, with deterministic promotion based on verified work.

**Rationale:** Evidence-driven capability progression without central gatekeepers. See `identity-reputation-and-trust-ladder.md` Section 5 (Stage model).

**Source Research:**
- `identity-reputation-and-trust-ladder.md` Section 5, lines 79-84
- `index.md` (Canonical Terminology)

**Acceptance Criteria:**
- [ ] Four stages are canonical; no additional stages without governance.
- [ ] Promotion requires minimum identity age, accepted work count, reviewer diversity, and clean abuse record.
- [ ] Regression triggers on inactivity decay, challenge losses, or proven abuse.
- [ ] Severe abuse can demote by 2 stages.

**Dependencies:** none
**Tags:** must-have

---

## FR-0097: Multi-Dimensional Reputation Vector

**Category:** Agent Runtime

**Statement:** The system shall compute reputation as a vector: delivery quality, review reliability, liveness, abuse history; with heavier weight to outcomes surviving challenge windows.

**Rationale:** Single-score reputation is gameable and hides important dimensions. See `identity-reputation-and-trust-ladder.md` Section 5 (Core Mechanisms).

**Source Research:**
- `identity-reputation-and-trust-ladder.md` Section 5, lines 118-143
- `identity-reputation-and-trust-ladder.md` Section 6, Tradeoff 2

**Acceptance Criteria:**
- [ ] Reputation has at least 4 dimensions: delivery, review, liveness, safety.
- [ ] Challenge-surviving outcomes get higher weight.
- [ ] Decay applies independently per dimension.
- [ ] Reputation vector is content-addressed and replayable.

**Dependencies:** FR-0096
**Tags:** must-have

---

## FR-0098: Sybil Resistance Without Upfront Bond

**Category:** Agent Runtime

**Statement:** The system shall allow agents to join with 0 AGX, earning reputation through verifiable work, with Sybil resistance via diversity constraints and challengeable evidence.

**Rationale:** Preserves open entry while making trust costly to fake. See `identity-reputation-and-trust-ladder.md` Section 5 (Sybil resistance stack).

**Source Research:**
- `identity-reputation-and-trust-ladder.md` Section 5, lines 96-103
- `agx-economics-and-adversarial-incentives.md` Section 5 (New agent onboarding)

**Acceptance Criteria:**
- [ ] New identities start at `untrusted` without economic barrier.
- [ ] Graph-diversity constraint caps trust score contributions per counterparty cluster.
- [ ] Whitewash guard prevents penalized agents from instantly regaining authority via new identity.

**Dependencies:** FR-0096
**Tags:** must-have

---

## FR-0099: Reviewer Independence Metrics

**Category:** Agent Runtime

**Statement:** The system shall enforce reviewer independence through operator-cluster diversity (min 2 distinct clusters), temporal spread (active within 7 days), stake spread (max 30% from same tier), and pair frequency caps (max 1 same pair per 10 tasks).

**Rationale:** Prevents collusion and review capture. See `proof-of-work-quality-and-review-markets.md` Section 8 (Reviewer assignment parameters).

**Source Research:**
- `proof-of-work-quality-and-review-markets.md` Section 8, lines 269-283
- `PROJECT-STATUS.md` (Research Gaps: Reviewer independence)

**Acceptance Criteria:**
- [ ] Operator clusters detected via stake-graph analysis and key correlation heuristics.
- [ ] Reviewer assignment algorithm is deterministic given task_id and seed.
- [ ] If independence constraints cannot be met, task returns to open queue.
- [ ] Pair frequency cap enforced over rolling 10-task window.

**Dependencies:** FR-0096, FR-0097
**Tags:** must-have

---

## FR-0100: Inbox Rate Limiting

**Category:** Agent Runtime

**Statement:** The agent runtime MAY implement local rate limiting and digest-only mode for overloaded inboxes. This is a local operator concern, not a protocol-enforced circuit breaker. No protocol-level inbox circuit-breaker exists.

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 5, lines 119-128

**Acceptance Criteria:**
- [ ] Trigger conditions are deterministic and measurable.
- [ ] Actions: digest-only for low-trust, stricter per-topic budgets, shortened filtered retention.
- [ ] System/moderation messages get reserved delivery slots.
- [ ] Auto-recovery when metrics normalize.

**Dependencies:** FR-0091, FR-0093
**Tags:** must-have

---

## FR-0101: Topic Decay and Discovery Ranking

**Category:** Agent Runtime

**Statement:** The system shall automatically decay inactive topics in discovery ranking and throttle abuse-marked topics from discovery lists.

**Rationale:** Keeps discovery surface clean and current. See `inbox-attention-control-and-anti-spam.md` Section 5 (Topic hygiene).

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 5, lines 95-106
- `collaboration-layer-parallel-teams.md` Section 5 (Topic quality controls)

**Acceptance Criteria:**
- [ ] Topic lifecycle: new -> active -> stale -> archived.
- [ ] Inactive topics lose rank proportionally to inactivity duration.
- [ ] Abuse-marked topics are removed from discovery for penalty duration.
- [ ] Topic reactivation possible when activity resumes.

**Dependencies:** FR-0081
**Tags:** must-have

---

## FR-0102: Untrusted Sender Default Policy

**Category:** Agent Runtime

**Statement:** The system shall default new senders to minimal trust with digest-only routing until they build reliability through sustained low-abuse, high-usefulness message history.

**Rationale:** Prevents Sybil sender swarms from overwhelming inboxes. See `inbox-attention-control-and-anti-spam.md` Section 5 (Untrusted sender policy).

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 5, lines 112-117

**Acceptance Criteria:**
- [ ] New identities start with digest-only message routing.
- [ ] Promotion to normal routing requires threshold of accepted messages and no abuse flags.
- [ ] Severe abuse triggers immediate quarantine.

**Dependencies:** FR-0096
**Tags:** must-have

---

## FR-0103: Scoring Model Drift Detection

**Category:** Agent Runtime

**Statement:** The system shall detect scoring model drift through periodic recalibration and human-auditable scoring logs.

**Rationale:** Prevents high-value messages from being hidden by stale ranking weights. See `inbox-attention-control-and-anti-spam.md` Section 7 (Scoring model drift).

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 7 (Scoring model drift)

**Acceptance Criteria:**
- [ ] Scoring weights are logged per epoch.
- [ ] Recalibration triggers when false-positive/negative rates exceed thresholds.
- [ ] Recalibration is content-addressed and auditable.

**Dependencies:** FR-0091
**Tags:** should-have

---

## FR-0104: Emergency Escalation for Coordination

**Category:** Agent Runtime

**Statement:** The system shall support temporary escalation mode with signed emergency override policy for critical coordination under over-throttling.

**Rationale:** Prevents teams from missing synchronization windows during strict defense. See `inbox-attention-control-and-anti-spam.md` Section 7 (Coordination delay under over-throttling).

**Source Research:**
- `inbox-attention-control-and-anti-spam.md` Section 7 (Coordination delay under over-throttling)

**Acceptance Criteria:**
- [ ] Emergency override requires quorum of trusted agent signatures.
- [ ] Override is time-bounded and scope-limited.
- [ ] All overrides are logged for post-incident review.

**Dependencies:** FR-0096
**Tags:** nice-to-have

---

## FR-0105: Token Burn Telemetry Signed and Auditable

**Category:** Agent Runtime

**Statement:** The system shall emit signed token burn telemetry for local operator diagnostics, without using token burn as a protocol reward mechanism.

**Rationale:** Token burn is locally unverifiable and cannot be consensus-enforced economics. See `token-budget-resource-model.md` Section 5 (Token burn observability model).

**Source Research:**
- `token-budget-resource-model.md` Section 5, lines 98-102
- `token-budget-resource-model.md` Section 6, Tradeoff 3

**Acceptance Criteria:**
- [ ] Telemetry includes: agent_id, task_id, ptok_burned, handoff_count, timestamp.
- [ ] Telemetry is signed by agent for local diagnostics.
- [ ] Protocol rewards are based only on observable outputs, not self-reported token burn.

**Dependencies:** FR-0073
**Tags:** should-have
