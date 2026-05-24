# Checkpoint — 2026-05-06 (Architecture Amendments: Committee BFT Hardening + Delegation)

**Stage:** 01 (Protocol Core) — Pre-Week-3 Architecture Amendments
**Status:** All doc changes applied. Code changes remain pending (see below).

## IMPORTANT: Pending Code Changes

BEFORE implementing any Stage 01 Week 3-4 code (or if resuming after a gap), the following code changes MUST be applied first. These correspond to architecture amendments made on 2026-05-06.

### Pending Code Change 1: Overlap 33%→20% + two-epoch guard

**Files:** `crates/hyperfluid-consensus/src/types.rs`
**Change:**
- `max_overlap` from `(committee_size * 33).div_ceil(100)` → `(committee_size * 20).div_ceil(100)`
- Add `consecutive_epochs: u8` tracking to committee selection
- Add two-epoch recency guard in sampling loop: skip validators who served 2 consecutive epochs

**Files:** `crates/hyperfluid-consensus/tests/conformance_consensus_spec.rs`
- Update overlap test from 33% → 20%
- Add test for two-epoch recency limit

**Spec reference:** `consensus-spec.md` §1.4 (updated), FR-0004 (updated)

---

### Pending Code Change 2: VDF fallback formula

**Files:** `crates/hyperfluid-consensus/src/types.rs` (or new fallback module)
**Change:**
- Implement the new fallback: `SHA3-256(previous_vdf_output || hash_of_epoch_N-1_block_headers || epoch_number || concatenated_valid_reveals)`
- Remove the old `block_hash_chain` formula
- Add method `compute_vdf_fallback(previous_vdf_output, previous_epoch_header_hashes, epoch_number, valid_reveals) -> Hash32`
- Also update the `commitment_reveal.rs` or equivalent module so the reveal window tracks reveal participation count
- Fallback triggers when <33% of committee reveals

**Spec reference:** `consensus-spec.md` §1.5 (updated), FR-0003 (updated)

---

### Pending Code Change 3: Committee stall tiered thresholds

**Files:** `crates/hyperfluid-consensus/src/types.rs`
**Change:**
- Replace `SAFETY_THRESHOLD = 67` with three-tier constants:
  - `NORMAL_THRESHOLD = 67`
  - `DEGRADED_THRESHOLD = 50`
  - `EMERGENCY_THRESHOLD = 0` (effectively <50 triggers emergency)
  - `EMERGENCY_IDLE_BLOCKS = 500`
- Add `fn committee_mode(active_count: u64) -> CommitteeMode` returning Normal/Degraded/Emergency
- Add `fn can_produce_tx_type(tx_type: TxType, mode: CommitteeMode) -> bool` — governance/fast-path blocked in degraded mode
- Add `fn emergency_transition(validator_pool, seed, active_validators) -> Committee` for auto-recovery
- Update `can_produce()` to return false only in emergency mode

**Files:** `crates/hyperfluid-consensus/tests/conformance_consensus_spec.rs`
- Update existing halt test for three tiers
- Add degraded-mode tx restriction test
- Add emergency transition test

**Spec reference:** `consensus-spec.md` §1.2, §1.5 (updated), FR-0001 (updated), `failure-model.md` F-01 (updated)

---

### Pending Code Change 4: Stake-graph analysis implementation

**Files (NEW):** `crates/hyperfluid-staking/src/graph.rs`
**Change:**
- Implement `ClusterDetectionResult` struct and `build_stake_funding_graph()` function
- Implement N-hop ancestor tracing (N=3) from SMT state
- Implement `detect_clusters(validators, edges, max_hops=3) -> ClusterDetectionResult`
- Implement `compute_committee_weights(clusters) -> HashMap<ValidatorId, EffectiveWeight>` mapping cluster weight sum
- Implement `prune_old_edges(threshold_height)` GC

**Files:** `crates/hyperfluid-staking/src/lib.rs` — add ClusterRecord struct, add `graph` module
**Files (NEW):** `crates/hyperfluid-staking/tests/conformance_stake_graph.rs`

**Spec reference:** `stake-graph-analysis-spec.md` (new), FR-0002 (updated), FR-0183 (updated)

---

### Pending Code Change 5: Delegation subsystem

**Files:** `crates/hyperfluid-staking/src/lib.rs`
**Change:**
- Add `DelegationRecord` struct, `DelegationStatus` enum
- Add `commission_rate: u8` and `total_delegated: u128` to `ValidatorRecord`
- Add `self_bond: u128` field to replace `bonded_stake: u128`
- Add delegation state transitions: `handle_delegate_tx()`, `handle_undelegate_tx()`, `handle_withdraw_delegation_tx()`, `handle_set_commission_tx()`
- Add `propagate_slash(delegator_shares, slash_pct) -> HashMap<AccountId, u128>` for proportional slash distribution

**Files:** `crates/hyperfluid-consensus/src/types.rs`
- Add `DelegateTx`, `UndelegateTx`, `WithdrawDelegationTx`, `SetCommissionTx` to `TxType` enum

**Files:** `crates/hyperfluid-state/src/state_machine.rs`
- Add delegation execution handlers: `execute_delegate()`, `execute_undelegate()`, `execute_withdraw_delegation()`, `execute_set_commission()`

**Files:** `crates/hyperfluid-staking/tests/conformance_staking_spec.rs`
- Add delegation lifecycle tests

**Spec reference:** `staking-spec.md` §1 (updated), `consensus-spec.md` §1.3 (updated), ADR-0015 (new), FR-0020a (new)

---

## What was done (doc changes only - all 2026-05-06)

### Amendment 1: Remove 15% operator cap (10 files)
| File | Change |
|------|--------|
| `consensus-spec.md` | Removed per-operator cap; committee influence is stake-proportional; anti-split clustering prevents Sybil evasion |
| `FR-0001-0010-consensus-and-bft.md` | FR-0002 rephrased as "anti-split clustering" with no seat cap |
| `FR-0176-0190-incentives-and-airdrop.md` | FR-0183: removed operator cap, kept stake-graph + overlap limits |
| `ADR-0007-committee-bft-vdf.md` | Decision updated: "anti-split clustering" replaces "15% per-operator cap" |
| `state-model.md` | COMMITTEE description: removed "Max 15% per operator" |
| `failure-model.md` | F-07: removed operator cap from mitigations |
| `agx-committee-bft-and-governance.md` | 4 sections updated to remove cap references |
| `decentralization-and-stack-benchmark.md` | Pseudocode and failure modes updated |
| `stage-01-protocol-core.md` | Work item and risk area updated |
| `checkpoint-2026-05-05d.md` | SPEC_DEVIATION for deferred cap enforcement resolved |

### Amendment 2: Overlap 33%→20% + two-epoch recency (6 files)
| File | Change |
|------|--------|
| `consensus-spec.md` | §1.4 step 5: 33%→20%, step 6: two-epoch recency added. §1.7: new conformance hook |
| `FR-0001-0010-consensus-and-bft.md` | FR-0004: 33%→20% + two-epoch guard added |
| `state-model.md` | COMMITTEE: 67%→80% rotation, 33%→20% overlap |
| `failure-model.md` | F-01, F-07: overlap references updated |
| `agx-committee-bft-and-governance.md` | Parameter and description updated |
| `ADR-0007.md` | Decision text updated to 20% |

### Amendment 3: VDF fallback hardened (4 files)
| File | Change |
|------|--------|
| `consensus-spec.md` | §1.5: fallback now uses `SHA3-256(previous_vdf_output \|\| epoch_N-1_headers_hash \|\| epoch_number \|\| valid_reveals)` |
| `FR-0001-0010-consensus-and-bft.md` | FR-0003: acceptance criteria updated |
| `agx-committee-bft-and-governance.md` | Fallback formula updated with immutable entropy |
| `ADR-0007.md` | Consequences updated |

### Amendment 4: New stake-graph-analysis-spec.md (9 files)
| File | Change |
|------|--------|
| **NEW:** `stake-graph-analysis-spec.md` | Full spec: N=3 hop ancestor trace, clustering algorithm, committee weight integration |
| `consensus-spec.md` | References the new spec |
| `FR-0001-0010-consensus-and-bft.md` | References the spec |
| `state-model.md` | Cluster detection description added |
| `failure-model.md` | F-07 updated |
| `agx-committee-bft-and-governance.md` | Updated to reference the spec |
| `decentralization-and-stack-benchmark.md` | Updated |
| `stage-01-protocol-core.md` | Updated work item |
| `traceability-matrix.md` | FR-0002 and ADR-0007 updated |

### Amendment 5: Committee stall tiered fallback (5 files)
| File | Change |
|------|--------|
| `consensus-spec.md` | §1.2: three-tier liveness table (Normal 67-100 / Degraded 50-66 / Emergency 0-49). §1.5: emergency auto-recovery after 500 idle blocks |
| `FR-0001-0010-consensus-and-bft.md` | FR-0001: three-tier acceptance criteria |
| `failure-model.md` | F-01: three-tier recovery path |
| `components.md` | Committee stall scenario updated |
| `stage-01-protocol-core.md` | Risk area updated |

### Amendment 6: Delegation subsystem (14 files)
| File | Change |
|------|--------|
| **NEW:** `ADR-0015-stake-delegation.md` | Architecture decision: delegation model, parameters, tx types |
| `consensus-spec.md` | §1.2: committee weight uses `self_bond + total_delegated`. §1.3: 4 new tx types (DelegateTx, UndelegateTx, WithdrawDelegationTx, SetCommissionTx) |
| `staking-spec.md` | §1.3: ValidatorRecord gains `self_bond`, `total_delegated`, `commission_rate`. NEW: DelegationRecord, DelegationStatus. §1.4: delegation transitions added. §1.5: delegation failure modes added |
| `FR-0011-0020-staking-and-validator-lifecycle.md` | NEW: FR-0020a (Stake Delegation) with full acceptance criteria |
| `state-model.md` | VALIDATOR: self_bond + total_delegated fields. NEW: DELEGATION entity (key prefix 0x10) |
| `failure-model.md` | NEW: F-11 (Delegator Abuse), F-12 (Delegation Power Concentration) |
| `components.md` | C3 responsibility updated with delegation |
| `agx-committee-bft-and-governance.md` | Staking section: delegation model, tx types, commission, slashing propagation |
| `stage-01-protocol-core.md` | Week 3-4: delegation added to C3 work items |
| `traceability-matrix.md` | FR-0020a, ADR-0015 added |
| `requirements/index.md` | FR-0020a added to staking section |

## Test Count
- Before: 57 tests (13 crates) — no change (doc-only session)
- All existing tests unchanged

## Verification
| Check | Result |
|-------|--------|
| Doc changes applied | All 6 amendments complete across ~40 files |
| Code changes | PENDING (see Pending Code Changes section above) |
| SPEC_DEVIATION flags | `checkpoint-2026-05-05d.md` deferred cap enforcement — RESOLVED (cap removed entirely) |

## Round 2 — Architecture Simplification (applied same session)

| Fix | Files | Key Changes |
|-----|-------|-------------|
| Circuit breaker killed | ~15 | Removed 3-tier CB, 22 thresholds, IncidentRecord, CircuitBreakerState from state-model. EIP-1559 base fee is sole congestion mechanism. |
| Mempool lanes removed | ~10 | Replaced 4-lane reservation with single fee-ordered pool + evidence/governance fee discounts. ADR-0006 superseded. |
| Trust ladder simplified | ~10 | 4 stages → 2 (untrusted/trusted). Removed reputation vector, decay, reviewer diversity, identity age. ADR-0010 superseded. |
| PDP simplified | ~15 | 10-step rule chain → 5 steps. Removed RiskClass, role/trust, ACL, taint, risk step-up, plan binding. Protocol no longer does LLM safety. DenyReasons: 12→5. |
| Review engine simplified | ~10 | 3 phases → 2 (removed objective checks). Fixed payout, 1 independence constraint (operator cluster). Removed clawback, quality-weighted scoring, reputation decay. ADR-0008 superseded. |
| TxType collapsed | ~12 | 16 variants → 7 base types (TransferTx, StakingTx, DelegationTx, TaskCreateTx, GovernanceTx, EvidenceTx, FastPathTx) with action sub-enums. |

## Test Count
- Still 57 tests (13 crates) — no test changes (doc-only session)

## Verification
| Check | Result |
|-------|--------|
| Doc changes applied | Round 1 (6 BFT+delegation amendments) + Round 2 (6 architecture simplifications) across ~72 files total |
| Code changes | PENDING (see Pending Code Changes section above — unchanged from prior) |

## What's Next (Stage 01 Week 3-4: Staking + Fee Market)
- **FIRST:** Apply the 5 pending code changes from Round 1 (overlap, VDF, stall, stake-graph, delegation)
- Then continue with Week 3-4: C3 Staking (simplified trust ladder, simplified delegation via sub-enums, simplified PDP) + C5 Fee Market

## Open Questions
None.
