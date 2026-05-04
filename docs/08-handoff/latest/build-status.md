# Build Status — Stage 00 (Foundation) COMPLETE

**Last updated:** 2026-05-04
**Stage:** 00 — Foundation — **COMPLETE**

## Week 1: Complete (2026-05-02)

| Task | Status |
|------|--------|
| Cargo workspace with 12 crate scaffolds | Complete |
| Root `Cargo.toml` with profiles, workspace deps | Complete |
| `justfile` (build, test, fmt, lint, bench, doc, clean, audit) | Complete |
| `rustfmt.toml` (edition 2021, stable-only features) | Complete |
| `clippy.toml` | Complete |
| `deny.toml` (cargo-deny: advisories, licenses, bans, sources) | Complete |
| `.github/CODEOWNERS` | Complete |
| `.github/PULL_REQUEST_TEMPLATE.md` | Complete |
| `CONTRIBUTING.md` | Complete |
| `.github/workflows/ci.yml` (fmt, clippy, test, doc, audit, bench-check) | Complete |
| `.devcontainer/devcontainer.json` | Complete |
| `.gitignore` updated (comprehensive Rust entries + genesis.json) | Complete |
| PDP crate: `error.rs` and `types.rs` stub modules with TrustStage/RiskLevel | Complete |

## Week 2: Complete (2026-05-04)

| Task | Status |
|------|--------|
| Core types: `hyperfluid-state` (KeyPrefix, Account, SMTNode, InclusionProof) | Complete |
| Core types: `hyperfluid-consensus` (Committee, BlockHeader, Block, TransactionEnvelope, TxType, GenesisConfig) | Complete |
| Core types: `hyperfluid-staking` (ValidatorRecord, ValidatorState, SlashRecord, SystemParameters, GovernanceVoteTx) | Complete |
| `hyperfluid-node` binary crate (config loading, genesis block, consensus loop, --gen-genesis) | Complete |
| Testnet config: `config/testnet-single.toml` (single-validator genesis) | Complete |
| Start/stop scripts: `scripts/testnet/start.ps1`, `scripts/testnet/stop.ps1` | Complete |
| `DEVELOPMENT.md` (developer onboarding guide) | Complete |
| `cargo-deny` installed and passing (advisories, bans, licenses, sources all ok) | Complete |
| `.gitignore` updated (genesis.json) | Complete |
| Full cold-start verification | Complete |

## Verification

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS (13 crates including node binary) |
| `cargo test --workspace` | PASS (21/21 tests) |
| `cargo fmt --all -- --check` | PASS (clean) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero warnings) |
| `cargo doc --workspace --no-deps` | PASS |
| `cargo deny check` | PASS (advisories ok, bans ok, licenses ok, sources ok) |
| `hyperfluid-node` boots (genesis block, consensus loop) | PASS |
| `hyperfluid-node --gen-genesis` produces genesis.json | PASS |
| Single-node testnet boots and stops cleanly | PASS |
| CI pipeline runs on push/PR | READY |
| Dependency licenses audited; no GPL/AGPL | PASS |
| All risks documented and acceptable | PASS |
| Next stage inputs prepared | PASS |

## Known Issues

1. SCALE encoding deferred to Stage 01. Spec requires SCALE (consensus-spec.md Section 2.2). Current crates use serde for test serialization; SCALE codec (`parity-scale-codec`) to be added in Stage 01.
2. PDP crate has stub `error.rs` + `types.rs` with `thiserror` dependency. Will grow in Stage 02.
3. Node consensus loop is a stub (100ms timer, no real block production). Stage 01 implements Committee BFT.

## Resolved Issues (Bug Audit 2026-05-04)

| Bug | Severity | Fix |
|-----|----------|-----|
| AGX monetary amounts overflow u64 (10M AGX at atto-AGX precision) | Critical | Changed all monetary fields to u128 across 3 crates. Total supply now correctly 10^25 atto-AGX |
| liveness_bitmap Vec<u8> instead of [u8; 1024] | Major | Added SPEC_DEVIATION comment; deferred to Stage 01 SCALE encoding |
| RiskLevel has spurious Critical variant not in spec RiskClass | Major | Removed Critical variant; matches spec's 3-level model |
| Wrong AGX unit conversion in spec comments | Major | Fixed conversion factor comments in staking-spec.md |
| parent_hash printed as block hash in log | Minor | Fixed log label to read parent_hash |
| Unused workspace dependencies (8 deps) | Minor | Removed ed25519-dalek, bincode, rand, bytes, chrono, async-trait, parking_lot, dashmap |
| Trivially passing scaffold tests | Minor | Added spec-value assertions for total supply and airdrop amount |

See `docs/01-research/_audit-bugs-2026-05-04.md` for full report.

## Exit Criteria Status

| Criterion | Status |
|-----------|--------|
| `cargo build` from clean checkout | PASS |
| `cargo test` passes | PASS |
| `just fmt` and `just lint` pass zero warnings | PASS |
| CI pipeline runs on push/PR | READY |
| Single-node testnet boots | PASS |
| Dependency licenses audited; no GPL/AGPL | PASS |
| All risks documented and acceptable | PASS |
| Next stage inputs prepared | PASS |

**Stage 00: COMPLETE. Ready for Stage 01 (Protocol Core).**
