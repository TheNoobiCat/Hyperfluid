# Protocol Spec: Consensus Engine & State Machine

**Components:** C1 Consensus Engine, C2 State Machine & SMT
**Source ADRs:** ADR-0007 (Committee BFT with VDF), ADR-0005 (Content-Addressed SMT)
**Covered FRs:** FR-0001, FR-0002, FR-0003, FR-0004, FR-0005, FR-0006, FR-0007, FR-0008, FR-0009, FR-0010, FR-0194, FR-0020a
**Dependencies:** Malachite BFT library, Ockam P2P transport, ML-DSA-65 signature scheme

---

## Section 1: Committee BFT Consensus

### 1.1 Purpose

Define the committee-based Byzantine Fault Tolerant consensus protocol from genesis through ongoing committee rotation.

### 1.2 Normative Behavior

- The system MUST use committee BFT with a rotating epoch committee of exactly 100 validators.
- The system MUST sample committees via stake-weighted draw from the active validator set. Effective weight for committee selection is `self_bond + total_delegated` per validator (see `staking-spec.md` for delegation mechanics).
- Committee influence is stake-proportional. Correlated validator keys detected via stake-graph analysis (see `stake-graph-analysis-spec.md`) MUST be treated as a single entity for committee weight computation.
- Committee liveness operation is tiered by active validator count:
  - **Normal (67-100):** Full consensus, all transaction types permitted.
  - **Degraded (50-66):** Block production continues. Only critical transaction types permitted: `TransferTx`, `StakeBondTx`, `StakeRenewTx`, `UnbondRequestTx`, `WithdrawUnbondedTx`, `EvidenceTx`. Governance and fast-path transactions are queued until normal mode resumes.
  - **Emergency (0-49):** Block production halts. After K=500 idle blocks (~83 minutes), an emergency epoch transition is triggered: seed = previous VDF output, new committee sampled from ALL validators in `active` or `paused` states (not `unbonding`/`withdrawn`). New committee starts at full target size.
- Committee selection MUST be fully deterministic given the same epoch seed and validator pool.
- The system MUST use Malachite BFT (or protocol-equivalent) for consensus message passing.

### 1.3 Data Structures

```rust
struct Committee {
    epoch: u64,
    seed: [u8; 32],
    members: Vec<[u8; 32]>,  // validator_id list, exactly 100 entries
    weights: Vec<u64>,        // corresponding stake weights
}

struct BlockHeader {
    height: u64,
    parent_hash: [u8; 32],
    state_root: [u8; 32],        // SMT root
    transaction_root: [u8; 32],  // Merkle root over ordered txs
    committee_id: u64,
    proposer_id: [u8; 32],
    timestamp: u64,               // Unix seconds
    epoch: u64,
}

struct Block {
    header: BlockHeader,
    transactions: Vec<TransactionEnvelope>,
}

struct TransactionEnvelope {
    tx_type: TxType,
    tx_payload: Vec<u8>,
    approved_plan_id: Option<[u8; 32]>,
    gateway_signature: Option<Vec<u8>>,
}

enum TxType {
    TransferTx,
    StakingTx(StakingAction),        // bond, renew, unbond, withdraw
    DelegationTx(DelegationAction),  // delegate, undelegate, withdraw, set commission
    TaskCreateTx,                     // create bounty-funded task (FR-0194)
    GovernanceTx(GovernanceAction),   // propose, vote
    EvidenceTx,
    FastPathTx(FastPathAction),       // proposal, review, challenge
}

enum StakingAction {
    Bond,
    Renew,
    Unbond,
    Withdraw,
}

enum DelegationAction {
    Delegate,
    Undelegate,
    WithdrawDelegation,
    SetCommission,
}

enum GovernanceAction {
    Propose,
    Vote,
}

enum FastPathAction {
    Propose,
    Review,
    Challenge,
}
```

### 1.4 State Transitions

**Epoch lifecycle:** Each epoch spans 8,192 blocks (approximately 1 day). At epoch boundary:
1. Validators submit hashed commitments in the last k blocks of epoch N.
2. Reveal phase occurs in first k blocks of epoch N+1 (k = 10% of epoch length, rounded up).
3. VDF evaluated over concatenated reveal preimages + prior seed.
4. New committee sampled from active validator pool using VDF-derived seed.
5. At most 20% overlap between consecutive committees (80% minimum rotation).
6. A validator MUST NOT serve on the committee for more than 2 consecutive epochs. After 2 consecutive epochs, the validator is ineligible for 1 epoch.

**Block production:**
1. Proposer selected from committee via round-robin weighted by stake.
2. Proposer collects transactions from mempool, ordered by fee (highest first). Evidence and governance transactions receive governance-set fee discounts (see `p2p-wire-spec.md` §2).
3. Proposer constructs block and broadcasts proposal to committee.
4. Committee members validate and vote. Block committed on 2f+1 weighted votes.
5. Block height increments. Finality: single-block (no additional confirmations needed).

**Liveness tiers** (see §1.2 for action restrictions per tier):

| Tier | Validator Range | Behavior |
|------|----------------|----------|
| Normal | 67-100 | Full block production |
| Degraded | 50-66 | Block production continues, critical txs only |
| Emergency | 0-49 | Block production halts, auto-recovery after K=500 idle blocks via emergency epoch |

Degraded mode resumes normal mode when validator count returns to >= 67 (new validators bond, or paused validators resume). Emergency mode transitions to normal mode at the next epoch boundary after the emergency cycle.

### 1.5 Failure Behavior

- **Committee stall:** If committee drops below 50 validators, block production halts. After 500 idle blocks, emergency epoch transition triggers automatically using remaining `active` and `paused` validators. No governance override possible during stall — recovery is automatic via the emergency cycle or next epoch boundary.
- **Equivocation:** Two conflicting votes from same validator for same height/round triggers automatic evidence-based slash. See `staking-spec.md` Section 1.
- **Partition:** Network partition isolating >33% of committee causes liveness failure. Block production resumes on partition heal at next epoch boundary. No rollback needed.
- **VDF failure:** If insufficient reveals available (less than 33% of committee committed), seed falls back to `SHA3-256(previous_vdf_output || hash_of_all_block_headers_in_epoch_N-1 || epoch_number || concatenated_valid_reveals)`. The fallback uses only finalized/historical entropy (previous epoch's VDF output and block headers) plus any valid reveals that arrived. No current-epoch malleable data is used. Proposers cannot grind this fallback.

### 1.6 Versioning and Compatibility

- Consensus protocol version embedded in block header.
- Version bumps require governance `git:head` update and committee activation at epoch boundary.
- Backward compatibility: nodes running old version reject blocks with unknown version. Forks resolved by stake weight.
- Transaction types are extensible via governance only.

### 1.7 Conformance Test Hooks

- Verify genesis block initializes committee BFT with 100 validators and defined safety threshold.
- Verify TxType enum accepts all transaction types and dispatches correctly by action sub-enum.
- Verify that correlated validator keys are correctly clustered by stake-graph analysis and treated as a single entity for committee weight computation.
- Verify committee rotation produces at most 20% overlap between consecutive epochs.
- Verify no validator serves on more than 2 consecutive committees.
- Verify VDF-based seed is deterministic from same inputs.
- Verify block production halts when active committee < 50 (emergency) and resumes via emergency epoch or epoch boundary.
- Verify degraded mode (50-66) allows critical transactions and blocks governance/fast-path.
- Verify emergency auto-recovery triggers after 500 idle blocks with correct seed (previous VDF output).
- Verify 10-second median block time under normal load (p95 finality < 3 seconds).
- Verify transaction ordering is deterministic.

### 1.8 Trust-Assumption Inventory

- Cryptographic primitive ML-DSA-65
  - Justification: Post-quantum signature security for all transactions.
  - Trust-minimised alternative: ML-DSA is itself the post-quantum choice. Hybrid with classical ECDSA possible during transition.
- Malachite BFT library correctness
  - Justification: Proven BFT implementation; formal verification deferred (NFR-0030).
  - Trust-minimised alternative: Tendermint Core or custom BFT — both share same underlying protocol assumptions.
- Network liveness under partial synchrony
  - Justification: BFT safety holds; liveness requires eventual message delivery (GST model).
  - Trust-minimised alternative: None — all BFT consensus requires partial synchrony assumption.
- VDF correct sequential computation
  - Justification: Committee randomness depends on VDF being truly sequential (no parallel shortcuts).
  - Trust-minimised alternative: Multiple independent VDF constructions with cross-verification.

---

## Section 2: Sparse Merkle Tree State

### 2.1 Purpose

Define the Sparse Merkle Tree state commitment structure, key schema, and deterministic state transition rules.

### 2.2 Normative Behavior

- The system MUST recompute and commit the SMT state root in every block header.
- State keys MUST follow the canonical key prefix schema (0x01-0x0F per `state-model.md`).
- All entity values MUST be serialized in SCALE encoding for deterministic byte output.
- State transitions MUST be applied in strict transaction inclusion order within each block.
- The system MUST support inclusion/exclusion proofs for any state key at any committed height.
- Merkle tree construction MUST use deterministic key ordering (keys sorted lexicographically before insertion).
- Timestamps in entity data MUST be expressed as block heights, not wall-clock time.
- State size growth MUST be bounded to <1 GB per month with pruning (NFR-0002).

### 2.3 Data Structures

```rust
struct SMTNode {
    key: [u8; 32],
    value: Vec<u8>,
    hash: [u8; 32],
}

struct InclusionProof {
    key: [u8; 32],
    value: Vec<u8>,
    proof: Vec<[u8; 32]>,  // sibling hashes from leaf to root
    root: [u8; 32],
    height: u64,
}

enum KeyPrefix {
    Account = 0x01,
    Validator = 0x02,
    GovernanceProposal = 0x03,
    Committee = 0x04,
    ArtifactManifest = 0x05,
    Task = 0x06,
    TelemetryEnvelope = 0x07,
    SystemParams = 0x08,
    TrustStage = 0x0A,
    ActionPlan = 0x0B,
    AirdropPool = 0x0C,
    ReplicationLease = 0x0D,
    ReviewAssignment = 0x0F,
}

struct Account {
    account_id: [u8; 32],       // SHA3-256 of ML-DSA pubkey
    balance: u128,               // atto-AGX (u128 required for 10M AGX total supply at atto-AGX precision)
    nonce: u64,
    pubkey_hash: [u8; 32],
    pubkey: Option<Vec<u8>>,    // revealed on first spend
}
```

### 2.4 State Transitions

**Block execution pipeline:**
1. Verify block header, proposer signature, parent hash.
2. Apply transactions sequentially by inclusion index.
3. For each transaction: validate execution rules, update relevant state keys, record events.
4. After all transactions: recompute SMT root by replaying key-value inserts in sorted order.
5. Commit state root into block header.

**Account lifecycle:** Created on first inbound transfer or airdrop → Active (perpetual) → Prugable if balance = 0 and nonce = 0 for 100,000 blocks.

**Task creation (TaskCreateTx):**
1. PDP validates action_plan fields: schema, signature, policy bundle, nonce uniqueness, TTL, creator trust stage >= `trusted`, creator active task count < stage-based cap, creator balance >= `bounty_agx + estimated_tx_fee`, `seed_ref` references valid seed idea, `topic_id` matches seed.
2. EIP-1559 fee deducted from creator balance (base fee burned, priority fee to block proposer).
3. State machine: debit `bounty_agx` from creator balance, credit task escrow.
4. Record `TaskRecord { task_id = SHA3-256(action_plan), topic_id, seed_ref, funder = creator_id, sponsor_id, bounty_agx, metadata_hash, required_skills_hash, status = Open, escrow_status = Locked, created_at_height, expires_at_height }`.
5. Emit `TaskCreated(task_id, topic_id, bounty_agx, metadata_hash)` event for gossip/DHT propagation (C7).

### 2.5 Failure Behavior

- **State root mismatch:** A node producing a different state root than the committed block root halts consensus participation. Logs discrepancy for operator investigation.
- **Serialization non-determinism:** Any non-deterministic field in hash input produces divergent state roots. Schema validation rejects non-canonical serializations.
- **Pruning:** Archived entities (telemetry envelopes, expired manifests, consumed action plans) are pruned by archive nodes after retention windows. Validating nodes keep SMT root and current state only.

### 2.6 Versioning and Compatibility

- State schema version is implicit in SCALE encoding: new fields are appended; field reordering is a breaking change.
- Breaking state schema changes require governance proposal with migration path.
- SMT implementation (hash function, tree structure) is pinned by `git:head`.

### 2.7 Conformance Test Hooks

- Verify state root is deterministic: two nodes receiving identical ordered transactions produce identical post-block state roots.
- Verify inclusion proof for any key at any height validates against the committed state root.
- Verify nonce enforcement: transaction with nonce != account.nonce + 1 is rejected.
- Verify replay protection: consumed plan_id is tracked and rejected on resubmission.
- Verify state size growth < 1 GB/month with archival node pruning.
- Verify first-spend pubkey reveal: SHA3-256(pubkey_reveal) == sender_address.

### 2.8 Trust-Assumption Inventory

- SCALE encoding determinism
  - Justification: Identical values must produce identical byte outputs across platforms.
  - Trust-minimised alternative: Borsh, Protobuf (with canonicalization), or CBOR deterministic — all require same degree of trust in serializer correctness.
- SHA3-256 collision resistance
  - Justification: Address compression, content addressing, and state key derivation depend on 128-bit collision resistance.
  - Trust-minimised alternative: SHA-256 or Blake2b — similar security properties.
- SMT implementation correctness
  - Justification: State root is the foundation of consensus convergence.
  - Trust-minimised alternative: Patricia Merkle Trie — different tradeoffs but same trust requirement.
