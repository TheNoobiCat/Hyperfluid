# Project Status

This file tracks the current state of Hyperfluid's build pipeline. It is separate from the build system definition so that the build system can remain focused on process while this file evolves with the project.

---

## Current Phase

| Phase | Status |
|-------|--------|
| Phase 0: Research Audit | COMPLETE |
| Phase 0: Decentralisation Audit | COMPLETE (issues in `docs/01-research/_overengineered.md`, fixes applied) |
| Phase 1: Requirements | NOT STARTED |
| Phase 2: Architecture | NOT STARTED |
| Phase 3: Specifications | NOT STARTED |
| Phase 4: Planning | NOT STARTED |
| Phase 5+: Build | NOT STARTED |

---

## Research Gaps (to resolve before Layer 2)

| Gap | Should Live In | Current State |
|-----|----------------|---------------|
| Token budget resource model (LOCAL runtime only) | `runtime/agent-runtime-spec.md` | Written: `docs/01-research/agents/token-budget-resource-model.md` |
| VDF-based committee randomness | `protocol/consensus-spec.md` | Partial in research; needs formal VDF parameter spec |
| Reviewer independence / operator-cluster diversity | `runtime/review-engine-spec.md` | Written in research; needs formal stake-graph clustering algorithm |
| No-vote timeout fairness proof | `protocol/governance-spec.md` | Implied only |
| Plan replay protection E2E | `runtime/policy-engine-spec.md` + storage specs | Partial |
| Telemetry threat model | `security/` research → specs | Written: `docs/01-research/security/telemetry-threat-model.md` |
| Sandbox escape analysis | `security/` research → specs | Unwritten |
| Content-addressing SLA | `storage/artifact-availability-spec.md` | Assumed |
| Economic timing parameters | `protocol/` specs | Partial in research |

---

## Decentralisation Audit Status

**Result:** PASS (with fixes applied)

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

1. Create Layer 2 (Requirements) from current research
2. Update traceability matrix as requirements are extracted

---

*Last updated: 2026-05-01*
