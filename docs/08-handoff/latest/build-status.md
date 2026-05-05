# Build Status — Stage 01 (Protocol Core) IN PROGRESS

**Last updated:** 2026-05-06
**Stage:** 01 — Protocol Core — **IN PROGRESS**
**Week 1-2 (Consensus + State Machine):** COMPLETE

## ⚠️ PENDING CODE CHANGES (Architecture Amendments 2026-05-06 — Round 1)

Before implementing ANY Stage 01 code, apply these 5 code changes. See `checkpoint-2026-05-06.md` for exact file-level details.

| # | Change | Components | Priority |
|---|--------|-----------|----------|
| 1 | Overlap 33%→20% + two-epoch recency guard | `hyperfluid-consensus/types.rs` | MUST fix before Week 3-4 |
| 2 | VDF fallback using immutable entropy | `hyperfluid-consensus/` (new module) | MUST fix before Week 3-4 |
| 3 | Committee stall 3-tier thresholds | `hyperfluid-consensus/types.rs` | MUST fix before Week 3-4 |
| 4 | Stake-graph clustering implementation | `hyperfluid-staking/src/graph.rs` (NEW) | MUST fix before Week 3-4 |
| 5 | Delegation subsystem + TxType collapse (7 generalized types with sub-enums, DelegationRecord, commission, slash propagation) | `hyperfluid-staking/src/lib.rs`, `hyperfluid-consensus/src/types.rs`, `hyperfluid-state/src/state_machine.rs`, `hyperfluid-consensus/tests/conformance_consensus_spec.rs` | MUST fix before Week 3-4 |

## Overengineering Fixes Applied (2026-05-06 — Round 2)

| Fix | What Changed | Complexity Removed |
|-----|-------------|-------------------|
| Circuit breaker killed | Removed 3-tier CB, 22+ thresholds, hysteresis, persistence windows, reporter quorums, sub-circuit-breakers, IncidentRecord, CircuitBreakerState. EIP-1559 base fee is the sole congestion mechanism. | ~15 files, ~250 lines of spec removed |
| Mempool lanes killed | Removed 4-lane reservation (Evidence 15%, Control 10%, Governance 10%, Transfer 65%). Single fee-ordered pool with evidence/governance fee discounts. | ~10 files |
| Trust ladder simplified | 4 stages → 2 (untrusted/trusted). Removed identity age, reviewer diversity, reputation vector, inactivity decay, promotion/regression tracking. Progression: 10 accepted tasks + clean record. | ~10 files |
| PDP simplified | 10-step rule chain → 5 steps. Removed RiskClass, role/trust check, ACL, taint, risk step-up, plan binding hash, policy bundle match. DenyReasons: 12→5. Protocol no longer does LLM safety filtering. | ~15 files |
| Review engine simplified | 3 phases → 2 (removed objective checks). Fixed payout (no quality-weighted curve). 1 independence constraint (operator cluster, not 4). Removed clawback, reputation decay, quality scoring. | ~10 files |
| TxType collapsed | 16 variants → 7 base types with action sub-enums (StakingAction, DelegationAction, GovernanceAction, FastPathAction). | ~12 files |

All doc changes complete. Only Rust code changes remain (5 items from Round 1 above).

## Stage 01: Week 1-2 — Consensus + State Machine (C1 + C2) — COMPLETE

| Task | Status |
|------|--------|
| TxType: added TaskCreateTx (missing amendment) | Complete |
| parity-scale-codec added to workspace, consensus + state crates | Complete |
| BlockHeader::block_hash() via SHA3-256 of SCALE-encoded header | Complete |
| Committee::sample() — deterministic stake-weighted sampling | Complete |
| Committee::sample_with_rotation() — max 33% overlap enforced | Complete |
| Committee::safety_threshold() + can_produce() — halt at <67 | Complete |
| SparseMerkleTree: insert, root, inclusion proof, proof verification | Complete |
| StateMachine: transfer, task_create, nonce enforcement, replay protection | Complete |
| Conformance tests: 7 C1 hooks + 8 C2 hooks all PASS | Complete |
| 57/57 workspace tests pass | Complete |
| clippy zero warnings, fmt clean, docs build | Complete |

## Verification (after Week 1-2)

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS (13 crates) |
| `cargo test --workspace` | PASS (57/57) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero) |
| `cargo doc --workspace --no-deps` | PASS |

## Stage 00 (Foundation) COMPLETE

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

## Resolved Issues (Bug Audit 2026-05-05)

| Bug | Severity | Fix |
|-----|----------|-----|
| state-model.md: 11 monetary fields still uint64 after B-01 fix (DB-01) | Major | Changed to uint128 across GOVERNANCE_PROPOSAL, GOVERNANCE_VOTE, REPLICATION_LEASE, AIRDROP_POOL, SYSTEM_PARAMETERS |
| Missing traceability-matrix.md per BUILD-SYSTEM.md requirement (DB-02) | Major | Created at docs/08-handoff/latest/traceability-matrix.md |
| f64 in PDP QuotaEntry violates determinism mandate (DB-03) | Major | Changed to rational pair (u64,u64) |
| f64 in ReputationVector causes SMT non-determinism (DB-04) | Major | Changed to u8 scaled 0-255 |
| policy-engine-spec.md section ordering error (DB-05) | Minor | Reordered 2.5→2.6→2.7→2.8; removed duplicate |
| architecture/index.md requirement count out of date (DB-06) | Minor | Updated 195→202; added FR-0194-0200 mapping |
| components.md requirement count out of date (DB-07) | Minor | Updated 195→202; tool list 5→9 |
| Spec headers missing FR-0194–0200 coverage (DB-08) | Minor | Added missing FRs to consensus-spec.md, agent-runtime-spec.md, collaboration-spec.md headers |

See `docs/01-research/_audit-bugs-2026-05-05.md` for full report.

## Resolved Issues (Bug Audit 2026-05-06)

| Bug | Severity | Fix |
|-----|----------|-----|
| SMT verify_proof broken for multi-leaf proofs (B-08) | Major | Added sibling_is_left to InclusionProof; verify_proof now uses correct hash ordering |
| Committee weights/sample use u64 not u128 (B-09) | Major | Changed to u128; fixed selector entropy width; updated all tests |
| f64 in deterministic committee sampling (B-10) | Major | Replaced f64*ceil with integer div_ceil |
| Unchecked addition overflow in recipient balance (B-11) | Minor | Changed to saturating_add |
| Trivially passing first_spend_pubkey_reveal test (B-12) | Minor | Replaced with full lifecycle test |
| Dead duplicate InclusionProof struct in lib.rs (B-13) | Minor | Removed dead code |
| Duplicate Hash32 types across crates (B-14) | Minor | Observation only — no code change needed |

See `docs/01-research/_audit-bugs-2026-05-06.md` for full report.

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
