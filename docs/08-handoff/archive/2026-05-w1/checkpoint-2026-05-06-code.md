# Checkpoint — 2026-05-06 (Stage 01 Week 3-4: Pending Code Changes + Staking + Fee Market)

**Stage:** 01 (Protocol Core) — Week 3-4 applied
**Status:** 5 pending code changes applied. C3 Staking base + C5 Fee Market implemented.

## What was done

### Pending Code Change 1: Overlap 33%→20% + two-epoch recency guard
- `max_overlap` changed from 33% to 20% in `Committee::sample_with_rotation()`
- Added `ineligible: &[Hash32]` parameter to `sample()` and `sample_with_rotation()` for two-epoch recency guard
- Priority-ordered constraint enforcement: (1) all constraints, (2) overlap relaxed, (3) ineligible relaxed
- Test: `conforms_to_consensus_spec_1_7_rotation_max_overlap_20_percent` — PASS
- Test: `conforms_to_consensus_spec_1_7_two_epoch_recency_guard` — PASS
- Test: `conforms_to_consensus_spec_1_7_two_epoch_recency_edge_case` — PASS

### Pending Code Change 2: VDF fallback formula
- `Committee::compute_vdf_fallback()` — SHA3-256(previous_vdf_output || epoch_headers_hash || epoch_number || valid_reveals)
- Uses only finalized/historical entropy
- Test: `conforms_to_consensus_spec_1_7_vdf_fallback_deterministic` — PASS
- Test: `conforms_to_consensus_spec_1_7_vdf_fallback_changes_with_input` — PASS
- Test: `conforms_to_consensus_spec_1_7_vdf_fallback_empty_reveals` — PASS

### Pending Code Change 3: Committee stall 3-tier thresholds
- Replaced single `SAFETY_THRESHOLD = 67` with 3-tier model:
  - `NORMAL_THRESHOLD = 67`
  - `DEGRADED_THRESHOLD = 50`
  - `EMERGENCY_IDLE_BLOCKS = 500`
- Added `CommitteeMode` enum (Normal, Degraded, Emergency)
- `Committee::committee_mode(active_count) -> CommitteeMode`
- `Committee::can_produce()` returns true for Normal and Degraded, false for Emergency
- `Committee::emergency_transition()` samples new committee from all active+paused validators
- Test: `conforms_to_consensus_spec_1_7_committee_three_tier_stall` — PASS
- Test: `conforms_to_consensus_spec_1_7_emergency_idle_blocks_constants` — PASS
- Test: `conforms_to_consensus_spec_1_7_emergency_transition` — PASS

### Pending Code Change 4: Stake-graph analysis
- NEW: `crates/hyperfluid-staking/src/graph.rs` — full implementation
- `ClusterRecord`, `ClusterDetectionResult`, `FundingEdge`, `ClusterAncestorType`
- `build_stake_funding_graph()` — directed graph from on-chain edges
- `detect_clusters()` — N=3 hop ancestor trace, airdrop agent exclusion
- `compute_committee_weights()` — splits cluster total_stake evenly among members
- `prune_old_edges()` — GC edges older than threshold_height
- Test: `cluster_detection_same_funder_within_hops` — PASS
- Test: `no_cluster_for_independent_funding` — PASS
- Test: `hop_limit_3_excludes_4_hop_chain` — PASS
- Test: `airdrop_agent_is_not_clustering` — PASS
- Test: `cluster_detection_deterministic` — PASS
- Test: `compute_weights_splits_cluster_stake` — PASS
- Test: `prune_edges_removes_old` — PASS

### Pending Code Change 5: Delegation subsystem + TxType collapse
- **TxType collapsed** from 12 variants to 7 base types with sub-enums:
  - `TransferTx`, `StakingTx(StakingAction)`, `DelegationTx(DelegationAction)`, `TaskCreateTx`, `GovernanceTx(GovernanceAction)`, `EvidenceTx`, `FastPathTx`
- **Added sub-action enums:** `StakingAction` (Bond, Renew, Unbond, Withdraw), `DelegationAction` (Delegate, Undelegate, WithdrawDelegation, SetCommission), `GovernanceAction` (Propose, Vote)
- **Staking types updated:**
  - `ValidatorRecord` gained `self_bond`, `total_delegated`, `commission_rate`
  - NEW: `DelegationRecord`, `DelegationStatus` (Active, Unbonding, Withdrawn)
  - `SystemParameters`: added `min_self_bond`, `min_delegation`, `max_commission_rate`, `delegation_unbond_delay`
- **State machine delegation handlers:**
  - `execute_delegate()` — delegate AGX to validator, enforces min_delegation, self-delegate rejection, balance checks
  - `execute_undelegate()` — initiate 7-day unbonding timer
  - `execute_withdraw_delegation()` — withdraw after unbonding delay expires
  - `execute_set_commission()` — set commission rate (max 20%)
- **15 delegation conformance tests** — all PASS (delegate, undelegate, withdraw, set_commission, positive + negative assertions)
- **4 staking tx type tests** — all PASS
- **2 governance tx type tests** — all PASS

### C5 Fee Market (EIP-1559)
- `FeeMarketState`, `FeeConfig` with default parameters
- `compute_next_base_fee()` — deterministic integer-only EIP-1559 adjustment with 12.5% cap, min floor, adjustment_denominator=8
- `compute_tx_fee()`, `tx_meets_min_fee()`, `compute_burn_amount()`
- `compute_validator_rebate()` — proportional to stake share
- `sender_within_mempool_limit()` — per-sender cap
- **14 fee market tests** — all PASS (increase, decrease, cap, floor, rebate, burn, mempool)

## Test Count
- Before: 57 tests (13 crates)
- After: **103 tests** (13 crates)
- New: 46 tests (15 C1 conformance + 7 stake-graph + 15 staking conformance + 14 fee market - 5 superseded/updated)

## Verification

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS (13 crates) |
| `cargo test --workspace` | PASS (103/103) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero) |
| `cargo doc --workspace --no-deps` | PASS |
| Determinism sweep (floating-point) | PASS (zero hits) |
| Determinism sweep (wall-clock/random) | PASS (zero hits) |

## SPEC_DEVIATION flags
1. `consensus-spec.md` Section 2.4: First-spend pubkey reveal deferred until ML-DSA sig verification integrated in C1 consensus proper. (Carried from prior checkpoint)
2. `staking-spec.md` Section 1.3: `liveness_bitmap` is `Vec<u8>` instead of `[u8; 1024]` due to serde derive limitations. (Carried from prior checkpoint)

## What's Next (Stage 01 Week 5-6: P2P + Artifact Storage)
- C7 P2P Networking: peer discovery, gossip protocol, mempool relay (5 crates: p2p, governance, fee-market, consensus, node)
- C8 Artifact Availability: content-addressed storage (2 crates: artifact, state)
- Delegated to next session.
