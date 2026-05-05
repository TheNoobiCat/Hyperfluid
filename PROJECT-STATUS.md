# Project Status

This file tracks the current state of Hyperfluid's build pipeline. It is separate from the build system definition so that the build system can remain focused on process while this file evolves with the project.

---

## Current Phase

| Phase | Status |
|-------|--------|
| Phase 0: Research Audit | COMPLETE |
| Phase 0: Decentralisation Audit | COMPLETE (issues in `docs/01-research/_overengineered.md`, fixes applied) |
| Phase 1: Requirements | COMPLETE (195 requirements: 165 FR + 30 NFR) |
| Phase 2: Architecture | COMPLETE (12 components, 12 ADRs, component model, interfaces, trust boundaries, failure model. Gate: PASS) |
| Phase 3: Specifications | COMPLETE (14 specs across 4 domains, all FRs mapped, trust-assumption inventories complete. Gate: PASS) |
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

## Next Actions

1. ~~Begin Stage 00 (Foundation): Create Cargo workspace with 12 crate scaffolds, `justfile`, CI pipeline, local testnet scaffold.~~ Stage 00 complete. See `docs/08-handoff/latest/checkpoint-2026-05-04.md`.
2. ~~Bug audit (code) completed. See `docs/01-research/_audit-bugs-2026-05-04.md`.~~
3. ~~Pre-Stage-01 amendment: Agent tools expanded 5→9, seed index created at `/ideas/`, ADR-0013. See `checkpoint-2026-05-05.md`.~~
4. ~~Seed-centric model + user-task-submission pipeline propagated through L2-L5: 7 FRs, ADR-0014, 4 specs, planning. See `checkpoint-2026-05-05b.md` and `checkpoint-2026-05-05c.md`.~~
5. ~~Documentation-layer bug audit completed. See `docs/01-research/_audit-bugs-2026-05-05.md`.~~
6. ~~Stage 01 Week 1-2 (C1 + C2): Consensus Engine + State Machine & SMT. See `checkpoint-2026-05-05d.md`.~~
7. Stage 01 Week 3-4 (C3 + C5): Staking & Validator Manager + Fee Market.
8. Stage 01 Week 5-6 (C7 + C8): P2P Networking + Artifact Storage.
9. Stage 01 Week 7-8: Integration, soak test, polish.
10. Stage 02 (Agent Runtime): Layer agent behavior, PDP, collaboration, review, governance, fast-path on top of the chain.
11. Stage 03 (Validation): Full conformance matrix, adversarial test suite, load testing, security audit, parameter calibration.
12. Stage 04 (Mainnet Prep): SLOs, monitoring, runbooks, private testnet soak, incident drill, launch checklist.
13. Freeze all 14 specs before Stage 01 implementation starts. Post-freeze spec changes require governance proposals.

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
| `protocol/consensus-spec.md` | C1 Consensus Engine, C2 State Machine (FR-0001-0010, FR-0194 — TaskCreateTx) | complete |
| `protocol/staking-spec.md` | C3 Staking & Validator Manager (FR-0011-0020) | complete |
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

## New Research Documents

- `docs/01-research/agents/sybil-detection-correlation-engine.md` — Sybil detection via behavioral correlation, automated adjudication, economic deterrence
- `docs/01-research/agents/agent-telemetry-interface.md` — Telegram bot dashboard and ratatui TUI setup wizard for agent operators

---

*Last updated: 2026-05-05 (Stage 01 Week 1-2 complete: C1 Consensus + C2 State Machine & SMT; 56 workspace tests passing)*
