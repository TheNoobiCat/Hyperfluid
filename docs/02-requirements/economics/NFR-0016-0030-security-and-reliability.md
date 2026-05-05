## NFR-0016: Byzantine Fault Tolerance Safety

**Category:** Security

**Statement:** The system shall maintain safety (no two honest nodes commit conflicting blocks) with up to f < 33% Byzantine validators in the committee.

**Rationale:** Core property of BFT consensus. See `agx-committee-bft-and-governance.md` Section 5 (Committee BFT from day 1).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 166-167
- `decentralization-and-stack-benchmark.md` Section 5 (Production targets)

**Acceptance Criteria:**
- [ ] Safety holds under all tested adversary ratios up to 32/100 committee members.
- [ ] Equivocation is detected and slashed within 24 hours.
- [ ] No fork can persist without 33% collusion.

**Dependencies:** FR-0001
**Tags:** must-have

---

## NFR-0017: Liveness Under Partial Synchrony

**Category:** Reliability

**Statement:** The system shall maintain liveness (continuous block production) under partial synchrony and occasional partitions, assuming honest majority.

**Rationale:** Agent coordination requires reliable block production. See `topic-fastpath-protocol-spec.md` Section 3 (System Overview).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 3 (Key constraints)
- `agx-committee-bft-and-governance.md` Section 7 (Mass validator churn)

**Acceptance Criteria:**
- [ ] Block production continues with <= 3 consecutive empty slots under normal churn.
- [ ] Recovery from partition heals within 5 minutes.
- [ ] Backup proposer schedule activates when primary is unreachable.

**Dependencies:** FR-0001, FR-0004
**Tags:** must-have

---

## NFR-0018: Crash Recovery Without Data Loss

**Category:** Reliability

**Statement:** The system shall recover from node or agent crashes without data loss, using SQLite WAL for agent state and block replay for node state.

**Rationale:** Autonomous agents must resume without human intervention. See `infinite-agent.md` Section 7 (Crash Mid-Session).

**Source Research:**
- `infinite-agent.md` Section 7.3 (Crash Mid-Session)
- `automatic-vs-agent-controlled.md` Section 5 (Crash recovery semantics)

**Acceptance Criteria:**
- [ ] Agent crash: node unaffected; agent resumes from last handoff and SQLite.
- [ ] Node crash: agent stalls but retains local state; node syncs to head on restart.
- [ ] All committed state is recoverable; no unacknowledged writes lost.

**Dependencies:** FR-0061, FR-0071
**Tags:** must-have

---

## NFR-0019: Deterministic State Machine Convergence

**Category:** Reliability

**Statement:** The system shall produce identical state roots on all honest nodes given the same ordered transaction inputs.

**Rationale:** State divergence breaks consensus and light client verification. See `agx-committee-bft-and-governance.md` Section 4 (Architecture).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 4 (Execution Plane)
- `agx-committee-bft-and-governance.md` Section 5 (Governance determinism)

**Acceptance Criteria:**
- [ ] Random transaction fuzz tests produce identical state roots across 100 nodes.
- [ ] Governance merge execution uses hermetic sandbox with pinned environment.
- [ ] Non-deterministic outcomes burn deposit and are rejected.

**Dependencies:** FR-0010, FR-0022
**Tags:** must-have

---

## NFR-0020: Signed Telemetry Integrity

**Category:** Security

**Statement:** The system shall sign all telemetry with ML-DSA, bound to block height and monotonic sequence numbers, preventing replay and fabrication.

**Rationale:** Telemetry drives economic and safety decisions; must be tamper-evident. See `telemetry-threat-model.md` Section 5 (Signed telemetry schema).

**Source Research:**
- `telemetry-threat-model.md` Section 5, lines 79-89
- `telemetry-threat-model.md` Section 5 (Mitigation strategies)

**Acceptance Criteria:**
- [ ] All telemetry envelopes cryptographically signed.
- [ ] Sequence numbers strictly monotonic per producer per metric class.
- [ ] Height binding prevents pre-computation.
- [ ] Replay of old telemetry is detected and rejected.

**Dependencies:** FR-0005, FR-0060
**Tags:** must-have

---

## NFR-0021: Multi-Source Metric Corroboration

**Category:** Security

**Statement:** The system shall never act on single-source telemetry; always require minimum independent reporters and trimmed-mean aggregation.

**Rationale:** Single-source telemetry is trivially gameable. See `telemetry-threat-model.md` Section 6, Tradeoff 1.

**Source Research:**
- `telemetry-threat-model.md` Section 5 (Aggregation rules)
- `telemetry-threat-model.md` Section 5 (Mitigation strategies)

**Acceptance Criteria:**
- [ ] Minimum M = max(5, committee_size / 10) reporters.
- [ ] Median or trimmed mean used; mean never used alone.
- [ ] Outlier suppression with z-threshold.
- [ ] Independent observation via policy gateway reconciliation.

**Dependencies:** FR-0139
**Tags:** must-have

---

## NFR-0022: Equivocation Detection and Response Time

**Category:** Security

**Statement:** The system shall detect and slash equivocation within 24 hours (8,640 blocks) of the event, with automatic jail application.

**Rationale:** Slow equivocation response reduces deterrence. See `agx-committee-bft-and-governance.md` Section 5 (Penalty matrix).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 136-139

**Acceptance Criteria:**
- [ ] Equivocation evidence inclusion window: 24 hours.
- [ ] Slash and jail applied automatically on valid evidence.
- [ ] Late evidence cancels slash but marks validator for review.

**Dependencies:** FR-0014
**Tags:** must-have

---

## NFR-0023: Governance Execution Hermeticity

**Category:** Security

**Statement:** The system shall execute governance merges in hermetic sandboxes with pinned runtime hash, sealed object bundles, and normalized environment.

**Rationale:** Non-hermetic execution causes divergence. See `agx-committee-bft-and-governance.md` Section 5 (Governance determinism).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 182-184
- `decentralization-and-stack-benchmark.md` Section 7 (Governance execution split)

**Acceptance Criteria:**
- [ ] Hermetic sandbox uses pinned gix/toolchain version.
- [ ] Environment variables and locale are normalized.
- [ ] Sealed bundle includes all required git objects.
- [ ] Any deterministic mismatch rejects proposal and burns deposit.

**Dependencies:** FR-0022
**Tags:** must-have

---

## NFR-0024: Key Compromise Containment

**Category:** Security

**Statement:** The system shall allow rapid key rotation with 100-block grace window, and prevent old revoked keys from being accepted after rotation finalization.

**Rationale:** Agent keys may be compromised; rotation must be safe and fast. See `network-policy-engine-spec.md` Section 7 (Signature key rotation mismatch).

**Source Research:**
- `network-policy-engine-spec.md` Section 7 (Signature key rotation mismatch)

**Acceptance Criteria:**
- [ ] Key rotation transaction is committed before new key is active.
- [ ] Grace window of 100 blocks allows in-flight transactions.
- [ ] Old keys rejected after grace window.
- [ ] Rotation is auditable in protocol state.

**Dependencies:** FR-0118
**Tags:** must-have

---

## NFR-0025: DDoS Resilience at Network Layer

**Category:** Security

**Statement:** The system shall sustain DDoS resilience at protocol level through identity-based rate limits and fee market pricing, without relying on IP blocking.

**Rationale:** Protocol-level resilience is required for permissionless networks. See `ockam-decentralized-network-architecture.md` Section 5 (Swarm-resistant ingress controls).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 5 (Swarm-resistant ingress controls)
- `agx-committee-bft-and-governance.md` Section 5 (Swarm hardening profile)

**Acceptance Criteria:**
- [ ] Network remains operational at 10x normal transaction rate.
- [ ] Critical lanes maintain service during flood.
- [ ] IP-based blocking is local hardening only, not protocol-enforced.

**Dependencies:** FR-0043, FR-0050
**Tags:** must-have

---

## NFR-0026: Data Availability Guarantees

**Category:** Reliability

**Statement:** The system shall maintain artifact availability with minimum replica counts and repair latency targets, measured by successful challenge-response ratios.

**Rationale:** Governance and review determinism depend on artifact availability. See `artifact-availability-and-retention.md` Section 5 (Retention policy).

**Source Research:**
- `artifact-availability-and-retention.md` Section 5 (Replication leases)
- `artifact-availability-and-retention.md` Section 7 (Replica collapse)

**Acceptance Criteria:**
- [ ] Governance bundles: >= 5 replicas, repair < 1 epoch.
- [ ] Review evidence: >= 3 replicas, repair < 2 epochs.
- [ ] Challenge-response success rate > 99% for healthy artifacts.

**Dependencies:** FR-0055, FR-0057
**Tags:** must-have

---

## NFR-0027: Consensus Upgradability without Hard Fork

**Category:** Reliability

**Statement:** The system shall support consensus parameter and logic updates through on-chain `git:head` governance without requiring manual node coordination hard forks.

**Rationale:** Protocol evolution must be governable by validators. See `agx-committee-bft-and-governance.md` Section 5 (Governance determinism).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5 (Governance determinism)
- `decentralization-and-stack-benchmark.md` Section 9 (Recommended Architecture)

**Acceptance Criteria:**
- [ ] Parameter changes apply at epoch boundary after vote finalization.
- [ ] Code changes require node binary update but are triggered by on-chain signal.
- [ ] Graceful handling of nodes that have not yet updated binary.

**Dependencies:** FR-0021
**Tags:** must-have

---

## NFR-0028: Agent Runtime Isolation from Node

**Category:** Security

**Statement:** The system shall isolate agent runtime from node infrastructure such that runtime compromise cannot affect consensus safety or liveness.

**Rationale:** Agent reasoning is non-deterministic and potentially vulnerable to injection. See `automatic-vs-agent-controlled.md` Section 2 (Executive Summary).

**Source Research:**
- `automatic-vs-agent-controlled.md` Section 2
- `automatic-vs-agent-controlled.md` Section 5 (Event flow and boundary enforcement)

**Acceptance Criteria:**
- [ ] Runtime process has no write access to node database.
- [ ] Runtime cannot construct transactions bypassing policy gate.
- [ ] Node API rejects all invalid transactions regardless of runtime state.

**Dependencies:** FR-0071, FR-0138
**Tags:** must-have

---

## NFR-0029: Backup and Restore for Agent State

**Category:** Reliability

**Statement:** The system shall support backup and restore of agent SQLite state with checksum verification, enabling migration and disaster recovery.

**Rationale:** Operators need reliable recovery options. See `infinite-agent.md` Section 4.3 (Database Schema).

**Source Research:**
- `infinite-agent.md` Section 4.3 (Database Schema)

**Acceptance Criteria:**
- [ ] SQLite database can be copied while WAL is active.
- [ ] Backup includes integrity checksum.
- [ ] Restore validates checksum before use.
- [ ] Restore resumes from last handoff without loss.

**Dependencies:** FR-0061
**Tags:** nice-to-have

---

## NFR-0030: Formal Verification Targets

**Category:** Security

**Statement:** The system shall define formal verification targets for committee sampling, liveness transitions, policy invariants, and fast-path state machine safety.

**Rationale:** Formal verification provides strongest safety guarantees. See `agx-committee-bft-and-governance.md` Section 11 (Future Improvements).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 11 (Future Improvements)
- `topic-fastpath-protocol-spec.md` Section 11 (Future Improvements)
- `network-policy-engine-spec.md` Section 11 (Future Improvements)

**Acceptance Criteria:**
- [ ] Formal specification exists for committee sampling logic.
- [ ] Liveness transition safety properties are stated.
- [ ] Policy invariant (no bypass) is formally specified.
- [ ] Verification is deferred to Phase 5 but specifications are frozen now.

**Dependencies:** none
**Tags:** nice-to-have
