# Checkpoint 2026-05-17b — GAP-02 Fill: Validator Lifecycle + Staking Dispatch + Fee Market

## Summary

Filled the remaining actionable items in GAP-02 (Stage 01 Week 3-4 — Staking + Fee Market).
Validator lifecycle (bond/unbond/withdraw/renew) implemented in StateMachine, all 8 StakingTx
and DelegationTx sub-actions dispatched in ConsensusDriver, and FeeMarket integrated into block
production with EIP-1559 base fee adjustment per block.

## Gaps Investigated

| Gap ID | Description | Still Present | Resolved | Evidence |
|--------|-------------|:---:|:---:|----------|
| GAP-01 | Malachite BFT protocol wiring | Yes | No | Zero Malachite imports in source. ~1,500 lines outstanding per ADR-0018. Not a Stage 02 blocker. |
| GAP-02 | Staking lifecycle execution | Partial | Partial | Validator bond/unbond/withdraw/renew + delegation dispatch + fee market done. Slashing/rewards deferred to Stage 03. |
| GAP-03 | P2P sockets / disk I/O | No | Yes | Fully resolved 2026-05-17. Verified: `tcp.rs`, `store.rs`, `multi_node_test.rs`. |
| GAP-04 | Node binary sleep stub | No | Yes | Fully resolved 2026-05-17. Verified: `ConsensusDriver::run_block_loop` produces real blocks. |

## Gaps Filled (This Checkpoint)

### GAP-02: Validator Lifecycle + Fee Market

**Files changed:**
- `crates/hyperfluid-state/src/state_machine.rs` — Added `ValidatorTracker`, `ValidatorLifecycleState`, `execute_bond`, `execute_unbond`, `execute_withdraw`, `execute_renew`, `init_validator`, `get_validator`. Updated `compute_state_root` to include validators (KeyPrefix::Validator). 13 new unit tests.
- `crates/hyperfluid-consensus/src/driver.rs` — Added `FeeMarketState`, `FeeConfig`, `SystemParameters` to `ConsensusDriver`. Added `StakingPayload`, `DelegationPayload` structs. Wired `StakingTx` (Bond/Unbond/Withdraw/Renew) and `DelegationTx` (Delegate/Undelegate/WithdrawDelegation/SetCommission) dispatch in `execute_tx`. Integrated fee market into `produce_block` with `compute_block_utilization`. Added `init_validator` in `init_genesis`.
- `crates/hyperfluid-consensus/Cargo.toml` — Added `hyperfluid-staking` and `hyperfluid-fee-market` dependencies.
- `crates/hyperfluid-node/tests/consensus_driver_tests.rs` — 6 new integration tests: `test_validator_bond_via_driver`, `test_validator_unbond_via_driver`, `test_validator_withdraw_via_driver`, `test_delegation_via_driver`, `test_fee_market_adjusts_per_block`, `test_validator_cycle_state_root_determinism`.

**Tests added:** 19 (13 unit + 6 integration)

### Integration Gate Verification

| Component | Must Demonstrate | Status | Evidence |
|-----------|-----------------|--------|----------|
| State machine (C2) | Real state transitions with observable SMT root changes | PASS | `validator_affects_state_root`, `validator_withdraw_removes_from_state_root` tests |
| Consensus driver (C1) | Real block production with staking/fee execution | PASS | `test_validator_bond_via_driver` through driver dispatch → state change → fee update |
| Fee market (C5) | EIP-1559 base fee adjusts per block | PASS | `test_fee_market_adjusts_per_block` — 5 empty blocks decrease base fee |
| Node binary | Wired to new components | PASS | `test_node_produces_real_blocks` passes; all 41 node tests pass |

### Determinism Sweep

| Check | Result |
|-------|--------|
| Floating-point in protocol code | PASS (zero hits) |
| Wall-clock/random in protocol logic | PASS (`SystemTime::now` only in block loop for timestamps — runtime concern) |
| `thread_local!`/`RefCell`/`SPEC_DEVIATION` shims | PASS (zero hits in new code) |
| `if let Some.get_mut` rejecting else arms | PASS (all 8 instances in new code have else arms returning Rejected) |
| `HashMap` iteration in consensus decisions | PASS (SMT sorts by key internally; HashMap used for lookup only) |
| Mock/shim features in default | PASS (no mock/shim features in state, consensus, or fee-market crates) |

## Remaining Gaps

| Gap | Severity | Status |
|-----|----------|--------|
| Malachite BFT protocol wiring | HIGH | OPEN — ~1,500 lines per ADR-0018. Not blocking Stage 02. |
| Slashing execution + reward distribution | MEDIUM | DEFERRED to Stage 03 |
| Full 24-hour soak test | MEDIUM | DEFERRED to Stage 03 |
| Clatter network bridge for consensus gossip | MEDIUM | DEFERRED |
| Liveness tracking (missed_blocks, liveness_bitmap) | LOW | DEFERRED — data types exist, no mutation logic |

## Verification (CI Mimic)

Full CI runs will be confirmed in Step 8. Pre-check: all 122 workspace tests pass (40 state + 18 consensus + 15 conformance + 4 node + 14 driver + 17 e2e + 6 multi-node + 8 additional).
