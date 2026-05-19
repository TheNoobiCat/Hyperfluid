# Checkpoint 2026-05-19 — Traceability Audit + Decentralization Score

## Summary

Ran Phase 0 traceability audit per fill-gaps.md. Found 2 unscheduled must-have FRs. Implemented FR-0183 decentralization score. Documented FR-0060 telemetry gap.

## Phase 0: Traceability Audit

Cross-referenced all 156 must-have FR/NFRs against build tasks in all 5 stage files. Used FR ID matching, keyword matching from FR titles, and spec reference table mapping.

### UNSCHEDULED REQUIREMENTS

| FR ID | Title | Reason |
|-------|-------|--------|
| FR-0060 | Signed Telemetry Summaries with Quorum Validation | No explicit build task in any week. Only KeyPrefix reservation + ArtifactClass enum exist. |
| FR-0183 | Stake Concentration Monitoring (item 3-4) | Anti-split detection done in graph.rs. Decentralization score + governance nudges had no build task. |

### GAP NOTES Added

- `stage-02-agent-runtime.md` — GAP NOTE (Traceability — FR-0060) before Week 7-8
- `stage-01-protocol-core.md` — GAP NOTE (Traceability — FR-0183) after Week 3-4 gap note

## GAP Verification Against Source Code

| Gap | File | still_present | blocks_next_stage |
|-----|------|:-------------:|:-----------------:|
| Malachite BFT wiring (effect handler, network bridge, host actor) | `consensus/src/malachite.rs`, `driver.rs` | true | false |
| Slashing execution + reward distribution | `staking/src/lib.rs`, `consensus/src/driver.rs` | true | true |
| FR-0183 decentralization score + governance nudges | `staking/src/graph.rs`, `economics/src/lib.rs` | **PARTIALLY RESOLVED** | false |
| FR-0060 signed telemetry | All crates | true | false |

## Code Changes

### FR-0183: Decentralization Score — IMPLEMENTED

**New crate:** `hyperfluid-economics` populated with production code:

| File | Change |
|------|--------|
| `crates/hyperfluid-economics/Cargo.toml` | Added `hyperfluid-staking` dependency |
| `crates/hyperfluid-economics/src/lib.rs` | Implemented `DecentralizationMetrics`, `compute_decentralization_metrics()` — integer-only determinism, 6 tests |
| `crates/hyperfluid-consensus/Cargo.toml` | Added `hyperfluid-economics` dependency |
| `crates/hyperfluid-consensus/src/driver.rs` | Wired epoch-boundary decentralization computation with tracing::debug output |

**Tests:** 6 unit tests (empty → max score, single large cluster → low score, many independents → high score, mixed → moderate, top3 share, determinism across calls)

**Determinism sweep:** PASS — zero floating-point, zero wall-clock, zero HashMap/HashSet, zero RefCell/SpecDeviation in new code.

## Status File Updates

- `build-status.md` — Added RESOLVED GAPS (2026-05-19) section for decentralization score. Added FR-0060 and FR-0183 governance nudges to OPEN GAPS.
- `PROJECT-STATUS.md` — Updated Blockers (added items 5-6), Next Actions (added traceability audit note + items 8-9), Last updated.
- `stage-01-protocol-core.md` — Updated FR-0183 GAP NOTE to Partially Resolved.
- `stage-02-agent-runtime.md` — Added FR-0060 GAP NOTE.

## CI Mimic

All 6 checks pass: build, test, fmt, clippy, doc, determinism sweep.

## Open Items

| Item | Status |
|------|--------|
| FR-0060 implementation | Open — no build task exists. Recommend absorbing into Stage 02 Week 5-6 or creating a telemetry crate. |
| FR-0183 governance nudges | Open — governance-proposable parameter nudges for stake concentration thresholds need design/scheduling. |
| Slashing/rewards | Deferred to Stage 03 |
| Malachite BFT effect handler | Deferred — not blocking Stage 02 |
