# Project Status

This file tracks the current state of Hyperfluid's build pipeline. It is separate from the build system definition so that the build system can remain focused on process while this file evolves with the project.

---

## Current Phase

| Phase | Status |
|-------|--------|
| Phase 0: Research Audit | COMPLETE |
| Phase 0: Decentralisation Audit | COMPLETE (issues in `docs/01-research/_overengineered.md`, fixes applied) |
| Phase 1: Requirements | COMPLETE (190 requirements: 160 FR + 30 NFR) |
| Phase 2: Architecture | COMPLETE (12 components, 12 ADRs, component model, interfaces, trust boundaries, failure model. Gate: PASS) |
| Phase 3: Specifications | COMPLETE (14 specs across 4 domains, all FRs mapped, trust-assumption inventories complete. Gate: PASS) |
| Phase 4: Planning | COMPLETE (5 stages, 21-30 week roadmap, spec-to-stage mapping, risk register. Gate: PASS) |
| Phase 5+: Build | IN PROGRESS (Stage 00, Week 1 complete) |

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

## Next Actions

1. ~~Begin Stage 00 (Foundation): Create Cargo workspace with 12 crate scaffolds, `justfile`, CI pipeline, local testnet scaffold.~~ Week 1 complete. See `docs/08-handoff/latest/checkpoint-2026-05-02.md`.
2. Stage 00 Week 2: Testnet scaffold (genesis cerberus, single-validator config, start/stop scripts), cold-start verification, `DEVELOPMENT.md`, `cargo-deny` audit pass.
3. Stage 01 (Protocol Core): Build Minimum Viable Chain — C1 Consensus, C2 State Machine, C3 Staking, C5 Fee Market, C7 P2P Networking, C8 Artifact Storage.
4. Stage 02 (Agent Runtime): Layer agent behavior, PDP, collaboration, review, governance, fast-path on top of the chain.
5. Stage 03 (Validation): Full conformance matrix, adversarial test suite, load testing, security audit, parameter calibration.
6. Stage 04 (Mainnet Prep): SLOs, monitoring, runbooks, private testnet soak, incident drill, launch checklist.
7. Freeze all 14 specs before Stage 01 implementation starts. Post-freeze spec changes require governance proposals.

## Layer 4 Spec Inventory (delivered)

| Spec | Covers (Component/FR) | Status |
|------|----------------------|--------|
| `protocol/consensus-spec.md` | C1 Consensus Engine, C2 State Machine (FR-0001-0010) | complete |
| `protocol/staking-spec.md` | C3 Staking & Validator Manager (FR-0011-0020) | complete |
| `protocol/governance-spec.md` | C4 Governance Engine (FR-0021-0030) | complete |
| `protocol/p2p-wire-spec.md` | C7 P2P Networking (FR-0041-0050) | complete |
| `protocol/fastpath-spec.md` | C6 Fast-Path Topic Protocol (FR-0031-0040) | complete |
| `protocol/fee-market-spec.md` | C5 Fee Market (FR-0146-0160) | complete |
| `storage/state-sync-spec.md` | C2 State Machine & SMT (FR-0010, NFR-0009) | complete |
| `storage/artifact-availability-spec.md` | C8 Artifact Availability (FR-0051-0060) | complete |
| `runtime/agent-runtime-spec.md` | C10 Agent Runtime (FR-0061-0075) | complete |
| `runtime/policy-engine-spec.md` | C9 Policy Decision Point (FR-0106-0120) | complete |
| `runtime/review-engine-spec.md` | C12 Economics - review markets (FR-0161-0175) | complete |
| `runtime/collaboration-spec.md` | C11 Collaboration & Inbox (FR-0076-0105) | complete |
| `security/telemetry-spec.md` | Telemetry integrity (FR-0060, FR-0139-0141, NFR-0020-0021) | complete |
| `security/incident-response-spec.md` | Incident response & recovery (FR-0142-0145) | complete |

**Dropped:** `security/key-management-spec.md` — no Layer 1 research doc backing; key rotation is covered in `policy-engine-spec.md` Section 3 (FR-0118, NFR-0024) and key binding is covered in `consensus-spec.md` Section 2 (FR-0005, FR-0006).

---

*Last updated: 2026-05-02 (Phase 5 Stage 00 Week 1 complete; Layer 4 spec gap resolution complete — all 190 FRs traceable, 29 subsections added, FR-0121-0135 prompt injection + FR-0136-0138 sandbox folded into existing specs)*
