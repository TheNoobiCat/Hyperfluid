# Bug Audit — 2026-05-06 (Round 2: Code Cross-Reference)

**Result:** 6 bugs found and fixed across 4 crates. 1 TDD process guard pattern added to `execute-build.md`.

## Scope

- **Code reviewed:** All 24 Rust source files across 13 crates
- **Specs reviewed:** All 16 Layer 4 specification documents
- **Architecture reviewed:** All 20 architecture documents (index, components, interfaces, state-model, trust-boundaries, failure-model, 14 ADRs)
- **Known state:** All handoff checkpoints, build-status, PROJECT-STATUS, 5 planning stages

## Bugs Found and Fixed

| ID | Severity | Description | Root Cause Category |
|----|----------|-------------|-------------------|
| B-15 | Major | Cluster detection non-transitive: validators sharing common ancestor through intermediate validator escape clustering | Logic error in graph algorithm |
| B-16 | Major | `compute_committee_weights` returns `HashMap` with non-deterministic iteration order | Determinism violation |
| B-17 | Medium | `execute_task_create` silently skips debit for non-existent creator with zero-cost tasks | Missing error handling |
| B-18 | Minor | `max_adjustment_pct` field stores per-mil but is named as percentage | Type/representation error |
| B-19 | Minor | Dead `safety_threshold()` function after 3-tier stall refactoring | Dead code |
| B-20 | Minor | No signal handler for clean node shutdown | Implementation gap |

## Additional Fixes

While grepping for the B-17 pattern (`if let Some` silent skip), two additional instances were found and fixed:
- `execute_undelegate` silently skipped nonce update (line 265)
- `execute_withdraw_delegation` silently skipped balance+nonce update (line 299)

## Systemic Patterns

1. **`if let Some` + `get_mut` silent skip (3 instances across 1 function):** State machine transaction handlers use `if let Some(x) = self.accounts.get_mut(...)` without an `else` arm. When the account doesn't exist, the mutation is silently skipped and `Success` is returned. Three of 4 existing instances were fixed; the `execute_transfer` handler was already correct (had `else { return Rejected }`).

2. **HashMap in deterministic return paths (B-16):** `compute_committee_weights` returns a `HashMap` whose iteration order is non-deterministic. If this feeds into committee weight computation, it causes state divergence.

3. **Graph algorithm non-transitivity (B-15):** The original clustering algorithm only checked pairwise connectivity with the first member of each group, missing transitive connections through intermediate validators.

## Process Changes

Three generic guards added to `execute-build.md` determinism sweep section:
- `pub fn` returning `HashMap` must be verified against feeding into deterministic state
- New graph/clustering algorithms must include transitive-closure test cases
- `if let Some.*get_mut` expressions in state machine handlers must have rejecting `else` arms

## Verification

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS |
| `cargo test --workspace` | PASS (103/103) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero) |
| `cargo doc --workspace --no-deps` | PASS |
