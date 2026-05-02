# Storage Spec: State Synchronization

**Component:** C2 State Machine & SMT
**Source ADRs:** ADR-0005 (Content-Addressed SMT)
**Covered FRs:** FR-0010, NFR-0009, NFR-0018, NFR-0019
**Dependencies:** C1 Consensus Engine

---

## Section 1: State Sync Protocol

### 1.1 Purpose

Define the state synchronization protocol for nodes joining the network, recovering from crashes, or catching up after being offline.

### 1.2 Normative Behavior

- The system MUST support full state sync from genesis via block replay.
- The system MUST support snap sync from a trusted epoch checkpoint for fast bootstrap.
- Snap sync MUST verify the checkpoint state root against a quorum of connected peers (minimum 3 matching roots).
- State sync MUST produce an identical SMT root to the committed block header state root.
- All blocks during sync MUST be fully validated (transaction signatures, nonces, state transitions).
- Crash recovery MUST load the latest SMT state root from local database and replay any unapplied blocks.
- Backup restore MUST verify backup checksum before loading state.

### 1.3 Data Structures

```rust
struct Snapshot {
    epoch: u64,
    height: u64,
    state_root: [u8; 32],
    block_hash: [u8; 32],
    sst_keys: Vec<([u8; 32], Vec<u8>)>,   // snapshot of all state keys at this height
    merkle_proof_batch: Vec<InclusionProof>,
}

enum SyncMode {
    Full,        // replay from genesis
    Snap,        // snap sync from checkpoint
    CatchUp,     // replay from last local height
}

struct SyncState {
    mode: SyncMode,
    current_height: u64,
    target_height: u64,
    validated_roots: u32,         // number of peers matching state root
    last_validated_block: [u8; 32],
}
```

### 1.4 State Transitions

**Snap sync flow:**
1. Node connects to peers, requests current height and checkpoint info.
2. Node selects a checkpoint (epoch boundary, within last 10 epochs).
3. Requests checkpoint state root from at least 3 peers; verifies quorum match.
4. Downloads snapshot data from fastest peer (all state keys at checkpoint height).
5. Verifies snapshot: recomputes SMT root from downloaded keys; matches peer state root.
6. Applies blocks from checkpoint height to current tip (catch-up phase).
7. Once caught up, transitions to normal consensus participation.

**Full sync flow:**
1. Node requests block 1 (after genesis) and sequentially replays all blocks.
2. Each block: validate header, verify signatures, apply transactions, compute state root.
3. State root after each block must match the committed root in the next block's header.
4. Continues until caught up to tip.

**Crash recovery flow:**
1. Node loads latest SMT root from local database (last committed block).
2. Scans for any blocks committed but not yet applied to local SMT cache.
3. Replays unapplied blocks.
4. Resumes consensus participation if validator.

### 1.5 Failure Behavior

- **State root mismatch during snap sync:** Abort sync from that peer; retry from another peer. If 3 peers give mismatched roots, node enters safe mode (sync from genesis).
- **Corrupted block during full sync:** Node blacklists the peer that served the block; retries from other peers.
- **Crash mid-sync:** Restart from last fully validated checkpoint. All partial state discarded.
- **Database corruption on crash recovery:** Restore from backup (NFR-0029). If no backup, full sync from genesis.

### 1.6 Versioning and Compatibility

- Snapshot format version embedded in checkpoint metadata.
- State key schema version tied to protocol version in `git:head`.
- Nodes running different protocol versions cannot sync from each other past the fork point.

### 1.7 Conformance Test Hooks

- Verify snap sync from checkpoint produces identical SMT root as full sync from genesis.
- Verify state root mismatch on snap sync triggers peer rotation and quorum check.
- Verify crash recovery restores state to exact pre-crash height.
- Verify backup restore with checksum verification rejects corrupted backup.
- Verify node startup and sync time under 10 minutes for 1 week of missed blocks (NFR-0009).
- Verify deterministic state convergence: two nodes starting from different initial states converge after syncing the same blocks (NFR-0019).

### 1.8 Trust-Assumption Inventory

- Peer state root honesty
  - Justification: Snap sync trust relies on a quorum of peers reporting the same state root.
  - Trust-minimised alternative: Full sync from genesis (no trust in peers, only in genesis block).
- Checkpoint availability
  - Justification: Checkpoints must be available from multiple peers; if no checkpoint peers, full sync is the fallback.
  - Trust-minimised alternative: Hardcoded checkpoint hashes in software releases (introduces centralized trust).
