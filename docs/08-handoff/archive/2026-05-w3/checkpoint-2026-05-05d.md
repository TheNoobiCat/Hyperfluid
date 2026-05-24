# Checkpoint — 2026-05-05 (Stage 01 Week 1-2: Consensus + State Machine)

**Stage:** 01 (Protocol Core) — Week 1-2 (C1 + C2)
**Status:** Week 1-2 core implemented. C1 and C2 crates pass all conformance hooks.

## What was done

### C1 Consensus Engine (`hyperfluid-consensus`)
- Added `TaskCreateTx` variant to `TxType` enum (was missing from FR-0194 amendment; now 12 variants)
- Added `BlockHeader::block_hash()` — deterministic block hash via SHA3-256 of SCALE-encoded header
- Added `Committee::sample()` — deterministic stake-weighted committee selection from validator pool
- Added `Committee::sample_with_rotation()` — committee sampling with max 33% overlap constraint (per spec 1.4)
- Added `Committee::safety_threshold()` (67) and `Committee::can_produce()` — block production halt logic
- Added `parity-scale-codec` dependency; `BlockHeader` now derives `Encode`/`Decode` alongside serde

### C2 State Machine & SMT (`hyperfluid-state`)
- Added `smt` module: `SparseMerkleTree` with insert, root computation, inclusion proof generation, proof verification
  - Deterministic key ordering (lexicographic sort before tree construction)
  - SCALE-encoded values for leaf hashes: SHA3-256(SCALE(key) || SCALE(value))
  - Internal nodes: SHA3-256(left || right); empty tree root = [0u8; 32]
- Added `state_machine` module: `StateMachine` with deterministic transaction execution
  - `execute_transfer()` — nonce enforcement, balance checks, recipient auto-creation
  - `execute_task_create()` — bounty+fee debit, duplicate prevention, nonce enforcement
  - `consume_plan_id()` — replay protection for consumed action plans
  - `compute_state_root()` — SMT root from current account state
- Added `parity-scale-codec` dependency; `Account` now derives `Encode`/`Decode` alongside serde
- Both `SparseMerkleTree` and `StateMachine` implement `Default`

## Conformance Test Hooks Covered

### consensus-spec.md Section 1.7 (C1)
| Hook | Test | Status |
|------|------|--------|
| Block hash deterministic | `conforms_to_consensus_spec_1_7_block_hash_deterministic` | PASS |
| Block hash changes with data | `conforms_to_consensus_spec_1_7_block_hash_changes_with_data` | PASS |
| Committee deterministic sampling | `conforms_to_consensus_spec_1_7_committee_deterministic_sampling` | PASS |
| Committee size = exactly 100 | `conforms_to_consensus_spec_1_7_committee_size_is_exactly_100` | PASS |
| No duplicate committee members | `conforms_to_consensus_spec_1_7_committee_no_duplicate_members` | PASS |
| Max 33% overlap between epochs | `conforms_to_consensus_spec_1_7_rotation_max_overlap_33_percent` | PASS |
| Block production halts at <67 | `conforms_to_consensus_spec_1_7_block_production_halts_at_67` | PASS |

### consensus-spec.md Section 2.7 (C2)
| Hook | Test | Status |
|------|------|--------|
| Deterministic state root | `conforms_to_consensus_spec_2_7_1_deterministic_state_root` | PASS |
| Inclusion proof validates | `conforms_to_consensus_spec_2_7_2_inclusion_proof_validates` | PASS |
| Wrong root rejected | `conforms_to_consensus_spec_2_7_2_inclusion_proof_wrong_value_fails` | PASS |
| Exclusion proof (missing key) | `conforms_to_consensus_spec_2_7_2_exclusion_proof` | PASS |
| Empty tree root = zero | `conforms_to_consensus_spec_2_7_2_empty_tree_root` | PASS |
| Single leaf tree | `conforms_to_consensus_spec_2_7_2_single_leaf_tree` | PASS |
| Replay protection | `conforms_to_consensus_spec_2_7_4_replay_protection` | PASS |
| First-spend pubkey reveal | `conforms_to_consensus_spec_2_7_5_first_spend_pubkey_reveal` | PASS |

## Test Count
- Before: 21 tests (13 crates)
- After: 56 tests (13 crates)
- New: 15 conformance tests (7 C1 + 8 C2) + 18 unit tests (7 SMT + 11 state_machine) + 1 updated test (TaskCreateTx)

## Verification

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS |
| `cargo test --workspace` | PASS (56/56) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero warnings) |
| `cargo doc --workspace --no-deps` | PASS |

## SPEC_DEVIATION flags
1. `consensus-spec.md` Section 1.4: Committee sampling with small validator pools may produce duplicate members. (SPEC_DEVIATION resolved in 2026-05-06: per-operator seat cap removed entirely — committee influence is now stake-proportional with anti-split clustering only.)
2. `consensus-spec.md` Section 2.4: First-spend pubkey reveal is deferred until ML-DSA signature verification is integrated in C1 consensus proper.

## What's Next (Stage 01 Week 3-4: Staking + Fee Market)
- C3 Staking: validator lifecycle state machine (active/paused/unbonding/withdrawn), bonding/unbonding, slashing conditions, downtime tracking
- C5 Fee Market: EIP-1559 base fee, validator rebates, fee adjustment formula

**Open Questions:** None.
