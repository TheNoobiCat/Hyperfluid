# Phase 03 Handoff: Specifications (Layer 4) Delivered

**Date:** 2026-05-02
**Completed by:** Agent
**Phase:** Layer 4 (Specifications)

---

## What was done

- Read all prerequisite documents: BUILD-SYSTEM.md (Layer 4 gates), TEMPLATES.md (Spec format), GLOSSARY.md, PROJECT-STATUS.md
- Read Phase 01 and Phase 02 handoff packages for context
- Read all Layer 3 architecture (6 documents, 12 ADRs) and Layer 2 requirements (190 FR/NFR)
- Created 14 specification documents across 4 domains (protocol, storage, runtime, security)
- Every spec contains 8 mandatory sections per TEMPLATES.md, including X.8 Trust-Assumption Inventory
- Every spec uses exact numbers (no placeholders); uncertain parameters marked `[TUNE]` with reasonable defaults
- All 190 requirements mapped to spec sections
- All 12 ADRs mapped to spec documents

---

## Artifact Manifest

```
docs/04-specifications/index.md
docs/04-specifications/protocol/consensus-spec.md
docs/04-specifications/protocol/staking-spec.md
docs/04-specifications/protocol/governance-spec.md
docs/04-specifications/protocol/p2p-wire-spec.md
docs/04-specifications/protocol/fastpath-spec.md
docs/04-specifications/protocol/fee-market-spec.md
docs/04-specifications/storage/state-sync-spec.md
docs/04-specifications/storage/artifact-availability-spec.md
docs/04-specifications/runtime/agent-runtime-spec.md
docs/04-specifications/runtime/policy-engine-spec.md
docs/04-specifications/runtime/review-engine-spec.md
docs/04-specifications/runtime/collaboration-spec.md
docs/04-specifications/security/telemetry-spec.md
docs/04-specifications/security/incident-response-spec.md
```

---

## Spec Summary

| # | Spec | Sections | FRs Covered | Key Content |
|---|------|----------|-------------|-------------|
| 1 | consensus-spec.md | 2 | FR-0001-0010 | Committee BFT, VDF randomness, SMT state, transaction types |
| 2 | staking-spec.md | 2 | FR-0011-0020 | Validator lifecycle, slashing, governance voting eligibility |
| 3 | governance-spec.md | 2 | FR-0021-0030 | git:head governance, sandbox review, anti-flood |
| 4 | p2p-wire-spec.md | 2 | FR-0041-0050 | Discovery, gossip, relays, mempool lanes |
| 5 | fastpath-spec.md | 1 | FR-0031-0040 | Topic merges, quorum certs, challenge windows, promotion |
| 6 | fee-market-spec.md | 2 | FR-0146, FR-0147, FR-0159-0160 | EIP-1559 fees, validator rebates, manipulation defense |
| 7 | state-sync-spec.md | 1 | FR-0010, NFR-0009 | Snap sync, full sync, crash recovery |
| 8 | artifact-availability-spec.md | 2 | FR-0051-0060 | Content-addressed storage, PoP, retention, repair |
| 9 | agent-runtime-spec.md | 4 | FR-0061-0075 | Agent loop, tools, system prompt, isolation |
| 10 | policy-engine-spec.md | 2 | FR-0106-0120 | 10-step PDP, risk step-up, quota matrix |
| 11 | review-engine-spec.md | 1 | FR-0161-0175 | 3-phase quality, reviewer independence, settlement |
| 12 | collaboration-spec.md | 3 | FR-0076-0105, 0091-0105 | Task board, inbox, trust ladder, reputation |
| 13 | telemetry-spec.md | 2 | FR-0060, FR-0139-0141 | Signed envelopes, aggregation, reconciliation |
| 14 | incident-response-spec.md | 2 | FR-0142-0145 | Incident FSM, emergency mode, recovery |

---

## Gate Status: Architecture → Specifications

| Check | Status |
|-------|--------|
| Every spec has conformance test hooks (Section X.7) | PASS (14/14) |
| Every spec has trust-assumption inventory (Section X.8) | PASS (14/14) |
| Trust boundaries align with architecture trust-boundaries.md | PASS |
| All FRs mapped to at least one spec section | PASS |
| All ADRs cross-referenced | PASS |
| Data structures are deterministic | PASS |
| Mermaid diagrams present where needed | PASS (state-model.md, trust-boundaries.md carry forward) |

**Gate result: READY FOR LAYER 5 (Planning).**

---

## Decentralisation Audit (Layer 4)

All 14 specs were reviewed against the decentralisation audit checklist from BUILD-SYSTEM.md:

1. **External trust inventory:** PASS. Every spec Section X.8 inventories external dependencies with justifications and trust-minimised alternatives. Zero external oracles/services mandated.
2. **Centralised coordination:** PASS. No spec mandates single dispatcher, scheduler, moderator, or admin override. All coordination is protocol-enforced (BFT, task board soft leases, governance voting, EIP-1559 fee market).
3. **Verifiable economic signals:** PASS. All reward/penalty parameters reference cryptographically verifiable on-chain records. Self-reported local metrics excluded from economic calculations.
4. **Single points of failure:** PASS. Committee overlap (max 33%), relay diversity, repair coordinator redundancy, EIP-1559 base fee dynamics for congestion handling.
5. **Sybil resistance:** PASS. Challenge-response Proof-of-Agent, locked airdrop bonds, stake-graph diversity, whitewash guard, trust ladder.

**No new issues found.** No `_overengineered.md` created.

---

## [TUNE] Parameter Inventory

Parameters marked with [TUNE] in specs — defaults are reasonable starting values; target for testnet calibration:

| Parameter | Default | Spec | Description |
|-----------|---------|------|-------------|
| min_base_fee | 1,000,000 atto-AGX | fee-market-spec.md | Minimum fee floor |
| emergency_fee_floor | 10x normal minimum | incident-response-spec.md | Emergency mode fee |
| promotion_thresholds | See collaboration-spec.md 3.3 | collaboration-spec.md | Trust ladder thresholds |
| recovery_exit_hysteresis | 0.7x entry thresholds | incident-response-spec.md | Circuit-breaker hysteresis |
| fee_adjustment_denominator | 8 | fee-market-spec.md | Base fee adjustment smoothness |
| cb_window_persist | 3 windows | incident-response-spec.md | Sustained breach before escalation |
| post_incident_quota_duration | 3 epochs | incident-response-spec.md | Recovery ramp-up duration |

---

## Next Stage Inputs (for Layer 5: Planning)

Recommended priority ordering for implementation stages:

1. **Stage 1 — Minimum Viable Chain:** C1 Consensus Engine + C2 State Machine + C7 P2P Networking
2. **Stage 2 — Economic Foundation:** C3 Staking + C5 Fee Market
3. **Stage 3 — Security Boundary:** C9 Policy Decision Point
4. **Stage 4 — Agent Autonomy:** C10 Agent Runtime + C11 Collaboration
5. **Stage 5 — Protocol Evolution:** C4 Governance Engine + C6 Fast-Path Topics
6. **Stage 6 — Storage & Quality:** C8 Artifact Availability + C12 Economics

Specs to include per stage: see each spec's component mapping in `docs/04-specifications/index.md`.

---

## Risk Register

| Risk | Severity | Mitigation |
|------|----------|------------|
| 14 specs create large planning surface | Low | 6-stage sequencing groups related components |
| [TUNE] parameters need calibration data | Medium | Testnet simulated at start of Phase 5; adversarial scenarios (FR-0190) inform tuning |
| Spec drift during implementation | Low | Specs are frozen at Phase 5 gate; changes require governance proposal |
| PDP rule chain untested under load | Medium | Conformance test hooks defined per spec; adversarial simulation covers edge cases |
| Sandbox escape threat model incomplete | Medium | Full threat model deferred to Layer 6 (Validation) per Phase 01 gap tracking |

---

## How to resume

1. Read `docs/04-specifications/index.md` for full spec inventory and traceability.
2. Read target spec for the stage being planned (see Next Stage Inputs above).
3. Cross-reference with `docs/03-architecture/index.md` and `docs/02-requirements/index.md`.
4. Begin Layer 5 (Planning) by creating stage definitions per BUILD-SYSTEM.md.
