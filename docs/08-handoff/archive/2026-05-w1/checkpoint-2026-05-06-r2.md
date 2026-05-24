# Checkpoint — 2026-05-06 (Bug Audit R2: Code Cross-Reference)

**Stage:** 01 (Protocol Core) — Post-Week-4 Bug Audit
**Status:** 6 bugs found and fixed across 4 crates. 2 additional silent-skip instances fixed. Process guards added.

## Bugs Fixed

| ID | Severity | Description | Crate | Fix |
|----|----------|-------------|-------|-----|
| B-15 | Major | Cluster detection non-transitive — validators sharing common ancestor through intermediate validator escape clustering | `hyperfluid-staking/graph.rs` | Replaced pairwise-first-member algorithm with connected-components via ancestor-set adjacency |
| B-16 | Major | `compute_committee_weights` returns `HashMap` (non-deterministic iteration) | `hyperfluid-staking/graph.rs` | Changed return type to `BTreeMap` |
| B-17 | Medium | Non-existent creator creates zero-cost tasks (debit silently skipped) | `hyperfluid-state/state_machine.rs` | Added `match` with rejecting `None` arm |
| B-18 | Minor | `max_adjustment_pct` stores per-mil but named as percentage | `hyperfluid-fee-market/lib.rs` | Renamed to `max_adjustment_per_mil`, widened to `u16` |
| B-19 | Minor | Dead `safety_threshold()` after 3-tier stall refactoring | `hyperfluid-consensus/types.rs` | Removed |
| B-20 | Minor | No signal handler for clean shutdown | `hyperfluid-node/main.rs` | Added `tokio::signal::ctrl_c()` handler |

## Additional Fixes (from B-17 pattern grep)

- `execute_undelegate`: silent nonce skip → `match` with rejecting `None` arm
- `execute_withdraw_delegation`: silent balance+nonce skip → `match` with rejecting `None` arm

## Systemic Patterns Documented

1. `if let Some` + `get_mut` silent skip (3 instances)
2. HashMap in deterministic return paths
3. Graph algorithm non-transitivity (pairwise-first-member instead of connected-components)

## TDD Process Improvements

3 generic guards added to `execute-build.md`:
- HashMap→BTreeMap guard for consensus-critical return types
- Transitive-closure test requirement for graph/clustering algorithms
- `if let Some.get_mut` must have rejecting `else` arm in state machine handlers

## Verification

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS (13 crates) |
| `cargo test --workspace` | PASS (103/103) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero) |
| `cargo doc --workspace --no-deps` | PASS |
