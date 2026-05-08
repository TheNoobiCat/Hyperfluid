# Bug Audit — 2026-05-08 (Code Cross-Reference: Stage 01 Week 5-6 Post-Audit)

**Result:** 3 bugs found and fixed across 2 crates and 1 spec document. 4 doc warnings fixed. Process guards added.

## Summary

| Severity | Count |
|----------|-------|
| Medium   | 1     |
| Minor    | 2     |

## Bugs Found and Fixed

| ID | Severity | Description | Crate/Spec | Root Cause | Fix |
|----|----------|-------------|------------|------------|-----|
| B-22 | Medium | Delegation state tracked in-memory but not included in SMT root (`compute_state_root()`). State root would not reflect delegation records, breaking state convergence. | `hyperfluid-state/state_machine.rs` | SMT completeness gap — `compute_state_root()` only iterated `accounts`, missing `delegations` | Added delegation iteration with key prefix 0x0E per state-model.md; added `Encode, Decode` to `DelegationState` |
| B-21 | Minor | `execute_undelegate` mutated delegation state (`active`, `unbonding_at_height`) before confirming the delegator account existed for nonce update. Violation of validate-then-mutate ordering. | `hyperfluid-state/state_machine.rs` | Validate-then-mutate ordering violation | Reordered: validate delegation exists → confirm account exists for nonce → then mutate delegation |
| B-23 | Minor | `fee-market-spec.md` struct definition still showed `max_adjustment_pct: u8` after B-18 renamed code field to `max_adjustment_per_mil: u16`. Spec formulas also referenced the old field name. | `fee-market-spec.md` | Documentation drift after code rename | Updated struct definition to `max_adjustment_per_mil: u16`; updated formulas to use per-mil arithmetic |

## Additional Fixes

- Fixed 4 `rustdoc` broken intra-doc link warnings in `hyperfluid-state/src/state_sync.rs` (pre-existing).

## Systemic Patterns Identified

1. **SMT root completeness gap**: When new in-memory state collections are added to the state machine, `compute_state_root()` must be updated to include them. The delegation collection was tracking state correctly but the root computation didn't include it — state would diverge across nodes.

2. **Validate-then-mutate ordering**: B-21 mutated delegation status before confirming the account update step would succeed. While safe in single-threaded context, the pattern is fragile and violates the principle of validating all inputs before mutating any state.

3. **Spec-code field name drift (recurrence)**: B-23 is a repeat of the B-18 pattern — a code field was renamed (`max_adjustment_pct` → `max_adjustment_per_mil`) but the spec was not updated. After any code rename that changes semantic meaning, all `.md` files must be grepped for the old name.

## TDD Process Improvements

4 generic guards added to `execute-build.md`:
- Validate-then-mutate ordering check in state-machine handlers
- SMT root completeness guard: grep for new entity fields in `compute_state_root()` after adding collections
- Spec drift guard: grep `.md` files for old field name after any code rename that changes semantic meaning
- Recurrence guard for `#[key[0]]` style intra-doc link warnings after code changes
