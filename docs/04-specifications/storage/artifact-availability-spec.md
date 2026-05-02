# Storage Spec: Artifact Availability

**Component:** C8 Artifact Availability & Storage
**Source ADRs:** ADR-0005 (Content-Addressed SMT)
**Covered FRs:** FR-0051, FR-0052, FR-0053, FR-0054, FR-0055, FR-0056, FR-0057, FR-0058, FR-0059, FR-0060
**Dependencies:** C2 State Machine, C7 P2P Networking

---

## Section 1: Content-Addressed Storage

### 1.1 Purpose

Define the content-addressed artifact storage system, including manifest registration, chunked retrieval, proof-of-possession, retention tiers, and repair coordination.

### 1.2 Normative Behavior

- The system MUST store artifacts as content-addressed objects using gix.
- Artifact manifests MUST be registered in protocol state (SMT key prefix 0x05).
- The artifact root hash MUST be the SHA3-256 of the canonical serialized manifest.
- The chunk root hash MUST be a Merkle root over ordered chunk hashes.
- Chunk size MUST be fixed by artifact class profile (default 1 MiB).
- Manifest MUST include required fields: artifact_root_hash, chunk_root_hash, size_bytes, chunk_count, class, retention_tier, min_replica_count, created_at_height, expires_at_height, producer_signature.
- Canonical manifest serialization MUST exclude non-deterministic fields from hash input.
- Timestamps MUST be expressed as block heights.

### 1.3 Data Structures

```rust
struct ArtifactManifest {
    artifact_root_hash: [u8; 32],       // SHA3-256 of canonical serialized manifest
    chunk_root_hash: [u8; 32],          // Merkle root over ordered chunk hashes
    size_bytes: u64,
    chunk_count: u32,
    class: ArtifactClass,
    retention_tier: RetentionTier,
    min_replica_count: u8,
    created_at_height: u64,
    expires_at_height: u64,
    producer_signature: Vec<u8>,
}

enum ArtifactClass {
    GovernanceBundle,
    ReviewEvidence,
    ResearchOutput,
    TelemetryArchive,
}

enum RetentionTier {
    Pinned,         // no expiry
    MediumTerm,     // default 90 days
    ShortTerm,      // default 30 days
}

struct ReplicationLease {
    lease_id: [u8; 32],             // SHA3-256(provider_id || artifact_root_hash || height)
    artifact_root_hash: [u8; 32],
    provider_id: [u8; 32],
    lease_start_height: u64,
    lease_end_height: u64,
    challenge_cadence: u16,          // blocks between proof challenges
    collateral: u64,                 // AGX locked
    status: LeaseStatus,
}

enum LeaseStatus {
    Active,
    AtRisk,
    Expired,
}

struct ProofOfPossession {
    lease_id: [u8; 32],
    chunk_index: u32,                // random index selected by verifier
    chunk_bytes: Vec<u8>,
    merkle_proof: Vec<[u8; 32]>,     // inclusion proof from chunk to chunk_root_hash
    lease_signature: Vec<u8>,
    response_height: u64,
}

struct RepairQueue {
    entries: Vec<RepairEntry>,
    max_concurrent: u8,              // bounded to prevent coordinator overload
}

struct RepairEntry {
    artifact_root_hash: [u8; 32],
    artifact_class: ArtifactClass,
    current_replica_count: u8,
    target_replica_count: u8,
    priority: u8,                    // 0=highest
    entered_at_height: u64,
}
```

### 1.4 State Transitions

**Artifact lifecycle:**

```
Producer creates artifact
  → computes chunk hashes + Merkle root
  → registers ArtifactManifest in protocol state
  → replication leases assigned to providers (target: min_replica_count)

Active (leases active)
  → periodic proof-of-possession challenges verify replica health
  → providers pass → lease renewed
  → provider fails → AtRisk → repair triggered

AtRisk
  → repair coordinator assigns new leases from repair pool
  → replica count restored → back to Active
  → replica count drops to 0 → artifact lost (logged, governance notified)

Expired (block height > expires_at_height + buffer)
  → leases terminated, collateral released
  → artifact pruned by archive nodes (kept by full archive nodes)
```

**Retention tiers:**

| Class | Tier | Min Replica Count | Default Expiry | Repair SLA |
|-------|------|-------------------|----------------|------------|
| Governance Bundle | Pinned | 5 | None (pinned) | 1 epoch |
| Review Evidence | MediumTerm | 3 | 90 days | 2 epochs |
| Research Output | MediumTerm | 2 | 90 days | 3 epochs |
| Telemetry Archive | ShortTerm | 2 | 30 days | N/A (pruned) |

### 1.5 Failure Behavior

- **Proof-of-possession failure:** Provider fails challenge → lease moves to AtRisk → collateral slashed if repeated within 3 windows.
- **Repair coordinator overload:** Maximum concurrent repairs bounded at configurable limit (default 10). Queue prioritized by artifact class.
- **Artifact retrieval corruption:** Chunk hash mismatch → blacklist provider for that artifact epoch. Retry from other providers. Parallel retrieval from min_replica_count + 2 providers.
- **Minimum replica breach:** If replica count drops below 1 (only copy), artifact is irrecoverably lost. Governance alert generated.
- **Git object verification:** For governance artifacts, fetched objects must match proposal commit hashes. SHA-256 object format preferred; SHA-1 is acceptable with collision warnings.

### 1.6 Versioning and Compatibility

- Artifact manifest schema version is embedded in the first byte of the canonical serialization.
- Chunking parameters are fixed per artifact class; changes require governance.
- Retention tier mappings are governance-adjustable.

### 1.7 Conformance Test Hooks

- Verify artifact root hash is deterministic: identical content → identical hash.
- Verify chunk root hash is correct Merkle root over ordered chunk hashes.
- Verify proof-of-possession challenge: provider responds with correct chunk + Merkle proof.
- Verify parallel retrieval from min_replica_count + 2 providers succeeds.
- Verify corrupted chunk from one provider is rejected; retrieval continues from other providers.
- Verify governance bundles require 5 replicas and repair within 1 epoch.
- Verify AtRisk detection triggers repair coordinator within bounded time.
- Verify expired artifact transitions to pruned state after buffer window.
- Verify repair queue prioritizes governance bundles over research outputs.

### 1.8 Trust-Assumption Inventory

- Provider honesty in proof-of-possession
  - Justification: Providers could serve correct chunks during challenge but wrong chunks during actual retrieval.
  - Trust-minimised alternative: Retrieval from multiple providers with hash cross-verification; statistically detects fraud.
- Repair coordinator liveness
  - Justification: Repair coordinator runs on all nodes; coordination requires at least one honest node initiating repair.
  - Trust-minimised alternative: Multiple repair coordinators with priority-based claim; any node can initiate repair for any artifact.
- gix content-address integrity
  - Justification: Artifact content addresses rely on gix (Git SHA-256) hash uniqueness.
  - Trust-minimised alternative: SHA3-256 for artifact addressing independent of git object format.

---

## Section 2: Telemetry Verification

### 2.1 Purpose

Define the signed telemetry envelope schema and quorum-based aggregation for protocol metrics.

### 2.2 Normative Behavior

- The system MUST publish signed telemetry summaries each epoch.
- Telemetry MUST be aggregated from a minimum of M = max(5, committee_size / 10) independent reporters per metric class.
- Aggregation MUST use median for latency metrics; trimmed mean (discard top/bottom 10%) for ratio metrics.
- Outlier producers deviating beyond z-threshold (z = 3.0) MUST be flagged as anomalous.
- Each envelope MUST include: producer_id, metric_class, value, height, seq_no, signature (ML-DSA).
- Validation MUST reject envelopes with seq_no <= last_seq_no for the same (producer_id, metric_class).
- Validation MUST reject envelopes with height outside current_height +/- 10 blocks.
- Telemetry MUST be reconciled against independently observable network events where possible.

### 2.3 Data Structures

```rust
struct TelemetryEnvelope {
    envelope_id: [u8; 32],       // SHA3-256(payload || signature)
    producer_id: [u8; 32],
    metric_class: MetricClass,
    value: u64,                   // encoded metric value
    height: u64,                  // block height at observation
    seq_no: u64,                  // monotonic per (producer_id, metric_class)
    signature: Vec<u8>,
}

enum MetricClass {
    FinalityLag,        // milliseconds
    RejectRatio,        // basis points (1/10000)
    FillRatio,          // basis points
    TxThroughput,       // transactions per block
}

struct EpochTelemetrySummary {
    epoch: u64,
    metric_class: MetricClass,
    aggregated_value: u64,
    reporter_count: u32,
    outlier_flags: Vec<([u8; 32], f64)>,  // (producer_id, z_score)
    reconciliation_status: ReconciliationStatus,
}

enum ReconciliationStatus {
    Consistent,
    DiscrepancyDetected,
    NotReconcilable,     // metric class has no independent observable
}
```

### 2.4 Failure Behavior

- **Insufficient reporters:** If fewer than M independent reporters submit for a metric class, the metric is marked as low-confidence and circuit-breaker triggers require additional corroboration.
- **Outlier detection:** Producers with envelopes beyond z=3.0 are flagged. Repeated flagging (3+ consecutive epochs) triggers anomaly report.
- **Reconciliation failure:** If aggregated telemetry disagrees with independently observable data by more than 10%, reconciliation is marked DiscrepancyDetected and circuit-breaker excludes that metric from triggers.

### 2.7 Conformance Test Hooks

- Verify telemetry envelope signature validation rejects forged signatures.
- Verify seq_no replay detection rejects envelope with seq_no <= last_seq_no.
- Verify height bound enforcement rejects stale/future telemetry.
- Verify aggregation requires minimum M = max(5, committee_size / 10) reporters.
- Verify trimmed mean aggregation discards top and bottom 10%.
- Verify outlier flagging at z > 3.0.
- Verify reconciliation cross-check against block header timestamps for finality lag.

### 2.8 Trust-Assumption Inventory

- Reporter independence
  - Justification: Aggregation from colluding reporters can produce fabricated metrics.
  - Trust-minimised alternative: Random sampling of reporters per epoch from active validator set.
- Reconciliation ground truth
  - Justification: Some metrics have no independent observable (e.g., reject ratio internal to PDP).
  - Trust-minimised alternative: Multi-node cross-validation requiring reporter diversity across operator clusters.
