# Checkpoint: fill-gaps Audit 2026-05-23

## Phase 0: Traceability Audit

**UNSCHEDULED MUST-HAVE FRs found:** 3

| FR ID | Title | Disposition |
|-------|-------|-------------|
| FR-0153a | Genesis-Only Mint and Fixed Supply | Header drift fix — added to consensus-spec.md. Content already existed. |
| ~FR-0181 | Bribery Resistance for Fast-Path | FR header added to requirements file. Added to review-engine-spec.md header. Content already existed in spec. |
| ~FR-0187 | Economic Parameter Governance | CLOSED as redundant with FR-0021 + FR-0155. Note added to FR file. |

**CLOSED FR GAP NOTEs updated:**
- FR-0183: Updated stage-01 GAP NOTE to reflect overengineering cleanup (code deleted, CLOSED).
- FR-0060: Was already closed in cleanup; no GAP NOTE to update (removed in cleanup).

**SPEC HEADER DRIFT FIXED:**
- consensus-spec.md: Added FR-0153a
- review-engine-spec.md: Added FR-0153, FR-0181
- collaboration-spec.md: Added FR-0156, FR-0157, FR-0192
- p2p-wire-spec.md: Added FR-0197

## Phase 0h: Vaporware Scan

**Total findings:** 29 (12 CRITICAL, 12 HIGH, 5 MODERATE)

**New untracked gaps fixed:**
1. ProposalState dead enum (fastpath) — removed. 6 variants never used.
2. EscrowStatus::Refunded (state) — wired refund path. 4 new tests.
3. hyperfluid-economics crate — empty scaffold acknowledged (C12 lives in other crates).

## Step 4: Gap Fills

### GAP-01a: Host commit persistence — RESOLVED
- **File:** `crates/hyperfluid-consensus/src/driver.rs`
- **Change:** `BlockCommitted` handler now pushes block to `block_store` and updates `driver.height`
- **Tests:** 2 new tests (bft_block_committed_persists_block_store_and_height, bft_block_committed_direct_push)
- **Build:** PASS, **Tests:** All pass except pre-existing stack overflow in `bft_driver_process_vote_from_other_validator`

### ProposalState dead enum — RESOLVED
- **File:** `crates/hyperfluid-fastpath/src/types.rs`
- **Change:** Removed dead `ProposalState` enum (6 variants). Fastpath tracked state implicitly through `certificates` + `challenged_proposals`.
- **Also:** Annotated `ReviewerVote::Deny` with `#[allow(dead_code)]`

### EscrowStatus::Refunded — RESOLVED
- **File:** `crates/hyperfluid-state/src/state_machine.rs`
- **Change:** `run_lease_expiry` now checks `escrow_status == Locked` and refunds bounty to funder, sets status to `Refunded`
- **Tests:** 4 new tests (happy path, already released, already redistributed, noop on Open task)

## Determinism Sweep: PASS
- Zero `f64`/`f32` in protocol code
- `SystemTime::now()` only in block timestamps (expected)
- Zero `Instant::now`/`thread_rng`/`rand::random` in PDP or state
- No mock/shim features in default Cargo.toml
- All `if let Some.get_mut` have rejecting else arms
- `HashMap` only in non-deterministic code paths (mempool, discovery)

## CI Mimic

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS |
| `cargo test --workspace` | PARTIAL PASS (1 pre-existing BftDriver stack overflow on Windows) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero) |
| `cargo doc --workspace --no-deps --document-private-items` | PASS |
| `cargo deny check` | PASS |
| `cargo bench --workspace --no-run` | PASS |

## Remaining Gaps

| Gap | Status |
|-----|--------|
| GAP-01b (Effect handler trait) | Deferred to Week 9-10 |
| GAP-01c (Clatter network bridge) | Deferred to Week 9-10 |
| GAP-02 (Slashing/rewards/liveness) | Deferred to Stage 03 |
| Full soak test (24h) | Deferred to Stage 03 |
| Vaporware #5 (hyperfluid-economics empty crate) | Low — C12 logic lives in other crates |
| PDP bypass + signature verification | Scheduled for Week 9-10 |

## Files Changed

- `crates/hyperfluid-consensus/src/driver.rs` (+15 lines, 2 new tests)
- `crates/hyperfluid-fastpath/src/types.rs` (-9 lines, removed dead enum)
- `crates/hyperfluid-state/src/state_machine.rs` (+20 lines, 4 new tests)
- `docs/02-requirements/economics/FR-0176-0190-incentives-and-airdrop.md` (FR-0181 header added, FR-0187 closed)
- `docs/04-specifications/protocol/consensus-spec.md` (header: +FR-0153a)
- `docs/04-specifications/protocol/p2p-wire-spec.md` (header: +FR-0197)
- `docs/04-specifications/runtime/review-engine-spec.md` (header: +FR-0153, +FR-0181)
- `docs/04-specifications/runtime/collaboration-spec.md` (header: +FR-0156, +FR-0157, +FR-0192)
- `docs/05-planning/stages/stage-01-protocol-core.md` (GAP NOTES updated)
- `docs/08-handoff/latest/build-status.md` (verification + resolved gaps)
- `PROJECT-STATUS.md` (updated)
