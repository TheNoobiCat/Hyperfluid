# Project Status

This file tracks the current state of Hyperfluid's build pipeline. It is separate from the build system definition so that the build system can remain focused on process while this file evolves with the project.

---

## Current Phase

| Phase | Status |
|-------|--------|
| Phase 0: Research Audit | COMPLETE |
| Phase 0: Decentralisation Audit | COMPLETE (issues in `docs/01-research/_overengineered.md`, fixes applied) |
| Phase 1: Requirements | COMPLETE (203 requirements: 173 FR + 30 NFR — FR-0020a added) |
| Phase 2: Architecture | COMPLETE (12 components, 14 ADRs, component model, interfaces, trust boundaries, failure model. Gate: PASS — ADR-0015 added) |
| Phase 3: Specifications | COMPLETE (15 specs across 4 domains — `stake-graph-analysis-spec.md` added) |
| Phase 4: Planning | COMPLETE (5 stages, 21-30 week roadmap, spec-to-stage mapping, risk register. Gate: PASS) |
| Phase 5+: Build | IN PROGRESS (Stage 01 integration gaps RESOLVED 2026-05-17. Stage 02 Week 1-2 libraries complete + wired. Stage 02 ready for Week 3-4.) |

---

## Research Gaps (RESOLVED - converted to Layer 2 requirements)

All gaps were either resolved in research or converted to explicit FR/NFR items. See `docs/02-requirements/index.md` "Gaps Addressed" for mapping.

| Gap | Should Live In | Status | Resolved In |
|-----|----------------|--------|-------------|
| Token budget resource model (LOCAL runtime only) | `runtime/agent-runtime-spec.md` | **RESOLVED** | FR-0073, FR-0074, FR-0075 |
| VDF-based committee randomness | `protocol/consensus-spec.md` | **RESOLVED** | FR-0003 |
| Reviewer independence / operator-cluster diversity | `runtime/review-engine-spec.md` | **RESOLVED** | FR-0099, FR-0033 |
| No-vote timeout fairness proof | `protocol/governance-spec.md` | **RESOLVED** | FR-0029 (with validation deferred note) |
| Plan replay protection E2E | `runtime/policy-engine-spec.md` + storage specs | **RESOLVED** | FR-0108 (E2E trace deferred to Layer 6) |
| Telemetry threat model | `security/` research -> specs | **RESOLVED** | NFR-0020, NFR-0021 |
| Sandbox escape analysis | `security/` research -> specs | **RESOLVED** | FR-0137 (full threat model still needed before Layer 4) |
| Content-addressing SLA | `storage/artifact-availability-spec.md` | **RESOLVED** | FR-0057 |
| Economic timing parameters | `protocol/` specs | **RESOLVED** | FR-0148, FR-0149, FR-0150, FR-0169 |

---

## Decentralisation Audit Status

**Result:** PASS (with fixes applied across research; Layer 4 specs audit also PASS)

See `docs/01-research/_overengineered.md` for:
- Original issues found
- Proposed fixes
- Applied corrections across research documents

Key fixes:
1. Per-IP rate limits removed from protocol, demoted to local DoS hardening
2. Airdrop anti-Sybil replaced with pubkey-bound challenge + locked bond (from airdrop, not upfront)
3. Manual reviewer triggers replaced with deterministic fallbacks + 3-reviewer floor
4. Token burn removed from protocol economics, kept as local runtime concern
5. Committee randomness changed from drand to on-chain VDF from genesis
6. Geographic reviewer spread replaced with operator-cluster diversity via stake-graph analysis

---

## Blockers

**All previous CRITICAL/HIGH blockers resolved 2026-05-17.** Malachite type-level integration (SigningScheme + Context) implemented 2026-05-17c (410 lines, 13 tests). Remaining work:

1. **Malachite BFT protocol wiring** — SigningScheme + Context implemented (malachite.rs). Effect handler, network bridge, host actor deferred (~1,100 lines remaining). Not blocking Stage 02 — ConsensusDriver produces blocks.
2. **Slashing execution + reward distribution** — deferred to Stage 03 (Validation).
3. **24-hour soak test** — deferred to Stage 03 (Validation).
4. **Clatter network bridge for consensus gossip** — TCP sockets functional, but gossip needs BFT protocol messages to route.

---

## Bug Audit (2026-05-04)

**Result:** 7 bugs found and fixed across 3 crates and 10 spec/architecture documents. See `docs/01-research/_audit-bugs-2026-05-04.md` for full report.

Key findings:
1. **CRITICAL:** All AGX monetary amounts changed from `u64` to `u128` — 10M AGX total supply at atto-AGX precision (10^25) overflows u64 (max ~1.8*10^19).
2. **MAJOR:** `liveness_bitmap` type divergence documented with SPEC_DEVIATION flag.
3. **MAJOR:** PDP `RiskLevel` had spurious `Critical` variant not in spec — removed.
4. Spec documents updated for u128 type migration across all 10 affected struct definitions.

## Bug Audit (2026-05-05) — Documentation Layer

**Result:** 8 bugs found and fixed across 8 documentation files. No code changes. See `docs/01-research/_audit-bugs-2026-05-05.md` for full report.

Key findings:
1. **MAJOR:** 11 monetary fields in `state-model.md` remained `uint64` after the B-01 u128 migration (incomplete fix propagation).
2. **MAJOR:** Missing `traceability-matrix.md` required by BUILD-SYSTEM.md.
3. **MAJOR:** `f64` in PDP `QuotaEntry.stage_multipliers` violates PDP determinism mandate.
4. **MAJOR:** `f64` in `ReputationVector` causes SMT non-determinism.
5. Architecture documents (index.md, components.md) referenced pre-amendment requirement count (195 vs 202).
6. Spec headers missing FR-0194–0200 coverage added in checkpoint amendments.

Systemic patterns identified: incomplete migration propagation, f64 in deterministic contexts, documentation drift after Layer 2 amendments.

## Bug Audit (2026-05-06) — Code

**Result:** 7 bugs found and fixed across 2 crates. See `docs/01-research/_audit-bugs-2026-05-06.md` for full report.

Key findings:
1. **MAJOR:** SMT `verify_proof()` always computed `hash(current || sibling)` regardless of left/right position — proof verification broken for multi-leaf trees. Added `sibling_is_left` to `InclusionProof` struct.
2. **MAJOR:** `Committee::weights` and `sample()` stakes remained `u64` after B-01's `u128` migration (incomplete migration recurrence). Changed to `u128` with 16-byte entropy selector.
3. **MAJOR:** `f64` used in `committee_size * 0.33` max-overlap computation (determinism violation recurrence). Replaced with integer `div_ceil`.

Systemic patterns identified: B-01 incomplete migration recurrence (committee weights), f64-in-deterministic recurrence (committee sampling), SMT proof correctness gap (no multi-leaf proof test).

## Bug Audit (2026-05-06 — Round 2: Code Cross-Reference)

**Result:** 6 bugs found and fixed across 4 crates. See `docs/01-research/_audit-bugs-2026-05-06-r2.md` for full report.

Key findings:
1. **MAJOR:** Cluster detection algorithm was non-transitive — validators sharing a common ancestor through an intermediate validator escaped clustering. Replaced pairwise-first-member comparison with connected-components algorithm.
2. **MAJOR:** `compute_committee_weights` returned `HashMap` (non-deterministic iteration) instead of `BTreeMap`. Fixed.
3. **MEDIUM:** `execute_task_create` silently skipped debit for non-existent accounts with zero-cost tasks (no `else` arm). Fixed. Two additional `if let Some` silent-skip instances found by grep and fixed in `execute_undelegate` and `execute_withdraw_delegation`.
4. TDD process improved: 3 generic guards added to `execute-build.md` (HashMap→BTreeMap, transitive graph tests, `if let Some.get_mut` rejecting arms).

Systemic patterns identified: `if let Some` + `get_mut` silent skip (3 instances), HashMap in deterministic return paths, graph algorithm non-transitivity.

## Bug Audit (2026-05-08) — Code Cross-Reference Round 3

**Result:** 3 bugs found and fixed across 2 crates and 1 spec document. See `docs/01-research/_audit-bugs-2026-05-08.md` for full report.

Key findings:
1. **MEDIUM:** Delegation state tracked in-memory but not committed to SMT root (`compute_state_root()` only included accounts). Fixed by adding delegation key-value insertion with key prefix `0x0E` per state-model.md.
2. **MINOR:** `execute_undelegate` mutated delegation state before confirming account existed for nonce update. Fixed by reordering validation before mutation.
3. **MINOR:** `fee-market-spec.md` still referenced `max_adjustment_pct: u8` after B-18's code rename to `max_adjustment_per_mil: u16`. Fixed by updating spec struct and formulas.

Systemic patterns identified: SMT root completeness gap (new entity added but root not updated), validate-then-mutate ordering violation in state handlers, spec-code field name drift recurrence.

## Recent Design Changes (2026-05-17c) — Malachite BFT Type-Level Integration

| Change | Summary | Docs Affected |
|--------|---------|---------------|
| Malachite SigningScheme | Implemented `MlDsa65Scheme` (SigningScheme trait) with byte-based `MlDsa65Signature`, `MlDsa65PublicKey`, `MlDsa65PrivateKey`. ML-DSA-65 FIPS 204 signatures for BFT consensus. | `hyperfluid-consensus/src/malachite.rs` |
| Malachite Context | Implemented `HyperfluidContext` with full type wiring: `Address32`, `BlockHeight`, `BlockValue`, `HyperfluidValidator`, `HyperfluidValidatorSet`, `HyperfluidVote`, `HyperfluidProposal`, `HyperfluidProposalPart`, `HyperfluidExtension`. Deterministic proposer selection via SHA3-256. | `hyperfluid-consensus/src/malachite.rs` |
| Malachite tests | 13 unit tests covering signing roundtrip, invalid decode, block height, value ID/hash, validator set ordering, context proposer selection, proposal/vote/cycle. | `hyperfluid-consensus/src/malachite.rs` |
| P2P conformance fix | Fixed cache collision in `e2e_encryption_across_relay` + `tampered_ciphertext_rejected` tests — shared global handshake cache with `e2e_empty_message`. Unique peer IDs per test. All 23 P2P conformance tests pass. | `hyperfluid-p2p/tests/conformance_p2p_spec.rs` |
| CI mimic all-green | fmt, clippy (zero warnings), test (386/386), doc, deny, bench-check — all PASS | |

## Recent Design Changes (2026-05-17) — Stage 01 Integration Gaps Filled

| Change | Summary | Docs Affected |
|--------|---------|---------------|
| P2P TCP sockets | `hyperfluid-p2p/src/tcp.rs` — TcpTransport, accept_loop, connect_to_peer, clatter handshake over wire, connection state machine wired to socket events. 9 new tests (2 socket integration). | `hyperfluid-p2p/` |
| Disk-backed artifact storage | `hyperfluid-artifact/src/store.rs` — StoreConfig, store_chunk, load_chunk, SHA3-256 verification on write+read, content-addressed paths. 10 new tests. | `hyperfluid-artifact/` |
| Consensus driver | `hyperfluid-consensus/src/driver.rs` — ConsensusDriver with block production loop, state machine execution, SMT roots, parent-hash chaining. Replaced node binary sleep stub. 14 new tests. | `hyperfluid-consensus/`, `hyperfluid-node/` |
| C4/C6/C9 wired | Governance, Fast-Path, PDP crates wired into ConsensusDriver. GovernanceTx and FastPathTx dispatched. 3 new integration tests. | `hyperfluid-consensus/`, `hyperfluid-node/` |
| Multi-node harness | `multi_node_test.rs` — 6 tests across 2-5 nodes verifying deterministic state convergence, genesis consistency, divergence detection. | `hyperfluid-node/tests/` |
| CI mimic all-green | fmt, clippy (zero warnings), test (353/353), doc, deny, bench-check — all PASS | |
| Malachite crates loaded | `arc-malachitebft-core-*` v0.7.0-pre added to workspace. MSRV bumped 1.85→1.88. All 5 core crates compile. | `Cargo.toml`, `hyperfluid-consensus/Cargo.toml` |

## Recent Design Changes (2026-05-17) — Stage 02 Week 1-2

| Change | Summary | Docs Affected |
|--------|---------|---------------|
| C9 PDP implemented | Full 5-step rule chain, quota matrix (14 entries), key rotation (ML-DSA-65, 100-block grace window), audit log. 58 tests. | `hyperfluid-pdp/` (types, rule_chain, quota, key_rotation, audit) |
| C4 Governance implemented | Proposal lifecycle (submit/vote/tally/execute), quorum + majority, anti-flood (32 open, 1 per epoch, 3-epoch cooldown). 9 tests. | `hyperfluid-governance/` (types, proposal) |
| C6 Fast-Path implemented | Merge lifecycle (propose/certify/challenge/rollback/finalize), 2f+1 quorum, 144-block challenge window. 7 tests. | `hyperfluid-fastpath/` (types, lifecycle) |
| CI mimic all-green | fmt, clippy (zero warnings), test (291/291), doc, deny, bench-check — all PASS | |
| SPEC_DEVIATION: InsufficientFunds | Added to PDP DenyReason enum (spec gap in §1.3) | `policy-engine-spec.md` §1.3, `hyperfluid-pdp/src/types.rs` |

## Bug Audit (2026-05-14) — Post-Stage-01 Full Code Audit

**Result:** 1 bug found and fixed across 1 crate. See `docs/01-research/_audit-bugs-2026-05-14.md` for full report.

Key findings:
1. **MINOR:** `execute_undelegate` missing `else` arm on `if let Some(del) = self.delegations.get_mut(&key)` — the only `get_mut` call in the state machine without a rejecting else arm. Fixed by adding `else { return ExecutionResult::Rejected; }`.

Systemic pattern identified: `get_mut` guard scope drift — when the delegation HashMap was added to the state machine, the existing `if let Some.*get_mut` guard was not re-run against it. All 3 `self.accounts.get_mut()` calls had else arms, but the new `self.delegations.get_mut()` did not. After adding any new entity collection, all existing grep guards must be re-scanned against the full file.

CI mimic: all 6 checks pass (fmt, clippy, test, doc, deny, bench-check).

## Bug Audit (2026-05-17c) — Comprehensive Code Cross-Reference Round 4

**Result:** 7 bugs found and fixed across 3 crates and 3 spec documents. See `docs/01-research/_audit-bugs-2026-05-17-c.md` for full report.

Key findings:
1. **CRITICAL:** `ValidatorRecord.bonded_stake` was a dead field never populated from `self_bond + total_delegated`. Committee weight computation in `graph.rs` read this field (always 0 in production), making all committee weights zero. Fixed by removing the field and computing effective stake as `self_bond + total_delegated`.
2. **MAJOR:** `execute_set_commission` validated the commission rate and consumed the nonce but never stored the rate anywhere — a no-op handler. Fixed by adding `commission_rate` to `ValidatorTracker` and persisting it in the handler.
3. **MAJOR:** `compute_state_root()` excluded consumed plan IDs (key prefix 0x0A) and task IDs (key prefix 0x06) — a determinism gap where nodes could agree on state root while having divergent consumed plan sets. Fixed by adding both collections to SMT root.
4. Two `f64` fields found in `telemetry-spec.md` data structures — changed to `u16` basis points.
5. `incident-response-spec.md` title mismatched filename — fixed title.

Systemic patterns identified: dead/redundant fields on protocol structs (after migration cleanup), state handler incomplete mutation (validated but didn't persist), f64 in spec data structures (existing float scan didn't cover `.md` files), document identity mismatch.

Process improvements: 5 generic guards added to `execute-build.md` `checkpoint.md` covering field population verification, handler mutation tracing, spec float scanning, and spec structural completeness.

## Next Actions

**Stage 02 can now continue to Week 3-4 (Agent Runtime + Sandbox + Operator Interface):**
1. Infinite agent loop, system prompt loader, core tool set, handoff mechanism, resource limits, process isolation.
2. TUI setup wizard (ratatui), Telegram bot client (optional), config.toml.
3. Crash recovery via handoff replay.
4. C4/C6/C9 libraries already built and wired — PDP validates transactions, governance processes proposals, fast-path manages topic merges.

**Recent resolution (2026-05-17b):**
- Validator lifecycle (bond/unbond/withdraw/renew) implemented in StateMachine with 13 tests.
- StakingTx (4 actions) and DelegationTx (4 actions) dispatched in ConsensusDriver.
- FeeMarket integrated into block production (EIP-1559 base fee adjusts per block).
- 6 new driver-level integration tests cover full staking lifecycle through ConsensusDriver.

**Before Stage 03:**
5. Complete slashing execution and reward distribution.
6. Integrate Malachite BFT when crates are available.
7. Build clatter network bridge for consensus gossip.

**Stage 02 Week 1-2 (Governance + Fast-Path + PDP): COMPLETE (libraries built, wired into node)**
**Stage 02 Week 3-4 (Agent Runtime + Sandbox + Operator Interface): READY TO START**
5. ~~Bug audit (code) completed.~~
2. ~~Bug audit (code) completed.~~
3. ~~Pre-Stage-01 amendment: Agent tools expanded 5→9, seed index created at `/ideas/`, ADR-0013.~~
4. ~~Seed-centric model + user-task-submission pipeline propagated through L2-L5.~~
5. ~~Documentation-layer bug audit completed.~~
6. ~~Stage 01 Week 1-2 (C1 + C2): Consensus Engine + State Machine & SMT.~~
7. ~~Architecture amendments (2026-05-06): 15% cap removed, overlap 33%→20%, VDF fallback hardened, stake-graph spec created, committee stall tiered, delegation spec'd.~~
8. ~~5 pending code changes applied: overlap+two-epoch guard, VDF fallback, stall thresholds, stake-graph clustering, delegation subsystem.~~
9. ~~Stage 01 Week 3-4 (C3 + C5): Staking base & Fee Market implemented. See `checkpoint-2026-05-06-code.md`.~~
10. ~~Stage 01 Week 5-6 (C7 + C8): P2P Networking + Artifact Storage.~~ COMPLETE (2026-05-08)
11. ~~Bug audit (2026-05-08) completed.~~
12. ~~Stage 01 Week 7-8: Integration, soak test, polish.~~ COMPLETE (2026-05-14)
13. ~~clatter+ml-dsa Secure Channel Implementation~~ — COMPLETE (2026-05-15). Wire clatter `HybridHandshakeCore` + ml-dsa identity keys into `hyperfluid-p2p` SecureChannel. See ADR-0016.
14. **NEXT: Stage 02 (Agent Runtime):** Layer agent behavior, PDP, collaboration, review, governance, fast-path on top of the chain.
14. Stage 03 (Validation): Full conformance matrix, adversarial test suite, load testing, security audit, parameter calibration.
15. Stage 04 (Mainnet Prep): SLOs, monitoring, runbooks, private testnet soak, incident drill, launch checklist.
16. Freeze all 15 specs before Stage 01 implementation continues. Post-freeze spec changes require governance proposals.

## Layer 4 Spec Inventory (delivered)

| Spec | Covers (Component/FR) | Status |
|------|----------------------|--------|
| `runtime/agent-runtime-spec.md` | C10 Agent Runtime (FR-0061-0075, FR-0193, FR-0199, FR-0200 — CLI + sponsored submission added) | complete |
| `runtime/policy-engine-spec.md` | C9 Policy Decision Point (FR-0106-0120) | complete |
| `runtime/review-engine-spec.md` | C12 Economics - review markets, Sybil adjudication (FR-0161-0175, FR-0191 — Section 2 added) | complete |
| `runtime/collaboration-spec.md` | C11 Collaboration & Inbox, bounty escrow (FR-0076-0105, FR-0153b, FR-0194-0198, FR-0176-0190 — Section 3 incentives, FR-0194 task_create) | complete |
| `security/telemetry-spec.md` | Telemetry integrity (FR-0060, FR-0139-0141, NFR-0020-0021) | complete |
| `security/incident-response-spec.md` | Incident response & recovery (FR-0142-0145) | complete |
| `protocol/fee-market-spec.md` | C5 Fee Market (FR-0146-0160) | complete |
| `protocol/consensus-spec.md` | C1 Consensus Engine, C2 State Machine (FR-0001-0010, FR-0194 — TaskCreateTx) | complete (amended 2026-05-06) |
| `protocol/staking-spec.md` | C3 Staking & Validator Manager (FR-0011-0020, FR-0020a — delegation added) | complete (amended 2026-05-06) |
| `protocol/stake-graph-analysis-spec.md` | C3 Staking & Validator Manager (FR-0002, FR-0183 — cluster detection algorithm) | **NEW (2026-05-06)** |
| `protocol/governance-spec.md` | C4 Governance Engine (FR-0021-0030) | complete |
| `protocol/p2p-wire-spec.md` | C7 P2P Networking (FR-0041-0050) | complete |
| `protocol/fastpath-spec.md` | C6 Fast-Path Topic Protocol (FR-0031-0040) | complete |
| `storage/state-sync-spec.md` | C2 State Machine & SMT (FR-0010, NFR-0009) | complete |
| `storage/artifact-availability-spec.md` | C8 Artifact Availability (FR-0051-0060) | complete |

**Dropped:** `security/key-management-spec.md` — no Layer 1 research doc backing; key rotation is covered in `policy-engine-spec.md` Section 3 (FR-0118, NFR-0024) and key binding is covered in `consensus-spec.md` Section 2 (FR-0005, FR-0006).

## Recent Design Changes (2026-05-03)

| Change | Summary | Docs Affected |
|--------|---------|---------------|
| AGX monetary policy | Switched from inflationary protocol issuance to genesis-only fixed supply. All AGX minted at genesis, held by airdrop agent. Agents earn via bounty marketplace, not protocol emissions. | `agx-economics-and-adversarial-incentives.md`, `FR-0146-0160`, `FR-0176-0190`, `automatic-vs-agent-controlled.md` |
| Airdrop agent dual role | Airdrop agent now also posts initial seed tasks with bounties from genesis supply to bootstrap the marketplace. | `agx-economics-and-adversarial-incentives.md`, `collaboration-layer-parallel-teams.md`, FR-0192 |
| Progressive Sybil bond | Bond increased to 20 AGX, released in 4 tranches gated by verified work and trust stage progression. | `agx-economics-and-adversarial-incentives.md`, FR-0157 |
| Dynamic proof-of-agent puzzle | SHA3-256 HashCash with difficulty scaling with registration rate. Replaces unspecified "deterministic puzzle." | `agx-economics-and-adversarial-incentives.md`, FR-0176 |
| Sybil detection correlation engine | New research doc. Five-signal behavioral correlation with automated adjudication panels. | `sybil-detection-correlation-engine.md` (new), FR-0191 |
| Telegram bot + TUI setup | New research doc. Optional single-tenant Telegram dashboard and ratatui first-launch config wizard. | `agent-telemetry-interface.md` (new), FR-0193 |
| Bounty escrow mechanism | Task creation now requires escrowing bounty from creator's balance. Formalized escrow lifecycle. | `agx-economics-and-adversarial-incentives.md`, FR-0153b |

## Recent Design Changes (2026-05-06) — Committee BFT Hardening + Delegation

| Change | Summary | Docs Affected |
|--------|---------|---------------|
| 15% operator cap removed | Committee influence is now stake-proportional with anti-split clustering via stake-graph analysis. No per-operator seat cap exists. | `consensus-spec.md`, `FR-0002`, `ADR-0007`, `failure-model.md`, `agx-committee-bft-and-governance.md`, `decentralization-and-stack-benchmark.md`, `stage-01-protocol-core.md`, `state-model.md`, `FR-0183`, `checkpoint-2026-05-05d.md` |
| Overlap 33%→20% + two-epoch recency | Max committee overlap reduced from 33% to 20%. Validators cannot serve more than 2 consecutive epochs. | `consensus-spec.md`, `FR-0004`, `state-model.md`, `failure-model.md`, `agx-committee-bft-and-governance.md`, `ADR-0007.md` |
| VDF fallback hardened | Fallback seed now uses `previous_vdf_output \|\| hash(epoch_N-1_headers) \|\| epoch \|\| reveals` — no current-epoch malleable data. | `consensus-spec.md`, `FR-0003`, `agx-committee-bft-and-governance.md`, `ADR-0007.md` |
| Stake-graph analysis spec created | New spec defining N=3 hop ancestor trace, clustering algorithm, committee weight integration. Deterministic, on-chain only. | `stake-graph-analysis-spec.md` (new), 8 referencing files updated |
| Committee stall tiered fallback | Three tiers: Normal (67-100), Degraded (50-66, critical txs only), Emergency (0-49, halt + auto-recovery after 500 blocks). | `consensus-spec.md`, `FR-0001`, `failure-model.md`, `components.md` |
| Stake delegation added | New ADR-0015. Validators gain `self_bond`/`total_delegated`/`commission_rate`. 4 new tx types (DelegateTx, UndelegateTx, WithdrawDelegationTx, SetCommissionTx). Delegation unbonding: 7 days. Commission cap: 20%. Slash propagates proportionally. | `ADR-0015` (new), `staking-spec.md`, `consensus-spec.md`, `FR-0020a`, `state-model.md`, `failure-model.md`, `components.md`, `agx-committee-bft-and-governance.md`, `stage-01-protocol-core.md`, `traceability-matrix.md`, `requirements/index.md` |

## New Documents (2026-05-14)

| Document | Type | Description |
|----------|------|-------------|
| `docs/01-research/stack-evaluations/clatter-vs-ockam-secure-channel.md` | Research | Secure channel stack evaluation: clatter (PQ-Noise) vs ockam vs snow. Recommends clatter+ml-dsa. |
| `docs/03-architecture/decisions/ADR-0016-clatter-ml-dsa-secure-channel.md` | ADR | Replace Ockam with clatter+ml-dsa. Hybrid PQ (X25519+ML-KEM-768) key exchange, ML-DSA-65 identity signatures. |
| `docs/08-handoff/latest/parameter-audit.md` | Handoff | 41 [TUNE] parameters audited, all match spec defaults. |

## Recent Design Changes (2026-05-14)

| Change | Summary | Docs Affected |
|--------|---------|---------------|
| Ockam→clatter+ml-dsa | Ockam crate unresolvable (yanked dep). Replaced with clatter (Noise hybrid XX, PQ key exchange) + ml-dsa (ML-DSA-65 identity). ~10 deps vs 400+. ADR-0016. | `p2p-wire-spec.md`, `consensus-spec.md`, `components.md`, `stage-01-protocol-core.md`, `build-status.md`, `open-questions.md`, `planning/index.md`, `index.md` |

## New Documents (2026-05-15)

| Document | Type | Description |
|----------|------|-------------|
| `docs/03-architecture/decisions/ADR-0018-malachite-core-library-integration.md` | ADR | Malachite integration strategy: use `core-*` crates only (no libp2p), implement `SigningScheme` for ML-DSA-65, build effect handler + clatter network bridge. No fork required. ~1,500 lines of new code. |

## New Requirements (2026-05-06)

| FR | Description | File |
|----|-------------|------|
| FR-0020a | Stake Delegation | `docs/02-requirements/protocol/FR-0011-0020-staking-and-validator-lifecycle.md` |

## Overengineering Fixes (2026-05-06 — Round 2)

| Fix | What Changed | Files Affected |
|-----|-------------|---------------|
| Circuit breaker killed | Removed 3-tier CB, 22+ thresholds, IncidentRecord, CircuitBreakerState. EIP-1559 base fee is sole congestion mechanism. | ~15 files (incident-response-spec.md, ADR-0012, failure-model.md, state-model.md, components.md, 5+ FR files, research docs) |
| Mempool lanes removed | Replaced 4-lane reservation with single fee-ordered pool + evidence/governance fee discounts. | ~10 files (p2p-wire-spec.md, ADR-0006, consensus-spec.md, components.md, FR-0050) |
| Trust ladder simplified | 4 stages → 2 (untrusted/trusted). Removed reputation vector, decay, reviewer diversity, identity age. | ~10 files (state-model.md, ADR-0010, components.md, failure-model.md, research docs) |
| PDP simplified to 5 checks | Removed RiskClass, role/trust, ACL, taint, risk step-up, plan binding, policy bundle match. Protocol no longer does LLM safety. DenyReasons: 12→5. | ~15 files (policy-engine-spec.md, ADR-0003, interfaces.md, components.md, FR docs) |
| Review engine simplified | 3 phases → 2 (removed objective checks). Fixed 90/10 payout, 1 independence constraint. Removed quality-weighted scoring. Challenge clawback retained (funds never leave escrow). | ~10 files (review-engine-spec.md, ADR-0008, ADR-0017, collaboration-spec.md, FR docs) |
| TxType collapsed | 16 variants → 7 base types with action sub-enums. | ~12 files (consensus-spec.md, staking-spec.md, FR-0007, research docs) |

---

## Documentation Audit — Resolved Questions (2026-05-06)

All 5 open questions from the documentation audit were resolved:

| # | Question | Resolution |
|---|----------|------------|
| Q1 | Magic number provenance | Accepted as-is — values are initial estimates, adjustable via governance. |
| Q2 | FR-0154 missing | Stale reference from removed circuit-breaker feature. Removed from spec index and dependency lists. |
| Q3 | Cross-spec dependency on research docs | Promoted: collaboration-spec.md §1.1 now references FR-0084 + ADR-0013; §1.2 now references ADR-0014. No Layer 1 research doc references remain in Layer 4 specs. |
| Q4 | VDF integration schedule | VDF is scheduled: Stage 01 builds committee rotation with SHA3-256 sampling; Stage 03 (Validation) includes VDF calibration and tuning for production randomness. In-scope and tracked. |
| Q5 | ADR-0012 stale references | Accepted — ADR-0012 status is `accepted` (circuit breaker removed). No further action needed. |

---

## Open Design Questions

| # | Question | Source | Status |
|---|----------|--------|--------|
| Q1 | **FR-0142/FR-0143 have no spec implementation.** `incident-response-spec.md` only covers FR-0145 (fee market congestion). FR-0142 (Incident State Machine) and FR-0143 (Emergency Mode Parameter Overrides) were designed for the circuit-breaker hierarchy which ADR-0012 removed. Should these FRs be marked superseded, or should a new spec be written for them? | `docs/04-specifications/security/incident-response-spec.md` | **RESOLVED** — Marked superseded in FR file and index. EIP-1559 base fee is sole congestion mechanism per `incident-response-spec.md` §1. |
| Q2 | **FR-0026 / FR-0087 (Review Sandbox) near-identical specs.** Both define review sandboxes with identical mechanics (fresh context, single `review` tool, 30-min timeout). FR-0026 targets governance proposals, FR-0087 targets topic merges. Should they cross-reference a single definition to prevent drift? | `docs/02-requirements/protocol/FR-0021-0030-governance-and-git-head.md`, `docs/02-requirements/runtime/FR-0076-0090-collaboration-layer.md` | **RESOLVED** — Canonical definition in `trust-boundaries.md` §6; both FRs and both specs cross-reference it. |
| Q3 | **FR-0121-0135 (Prompt Injection Defense) — 15 requirements with no spec.** The spec index maps them to a research doc instead of a spec file. The checkpoint amendment claimed Section 4 was added to `policy-engine-spec.md` but the file was never updated. Needs a proper spec written. | `docs/04-specifications/index.md`, `docs/04-specifications/runtime/policy-engine-spec.md`, `docs/08-handoff/latest/checkpoint-2026-05-02-spec-amendment.md` | **RESOLVED** — No action taken. Prompt injection defense is a runtime evaluation/integrity concern, not a PDP authorization concern. If needed post-mainnet, create a separate `prompt-injection-defense-spec.md`. Deferred. |
| Q4 | **Data model drift between specs.** Three entities have field mismatches between their canonical spec definition and `state-model.md`: (a) `TASK` — `depends_on` missing from state-model, 3 fields (`metadata_hash`, `sponsor_id`, `requester_pubkey`) missing from collaboration-spec, `topic_id` type mismatch; (b) `REVIEW_ASSIGNMENT` — state-model uses 1 struct, review-engine-spec uses 3; (c) `SYSTEM_PARAMETERS` — different fields between `state-model.md` and `staking-spec.md`. | `docs/04-specifications/runtime/collaboration-spec.md`, `docs/03-architecture/data-model/state-model.md`, `docs/04-specifications/protocol/staking-spec.md` | **RESOLVED** — `state-model.md` designated canonical source. `collaboration-spec.md` Task struct aligned to state-model fields. `staking-spec.md` SystemParameters annotated to reference state-model for full parameter set. REVIEW_ASSIGNMENT: state-model is canonical; review-engine-spec uses the same struct with runtime context. |
| Q5 | **Magic numbers without rationale.** Challenge window (144 blocks), review deadlines (72h/24h), lease TTL (20m/5m), resource limits (4GB/2CPU/10GB), inbox budgets (2000/hr, 500/5min), 12.5% fee cap have no documented rationale for their specific values. All are governance-adjustable but have no research-grounded initial values. | Cross-cutting across FR files and specs | **RESOLVED** — All accepted as `[TUNE]` parameters. Calibration to occur in Stage 03 (Validation) with real testnet data. |
| Q6 | **TaskRecord/Task struct drift.** `consensus-spec.md` §2.4 defines `TaskRecord` with 12 fields; `collaboration-spec.md` §1.3 defines `Task` with different fields (has `primary_owner`, `lease_expires_height` but missing `sponsor_id`, `metadata_hash`, `expires_at_height`). Same entity, two schemas. | `docs/04-specifications/protocol/consensus-spec.md` §2.4, `docs/04-specifications/runtime/collaboration-spec.md` §1.3 | **RESOLVED** — `state-model.md` is canonical. `collaboration-spec.md` Task struct updated to include all state-model fields (`metadata_hash`, `sponsor_id`, `requester_pubkey`, `depends_on`). `consensus-spec.md` TaskRecord references state-model TASK entity. |
| Q7 | **Nice-to-have requirements with full spec treatment.** FR-0104 (Emergency Escalation), NFR-0029 (Backup/Restore), NFR-0030 (Formal Verification) all tagged `nice-to-have` but carry full acceptance criteria, dependencies, and source research. Either promote to `should-have`/`must-have` or strip down to stub references. | `docs/02-requirements/runtime/FR-0091-0105-inbox-and-attention.md`, `docs/02-requirements/economics/NFR-0016-0030-security-and-reliability.md` | **RESOLVED** — Demoted to "deferred" status. Acceptance criteria stripped to single-line stubs. Post-mainnet consideration. |
| Q8 | **Undefined constants in gas cost formula.** `collaboration-spec.md` §1.4: `gas_cost = base_cost + per_child * N + per_edge * E` — `base_cost`, `per_child`, `per_edge` never defined or given defaults anywhere in the spec corpus. | `docs/04-specifications/runtime/collaboration-spec.md` §1.4 | **RESOLVED** — Marked as `[TUNE]` with defaults: `base_cost = 1000`, `per_child = 100`, `per_edge = 50` (gas units). Governance-adjustable. |
| Q9 | **Undefined "failure" in agent runtime guard.** `agent-runtime-spec.md` §2.1: "block after 3 failures within 1 hour" — what constitutes a "failure" is never defined. §2.5 clarifies non-zero exit codes do NOT trigger the guard, leaving the failure condition unspecified. | `docs/04-specifications/runtime/agent-runtime-spec.md` §2.1, §2.5 | **RESOLVED** — Guard removed entirely. Unimplementable without a clear definition. |
| Q10 | **ADR-0008 clawback ambiguity.** "clawback mechanism" referenced in Related field but Decision section explicitly states clawback was removed. Challenge clawback (FR-0172) is retained but distinct from the removed quality-weighted clawback. | `docs/03-architecture/decisions/ADR-0008-two-phase-quality-pipeline.md` | **RESOLVED** — Clarifying note added to ADR-0008 distinguishing challenge clawback (retained) from quality-weighted clawback (removed). |

---

*Last updated: 2026-05-17c (Malachite SigningScheme + Context implemented; gap fill complete; P2P conformance test fix)*
