# Checkpoint — 2026-05-04 (Bug Audit)

**Completed:** Comprehensive code-vs-spec bug audit across all 13 crates and 14 Layer 4 specifications.

## Scope

- **Code reviewed:** All 17 Rust source files across 13 crates
- **Specs reviewed:** All 14 Layer 4 specification documents
- **Architecture reviewed:** All 6 architecture documents, 12 ADRs
- **Known state:** All handoff checkpoints, build-status, PROJECT-STATUS, 5 planning stages

## Bugs Found and Fixed

| ID | Severity | Description | Root Cause Category |
|----|----------|-------------|-------------------|
| B-01 | Critical | AGX monetary amounts overflow u64 at atto-AGX precision | Type/representation error |
| B-02 | Major | liveness_bitmap Vec<u8> instead of fixed [u8; 1024] | Type/representation error |
| B-03 | Major | Spurious Critical variant in PDP RiskLevel | Spec deviation |
| B-04 | Major | Wrong AGX unit conversion in spec comments | Documentation error |
| B-05 | Minor | parent_hash logged as block hash | Logic error |
| B-06 | Minor | Unused workspace dependencies (8 deps) | Dead code |
| B-07 | Minor | Trivially passing scaffold tests | Test quality |

**Total: 7 bugs (1 critical, 3 major, 3 minor)**

## Systemic Pattern

The critical bug (B-01) reveals a systemic issue: the spec defines `atto-AGX` as 10^-18 AGX, but a 10,000,000 AGX total supply requires 10^25 units — exceeding u64 range by >500,000x. All monetary fields across the codebase used u64. Fixed by migrating to u128.

## Verification

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS |
| `cargo test --workspace` | PASS (23/23 tests, +2 new) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS |
| `cargo doc --workspace --no-deps` | PASS |

## Next

Stage 01 (Protocol Core) — build Minimum Viable Chain. All monetary types now use u128 with correct atto-AGX scaling.
