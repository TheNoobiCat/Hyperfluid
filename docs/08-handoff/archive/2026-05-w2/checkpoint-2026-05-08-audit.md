# Checkpoint — 2026-05-08 (Bug Audit: Code Cross-Reference Round 3)

**Stage:** 01 (Protocol Core) — Post-Week-6 Bug Audit
**Status:** 3 bugs found and fixed across 2 crates and 1 spec. 4 doc warnings fixed. Process guards added.

## Bugs Fixed

| ID | Severity | Description | Crate/Spec | Fix |
|----|----------|-------------|------------|-----|
| B-22 | Medium | Delegation state not committed to SMT root — `compute_state_root()` missed `delegations` HashMap | `hyperfluid-state/state_machine.rs` | Added delegation SMT insertion with key prefix 0x0E; added `Encode, Decode` to `DelegationState` |
| B-21 | Minor | `execute_undelegate` mutated delegation state before confirming account existence for nonce update | `hyperfluid-state/state_machine.rs` | Reordered to validate all conditions before mutating any state |
| B-23 | Minor | `fee-market-spec.md` struct + formulas still used `max_adjustment_pct: u8` after B-18 code rename to `max_adjustment_per_mil: u16` | `fee-market-spec.md` | Updated struct definition and formulas to use per-mil naming |

## Additional Fixes

- Fixed 4 `rustdoc` broken intra-doc link warnings in `state_sync.rs`

## Systemic Patterns Documented

1. SMT root completeness gap when adding in-memory entity collections
2. Validate-then-mutate ordering violations
3. Spec-code field name drift recurrence (B-23 = B-18 repeat)

## TDD Process Improvements

4 generic guards added to `execute-build.md`:
- Validate-then-mutate ordering check for state-machine handlers
- SMT root completeness guard: grep for new entity fields in `compute_state_root()` after adding collections
- Spec drift guard: grep `.md` files for old field name after any code rename
- Intra-doc link guard for bracket patterns in doc comments

## Verification

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS (13 crates) |
| `cargo test --workspace` | PASS (181/181) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero) |
| `cargo doc --workspace --no-deps` | PASS (zero warnings) |
