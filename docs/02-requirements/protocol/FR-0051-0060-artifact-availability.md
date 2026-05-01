## FR-0051: Content-Addressed Artifact Storage

**Category:** Networking

**Statement:** The system shall store artifacts as content-addressed git objects using gix, with manifests in protocol state and replication off-state.

**Rationale:** Keeps consensus state compact while preserving deterministic artifact availability. See `artifact-availability-and-retention.md` Section 4 (Architecture).

**Source Research:**
- `artifact-availability-and-retention.md` Section 4 (How It Works)
- `artifact-availability-and-retention.md` Section 5 (Artifact model)

**Acceptance Criteria:**
- [ ] Artifact root hash is a hash of canonical serialized manifest.
- [ ] Chunk root hash is Merkle root over ordered chunk hashes.
- [ ] Manifest schema includes required fields: artifact_root_hash, chunk_root_hash, size_bytes, chunk_count, class, retention_tier, min_replica_count, created_at_height, expires_at_height, producer_signature.

**Dependencies:** none
**Tags:** must-have

---

## FR-0052: Proof-of-Possession for Artifact Providers

**Category:** Networking

**Statement:** The system shall require artifact providers to respond to random chunk-index challenges with challenged chunk + Merkle proof + lease signature.

**Rationale:** Converts availability from trust claim to verifiable evidence. See `artifact-availability-and-retention.md` Section 5 (Proof-of-possession).

**Source Research:**
- `artifact-availability-and-retention.md` Section 5, lines 99-106

**Acceptance Criteria:**
- [ ] Verifier issues random chunk-index challenge per lease window.
- [ ] Provider response includes chunk bytes, Merkle inclusion proof, and lease signature.
- [ ] Verification checks chunk hash, Merkle root, and timeliness.
- [ ] Repeated proof failures slash collateral and lower provider score.

**Dependencies:** FR-0051
**Tags:** must-have

---

## FR-0053: Multi-Source Parallel Retrieval with Hash Verification

**Category:** Networking

**Statement:** The system shall retrieve artifacts from multiple providers in parallel, rejecting any chunk failing hash/proof verification, and reassemble only when all chunk hashes match manifest.

**Rationale:** Improves liveness and resilience against malicious providers. See `artifact-availability-and-retention.md` Section 5 (Deterministic retrieval).

**Source Research:**
- `artifact-availability-and-retention.md` Section 5, lines 107-112

**Acceptance Criteria:**
- [ ] Client selects top-N providers (min_replica_count + 2).
- [ ] Parallel chunk download with per-chunk hash verification.
- [ ] Corrupted chunks trigger blacklist for that artifact epoch.
- [ ] Reassembly proceeds only when all chunks are present and verified.

**Dependencies:** FR-0052
**Tags:** must-have

---

## FR-0054: Class-Based Retention Tiers

**Category:** Networking

**Statement:** The system shall enforce class-based retention: governance bundles pinned long-term, review evidence medium-term, research outputs medium-term with optional pin, telemetry archives short-term.

**Rationale:** Allocates storage to highest protocol-value artifacts. See `artifact-availability-and-retention.md` Section 5 (Retention policy).

**Source Research:**
- `artifact-availability-and-retention.md` Section 5, lines 113-119

**Acceptance Criteria:**
- [ ] Each artifact class has explicit retention tier and default expiry.
- [ ] Governance bundles default to pinned (no expiry).
- [ ] Expired artifacts transition to Pruned state after buffer window.
- [ ] Pin extensions can be requested via governance or high-trust action.

**Dependencies:** FR-0051
**Tags:** must-have

---

## FR-0055: Replication Lease Assignment

**Category:** Networking

**Statement:** The system shall assign replication leases containing provider ID, artifact hash, lease start/end height, challenge cadence, and collateral.

**Rationale:** Economic accountability for storage providers. See `artifact-availability-and-retention.md` Section 5 (Replication leases).

**Source Research:**
- `artifact-availability-and-retention.md` Section 5, lines 93-98

**Acceptance Criteria:**
- [ ] Lease assignment targets min_replica_count with diversity constraints.
- [ ] Provider collateral is locked for lease duration.
- [ ] Successful proof responses earn rewards; failures lose collateral.
- [ ] AtRisk state triggers repair coordinator to reseed replicas.

**Dependencies:** FR-0052
**Tags:** should-have

---

## FR-0056: Repair Coordinator

**Category:** Networking

**Statement:** The system shall implement a repair coordinator that triggers re-replication when replica count falls below minimum, with priority queue by artifact class.

**Rationale:** Maintains availability under provider churn. See `artifact-availability-and-retention.md` Section 5 (Retention policy state diagram).

**Source Research:**
- `artifact-availability-and-retention.md` Section 7 (Repair coordinator overload)
- `artifact-availability-and-retention.md` Section 5, lines 123-130

**Acceptance Criteria:**
- [ ] AtRisk detection triggers repair within bounded time.
- [ ] Repair queue prioritizes governance bundles and review evidence.
- [ ] Concurrent repairs are bounded to prevent coordinator overload.
- [ ] Repair completion restores Healthy state.

**Dependencies:** FR-0055
**Tags:** should-have

---

## FR-0057: Content-Addressing SLA

**Category:** Networking

**Statement:** The system shall define concrete minimum replica targets and repair-latency bounds for each artifact class, with availability measured by successful challenge-response ratio.

**Rationale:** Production storage requires measurable SLAs. See `PROJECT-STATUS.md` (Research Gaps: Content-addressing SLA).

**Source Research:**
- `artifact-availability-and-retention.md` Section 8 (Scalability Analysis)
- `PROJECT-STATUS.md` (Gap: Content-addressing SLA)

**Acceptance Criteria:**
- [ ] Governance bundles: min 5 replicas, repair within 1 epoch.
- [ ] Review evidence: min 3 replicas, repair within 2 epochs.
- [ ] Research output: min 2 replicas, repair within 3 epochs.
- [ ] Availability metric = successful_challenge_responses / total_challenges per epoch.

**Dependencies:** FR-0055
**Tags:** must-have

---

## FR-0058: Artifact Registration Determinism

**Category:** Networking

**Statement:** The system shall compute artifact root hash deterministically from canonical manifest serialization, with chunking and Merkle root computed identically across all nodes.

**Rationale:** Hash equality must be reproducible across heterogeneous machines. See `artifact-availability-and-retention.md` Section 5 (Pseudocode).

**Source Research:**
- `artifact-availability-and-retention.md` Section 5, lines 133-144

**Acceptance Criteria:**
- [ ] Canonical manifest serialization excludes non-deterministic fields (timestamps are in metadata, not in hash input).
- [ ] Chunk size is fixed by class profile (default 1 MiB).
- [ ] Merkle tree construction uses deterministic ordering and hash function.

**Dependencies:** FR-0051
**Tags:** must-have

---

## FR-0059: Git Object Verification for Governance Artifacts

**Category:** Networking

**Statement:** The system shall verify that fetched git objects match proposal commit hashes, using Git SHA-256 object format for new repos.

**Rationale:** Commit-hash equality is sufficient integrity check only with strong object IDs. See `agx-committee-bft-and-governance.md` Section 5 (Governance determinism).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 180-183

**Acceptance Criteria:**
- [ ] Governance proposal fetch verifies all object IDs against manifest.
- [ ] Resulting commit hash equals `proposed_commit` in proposal.
- [ ] Prefer Git SHA-256 object format; SHA-1 is acceptable only for backward compatibility with explicit collision warnings.

**Dependencies:** FR-0023, FR-0058
**Tags:** must-have

---

## FR-0060: Signed Telemetry Summaries with Quorum Validation

**Category:** Networking

**Statement:** The system shall publish signed telemetry summaries each epoch, aggregate from minimum independent reporters, and validate with quorum before incident classification.

**Rationale:** Telemetry integrity is consensus-adjacent and cannot be trusted from single source. See `telemetry-threat-model.md` Section 4 (Architecture).

**Source Research:**
- `telemetry-threat-model.md` Section 5 (Signed telemetry schema)
- `telemetry-threat-model.md` Section 5 (Aggregation rules)
- `decentralized-incident-response-and-recovery.md` Section 4 (Architecture)

**Acceptance Criteria:**
- [ ] Telemetry envelope fields: producer_id, metric_class, value, height, seq_no, signature (ML-DSA).
- [ ] Minimum M = max(5, committee_size / 10) independent reporters per metric class.
- [ ] Aggregation uses median for latency; trimmed mean for ratios (discard top/bottom 10%).
- [ ] Outlier producers deviating beyond z-threshold are flagged as anomalous.

**Dependencies:** FR-0005
**Tags:** must-have
