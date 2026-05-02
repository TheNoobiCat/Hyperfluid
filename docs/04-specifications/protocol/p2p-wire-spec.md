# Protocol Spec: P2P Networking & Wire Protocol

**Component:** C7 P2P Networking & Connection Manager
**Source ADRs:** ADR-0001 (12-Component Architecture)
**Covered FRs:** FR-0041, FR-0042, FR-0043, FR-0044, FR-0045, FR-0046, FR-0047, FR-0048, FR-0049, FR-0050
**Dependencies:** Ockam P2P framework, ML-DSA-65 identity keys

---

## Section 1: Peer Discovery & Connection Management

### 1.1 Purpose

Define the peer discovery, connection management, and routing protocol for the Hyperfluid P2P network.

### 1.2 Normative Behavior

- The system MUST use direct-first routing: peers attempt direct IP connectivity before relay paths.
- The system MUST use hybrid discovery: dynamic bootstrap nodes + gossip for anti-entropy + Kademlia DHT for targeted lookup.
- The system MUST preserve end-to-end confidentiality, integrity, and mutual authentication regardless of relay hops.
- Identity-based connection limits MUST be enforced at the protocol level; IP/ASN limits are local hardening only.
- The system MUST support NAT traversal via STUN (minimum 3 geographically distributed servers) and ICE candidate gathering.
- Relay paths MUST run continuous upgrade probes every 60 seconds with +/- 20% jitter.
- On successful direct probe, traffic MUST migrate to direct path and relay SHOULD be retired.
- Bootstrap nodes MUST return signed seed peer list, relay list, and trust policy.
- Any validator or long-running node (identity age > 100,000 blocks) MAY serve as bootstrap.
- No single bootstrap authority is required; peers cache multiple bootstrap sources.

### 1.3 Data Structures

```rust
struct PeerInfo {
    peer_id: [u8; 32],            // SHA3-256 of identity pubkey
    endpoints: Vec<String>,        // IP:port addresses
    relay_routes: Vec<[u8; 32]>,  // relay peer_ids in path order
    last_seen_height: u64,
    capabilities: CapabilityFlags,
}

struct ConnectionState {
    peer_id: [u8; 32],
    state: ConnState,
    direct_endpoint: Option<String>,
    relay_path: Option<Vec<[u8; 32]>>,
    last_probe_height: u64,
    consecutive_failures: u32,
}

enum ConnState {
    Unknown,
    DirectProbing,
    DirectActive,
    RelayActive,
    Upgrading,
}

struct DHTEntry {
    key: [u8; 32],               // SHA3-256(identity_pubkey)
    value: PeerInfo,
    ttl_blocks: u64,
    signature: Vec<u8>,
}

struct GossipMessage {
    message_id: [u8; 32],
    ttl: u8,                       // max 16 hops
    fanout: u8,                    // max 8 peers
    payload: Vec<u8>,
    origin_peer_id: [u8; 32],
    timestamp: u64,
}
```

### 1.4 State Transitions

**Connection state machine:**

```
Unknown ──(probe initiated)──► DirectProbing
DirectProbing ──(direct connect success)──► DirectActive
DirectProbing ──(direct connect timeout/refused)──► RelayActive [fallback]
DirectActive ──(upgrade probe success on relay path)──► Upgrading
Upgrading ──(migration complete)──► DirectActive [relay retired]
RelayActive ──(upgrade probe detects direct reachable)──► Upgrading
DirectActive ──(connection lost)──► Unknown [after grace period]
RelayActive ──(all relay paths lost)──► Unknown
```

**Discovery flow:**
1. On startup, peer contacts bootstrap nodes for signed seed peer list and relay list.
2. Peer initiates connections to seed peers (direct-first with relay fallback).
3. Gossip disseminates peer endpoint updates with fanout 8, TTL 16.
4. DHT stores peer records with k=20 per bucket, keyed by SHA3-256(identity_pubkey).
5. DHT refresh every 30 minutes. Replication factor k for redundancy.
6. Gossip Bloom filter (100,000 entries, 1% false positive rate) suppresses duplicate message propagation.

### 1.5 Failure Behavior

- **Direct connection failure:** After bounded retry budget (3 attempts, 5 seconds each), fallback to relay.
- **Relay path failure:** If all relay paths for a peer are exhausted, peer transitions to Unknown. Re-discovery through DHT.
- **Network partition:** Peers continue operating with cached peer/relay sets. On heal, DHT versions reconciled, gossip deltas replayed.
- **Bootstrap failure:** Peer rotates through cached bootstrap sources (minimum 5 cached). If all fail, peer uses stored DHT peers.
- **Gossip duplication:** Bloom filter prevents re-propagation; false positives handled by downstream content hash verification.

### 1.6 Versioning and Compatibility

- Wire protocol version embedded in GossipMessage header.
- DHT key format versioned by key prefix byte.
- Connection state machine version tied to node software version in policy bundle.

### 1.7 Conformance Test Hooks

- Verify direct channel attempted before relay for all peer contacts.
- Verify relay upgrade probes fire at 60-second intervals with jitter.
- Verify hybrid discovery: bootstrap returns signed peer list, gossip converges within 30s for 99% of nodes.
- Verify DHT with k=20 and refresh every 30 minutes.
- Verify gossip fanout <= 8 and TTL <= 16.
- Verify duplicate message suppression via Bloom filter.
- Verify end-to-end encryption maintained across relay hops.
- Verify partition resilience: nodes operate with cached peers; reconcile on heal.
- Verify connection state machine transitions are deterministic.

### 1.8 Trust-Assumption Inventory

- Bootstrap node availability
  - Justification: Cold start requires at least one bootstrap node. Multiple fallback sources and caching mitigate.
  - Trust-minimised alternative: Hardcoded genesis peer list in genesis block; bootstrap nodes are validators (decentralized).
- Ockam secure channel correctness
  - Justification: End-to-end encryption and authentication depend on Ockam or equivalent.
  - Trust-minimised alternative: Noise protocol framework with ML-DSA — same trust model.
- STUN server availability
  - Justification: NAT traversal depends on at least one STUN server. Minimum 3 geographically distributed required.
  - Trust-minimised alternative: TURN relays as fallback; relay infrastructure is decentralized.

---

## Section 2: Mempool Lane Reservation

### 2.1 Purpose

Define the mempool lane architecture for admission control under congestion.

### 2.2 Normative Behavior

- The system MUST split the mempool into bounded lanes with reserved capacity.
- Lane capacities MUST be: Evidence 15%, Consensus-Control 10%, Governance 10%, Transfer 65%.
- Lane reservation MUST be enforced at transaction admission.
- Critical lanes (evidence, control, governance) MUST maintain reserved capacity even under transfer spam.
- Dynamic reallocation MUST only move capacity toward critical lanes, never away.
- Each lane MUST have independent eviction policy (FIFO by fee within lane).

### 2.3 Data Structures

```rust
enum MempoolLane {
    Evidence = 0,        // 15% capacity
    ConsensusControl,    // 10%
    Governance,          // 10%
    Transfer,            // 65%
}

struct MempoolConfig {
    lane_capacities: [u8; 4],    // percentages
    max_total_tx: u64,           // total mempool size
    per_sender_tx_limit: u32,   // per sender per block
}
```

### 2.4 Failure Behavior

- Lane starvation: critical lane transactions always admitted up to reserved capacity.
- Transfer lane full: lowest-fee transactions evicted first.
- Per-sender limit reached: additional transactions from same sender are rejected.

### 2.7 Conformance Test Hooks

- Verify lane allocation at admission: evidence tx goes to evidence lane only.
- Verify reserved capacity for governance lane under 100% transfer saturation.
- Verify dynamic reallocation does not reduce critical lane capacities.
- Verify per-lane eviction: lowest fee within lane evicted first.
