# Checkpoint 2026-05-20 — Stage 02 Week 5-6 Conformance Tests

## Summary

Wrote conformance tests for collaboration-spec.md (§1.7, §3.7) and review-engine-spec.md (§1.7). Added `leases_iter()` accessor to `StateMachine`. Fixed pre-existing clippy warnings (unused_mut, dead_code, unnecessary_cast) across 3 crates. Updated deny.toml license allowlist (ISC, CDLA-Permissive-2.0).

## Spec Coverage Added

| Spec | Section | Hooks Covered | Tests |
|------|---------|---------------|-------|
| collaboration-spec.md | 1.7 (Task Board) | 7 hooks: task transitions, nonce rejection, insufficient funds, duplicate ID, lease TTL, empty heartbeat, valid heartbeat, lease caps, collateral, bounty escrow, non-existent creator, already-claimed task | 11 tests |
| collaboration-spec.md | 3.7 (Trust Ladder) | 5 hooks: new agent untrusted, promotion threshold, abuse blocks promotion, high-severity reset, whitewash guard | 5 tests |
| review-engine-spec.md | 1.7 (Review Pipeline) | 6 hooks: InReview transition, wrong owner rejection, untrusted rejection, trusted claim, accept payout, reject return, tie vote, single verdict, expiry, pre-TTL guard | 10 tests |

## Code Changes

| File | Change |
|------|--------|
| `crates/hyperfluid-state/src/state_machine.rs` | Added `leases_iter()` accessor. Removed redundant `as u128`/`as u64` casts. |
| `crates/hyperfluid-state/src/smt.rs` | Fixed clippy: `&[b]` → `[b]` |
| `crates/hyperfluid-state/tests/conformance_collaboration_spec.rs` | **NEW** — 17 tests covering collaboration-spec §1.7 and §3.7 |
| `crates/hyperfluid-state/tests/conformance_review_spec.rs` | **NEW** — 10 tests covering review-engine-spec §1.7 |
| `crates/hyperfluid-pdp/src/rule_chain.rs` | Fixed 10 clippy `unused_mut` warnings in test module. Added `Hash32` import. |
| `crates/hyperfluid-pdp/tests/conformance_pdp_spec.rs` | Fixed 7 clippy `unused_mut` + 4 `dead_code` warnings |
| `crates/hyperfluid-agent/src/llm.rs` | Fixed clippy: `#[allow(dead_code)]` on `total_duration` field |
| `deny.toml` | Added ISC and CDLA-Permissive-2.0 to license allowlist |

## CI Mimic

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero warnings) |
| `cargo test --workspace` | PASS (467/467) |
| `cargo doc --workspace --no-deps --document-private-items` | PASS (2 minor rustdoc warnings) |
| `cargo deny check` | PASS (advisories ok, bans ok, licenses ok, sources ok) |
| `cargo bench --workspace --no-run` | PASS |

## Determinism Sweep

| Check | Result |
|-------|--------|
| Floating-point (`as f64`/`as f32`/`f64::`/`f32::`) | PASS (zero hits in protocol code) |
| Wall-clock/Random (`Instant::now`/`thread_rng`) | PASS (only in agent/non-protocol code) |
| Test shims (`thread_local!`/`RefCell`/`SPEC_DEVIATION`) | PASS (only justified SPEC_DEVIATION for pubkey reveal) |
| `panic!`/`assert!` in production code | PASS (all in `#[cfg(test)]` modules) |

## Open Items

| Item | Status |
|------|--------|
| P2P not wired into node binary | `TcpTransport`/`accept_loop` exist but not called from `main.rs` |
| Mempool not wired into `produce_block()` | Block production uses empty `vec![]` not real mempool |
| Stage file Week 5-6 scope mismatch | Stage file still describes P2P+Mempool+PDP; actual scope per cleanup checkpoint adds collaboration/review conformance |
| FR-0060, FR-0183, FR-0191 | CLOSED by `checkpoint-2026-05-19-cleanup.md` — all overengineered, removed |
