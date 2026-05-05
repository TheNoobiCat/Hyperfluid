# Build Status — Stage 00 (Foundation) COMPLETE

**Last updated:** 2026-05-05
**Stage:** 00 — Foundation — **COMPLETE**
**Amendment:** Agent tools expanded (5→9), seed index created, ADR-0013

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

## Pre-Stage-01 Amendments (2026-05-05)

### Round 1 — Agent tools + seed index
| Change | Type | Docs Affected |
|--------|------|---------------|
| Agent tools expanded from 5 to 9 (read, edit, write, apply_patch) | Design | `agent-tools-spec.md`, `agent-runtime-spec.md`, FR-0062, ADR-0013 |
| Physical `/ideas/` seed index directory created | Infrastructure | `/ideas/README.md`, `/ideas/_template.md`, `collaboration-spec.md`, `collaboration-layer-parallel-teams.md` |
| `hyperfluid idea` CLI subcommand added | Design | `agent-tools-spec.md`, FR-0068 |
| ADR-0013 created | Architecture | `ADR-0013-expanded-agent-tools-and-seed-index.md`, `index.md` (registered) |

### Round 2 — Seed-centric model + single-agent tasks
| Change | Type | Docs Affected |
|--------|------|---------------|
| Seeds are abstract topic buckets (not tasks); many tasks per seed | Design | `/ideas/_template.md`, `/ideas/README.md`, `collaboration-layer-parallel-teams.md`, `agx-economics-and-adversarial-incentives.md`, FR-0192 |
| `seed_ref` made required; no orphan tasks; PDP-enforced | Design | `user-task-submission-and-sponsorship.md`, `collaboration-spec.md`, FR-0084, FR-0081 |
| New seeds enter via `git:head` governance | Design | `/ideas/README.md`, `user-task-submission-and-sponsorship.md`, FR-0084, `GLOSSARY.md` |
| No "entirely novel" tasks — removed from user-task-submission doc | Fix | `user-task-submission-and-sponsorship.md` §5, §7, §10 |
| Single-agent per task; no team formation; no subtask splitting | Design | `collaboration-layer-parallel-teams.md`, `collaboration-spec.md`, FR-0080, FR-0088, ADR-0013 |
| Bounty goes entirely to single worker; reviewers paid via review market | Design | `collaboration-spec.md`, FR-0088 |
| Airdrop agent creates many small tasks per seed | Clarification | `agx-economics-and-adversarial-incentives.md`, FR-0192, `GLOSSARY.md` |
| System prompt instructs agents on seed requirement | Design | `agent-runtime-spec.md` §3.2 |
| `parent_seed_ref` → `seed_ref` terminology unified | Fix | `phase-02-status.md` |
| `idea-seeds-spec.md` gap resolved (no separate spec needed) | Resolution | `phase-02-status.md` |
| Seed idea added to GLOSSARY.md | Documentation | `GLOSSARY.md` |

### Round 3 — User task submission pipeline propagated through all layers
| Change | Type | Docs Affected |
|--------|------|---------------|
| 7 new FRs: FR-0194–FR-0200 (task_create, quotas, sponsorship, discovery, cancellation fee, CLI, Telegram sponsored submission) | Requirements | `FR-0176-0190-incentives-and-airdrop.md`, `index.md` |
| ADR-0014: User Task Submission and Agent Sponsorship | Architecture | `ADR-0014-task-submission-and-sponsorship.md` (new), `index.md` |
| Data model: TASK entity updated with seed_ref, metadata_hash, sponsor_id, requester_pubkey; bounty_agx → u128 | Architecture | `state-model.md` |
| policy-engine-spec.md: TaskCreate added to ActionType, InvalidSeedRef + InsufficientFunds added to DenyReason, task_create_per_stage quota added | Spec | `policy-engine-spec.md` |
| consensus-spec.md: TaskCreateTx added to TxType, task creation state transition added | Spec | `consensus-spec.md` |
| collaboration-spec.md §1.2: task submission pipeline reference added | Spec | `collaboration-spec.md` |
| agent-runtime-spec.md §3.2: `hyperfluid task submit` CLI in system prompt; §5.1: sponsored submission note | Spec | `agent-runtime-spec.md` |
| stage-02-agent-runtime.md: task submit CLI, sponsored Telegram, task discovery via gossip/DHT, task quotas added to weeks 3-6 | Planning | `stage-02-agent-runtime.md` |
| Requirements index: total updated to 202 (172 FR + 30 NFR), FR-0191-0200 registered | Index | `docs/02-requirements/index.md` |

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
