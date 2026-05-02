# Checkpoint — 2026-05-02 (Layer 4 Spec Gap Resolution)

**Completed:** Comprehensive spec gap resolution across all 14 specs. All 190 FRs now traceable to normative spec sections.

## Gap Resolution Summary

### Orphaned FRs (18) — folded into existing specs
- **FR-0121-0135 (Prompt Injection Defense):** New Section 4 in `policy-engine-spec.md` — attack corpus, classifier as auxiliary signal, canary drift detection, multi-turn trigger detection, role confusion blocking, staged rollouts, signed eval telemetry, hidden scenario pools.
- **FR-0136-0138 (Sandbox):** Folded into `agent-runtime-spec.md` Section 4 (process isolation) — sandbox escape prevention, network-only policy scope, agent-node process separation.

### Unclaimed FRs (26) — mapped to correct specs
- FR-0148, FR-0149, FR-0150, FR-0153 → `review-engine-spec.md`
- FR-0151 → `staking-spec.md`
- FR-0152 → `p2p-wire-spec.md`
- FR-0154 → `incident-response-spec.md`
- FR-0155 → `governance-spec.md`
- FR-0156, FR-0157, FR-0158, FR-0176-0190 → `collaboration-spec.md`

### Structural gaps — 29 subsection additions
- 12 sections: added missing X.6 Versioning subsections
- 5 sections: added missing X.8 Trust-Assumption Inventory subsections
- 4 sections: added missing X.4 State Transitions subsections
- 4 sections: fixed misnumbered subsection labels (2.4→2.5, 3.4→3.5)
- 1 broken cross-reference: `consensus-spec.md:95` → `staking-spec.md Section 1`

### Index fixes
- FR→Spec Mapping table: expanded from 16 to 24 rows with accurate per-FR mappings
- Removed redundant FR-0091-0105 (subset of FR-0076-0105)
- Fixed [TUNE] promotion thresholds location (3.2 → 3.3)
- 5 spec description rows updated for new FR coverage

### NFR traceability
- 26 of 30 NFRs remain implicitly covered (cross-cutting). 4 explicitly referenced in spec bodies. Deemed acceptable for characteristics applying broadly.

## What changed
| Layer | File | Change |
|-------|------|--------|
| L4 | `consensus-spec.md` | Fixed broken xref |
| L4 | `staking-spec.md` | +S2 subsections, +FR-0151 header |
| L4 | `governance-spec.md` | +FR-0155 header |
| L4 | `p2p-wire-spec.md` | +S2 subsections, +FR-0152 header |
| L4 | `fee-market-spec.md` | +S2 subsections |
| L4 | `artifact-availability-spec.md` | +S2 subsections |
| L4 | `agent-runtime-spec.md` | +S2/S3/S4 subsections, sandbox FRs folded, header updated |
| L4 | `policy-engine-spec.md` | +S2 subsections, +S4 prompt injection defense, header updated |
| L4 | `review-engine-spec.md` | +FR-0148/0149/0150/0153 header |
| L4 | `collaboration-spec.md` | +S3 subsection, +FR-0176-0190 header |
| L4 | `telemetry-spec.md` | +S2 subsections |
| L4 | `incident-response-spec.md` | +S2 subsections, +FR-0154 header |
| L4 | `index.md` | Full FR→Spec table rewrite, spec rows updated |

## Verification
- All 190 FRs have at least one spec + section mapping in index
- All 14 specs have complete trust-assumption inventories (all sections)
- All 29 spec sections now have the required 8 subsections
- No broken cross-references remain
- 0 code files modified

**Next:** Stage 00 Week 2 continues as planned. No downstream spec revisions needed. Complete specs available for Stages 01-04.

**Blockers:** None.
