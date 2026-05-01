## FR-0001: Committee BFT from Genesis

**Category:** Consensus

**Statement:** The system shall use committee-based BFT consensus from genesis, with a rotating epoch committee of 100 validators.

**Rationale:** Full-set BFT does not preserve liveness under rapid growth. Committee BFT bounds communication overhead while maintaining safety. See `agx-committee-bft-and-governance.md` Section 4, Tradeoff 1.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 4 (Architecture)
- `agx-committee-bft-and-governance.md` Section 5 (Committee BFT from day 1)
- `decentralization-and-stack-benchmark.md` Section 9 (Recommended Architecture)

**Acceptance Criteria:**
- [ ] Genesis block initializes committee BFT with Malachite (or equivalent) and a defined committee size of 100.
- [ ] Block production halts if committee size drops below safety threshold (67 active validators for 2f+1 quorum at f=33).
- [ ] Committee rotation occurs deterministically at each epoch boundary without manual intervention.

**Dependencies:** FR-0011 (validator lifecycle), FR-0041 (P2P networking)
**Tags:** must-have

---

## FR-0002: Epoch Committee Sampling with Anti-Concentration Caps

**Category:** Consensus

**Statement:** The system shall sample epoch committees using a stake-weighted draw with a per-operator cap of 15% of committee seats and anti-split detection via correlated key analysis.

**Rationale:** Prevents stake concentration and whale-splitting from capturing committee majority. See `agx-committee-bft-and-governance.md` Section 5 (Committee BFT from day 1), lines 147-170.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 147-170
- `decentralization-and-stack-benchmark.md` Section 7 (Failure Modes)

**Acceptance Criteria:**
- [ ] A single operator identity cannot hold more than 15% of committee seats in any epoch.
- [ ] Correlated validator keys detected via stake-graph analysis are treated as a single operator for cap purposes.
- [ ] Committee sampling is deterministic given the same epoch seed and validator pool.

**Dependencies:** FR-0001, FR-0011
**Tags:** must-have

---

## FR-0003: VDF-Based Epoch Randomness

**Category:** Consensus

**Statement:** The system shall derive epoch seeds from a verifiable-delay function (VDF) evaluated over validator commitment-reveal inputs, with a fallback to a deterministic hash if insufficient reveals are available.

**Rationale:** Eliminates last-revealer bias and grinding attacks in committee selection. See `agx-committee-bft-and-governance.md` Section 5, lines 147-161.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 147-161
- `decentralization-and-stack-benchmark.md` Section 5 (Core Mechanisms)

**Acceptance Criteria:**
- [ ] Validators submit hashed commitments in block N and reveal preimages in block N+k.
- [ ] VDF sequential evaluation time exceeds 2x the reveal window to prevent grinding.
- [ ] VDF proof verification is O(1) per epoch.
- [ ] Fallback seed formula `SHA3-256(previous_seed + block_hash_chain + epoch_number)` is used when insufficient valid reveals exist.

**Dependencies:** FR-0001, FR-0002
**Tags:** must-have

---

## FR-0004: Committee Partial Overlap Between Epochs

**Category:** Consensus

**Statement:** The system shall retain at most 33% of committee members between consecutive epochs, rotating at least 67% each epoch.

**Rationale:** Prevents abrupt liveness loss from mass validator churn while maintaining fresh anti-capture properties. See `agx-committee-bft-and-governance.md` Section 5, lines 169-170.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 169-170

**Acceptance Criteria:**
- [ ] Overlap ratio between epoch N and N+1 committees never exceeds 33%.
- [ ] Committee transition occurs atomically at epoch boundary without consensus stall.

**Dependencies:** FR-0001, FR-0002
**Tags:** must-have

---

## FR-0005: Post-Quantum Transaction Signatures (ML-DSA)

**Category:** Consensus

**Statement:** The system shall use ML-DSA (CRYSTALS-Dilithium, security level ML-DSA-65) for all transaction and message signatures, with SHA3-256 for hashing.

**Rationale:** Protects against future quantum computer attacks on ECDSA. Larger signatures require batching for throughput. See `agx-committee-bft-and-governance.md` Section 5 (Cryptography).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 76-86

**Acceptance Criteria:**
- [ ] All transactions include ML-DSA-65 signatures verifiable by any node.
- [ ] Signature batching is supported for throughput optimization.
- [ ] Key derivation follows BIP-32 style hierarchical deterministic wallets adapted for ML-DSA.

**Dependencies:** none
**Tags:** must-have

---

## FR-0006: First-Spend Public Key Reveal

**Category:** Consensus

**Statement:** The system shall require the first outbound transaction from any address to reveal the public key and prove hash binding to the address.

**Rationale:** Enables address compression (SHA3-256 hash) while preventing address reuse attacks. See `agx-committee-bft-and-governance.md` Section 5 (Addressing and first-spend reveal).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 87-91

**Acceptance Criteria:**
- [ ] Address format is `SHA3-256(pubkey)`.
- [ ] First outbound `TransferTx` must include `pubkey_reveal` field.
- [ ] Node rejects first-spend transactions where `SHA3-256(pubkey_reveal) != sender_address`.

**Dependencies:** FR-0005
**Tags:** must-have

---

## FR-0007: Transaction Types and Schema

**Category:** Consensus

**Statement:** The system shall support the following canonical transaction types with deterministic schemas: TransferTx, StakeBondTx, StakeRenewTx, UnbondRequestTx, WithdrawUnbondedTx, GovernanceProposeTx, GovernanceVoteTx, EvidenceTx, FastPathProposalTx, FastPathReviewTx, FastPathChallengeTx.

**Rationale:** Explicit minimal transaction taxonomy prevents ambiguity and enables deterministic state transitions. See `agx-committee-bft-and-governance.md` Section 5 (Transaction types).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 66-75
- `topic-fastpath-protocol-spec.md` Section 5 (Protocol message types)

**Acceptance Criteria:**
- [ ] Each transaction type has a canonical JSON/binary schema with required and optional fields documented.
- [ ] Unknown transaction type bytes are rejected at admission.
- [ ] Schema changes require governance `git:head` update.

**Dependencies:** FR-0005, FR-0006
**Tags:** must-have

---

## FR-0008: Strict Account Nonce and Chain-Domain Separation

**Category:** Consensus

**Statement:** The system shall enforce strictly monotonic account nonces and chain-domain separation for all transactions.

**Rationale:** Prevents replay attacks across accounts and chains. See `agx-committee-bft-and-governance.md` Section 5 (Addressing and first-spend reveal).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 90-91
- `network-policy-engine-spec.md` Section 5 (Replay protection)

**Acceptance Criteria:**
- [ ] Each account maintains a monotonically increasing nonce; transactions with nonce != expected are rejected.
- [ ] Replay of a valid transaction on a different chain ID is rejected.
- [ ] Replay of a consumed `action_plan_id` is rejected.

**Dependencies:** FR-0007
**Tags:** must-have

---

## FR-0009: 10-Second Block Time Target with Single-Block Finality

**Category:** Consensus

**Statement:** The system shall target a 10-second block time with single-block BFT finality (no additional confirmation blocks required).

**Rationale:** Fast finality is essential for agent coordination loops. See `decentralization-and-stack-benchmark.md` Section 5 (Production targets).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 5, lines 66-70

**Acceptance Criteria:**
- [ ] Median block production interval is 10 seconds under normal load.
- [ ] Once a block is committed, it is irreversible without 33% Byzantine collusion.
- [ ] Finality latency p95 is under 3 seconds for committee sizes up to 100.

**Dependencies:** FR-0001
**Tags:** must-have

---

## FR-0010: SMT State Commitments

**Category:** Consensus

**Statement:** The system shall use a Sparse Merkle Tree (SMT) for compact state commitments, storing balances, staking state, committee seed, liveness status, and `git:head`.

**Rationale:** Enables lightweight state verification and witnesses without full history. See `agx-committee-bft-and-governance.md` Section 4 (Architecture).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 4, lines 31-39
- `decentralization-and-stack-benchmark.md` Section 4 (Architecture)

**Acceptance Criteria:**
- [ ] State root is recomputed and committed in every block header.
- [ ] SMT supports efficient inclusion/exclusion proofs for any state key.
- [ ] State size growth is bounded to <1GB per month with pruning.

**Dependencies:** FR-0007
**Tags:** must-have
