# Checkpoint — 2026-05-18 Bug Audit Round 5

## Summary

Comprehensive code cross-reference audit. 12 bugs found and fixed across 6 crates and 2 spec documents. 6 generic guards added to `execute-build.md` checkpoint.

## Verification

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo test --workspace` | PASS (all tests) |
| `cargo doc --workspace --no-deps --document-private-items` | PASS |
| `cargo deny check` | PASS |
| `cargo bench --workspace --no-run` | PASS |

## Bugs Fixed

| ID | Severity | Crate | Description |
|----|----------|-------|-------------|
| G-01 | Critical | collaboration | Priority comparison inverted in `get_inbox_signal` |
| G-02 | Critical | p2p | `ClatterHandshake` remote_id returns local peer ID |
| G-03 | Major | state | `snapshot_state()` excludes 4 of 5 collections |
| G-04 | Major | fastpath | Challenge tracking dead code — `Challenged` guard never matched |
| G-05 | Major | pdp | `reserve_quota` hard-codes `TrustStage::Trusted` |
| G-06 | Major | pdp | `check_quota` ignores stage multipliers |
| G-07 | Major | pdp | `step5_fee_check` ignores action type |
| G-08 | Major | agent | `dispatch_tool` 10 `.unwrap()` calls panic on malformed JSON |
| G-09 | Medium | staking | `compute_committee_weights` integer division remainder loss |
| G-10 | Medium | state | `execute_delegate` skips validator existence check |
| G-11 | Medium | state | `SMTNode` dead struct |
| G-12 | Minor | spec | `incident-response-spec.md` f64 in fee formula |

## Process Changes

6 new generic guards added to `.opencode/commands/execute-build/checkpoint.md`:
1. Enum discriminant comparison direction tests
2. Constructor identity-field verification
3. `snapshot_state()` vs `compute_state_root()` collection parity
4. Challenge-then-finalize negative lifecycle tests
5. Multi-stage function parameter behavior tests
6. Fractional percentage type verification (u8 vs u16 basis points)

## Next Actions

- Stage 02 Week 5-6 (P2P Wire + Mempool + PDP Integration): READY
- Remediate deferred items: slashing execution, reward distribution, Malachite BFT wiring
