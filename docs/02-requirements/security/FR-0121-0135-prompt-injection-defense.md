## FR-0121: Prompt Injection as Network Boundary Problem

**Category:** Security

**Statement:** The system shall treat prompt injection as a network boundary security problem, not a pure prompting problem, with external text untrusted by default.

**Rationale:** Relies on deterministic policy checks rather than model instruction-following for safety. See `prompt-injection-and-network-policy-boundary.md` Section 2 (Executive Summary).

**Source Research:**
- `prompt-injection-and-network-policy-boundary.md` Section 2
- `prompt-injection-and-network-policy-boundary.md` Section 3 (System Overview)

**Acceptance Criteria:**
- [ ] All inbound payloads (DM/topic/doc/web/code) are marked untrusted at ingress.
- [ ] Untrusted text cannot directly trigger network-mutating tools.
- [ ] Safety-critical controls are model-agnostic and auditable.

**Dependencies:** FR-0106
**Tags:** must-have

---

## FR-0122: Classifier as Auxiliary Signal Only

**Category:** Security

**Statement:** The system shall use classifiers (e.g., ModernBERT-style) only as auxiliary signals for scoring and quarantine, not as root authorization truth.

**Rationale:** Classifiers are probabilistic and bypassable; deterministic policy must remain root guard. See `prompt-injection-and-network-policy-boundary.md` Section 6, Tradeoff 3.

**Source Research:**
- `prompt-injection-and-network-policy-boundary.md` Section 5 (Abuse/Anomaly Signals)
- `prompt-injection-and-network-policy-boundary.md` Section 6, Tradeoff 3

**Acceptance Criteria:**
- [ ] Classifier output cannot directly authorize or deny execution.
- [ ] Classifier signals can tighten quotas or trigger quarantine.
- [ ] Authorization truth is always deterministic policy evaluation.

**Dependencies:** FR-0106
**Tags:** must-have

---

## FR-0123: Attack Corpus Registry

**Category:** Security

**Statement:** The system shall maintain a versioned attack corpus registry with deterministic scenario schemas, expected policy outcomes, and severity weights.

**Rationale:** Enables repeatable, measurable injection resilience testing. See `prompt-injection-redteam-and-evals.md` Section 4 (Architecture).

**Source Research:**
- `prompt-injection-redteam-and-evals.md` Section 5 (Attack taxonomy)
- `prompt-injection-redteam-and-evals.md` Section 5 (Scenario schema)

**Acceptance Criteria:**
- [ ] Corpus is pinned by hash for reproducible runs.
- [ ] Scenario schema: scenario_id, corpus_version_hash, ingress_channel, initial_state_hash, interaction_script, expected_outcome, expected_reason_code, severity_weight.
- [ ] Expected outcomes are deterministic: deny, allow_low_risk, allow_with_step_up.

**Dependencies:** FR-0121
**Tags:** must-have

---

## FR-0124: Scenario Runner with Deterministic Seeds

**Category:** Security

**Statement:** The system shall replay attack scenarios with deterministic seeds and state hash pinning, producing reproducible traces.

**Rationale:** Non-deterministic testing cannot catch regressions reliably. See `prompt-injection-redteam-and-evals.md` Section 4 (Scenario Runner).

**Source Research:**
- `prompt-injection-redteam-and-evals.md` Section 5 (Pseudocode)
- `prompt-injection-redteam-and-evals.md` Section 7 (Non-deterministic scenario replay)

**Acceptance Criteria:**
- [ ] Scenario runner uses deterministic random seeds derived from initial_state_hash.
- [ ] Same scenario + same runtime build produces identical trace.
- [ ] Traces include policy decisions, tool calls, and outcome classification.

**Dependencies:** FR-0123
**Tags:** must-have

---

## FR-0125: Dual Metric Release Gating

**Category:** Security

**Statement:** The system shall gate runtime promotion on both safety metrics (ASR, UER) and productivity metrics (FPR-deny, P99 latency, task throughput delta).

**Rationale:** Avoids "safe but useless" overblocking. See `prompt-injection-redteam-and-evals.md` Section 5 (Release gating rules).

**Source Research:**
- `prompt-injection-redteam-and-evals.md` Section 5, lines 92-106
- `prompt-injection-redteam-and-evals.md` Section 6, Tradeoff 2

**Acceptance Criteria:**
- [ ] Block release if ASR > threshold_asr or UER > threshold_uer.
- [ ] Block release if FPR-deny exceeds collaboration ceiling.
- [ ] Block release if P99_policy_latency exceeds SLO.
- [ ] Severity-weighted failures can hard-fail even if aggregate passes.

**Dependencies:** FR-0124
**Tags:** must-have

---

## FR-0126: Continuous Canary Drift Detection

**Category:** Security

**Statement:** The system shall run canary scenario subset periodically in production-like environments, alerting when rolling ASR/UER exceeds baseline drift budget.

**Rationale:** Catches environment/model drift and evolving attack patterns. See `prompt-injection-redteam-and-evals.md` Section 5 (Continuous drift detection).

**Source Research:**
- `prompt-injection-redteam-and-evals.md` Section 5, lines 109-112
- `prompt-injection-redteam-and-evals.md` Section 6, Tradeoff 4

**Acceptance Criteria:**
- [ ] Canary runs at regular intervals with fixed scenario subset.
- [ ] Drift budget is defined per metric.
- [ ] Alert triggers auto-regression investigation bundle.
- [ ] Canary results are archived for trend analysis.

**Dependencies:** FR-0125
**Tags:** should-have

---

## FR-0127: Incident-to-Corpus Feedback Loop

**Category:** Security

**Statement:** The system shall require discovered attack vectors from incidents to be added to the attack corpus within defined time windows.

**Rationale:** Prevents corpus blind spots from persisting. See `prompt-injection-redteam-and-evals.md` Section 7 (Corpus blind spot).

**Source Research:**
- `prompt-injection-redteam-and-evals.md` Section 7 (Corpus blind spot)

**Acceptance Criteria:**
- [ ] Security incidents trigger mandatory corpus review.
- [ ] New attack families are added to corpus with target SLA.
- [ ] Corpus version is bumped when new scenarios are added.

**Dependencies:** FR-0123
**Tags:** should-have

---

## FR-0128: Hidden Scenario Pools for Metric Gaming Prevention

**Category:** Security

**Statement:** The system shall maintain rotating hidden scenario subsets to prevent runtimes from overfitting to benchmark quirks.

**Rationale:** Static metrics incentivize gaming without real resilience. See `prompt-injection-redteam-and-evals.md` Section 7 (Metric gaming).

**Source Research:**
- `prompt-injection-redteam-and-evals.md` Section 7 (Metric gaming)

**Acceptance Criteria:**
- [ ] Hidden pool is not visible to runtime developers.
- [ ] Hidden scenarios are included randomly in evaluation runs.
- [ ] Hidden pool rotates on a schedule.

**Dependencies:** FR-0125
**Tags:** should-have

---

## FR-0129: Signed Telemetry Envelopes for Eval Integrity

**Category:** Security

**Statement:** The system shall sign evaluation telemetry with ML-DSA to prevent compromised runtimes from under-reporting unsafe executions.

**Rationale:** Local log manipulation can hide security failures. See `prompt-injection-redteam-and-evals.md` Section 7 (Telemetry tampering).

**Source Research:**
- `prompt-injection-redteam-and-evals.md` Section 7 (Telemetry tampering)
- `telemetry-threat-model.md` Section 5 (Signed telemetry schema)

**Acceptance Criteria:**
- [ ] Telemetry envelopes include producer_id, metric, value, height, seq_no, signature.
- [ ] Sequence numbers prevent replay attacks.
- [ ] Independent policy gateway reconciles observed events against telemetry.

**Dependencies:** FR-0005
**Tags:** must-have

---

## FR-0130: Staged Rollout with Rollback Guardrails

**Category:** Security

**Statement:** The system shall support staged policy bundle rollout with automatic rollback if canary metrics breach thresholds.

**Rationale:** Prevents overblocking releases from degrading collaboration globally. See `prompt-injection-redteam-and-evals.md` Section 7 (Overblocking release).

**Source Research:**
- `prompt-injection-redteam-and-evals.md` Section 7 (Overblocking release)

**Acceptance Criteria:**
- [ ] Rollout deploys to canary subset first.
- [ ] Automatic rollback triggers on safety/productivity metric breach.
- [ ] Rollback is deterministic and does not require human intervention.

**Dependencies:** FR-0126
**Tags:** should-have
