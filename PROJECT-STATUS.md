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
| Phase 5+: Build | IN PROGRESS (Stage 00 complete. Stage 01 next: Protocol Core — Minimum Viable Chain) |

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

None.

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

## Next Actions

1. ~~Begin Stage 00 (Foundation): Create Cargo workspace with 12 crate scaffolds, `justfile`, CI pipeline, local testnet scaffold.~~ Stage 00 complete. See `docs/08-handoff/latest/checkpoint-2026-05-04.md`.
2. ~~Bug audit (code) completed. See `docs/01-research/_audit-bugs-2026-05-04.md`.~~
3. ~~Pre-Stage-01 amendment: Agent tools expanded 5→9, seed index created at `/ideas/`, ADR-0013. See `checkpoint-2026-05-05.md`.~~
4. ~~Seed-centric model + user-task-submission pipeline propagated through L2-L5: 7 FRs, ADR-0014, 4 specs, planning. See `checkpoint-2026-05-05b.md` and `checkpoint-2026-05-05c.md`.~~
5. ~~Documentation-layer bug audit completed. See `docs/01-research/_audit-bugs-2026-05-05.md`.~~
6. ~~Stage 01 Week 1-2 (C1 + C2): Consensus Engine + State Machine & SMT. See `checkpoint-2026-05-05d.md`.~~
7. ~~Architecture amendments (2026-05-06): 15% cap removed, overlap 33%→20%, VDF fallback hardened, stake-graph spec created, committee stall tiered, delegation spec'd. Changes across ~40 files. See `checkpoint-2026-05-06.md`.~~
8. **PENDING:** 5 code changes required before Stage 01 Week 3-4: overlap+two-epoch guard, VDF fallback, stall thresholds, stake-graph clustering, delegation subsystem. See `checkpoint-2026-05-06.md` for exact file-level details.
9. Stage 01 Week 3-4 (C3 + C5): Staking & Validator Manager + Fee Market. (Apply pending code changes first.)
10. Stage 01 Week 5-6 (C7 + C8): P2P Networking + Artifact Storage.
11. Stage 01 Week 7-8: Integration, soak test, polish.
12. Stage 02 (Agent Runtime): Layer agent behavior, PDP, collaboration, review, governance, fast-path on top of the chain.
13. Stage 03 (Validation): Full conformance matrix, adversarial test suite, load testing, security audit, parameter calibration.
14. Stage 04 (Mainnet Prep): SLOs, monitoring, runbooks, private testnet soak, incident drill, launch checklist.
15. Freeze all 15 specs before Stage 01 implementation continues. Post-freeze spec changes require governance proposals.

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

## New Documents (2026-05-06)

| Document | Type | Description |
|----------|------|-------------|
| `docs/04-specifications/protocol/stake-graph-analysis-spec.md` | Spec | N-hop ancestor trace for correlated validator detection. Deterministic, on-chain clustering algorithm. |
| `docs/03-architecture/decisions/ADR-0015-stake-delegation.md` | ADR | Delegation model: self-bond + delegated stake for committee weight, commission, proportional slash propagation. |

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
| Review engine simplified | 3 phases → 2 (removed objective checks). Fixed payout, 1 independence constraint. Removed clawback, quality-weighted scoring. | ~10 files (review-engine-spec.md, ADR-0008, collaboration-spec.md, FR docs) |
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

*Last updated: 2026-05-06 (Documentation audit: ~47 stale/simplified items fixed across L2-L5 docs, 5 open questions flagged.)*
