## FR-0136: Network-Only Policy Scope

**Category:** Security

**Statement:** The system shall limit the policy engine to network-mutating actions only; local machine operations remain operator-controlled and out of protocol scope.

**Rationale:** Respects operator autonomy and keeps protocol scope minimal. See `network-policy-engine-spec.md` Section 6, Tradeoff 2.

**Source Research:**
- `network-policy-engine-spec.md` Section 6, Tradeoff 2
- `prompt-injection-and-network-policy-boundary.md` Section 3 (System Overview)

**Acceptance Criteria:**
- [ ] Policy gate rejects network actions without valid plans.
- [ ] Local `bash` commands are not subject to network policy gate.
- [ ] Local resource limits are enforced by operator sandbox (cgroup), not protocol.

**Dependencies:** FR-0106
**Tags:** must-have

---

## FR-0137: Sandbox Escape Prevention

**Category:** Security

**Statement:** The system shall prevent agent runtime sandbox escape through resource limits, restricted syscalls, and isolated filesystem access.

**Rationale:** Agents execute untrusted code; sandbox containment is critical. See `PROJECT-STATUS.md` (Research Gaps: Sandbox escape analysis).

**Source Research:**
- `infinite-agent.md` Section 4.0 (Resource Limits)
- `prompt-injection-and-network-policy-boundary.md` Section 5 (Minimal network-only policy engine)
- `PROJECT-STATUS.md` (Gap: Sandbox escape analysis)

**Acceptance Criteria:**
- [ ] Agent runtime executes in restricted sandbox (e.g., seccomp, namespace isolation).
- [ ] Filesystem access limited to designated working directory.
- [ ] Network sockets are mediated by node API, not direct from sandbox.
- [ ] Escape attempts trigger runtime termination and evidence logging.

**Dependencies:** FR-0066
**Tags:** must-have

---

## FR-0138: Agent Runtime-NODE Process Separation

**Category:** Security

**Statement:** The system shall separate agent runtime and node infrastructure into distinct processes with defined API boundary.

**Rationale:** Allows independent scaling, language choice, and failure isolation. See `automatic-vs-agent-controlled.md` Section 4 (Architecture).

**Source Research:**
- `automatic-vs-agent-controlled.md` Section 4 (Architecture)
- `automatic-vs-agent-controlled.md` Section 6, Tradeoff 4

**Acceptance Criteria:**
- [ ] Runtime and node are separate OS processes.
- [ ] Communication is via typed HTTP/gRPC API.
- [ ] Node crash does not corrupt agent SQLite state.
- [ ] Agent crash does not affect consensus or networking.

**Dependencies:** FR-0071
**Tags:** must-have

---

## FR-0139: Telemetry Aggregation Robustness

**Category:** Security

**Statement:** The system shall aggregate telemetry using trimmed-mean or median requiring minimum independent reporters, rejecting outlier submissions.

**Rationale:** Single-source telemetry is trivially gameable in a decentralized network. See `telemetry-threat-model.md` Section 6, Tradeoff 1.

**Source Research:**
- `telemetry-threat-model.md` Section 5 (Aggregation rules)
- `telemetry-threat-model.md` Section 5 (Anomaly detection)

**Acceptance Criteria:**
- [ ] Minimum M = max(5, committee_size / 10) reporters per metric class.
- [ ] Trimmed mean discards top/bottom 10% before aggregation.
- [ ] Outliers beyond z-threshold are flagged and receive lower weight.
- [ ] Suppression detection flags producers that stop reporting during incidents.

**Dependencies:** FR-0060
**Tags:** must-have

---

## FR-0140: Temporal Binding of Telemetry

**Category:** Security

**Statement:** The system shall bind telemetry metrics to specific block heights with sequence numbers to prevent replay and pre-computation attacks.

**Rationale:** Prevents retroactive metric fabrication. See `telemetry-threat-model.md` Section 5 (Signed telemetry schema).

**Source Research:**
- `telemetry-threat-model.md` Section 5, lines 79-89

**Acceptance Criteria:**
- [ ] Telemetry envelope includes height and monotonic seq_no.
- [ ] Validation rejects envelopes with seq_no <= last_seq_no for producer+metric_class.
- [ ] Validation rejects envelopes with height outside current_height ± 10.

**Dependencies:** FR-0060
**Tags:** must-have

---

## FR-0141: Independent Policy Gateway Reconciliation

**Category:** Security

**Statement:** The system shall reconcile aggregated telemetry against independently observable network events (block headers, tx receipts) to detect metric spoofing.

**Rationale:** Aggregated metrics can still be skewed by coordinated spoofing. See `telemetry-threat-model.md` Section 5 (Mitigation strategies).

**Source Research:**
- `telemetry-threat-model.md` Section 5 (Independent observation)
- `telemetry-threat-model.md` Section 5, lines 150-156 (Pseudocode)

**Acceptance Criteria:**
- [ ] Finality lag telemetry is cross-checked against actual block header timestamps.
- [ ] Reject ratio is cross-checked against actual mempool admission logs.
- [ ] Discrepancies beyond tolerance trigger reconciliation failure flag.

**Dependencies:** FR-0139
**Tags:** must-have

---

## FR-0142: Incident State Machine with Signed Evidence (SUPERSEDED)

> **Superseded by ADR-0012.** Emergency mode and circuit-breaker hierarchy were removed. The EIP-1559 base fee mechanism is the sole congestion mechanism. See `incident-response-spec.md` §1 and FR-0145 for the current approach.

**Category:** Security

**Statement:** The system shall implement a binary incident mode (Normal / Emergency) with deterministic triggers, signed evidence quorum, and explicit exit criteria.

**Rationale:** Removes central bottleneck from incident declaration. See `decentralized-incident-response-and-recovery.md` Section 5 (Core Mechanisms).

**Source Research:**
- `decentralized-incident-response-and-recovery.md` Section 5 (Trigger logic)
- `decentralized-incident-response-and-recovery.md` Section 6, Tradeoff 1

**Acceptance Criteria:**
- (superseded — no implementation required)

**Dependencies:** FR-0060
**Tags:** superseded

---

## FR-0143: Emergency Mode Parameter Overrides (SUPERSEDED)

> **Superseded by ADR-0012.** Emergency mode and circuit-breaker hierarchy were removed. The EIP-1559 base fee mechanism is the sole congestion mechanism. See `incident-response-spec.md` §1 and FR-0145 for the current approach.

**Category:** Security

**Statement:** The system shall apply deterministic parameter overrides in Emergency mode: 3x PoW difficulty, 50% unknown-sender budgets, reserved control lanes, frozen low-trust fast-path, emergency fee floor.

**Rationale:** Automatic defense escalation preserves safety-critical operations. See `decentralized-incident-response-and-recovery.md` Section 5 (Emergency mode).

**Source Research:**
- `decentralized-incident-response-and-recovery.md` Section 5, lines 96-103

**Acceptance Criteria:**
- (superseded — no implementation required)

**Dependencies:** FR-0060
**Tags:** should-have

---

## FR-0144: False-Alarm Reporter Penalties

**Category:** Security

**Statement:** The system shall penalize validators that submit fabricated incident evidence after adjudication.

**Rationale:** Prevents malicious false-alarm campaigns. See `decentralized-incident-response-and-recovery.md` Section 5 (Abuse resistance).

**Source Research:**
- `decentralized-incident-response-and-recovery.md` Section 5, lines 119-121
- `decentralized-incident-response-and-recovery.md` Section 7 (Malicious false-alarm campaign)

**Acceptance Criteria:**
- [ ] False incident evidence is detected by post-incident review.
    - [ ] Penalty includes trust regression and temporary reporting restriction.
- [ ] Penalty severity scales with intent and frequency.

**Dependencies:** FR-0142
**Tags:** should-have

---

## FR-0145: Fee Market Congestion Recovery

**Category:** Security

**Statement:** The system shall rely on the EIP-1559 base fee mechanism for congestion recovery. Staged ramp-up and post-incident quotas are local operator concerns, not protocol-enforced. The base fee automatically decreases as demand subsides.

**Source Research:**
- `decentralized-incident-response-and-recovery.md` Section 7 (Recovery traffic surge)

**Acceptance Criteria:**
- [ ] Base fee decreases when congestion subsides per EIP-1559 formula.
- [ ] Deferred low-priority operations are replayed safely.

**Dependencies:** FR-0146
**Tags:** should-have
