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
    collateral: u128,                // AGX locked in atto-AGX
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
 
### 1.5 Failure Behavior

- **Proof-of-possession failure:** Provider fails challenge → lease moves to AtRisk → collateral slashed if repeated within 3 windows.
- **Repair coordinator overload:** Maximum concurrent repairs bounded at configurable limit (default 10). Queue prioritized by artifact class.
- **Artifact retrieval corruption:** Chunk hash mismatch → blacklist provider for that artifact epoch. Retry from other providers. Parallel retrieval from min_replica_count + 2 providers.
- **Minimum replica breach:** If replica count drops below 1 (only copy), artifact is irrecoverably lost. Governance alert generated.
- **Git object verification:** For governance artifacts, fetched objects must match proposal commit hashes. SHA-256 object format preferred.

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


