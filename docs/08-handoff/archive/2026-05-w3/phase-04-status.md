# Phase 04 Handoff: Planning (Layer 5) Delivered

**Date:** 2026-05-02
**Completed by:** Agent
**Phase:** Layer 5 (Planning)

---

## What was done

- Read all prerequisite documents: BUILD-SYSTEM.md (Layer 5 gates), TEMPLATES.md (Stage format), GLOSSARY.md, PROJECT-STATUS.md
- Read Phase 03 handoff package for context
- Read all Layer 4 specifications (14 specs across 4 domains)
- Read Layer 3 architecture (12 components, 12 ADRs)
- Created 6 planning documents: 1 index + 5 stage definitions
- Every stage follows the TEMPLATES.md Stage Definition format (Inputs, Outputs, Exit Criteria, Duration Estimate, Dependencies, Risk Areas)
- Each stage includes week-by-week breakdown with concrete deliverables and exit checkpoints
- All 14 specs and 12 components mapped to implementation stages
- Decentralisation audit gate (Specifications → Planning) checked — passes (Layer 4 audit clean, all stage definitions account for decentralised control)

---

## Artifact Manifest

```
docs/05-planning/index.md
docs/05-planning/stages/stage-00-foundation.md
docs/05-planning/stages/stage-01-protocol-core.md
docs/05-planning/stages/stage-02-agent-runtime.md
docs/05-planning/stages/stage-03-validation.md
docs/05-planning/stages/stage-04-mainnet-prep.md
```

---

## Stage Summary

| Stage | Name | Duration | Components | Exit Checkpoints |
|-------|------|----------|------------|-----------------|
| 00 | Foundation | 1-2 weeks | — | Workspace, CI, local testnet scaffold |
| 01 | Protocol Core | 6-8 weeks | C1, C2, C3, C5, C7, C8 | Multi-node chain, staking, fees, P2P, artifact storage |
| 02 | Agent Runtime | 6-8 weeks | C4, C6, C9, C10, C11, C12 | Agent lifecycle, PDP, review pipeline, governance |
| 03 | Validation | 4-6 weeks | All | Conformance matrix, adversarial suite, load tests, security audit |
| 04 | Mainnet Prep | 4-6 weeks | All | SLOs, runbooks, private testnet, incident drill, launch checklist |

**Total estimated duration:** 21-30 weeks (5-7.5 months). Realistic estimate: 30 weeks including contingency.

---

## Gate Status: Specifications → Planning

Per BUILD-SYSTEM.md gate (Specifications → Planning):
- Every spec has conformance test hooks (Section X.7) — verified in Phase 3
- Trust-assumption inventory complete (Section X.8) — verified in Phase 3
- All FRs mapped to spec sections — verified in Phase 3
- Decentralisation audit (Layer 4) — PASS (no issues)

**Gate result: READY FOR LAYER 6 (Validation — formal validation strategy).**

---

## Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Bottom-up ordering (chain → agents → validation → ops) | Infrastructure must exist before agents; agents must work before validation; validation before launch |
| 6-8 week stage windows | Balances momentum against complexity. Long enough for meaningful progress, short enough to maintain context |
| Protocol Core bundles 6 components | Consensus, staking, fees, P2P, state sync, and artifact storage are tightly coupled — building together avoids integration churn |
| Agent Runtime bundles 6 components | PDP, runtime, collaboration, review, governance, and fast-path form the agent layer — separate from chain infrastructure |
| `[TUNE]` parameters use spec defaults initially | Production values derived in Stage 03 from testnet data. Prevents premature optimization |
| Stage 00 includes testnet scaffold | Single-node chain must boot before Protocol Core can validate consensus progress |
| Private testnet in Stage 04 (not Stage 03) | Stage 03 validates correctness and performance; Stage 04 validates operations and procedures on a 20+ node simulated network |

---

## Risk Register

| Risk | Severity | Stage | Mitigation |
|------|----------|-------|------------|
| Malachite BFT integration complexity | Medium | 01 | 2 weeks reserved for integration; fallback to vendored fork |
| PDP determinism across platforms | Medium | 02 | BTreeMap, sorted Vec, no floats; cross-platform CI |
| Adversarial testing reveals protocol flaw | Medium | 03 | Governance engine in Stage 02 can process spec amendments |
| Private testnet not representative of production | Medium | 04 | Extrapolate from 100-node Stage 03 load tests |
| [TUNE] parameter calibration without production data | Low | 03 | Spec defaults are reasonable; calibration in Stage 03 with load data |
| LLM provider availability/cost | Low | 02 | Multi-provider abstraction; Ollama local fallback |
| Sandbox escape vectors | Medium | 03 | WASM/Firecracker; full threat model in Stage 03 |
| Genesis ceremony coordination | Low | 04 | Over-recruit validators; dry-run before actual ceremony |

---

## Next Stage Inputs (for Phase 5: Implementation — Stage 00)

1. Read `docs/05-planning/index.md` for stage overview and spec-to-stage mapping.
2. Begin with `docs/05-planning/stages/stage-00-foundation.md`:
   - Create Cargo workspace with 12 crate scaffolds.
   - Write `justfile` with build/test/lint/fmt/bench/audit targets.
   - Set up CI pipeline (GitHub Actions).
   - Create local testnet scaffold (genesis, single-validator, start/stop scripts).
3. Cross-reference with Layer 4 specs for data structures and interfaces.
4. Freeze spec versions before starting implementation — spec changes after Stage 01 start require governance proposal.

---

## How to resume

1. Read `docs/05-planning/index.md` for full stage inventory and traceability.
2. Read target stage file (e.g., `stage-00-foundation.md`) for week-by-week breakdown, exit criteria, and risk areas.
3. Cross-reference with `docs/04-specifications/index.md` for spec-level requirements.
4. Cross-reference with `docs/03-architecture/index.md` for component boundaries and interfaces.
5. Begin Stage 00 implementation. Track progress against exit criteria.
6. At each stage boundary, produce a Phase 05 checkpoint handoff and update PROJECT-STATUS.md.
