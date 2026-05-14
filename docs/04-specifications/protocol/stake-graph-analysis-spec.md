# Protocol Spec: Stake-Graph Analysis for Anti-Split Clustering

**Components:** C3 Staking & Validator Manager
**Source ADRs:** ADR-0007 (Committee BFT with VDF)
**Covered FRs:** FR-0002, FR-0183
**Dependencies:** C2 State Machine & SMT, C1 Consensus Engine

---

## Section 1: Validator Cluster Detection

### 1.1 Purpose

Define the deterministic, on-chain algorithm for detecting correlated validator keys via stake-graph analysis. Clusters of correlated validators are treated as a single entity for committee weight computation. This replaces the previous per-operator seat cap (removed in 2026-05-06 architecture amendment) with a Sybil-evasion-only clustering mechanism — no hard cap is applied to legitimate stake concentration.

**Non-goals:**
- This spec does NOT define behavioral Sybil detection (see `sybil-detection-correlation-engine.md`).
- This spec does NOT impose a per-operator seat cap on committee influence.
- This spec does NOT track off-chain relationships or IP/geo correlation.

### 1.2 Normative Behavior

- The system MUST build a directed stake-funding graph from on-chain `TransferTx`, `StakingTx(Bond)`, and airdrop disbursement records.
- For each validator, the system MUST trace backwards up to N=3 hops to find all ancestor accounts that contributed AGX to the validator's stake.
- Two or more validators that share a common ancestor within N hops MUST be grouped into the same cluster.
- The cluster's effective weight for committee selection MUST be computed as the sum of all member validators' bonded stake, but only the first M members (where M = min(5, cluster_size)) count toward diversity metrics.
- Cluster detection MUST be computed deterministically at each epoch boundary from SMT state only. No external data, no IP/geo, no behavioral heuristics.
- The result (membership assignments and cluster IDs) MUST be committed to SMT state for deterministic replay across all nodes.

### 1.3 Data Structures

```rust
struct ClusterRecord {
    cluster_id: [u8; 32],             // SHA3-256 of concatenated sorted member IDs
    members: Vec<[u8; 32]>,            // validator_id list, sorted
    ancestor_root: [u8; 32],           // SHA3-256 of the common ancestor that triggered clustering
    total_bonded_stake: u128,          // sum of all member bonded_stake (for committee draw)
    cluster_size_diversity_bonus: u8,  // min(5, cluster_size) for diversity scoring
    detected_at_epoch: u64,            // epoch when first detected
}

struct FundingEdge {
    from_account: [u8; 32],           // source of funds
    to_account: [u8; 32],             // destination
    amount: u128,                      // AGX transferred
    height: u64,                       // block of transaction
}

enum ClusterAncestorType {
    AirdropAgent,                      // genesis airdrop distribution
    DirectFunding,                     // direct TransferTx between accounts
    IndirectFunding,                   // chain of 2+ transfers
}

struct ClusterDetectionResult {
    epoch: u64,
    clusters: Vec<ClusterRecord>,
    unclustered_validators: Vec<[u8; 32]>,  // no common ancestor found
}
```

### 1.4 State Transitions

**Epoch-boundary cluster detection pipeline:**

1. Read all validators in `active` and `paused` states from SMT state.
2. For each validator V_i:
   a. Walk backwards up to N=3 hops following TransferTx and StakingTx(Bond) records.
   b. Collect all ancestor accounts reachable within 3 hops.
   c. If any ancestor account is shared with another validator V_j, mark V_i and V_j as same cluster.
3. Assign cluster IDs deterministically: `SHA3-256(sorted(member_ids))`.
4. Compute total_bonded_stake per cluster (sum of members).
5. Update SMT state with new ClusterDetectionResult for this epoch.
6. Consensus engine reads cluster weights for committee sampling instead of raw per-key weights.

**Committee sampling integration:**

```rust
fn select_committee_with_clusters(epoch, validator_pool, clusters, seed):
    // For committee selection, use cluster-based weight, not per-key weight
    candidates = filter(validator_pool, status == active)
    cluster_weighted = merge_validators_into_clusters(candidates, clusters)
    // Each cluster enters as one weighted entry; weights are cluster total_bonded_stake
    committee = deterministic_weighted_sample(cluster_weighted, seed, committee_size)
    return committee
```

### 1.5 Failure Behavior

- **Stake-graph traversal depth insufficient:** With N=3 hops, a deeply hidden funding chain (A→B→C→D→validator) may not be detected. This is an accepted residual risk — deeper traversal increases state I/O cost per epoch. The behavioral Sybil detection engine (see `sybil-detection-correlation-engine.md`) provides a secondary detection layer.
- **Large cluster dominates committee:** A cluster with >33% of total active stake could theoretically approach the Byzantine threshold. The 80% committee rotation (FR-0004) and two-epoch recency limit mitigate persistent capture. No per-cluster seat cap exists — the market distribution of stake is the primary guard.
- **False positive clustering:** Two independent validators that received AGX from the same exchange hot wallet could be falsely clustered. Mitigation: exchanges should document deposit addresses; governance can petition for cluster splitting via EvidenceTx with proof of independent operation.
- **State growth from edge storage:** Every TransferTx and StakingTx(Bond) creates a FundingEdge record. Growth is bounded to approximately 100k transactions/day = 100k edges/day. Pruning: edges older than 100,000 blocks are archived. SMT retains only the root and the cluster detection result.

### 1.6 Versioning and Compatibility

- N-hopping depth (currently 3) is a governance-adjustable parameter.
- Adding new hop depth requires re-running cluster detection from genesis for deterministic consistency — this is a breaking change requiring governance `git:head` update.
- Cluster detection result schema is versioned per SYSTEM_PARAMETERS.

### 1.7 Conformance Test Hooks

- Verify cluster detection: two validators funded by the same account within N=3 hops are grouped into the same cluster.
- Verify cluster detection: two validators funded by completely independent accounts are NOT grouped.
- Verify N=3 hop limit: a validator funded through a 4-hop chain (A→B→C→D→validator) is NOT clustered with a validator funded by A.
- Verify committee weight computation: cluster total_bonded_stake is used for committee draw weight instead of individual validator weights.
- Verify determinism: two nodes processing the exact same transaction history produce identical cluster assignments.
- Verify garbage collection: FundingEdge records older than 100,000 blocks are pruned.
- Verify no per-operator seat cap is enforced — a single cluster with 20% of total stake receives exactly 20% weight in committee selection.

### 1.8 Trust-Assumption Inventory

- Transaction graph completeness
  - Justification: Only on-chain transactions are visible to the protocol. Off-chain relationships (same operator, same hosting provider) are not detectable through this mechanism.
  - Trust-minimised alternative: Behavioral correlation engine (FR-0191) adds a second detection layer using voting patterns, task co-claiming, and temporal activity overlap.
- N=3 hop depth
  - Justification: 3 hops captures direct funding, intermediary funding, and wallet-service funding chains. Deep chains become increasingly costly to construct and maintain.
  - Trust-minimised alternative: Depth is governance-adjustable; increasing to N=5 catches more chains but increases epoch-boundary computation cost.
- Genesis airdrop agent as root ancestor
  - Justification: All airdrop recipients share the airdrop agent as a common ancestor at hops > 0. The system MUST distinguish "all received genesis airdrop" from "same operator funded both."
  - Trust-minimised alternative: Airdrop agent is treated as a special root — validators whose only shared ancestor is the airdrop agent within N hops are NOT clustered. Only non-airdrop common ancestors trigger clustering.
