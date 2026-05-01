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

Per `BUILD-SYSTEM.md`, every gap must be either **resolved in research** or **converted to an explicit requirement** before Layer 2. Gaps marked "Explicit Requirement" will be captured as FR/NFR items rather than additional research documents.

| Gap | Should Live In | Current State | Layer 2 Action |
|-----|----------------|---------------|----------------|
| Token budget resource model (LOCAL runtime only) | `runtime/agent-runtime-spec.md` | Written: `docs/01-research/agents/token-budget-resource-model.md` | Extract to FR-token-budget-* |
| VDF-based committee randomness | `protocol/consensus-spec.md` | Partial in research (`agx-committee-bft-and-governance.md` lines 147-161); needs formal difficulty parameters | Extract to FR-consensus-vdf-* with parameter placeholders for spec |
| Reviewer independence / operator-cluster diversity | `runtime/review-engine-spec.md` | Written in research (`proof-of-work-quality-and-review-markets.md` lines 273-277); needs formal clustering algorithm | Extract to FR-review-independence-* with algorithm placeholder for spec |
| No-vote timeout fairness proof | `protocol/governance-spec.md` | Implied only (`agx-committee-bft-and-governance.md` lines 140-144) | **Explicit Requirement**: FR-governance-no-vote-fairness with assumption note that systematic exclusion analysis is deferred to validation |
| Plan replay protection E2E | `runtime/policy-engine-spec.md` + storage specs | Partial in research (`network-policy-engine-spec.md` Section 5) | Extract to FR-policy-replay-*; E2E trace deferred to Layer 6 (Validation) |
| Telemetry threat model | `security/` research → specs | Written: `docs/01-research/security/telemetry-threat-model.md` | Extract to NFR-telemetry-integrity-* |
| Sandbox escape analysis | `security/` research → specs | Unwritten | **Explicit Requirement**: FR-security-sandbox-escape-prevention with reference to `infinite-agent.md` resource limits and `prompt-injection-and-network-policy-boundary.md` taint tracking. Short research note optional. |
| Content-addressing SLA | `storage/artifact-availability-spec.md` | Assumed (`artifact-availability-and-retention.md` defines tiers but no SLA values) | **Explicit Requirement**: NFR-storage-availability-* with concrete min-replica and repair-latency targets |
| Economic timing parameters | `protocol/` specs | Partial in research (challenge window, lease TTL, heartbeat, review deadlines are all concrete) | Extract to FR-economics-timing-* |

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
