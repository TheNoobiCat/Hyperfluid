# Layer 4: Specifications

**Status:** COMPLETE
**Last updated:** 2026-05-02
**Total specs:** 14 (6 protocol, 2 storage, 4 runtime, 2 security)
**Phase:** 3 (Specifications)

---

## How to use this index

This directory contains the canonical normative specifications for Hyperfluid. Each spec follows the format defined in `TEMPLATES.md` (8 sections per topic: Purpose, Normative Behavior, Data Structures, State Transitions, Failure Behavior, Versioning, Conformance Test Hooks, Trust-Assumption Inventory). Every spec is traceable to Layer 2 requirements and Layer 3 architecture components.

---

## Protocol Specs

Located in `docs/04-specifications/protocol/`

| Spec | Component | FRs | Lines |
|------|-----------|-----|-------|
| [`consensus-spec.md`](protocol/consensus-spec.md) | C1 Consensus Engine, C2 State Machine | FR-0001-0010 | Consensus BFT, committee rotation, VDF randomness, SMT state, transaction types, nonces, finality |
| [`staking-spec.md`](protocol/staking-spec.md) | C3 Staking & Validator Manager | FR-0011-0020 | Four-state validator lifecycle, bonding/unbonding, slashing, downtime, evidence pipeline |
| [`governance-spec.md`](protocol/governance-spec.md) | C4 Governance Engine | FR-0021-0030 | git:head management, proposal lifecycle, vote window, sandbox review, anti-flood controls |
| [`p2p-wire-spec.md`](protocol/p2p-wire-spec.md) | C7 P2P Networking | FR-0041-0050 | Peer discovery, connection state machine, gossip, relay, fee-ordered mempool |
| [`fastpath-spec.md`](protocol/fastpath-spec.md) | C6 Fast-Path Topic Protocol | FR-0031-0040 | Topic-scoped merges, quorum certificates, challenge windows, rollback, promotion bridge |
| [`fee-market-spec.md`](protocol/fee-market-spec.md) | C5 Fee Market | FR-0146, FR-0147, FR-0159, FR-0160 | EIP-1559 base fee, validator rebates, manipulation defense, front-running protection |

## Storage Specs

Located in `docs/04-specifications/storage/`

| Spec | Component | FRs | Lines |
|------|-----------|-----|-------|
| [`state-sync-spec.md`](storage/state-sync-spec.md) | C2 State Machine | FR-0010, NFR-0009, NFR-0018, NFR-0019 | Snap sync, full sync, crash recovery, deterministic replay |
| [`artifact-availability-spec.md`](storage/artifact-availability-spec.md) | C8 Artifact Availability | FR-0051-0060 | Content-addressed storage, proof-of-possession, retention tiers, repair coordinator, SLA |

## Runtime Specs

Located in `docs/04-specifications/runtime/`

| Spec | Component | FRs | Lines |
|------|-----------|-----|-------|
| [`agent-runtime-spec.md`](runtime/agent-runtime-spec.md) | C10 Agent Runtime | FR-0061-0075, FR-0136-0138 | Infinite loop, core tools, system prompt, handoff, resource limits, process isolation, sandbox security |
| [`policy-engine-spec.md`](runtime/policy-engine-spec.md) | C9 Policy Decision Point | FR-0106-0120 | 10-step deterministic rule chain, action plans, quota matrix, key rotation, audit log |
| [`review-engine-spec.md`](runtime/review-engine-spec.md) | C12 Economics (Review Markets) | FR-0161-0175 | Two-phase quality pipeline, reviewer assignment, anti-collusion, settlement |
| [`collaboration-spec.md`](runtime/collaboration-spec.md) | C11 Collaboration & Inbox | FR-0076-0105, FR-0176-0190 | Task board, soft leases, team formation, inbox routing, trust ladder, reputation, airdrop economics |

## Security Specs

Located in `docs/04-specifications/security/`

| Spec | Component | FRs | Lines |
|------|-----------|-----|-------|
| [`telemetry-spec.md`](security/telemetry-spec.md) | C1, C2 (Telemetry) | FR-0060, FR-0139-0141, NFR-0020-0021 | Signed envelopes, aggregation, reconciliation, outlier detection |
| [`incident-response-spec.md`](security/incident-response-spec.md) | C12, C4 (Incident Response) | FR-0142-0145 | Incident state machine, emergency mode, recovery ramp-up, congestion control |

---

## Traceability

### FR → Spec Mapping

| FR Range | Spec | Section |
|----------|------|---------|
| FR-0001-0010 | consensus-spec.md | Sections 1-2 |
| FR-0011-0020 | staking-spec.md | Sections 1-2 |
| FR-0021-0030 | governance-spec.md | Sections 1-2 |
| FR-0031-0040 | fastpath-spec.md | Section 1 |
| FR-0041-0050 | p2p-wire-spec.md | Sections 1-2 |
| FR-0051-0060 | artifact-availability-spec.md | Sections 1-2 |
| FR-0061-0075 | agent-runtime-spec.md | Sections 1-4 |
| FR-0076-0105 | collaboration-spec.md | Sections 1-3 |
| FR-0106-0120 | policy-engine-spec.md | Sections 1-3 |
| FR-0121-0135 | docs/01-research/security/prompt-injection-defense-framework.md | — |
| FR-0136-0138 | agent-runtime-spec.md | Section 4 |
| FR-0139-0141 | telemetry-spec.md | Sections 1-2 |
| FR-0142-0145 | incident-response-spec.md | Sections 1-2 |
| FR-0146, FR-0147, FR-0159, FR-0160 | fee-market-spec.md | Sections 1-2 |
| FR-0148, FR-0149, FR-0150, FR-0153 | review-engine-spec.md | Section 1 |
| FR-0151 | staking-spec.md | Section 1 |
| FR-0152 | p2p-wire-spec.md | Section 2 |
| FR-0154 | incident-response-spec.md | Section 1 |
| FR-0155 | governance-spec.md | Section 1 |
| FR-0156, FR-0157, FR-0158 | collaboration-spec.md | Section 3 |
| FR-0161-0175 | review-engine-spec.md | Section 1 |
| FR-0176-0190 | collaboration-spec.md | Section 3 |
| FR-0191 | collaboration-spec.md | Section 3 |
| FR-0192 | collaboration-spec.md | Section 3 |
| FR-0193 | agent-runtime-spec.md | Section 2 |
| FR-0194 | consensus-spec.md, policy-engine-spec.md | Sections 2, 1 |
| FR-0195 | policy-engine-spec.md, collaboration-spec.md | Sections 2, 1 |
| FR-0196 | agent-runtime-spec.md | Section 5 |
| FR-0197 | p2p-wire-spec.md | Section 2 |
| FR-0198 | collaboration-spec.md | Section 1 |
| FR-0199 | agent-runtime-spec.md | Section 3 |
| FR-0200 | agent-runtime-spec.md | Section 5 |
| NFR-0001-0015 | (cross-cutting, referenced per spec) | — |
| NFR-0016-0030 | (cross-cutting, referenced per spec) | — |

### ADR → Spec Mapping

| ADR | Spec |
|-----|------|
| ADR-0001 (12 Components) | All specs (component structure) |
| ADR-0002 (Three-Layer Trust) | agent-runtime-spec.md, policy-engine-spec.md |
| ADR-0003 (PDP Deterministic Chain) | policy-engine-spec.md, governance-spec.md |
| ADR-0004 (Agent Process Separation) | agent-runtime-spec.md Section 4 |
| ADR-0005 (Content-Addressed SMT) | consensus-spec.md Section 2, state-sync-spec.md |
| ADR-0007 (Committee BFT VDF) | consensus-spec.md Section 1, staking-spec.md |
| ADR-0008 (Two-Phase Quality) | review-engine-spec.md Section 1 |
| ADR-0009 (EIP-1559 Fee Market) | fee-market-spec.md Section 1 |
| ADR-0010 (Two-Stage Trust Ladder) | collaboration-spec.md Section 3 |
| ADR-0011 (Review Sandbox Isolation) | governance-spec.md Section 2, fastpath-spec.md |
| ADR-0012 (Congestion Response via EIP-1559) | fee-market-spec.md |

---

## [TUNE] Parameters

Parameters marked `[TUNE]` in specs (reasonable default provided; calibration target for testnet):

| Parameter | Default | Spec Location | Notes |
|-----------|---------|---------------|-------|
| min_base_fee | 1,000,000 atto-AGX | fee-market-spec.md 1.3 | Minimum fee floor |
| promotion thresholds | See collaboration-spec.md 3.3 | collaboration-spec.md 3.3 | Trust ladder promotion requirements |
| fee adjustment denominator | 8 | fee-market-spec.md 1.4 | Base fee adjustment rate |

---

## Gate Status: Architecture → Specifications

| Check | Status |
|-------|--------|
| All component interfaces documented | PASS (10 interfaces per `interfaces.md`) |
| All trust boundaries defined | PASS (per `trust-boundaries.md`) |
| Every spec has trust-assumption inventory (Section X.8) | PASS (14/14 specs) |
| Every spec has conformance test hooks (Section X.7) | PASS (14/14 specs) |
| Every FR mapped to at least one spec | PASS |
| All data structures have deterministic types | PASS |
| Failure behavior documented per spec | PASS |
| Versioning rules documented per spec | PASS |

**Gate result: READY FOR LAYER 5 (Planning).**

---

## Spec Inventory

| # | Spec | Status |
|---|------|--------|
| 1 | protocol/consensus-spec.md | complete |
| 2 | protocol/staking-spec.md | complete |
| 3 | protocol/governance-spec.md | complete |
| 4 | protocol/p2p-wire-spec.md | complete |
| 5 | protocol/fastpath-spec.md | complete |
| 6 | protocol/fee-market-spec.md | complete |
| 7 | storage/state-sync-spec.md | complete |
| 8 | storage/artifact-availability-spec.md | complete |
| 9 | runtime/agent-runtime-spec.md | complete |
| 10 | runtime/policy-engine-spec.md | complete |
| 11 | runtime/review-engine-spec.md | complete |
| 12 | runtime/collaboration-spec.md | complete |
| 13 | security/telemetry-spec.md | complete |
| 14 | security/incident-response-spec.md | complete |
