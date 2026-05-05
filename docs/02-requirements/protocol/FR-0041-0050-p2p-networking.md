## FR-0041: Direct-First Routing with Relay Fallback

**Category:** Networking

**Statement:** The system shall implement a direct-first routing policy: peers always attempt direct IP connectivity before using relay paths, with continuous upgrade probes to migrate relay paths to direct when reachability changes.

**Rationale:** Minimizes latency, relay load, and cost while preserving liveness under NAT/firewall constraints. See `ockam-decentralized-network-architecture.md` Section 5 (Direct vs relay path selection).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 4 (Architecture)
- `ockam-decentralized-network-architecture.md` Section 5 (Core Mechanisms)

**Acceptance Criteria:**
- [ ] Direct secure channel is attempted first for all peer contacts.
- [ ] Relay path is created only after bounded retry budget and classified direct failure.
- [ ] Active relay paths run upgrade probes every 60 seconds (±20% jitter).
- [ ] On successful direct probe, traffic migrates to direct path and relay is retired.

**Dependencies:** none
**Tags:** must-have

---

## FR-0042: Hybrid Discovery (Bootstrap + Gossip + DHT)

**Category:** Networking

**Statement:** The system shall use hybrid peer discovery combining dynamic bootstrap nodes, gossip for fast anti-entropy, and Kademlia-style DHT for scalable targeted lookup.

**Rationale:** Gossip alone is expensive at scale; DHT gives efficient lookup; bootstrap prevents cold-start partitioning. See `ockam-decentralized-network-architecture.md` Section 5 (Discovery mechanism).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 5 (Discovery specifics)
- `ockam-decentralized-network-architecture.md` Section 6, Tradeoff 2

**Acceptance Criteria:**
- [ ] Bootstrap nodes return signed seed peer list, relay list, and trust policy.
- [ ] Gossip disseminates endpoint freshness with bounded fanout (max 8 peers per round, TTL 16 hops).
- [ ] DHT uses `k=20` contacts per bucket, key = `SHA3-256(identity_pubkey)`, refresh every 30 minutes.
- [ ] No single bootstrap authority; any validator or long-running node can serve as bootstrap.

**Dependencies:** FR-0041
**Tags:** must-have

---

## FR-0043: Identity-Only Rate Limits and Ingress Guards

**Category:** Networking

**Statement:** The system shall enforce per-identity connection caps and rate limits at the protocol level; IP/ASN limits are local hardening only, not protocol policy.

**Rationale:** In a permissionless network, one IP does not equal one identity. See `ockam-decentralized-network-architecture.md` Section 5 (Swarm-resistant ingress controls).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 5, lines 142-148
- `agx-committee-bft-and-governance.md` Section 5 (Rate limiting, lines 223-233)

**Acceptance Criteria:**
- [ ] Protocol ingress decisions are based on cryptographic identity, not IP address.
- [ ] Per-identity burst limit: max 20 txs in 60 seconds.
- [ ] Gossip budget per sender: max 100 messages per minute.
- [ ] IP/ASN limits may exist as local firewall rules but cannot affect consensus or protocol state.

**Dependencies:** FR-0042
**Tags:** must-have

---

## FR-0044: Relay Service with Per-Identity Quotas

**Category:** Networking

**Statement:** The system shall provide relay forwarding with per-identity quotas, service class prioritization, and stake-weighted priority for credentialed identities.

**Rationale:** Relays are shared infrastructure that require admission controls to prevent abuse. See `ockam-decentralized-network-architecture.md` Section 5 (Relay model).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 5 (Relay model)
- `ockam-decentralized-network-architecture.md` Section 7 (Relay queue flooding)

**Acceptance Criteria:**
- [ ] Relay nodes enforce per-identity quotas and unknown-sender caps.
- [ ] Control-plane traffic gets reserved capacity on relays.
- [ ] Relay capacity is proportional to stake for staked nodes.
- [ ] Nodes can opt-out of relaying if bandwidth-constrained.

**Dependencies:** FR-0041, FR-0043
**Tags:** must-have

---

## FR-0045: Secure Channel End-to-End Trust

**Category:** Networking

**Statement:** The system shall preserve end-to-end confidentiality, integrity, and mutual authentication regardless of hop count, by placing secure channels above routing/transports.

**Rationale:** Route-independent security ensures trust semantics hold across relay and direct paths alike. See `ockam-decentralized-network-architecture.md` Section 4 (Architecture).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 4 (Components)
- `ockam-decentralized-network-architecture.md` Section 5 (Ockam internals mapped)

**Acceptance Criteria:**
- [ ] Ockam secure channels provide mutual authentication and encrypted transport over any route.
- [ ] Message tampering or man-in-the-middle is cryptographically prevented regardless of relay hops.
- [ ] Secure channel identity is bound to the same ML-DSA key used for consensus.

**Dependencies:** FR-0005, FR-0041
**Tags:** must-have

---

## FR-0046: NAT Traversal Support

**Category:** Networking

**Statement:** The system shall support NAT traversal via STUN (min 3 geographically distributed), ICE candidate gathering (5s timeout), and relay fallback.

**Rationale:** Large percentages of peers are behind NAT/firewalls. See `ockam-decentralized-network-architecture.md` Section 5 (NAT traversal).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 5 (NAT traversal)

**Acceptance Criteria:**
- [ ] STUN servers: minimum 3 geographically distributed.
- [ ] ICE candidate gathering timeout: 5 seconds.
- [ ] Relay fallback is guaranteed for hard NAT cases.
- [ ] Direct probe interval: 60 seconds with ±20% jitter.

**Dependencies:** FR-0041
**Tags:** should-have

---

## FR-0047: Gossip Duplicate Suppression

**Category:** Networking

**Statement:** The system shall implement gossip duplicate suppression using Bloom filters (100,000 entries, 1% false positive rate) and message IDs with TTL.

**Rationale:** Prevents amplification loops in gossip dissemination. See `ockam-decentralized-network-architecture.md` Section 5 (Routing strategies).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 5, lines 117-121

**Acceptance Criteria:**
- [ ] Gossip fanout limit: max 8 peers per round.
- [ ] Message TTL: max 16 hops.
- [ ] Bloom filter with 100,000 entries and 1% false positive rate suppresses duplicates.
- [ ] False positives are handled by downstream verification (content hash).

**Dependencies:** FR-0042
**Tags:** must-have

---

## FR-0048: Network Partition Resilience

**Category:** Networking

**Statement:** The system shall continue local partition operation with cached peer/relay sets and reconcile DHT versions and replay gossip deltas on partition heal.

**Rationale:** Liveness must continue during routing outages. See `ockam-decentralized-network-architecture.md` Section 7 (Network partitions).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 7 (Network partitions)
- `decentralized-incident-response-and-recovery.md` Section 7 (Telemetry partition disagreement)

**Acceptance Criteria:**
- [ ] Peers continue operating with cached peer/relay sets during partition.
- [ ] On heal, DHT versions are reconciled deterministically.
- [ ] Gossip deltas are replayed to catch up missed events.
- [ ] No central coordinator is required for partition detection or healing.

**Dependencies:** FR-0042
**Tags:** must-have

---

## FR-0049: Connection Manager State Machine

**Category:** Networking

**Statement:** The system shall implement a finite-state connection manager with states: UNKNOWN, DIRECT_PROBING, DIRECT_ACTIVE, RELAY_ACTIVE, UPGRADING.

**Rationale:** Explicit state machine enables deterministic path management and debugging. See `ockam-decentralized-network-architecture.md` Section 5 (Pseudocode).

**Source Research:**
- `ockam-decentralized-network-architecture.md` Section 5, lines 154-184

**Acceptance Criteria:**
- [ ] Connection state transitions are deterministic and logged.
- [ ] State machine prevents oscillation with hysteresis on path switching.
- [ ] UPGRADING state handles in-flight traffic during path migration.

**Dependencies:** FR-0041
**Tags:** should-have

---

## FR-0050: Mempool Fee Ordering with Evidence/Governance Discounts

**Category:** Networking

**Statement:** The system shall maintain a single mempool priority queue ordered by fee (highest first). Evidence and governance transactions shall receive governance-set fee discounts to ensure they clear during congestion. No lane reservation exists.

**Rationale:** Single-pool fee ordering is simpler than lane reservation and wastes no capacity. Evidence/governance discounts ensure critical operations are not starved under load. See `p2p-wire-spec.md` Section 2.

**Source Research:**
- `p2p-wire-spec.md` Section 2
- `fee-market-spec.md` Section 1

**Acceptance Criteria:**
- [ ] Mempool is a single priority queue ordered by fee.
- [ ] Evidence transactions receive effective base fee discount (governance-set percentage).
- [ ] Governance transactions receive effective base fee discount (governance-set percentage).
- [ ] No lane reservation exists — all transaction types share the same pool.
- [ ] Per-sender pending transaction limit prevents spam.

**Dependencies:** FR-0007
**Tags:** must-have
