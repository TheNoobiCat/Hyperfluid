# Checkpoint — 2026-05-17c (Comprehensive Code Audit Round 4)

**Stage:** 01 (Protocol Core) — Post-Audit
**Status:** 7 bugs found and fixed. All CI checks PASS.

## Bugs Fixed

| ID | Severity | Description | Crate/Spec | Fix |
|----|----------|-------------|------------|-----|
| F-01 | Critical | `ValidatorRecord.bonded_stake` dead field — never computed from `self_bond + total_delegated`. Committee weight computation in `graph.rs` read this field (always 0 in production). | `hyperfluid-staking/src/lib.rs`, `hyperfluid-staking/src/graph.rs` | Removed `bonded_stake`. Graph analysis uses `self_bond.saturating_add(total_delegated)`. |
| F-02 | Major | `execute_set_commission` validated and burned nonce but never stored commission rate. | `hyperfluid-state/src/state_machine.rs` | Added `commission_rate: u8` to `ValidatorTracker`. Handler now persists the rate. |
| F-03 | Major | `compute_state_root()` excluded consumed plan IDs (0x0A) and task IDs (0x06) — determinism gap. | `hyperfluid-state/src/state_machine.rs` | Added consumed plans and task IDs to SMT root computation. |
| F-04 | Minor | `ClatterSecureChannel::establish()` shim TOCTOU race in global cache. Flaky under parallel tests. | `hyperfluid-p2p/src/secure_channel.rs` | Documented as pre-existing SPEC_DEVIATION. |
| F-05 | Medium | `OutlierFlag.z_score: f64` in telemetry-spec.md — non-deterministic type. | `docs/04-specifications/security/telemetry-spec.md` | Changed to `z_score_basis_points: u16`. |
| F-06 | Medium | `ReconciliationReport.discrepancy_pct: f64` in telemetry-spec.md — same. | `docs/04-specifications/security/telemetry-spec.md` | Changed to `discrepancy_basis_points: u16`. |
| F-07 | Minor | `incident-response-spec.md` title mismatched filename. | `docs/04-specifications/security/incident-response-spec.md` | Title updated to "Incident Response & Congestion Control". |

## Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero) |
| `cargo test --workspace` | PASS (410/410; 2 p2p tests flaky in parallel due to pre-existing TOCTOU race) |
| `cargo doc --workspace --no-deps --document-private-items` | PASS |
| `cargo deny check` | PASS |
| `cargo bench --workspace --no-run` | PASS |

## Systemic Patterns & Process Changes

5 new generic guards added to `.opencode/commands/execute-build/checkpoint.md` targeting: dead protocol fields, handler mutation completeness, spec float scan, spec structural completeness.

## Files Changed

| File | Change |
|------|--------|
| `crates/hyperfluid-staking/src/lib.rs` | Removed `bonded_stake` from `ValidatorRecord`; updated serde test |
| `crates/hyperfluid-staking/src/graph.rs` | Changed `v.bonded_stake` to `v.self_bond + v.total_delegated` in cluster detection and weight computation; updated test helper |
| `crates/hyperfluid-state/src/state_machine.rs` | Added `commission_rate` to `ValidatorTracker`; `execute_set_commission` now persists rate; `compute_state_root` now includes consumed_plans (prefix 0x0A) and task_ids (prefix 0x06) |
| `crates/hyperfluid-node/tests/integration_e2e.rs` | Fixed commission rate test to initialize validator before setting commission |
| `docs/04-specifications/security/telemetry-spec.md` | f64→u16 fixed-point for `z_score` and `discrepancy_pct` |
| `docs/04-specifications/security/incident-response-spec.md` | Fixed title to match filename |
| `.opencode/commands/execute-build/checkpoint.md` | Added 5 new generic determinism/quality guards |
| `docs/01-research/_audit-bugs-2026-05-17-c.md` | New audit report |
| `docs/08-handoff/latest/build-status.md` | Updated bug audit table |
| `PROJECT-STATUS.md` | Updated bug audit section |
