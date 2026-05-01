## NFR-0001: Consensus Throughput Targets

**Category:** Performance

**Statement:** The system shall sustain 100 tx/s with burst capacity of 500 tx/s, block time 10 seconds, and single-block finality.

**Rationale:** Production target for agent-native workload profiles. See `decentralization-and-stack-benchmark.md` Section 5 (Production targets).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 5, lines 66-70

**Acceptance Criteria:**
- [ ] Sustained throughput >= 100 tx/s under normal load.
- [ ] Burst throughput >= 500 tx/s with mempool buffering.
- [ ] Block time p50 = 10 seconds, p95 < 15 seconds.
- [ ] Finality latency p95 < 3 seconds for committee size 100.

**Dependencies:** FR-0001, FR-0009
**Tags:** must-have

---

## NFR-0002: State Size Growth Bound

**Category:** Performance

**Statement:** The system shall bound state size growth to less than 1GB per month with pruning, for SMT and other protocol state.

**Rationale:** Keeps node hardware requirements accessible for broad participation. See `decentralization-and-stack-benchmark.md` Section 5 (Production targets).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 5, line 71

**Acceptance Criteria:**
- [ ] Monthly state growth measured and reported per epoch.
- [ ] Pruning policy removes old telemetry and expired artifacts.
- [ ] Archive nodes may retain full history; validating nodes do not require it.

**Dependencies:** FR-0010
**Tags:** must-have

---

## NFR-0003: Policy Decision Latency

**Category:** Performance

**Statement:** The system shall evaluate policy decisions in O(1)-ish time per call, with P99 latency under 100ms for low-risk actions.

**Rationale:** High agent counts require fast policy gate throughput. See `network-policy-engine-spec.md` Section 8 (Scalability).

**Source Research:**
- `network-policy-engine-spec.md` Section 8, lines 274-288

**Acceptance Criteria:**
- [ ] Low-risk plan evaluation P99 < 100ms.
- [ ] Medium-risk plan evaluation P99 < 500ms (includes attestation fetch).
- [ ] High-risk plan evaluation P99 < 2s (includes quorum verification).
- [ ] Policy checks do not require graph scans or unbounded queries.

**Dependencies:** FR-0106
**Tags:** must-have

---

## NFR-0004: Gossip Convergence Time

**Category:** Performance

**Statement:** The system shall achieve gossip convergence for freshness-critical events within 30 seconds for 99% of nodes under normal conditions.

**Rationale:** Agent coordination depends on timely event propagation. See `ockam-decentralized-network-architecture.md` Section 8 (Scalability).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 8 (Medium scale)
- `ockam-decentralized-network-architecture.md` Section 10 (Testing strategy)

**Acceptance Criteria:**
- [ ] Freshness-critical event reaches 99% of nodes within 30 seconds at 1k-10k scale.
- [ ] Convergence measured via synthetic heartbeat events.
- [ ] Partition healing convergence time bounded to 5 minutes.

**Dependencies:** FR-0042
**Tags:** should-have

---

## NFR-0005: Artifact Retrieval Latency

**Category:** Performance

**Statement:** The system shall retrieve governance-sized artifacts (<10MB) within 30 seconds from first provider response under normal network conditions.

**Rationale:** Governance voting cannot stall on artifact fetch. See `artifact-availability-and-retention.md` Section 7 (Governance bundle unavailable).

**Source Research:**
- `artifact-availability-and-retention.md` Section 7 (Governance bundle unavailable)

**Acceptance Criteria:**
- [ ] Parallel fetch from min_replica_count + 2 providers.
- [ ] P95 retrieval time < 30s for artifacts < 10MB.
- [ ] P99 retrieval time < 60s.
- [ ] Timeout triggers AtRisk state and repair.

**Dependencies:** FR-0053
**Tags:** must-have

---

## NFR-0006: Agent Context Window Assembly Latency

**Category:** Performance

**Statement:** The system shall assemble agent context prompt within 500ms, including priority sorting, truncation, and hash verification.

**Rationale:** Agent loop iteration time is dominated by LLM inference; context assembly must not add significant overhead. See `token-efficiency-under-high-interaction.md` Section 8 (Scalability).

**Source Research:**
- `token-efficiency-under-high-interaction.md` Section 8 (Medium scale)
- `infinite-agent.md` Section 4.1 (Runtime Loop)

**Acceptance Criteria:**
- [ ] Context assembly P99 < 500ms.
- [ ] Includes inbox priority scoring, reference fetch, and block allocation.
- [ ] Hash verification for fetched references is asynchronous where possible.

**Dependencies:** FR-0063
**Tags:** should-have

---

## NFR-0007: Review Sandbox Startup Latency

**Category:** Performance

**Statement:** The system shall start a review sandbox within 2 seconds, including context isolation and tool injection.

**Rationale:** Review latency impacts overall task throughput. See `topic-fastpath-protocol-spec.md` Section 8 (Scalability).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 8 (Medium scale)
- `agx-committee-bft-and-governance.md` Section 5 (Review sandbox)

**Acceptance Criteria:**
- [ ] Sandbox creation P99 < 2s.
- [ ] Context isolation enforced within startup time.
- [ ] Tool injection limited to single review tool.

**Dependencies:** FR-0087
**Tags:** should-have

---

## NFR-0008: Sustained Adversarial Load

**Category:** Performance

**Statement:** The system shall maintain >80% of baseline throughput under 10x malicious sender ratio.

**Rationale:** Swarm hardening must preserve liveness under attack. See `decentralization-and-stack-benchmark.md` Section 10 (Implementation Plan).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 10, line 247
- `agx-economics-and-adversarial-incentives.md` Section 10 (Implementation Plan)

**Acceptance Criteria:**
- [ ] Baseline throughput measured under honest-only load.
- [ ] 10x malicious sender ratio test shows >= 80% baseline throughput.
- [ ] Critical lanes (evidence, governance, control) maintain 100% throughput.

**Dependencies:** FR-0142
**Tags:** must-have

---

## NFR-0009: Node Startup and Sync Time

**Category:** Performance

**Statement:** The system shall allow a new validator node to sync from genesis to head within 24 hours for chains up to 6 months old.

**Rationale:** Low sync time encourages participation and fault recovery. See `decentralization-and-stack-benchmark.md` Section 5 (Production targets).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 5 (Production targets)

**Acceptance Criteria:**
- [ ] Sync time measured for 6-month chain history.
- [ ] Snap sync support for catching up from recent checkpoint.
- [ ] Full validation of all historical blocks remains possible.

**Dependencies:** FR-0001
**Tags:** should-have

---

## NFR-0010: Memory Footprint Bound

**Category:** Performance

**Statement:** The system shall keep per-node memory footprint under 8GB for validating nodes and 16GB for archive nodes under normal load.

**Rationale:** Accessible hardware requirements support decentralization. See `decentralization-and-stack-benchmark.md` Section 5 (Production targets).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 5 (Production targets)
- `infinite-agent.md` Section 4.0 (Resource Limits)

**Acceptance Criteria:**
- [ ] Validating node RSS <= 8GB at 100 tx/s sustained.
- [ ] Archive node RSS <= 16GB.
- [ ] Memory growth is bounded; no unbounded caches.

**Dependencies:** NFR-0002
**Tags:** should-have

---

## NFR-0011: Network Bandwidth Efficiency

**Category:** Performance

**Statement:** The system shall keep median per-node egress bandwidth under 10 Mbps for validating nodes at target throughput.

**Rationale:** Bandwidth constraints are common for home/edge operators. See `ockam-decentralized-network-architecture.md` Section 8 (Scalability).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 8 (Large scale)

**Acceptance Criteria:**
- [ ] Median egress <= 10 Mbps at 100 tx/s sustained.
- [ ] P99 egress <= 50 Mbps during churn bursts.
- [ ] Bloom filter gossip reduces duplicate traffic.

**Dependencies:** FR-0047
**Tags:** should-have

---

## NFR-0012: Database Query Latency for Agent Runtime

**Category:** Performance

**Statement:** The system shall serve agent runtime SQLite queries within 10ms for common operations (todo read/write, knowledge lookup, handoff fetch).

**Rationale:** Agent loop requires fast local state access. See `infinite-agent.md` Section 4.3 (Database Schema).

**Source Research:**
- `infinite-agent.md` Section 4.3 (Database Schema)

**Acceptance Criteria:**
- [ ] Common query latency P99 < 10ms.
- [ ] Indexes exist on all queried columns.
- [ ] WAL mode prevents read blocking.

**Dependencies:** FR-0061
**Tags:** should-have

---

## NFR-0013: Fee Market Responsiveness

**Category:** Performance

**Statement:** The system shall adjust base fee within 5 blocks in response to demand spikes.

**Rationale:** Fee market must adapt quickly to prevent queue saturation. See `agx-committee-bft-and-governance.md` Section 5 (Fee-market anti-spam).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 202-208

**Acceptance Criteria:**
- [ ] Base fee reaches 2x within 5 blocks of sustained 2x demand.
- [ ] Base fee decreases at bounded rate when demand drops.
- [ ] No fee oscillation under stable load.

**Dependencies:** FR-0146
**Tags:** must-have

---

## NFR-0014: Cross-Region Latency for Relays

**Category:** Performance

**Statement:** The system shall provide relay services with median RTT under 150ms within continental regions.

**Rationale:** Relay-dependent peers need acceptable latency. See `ockam-decentralized-network-architecture.md` Section 5 (Relay model).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 5 (NAT traversal)
- `ockam-decentralized-network-architecture.md` Section 6, Tradeoff 3

**Acceptance Criteria:**
- [ ] Median relay RTT < 150ms within same continent.
- [ ] P95 relay RTT < 300ms globally.
- [ ] Geographically distributed relay assignment.

**Dependencies:** FR-0044
**Tags:** should-have

---

## NFR-0015: Committee Sampling Computation Time

**Category:** Performance

**Statement:** The system shall compute epoch committee sampling within 1 second for 10,000 validator candidates.

**Rationale:** Committee transition must not stall block production. See `agx-committee-bft-and-governance.md` Section 5 (Committee BFT from day 1).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 147-170

**Acceptance Criteria:**
- [ ] Committee sampling computation <= 1s for 10k candidates.
- [ ] Computation is deterministic and single-threaded verifiable.
- [ ] Result is cached for full epoch to avoid recomputation.

**Dependencies:** FR-0002
**Tags:** must-have
