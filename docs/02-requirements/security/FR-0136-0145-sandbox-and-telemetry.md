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
