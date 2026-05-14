# Layer 3: Architecture

**Status:** COMPLETE
**Last updated:** 2026-05-14
**Total components:** 12
**ADRs:** 16

---

## How to use this index

This directory defines Hyperfluid's system decomposition, component boundaries, trust boundaries, invocation-level design, and architecture decisions. Every component maps to one or more Layer 2 requirements. Every significant design decision is recorded as an ADR.

---

## Component Model

Located in `docs/03-architecture/component-model/`

| Document | Description |
|----------|-------------|
| [`components.md`](component-model/components.md) | 12-component decomposition, responsibilities, owned state, interfaces, and dependencies. Includes Mermaid system diagram. |
| [`interfaces.md`](component-model/interfaces.md) | Inter-component contracts, message formats, error handling, versioning. |

### Component List

| ID | Component | Layer | Key FRs |
|----|-----------|-------|---------|
| C1 | Consensus Engine | Protocol Core | FR-0001-0010 |
| C2 | State Machine & SMT | Protocol Core | FR-0007-0008, FR-0010 |
| C3 | Staking & Validator Manager | Protocol Core | FR-0011-0019 |
| C4 | Governance Engine | Protocol Core | FR-0021-0030 |
| C5 | Fee Market | Protocol Core | FR-0146-0147 |
| C6 | Fast-Path Topic Protocol | Protocol Services | FR-0031-0040 |
| C7 | P2P Networking & Connection Manager | Protocol Services | FR-0041-0050 |
| C8 | Artifact Availability & Storage | Protocol Services | FR-0051-0060 |
| C9 | Policy Decision Point (PDP) | Security Boundary | FR-0106-0120 |
| C10 | Agent Runtime | Runtime | FR-0061-0075, FR-0193 |
| C11 | Collaboration & Inbox Layer | Runtime | FR-0076-0105, FR-0153b |
| C12 | Economics & Incentives | Economics | FR-0148-0160, FR-0161-0175, FR-0176-0193 |

---

## Data Model

Located in `docs/03-architecture/data-model/`

| Document | Description |
|----------|-------------|
| [`state-model.md`](data-model/state-model.md) | Core entities, fields, types, relationships. Includes entity relationship diagram. |

---

## Trust Boundaries

Located at [`trust-boundaries.md`](trust-boundaries.md).

Covers:
- Security zones and trust domains
- In-protocol vs local-only state
- Sandboxed vs unsandboxed execution
- Component trust assumptions

---

## Failure Model

Located at [`failure-model.md`](failure-model.md).

Covers:
- System-level failure scenarios
- Cascading failure prevention
- Graceful degradation strategies
- Recovery procedures

---

## Architecture Decision Records

Located in `docs/03-architecture/decisions/`

| ADR | Title | Status |
|-----|-------|--------|
| ADR-0001 | 12-Component Architecture Decomposition | Accepted |
| ADR-0002 | Three-Zone Security Architecture | Accepted |
| ADR-0003 | Policy Decision Point as Deterministic Rule Chain | Accepted |
| ADR-0004 | Agent Runtime Process Separation from Node | Accepted |
| ADR-0005 | Content-Addressed State with SMT | Accepted |
| ADR-0007 | Committee BFT with VDF Randomness | Accepted |
| ADR-0008 | Two-Phase Quality Pipeline | Accepted |
| ADR-0009 | EIP-1559 Fee Market | Accepted |
| ADR-0010 | Two-Stage Trust Ladder | Accepted |
| ADR-0011 | Review Sandbox Isolation | Accepted |
| ADR-0012 | Congestion Response via EIP-1559 Base Fee | Accepted |
| ADR-0013 | Expanded Agent Tool Set, CLI Seed Index Discovery, and Seed-Centric Task Model | Accepted |
| ADR-0014 | User Task Submission and Agent Sponsorship | Accepted |
| ADR-0015 | Stake Delegation | Accepted |
| ADR-0016 | clatter + ml-dsa Secure Channel Stack (replaces Ockam) | Accepted |

---

## Requirement-to-Component Mapping

Every FR/NFR maps to at least one component. For the full traceability matrix, see `docs/08-handoff/latest/traceability-matrix.md`.

### FR Mapping Summary

| FR Range | Domain | Primary Component |
|----------|--------|-------------------|
| FR-0001-0010 | Consensus & BFT | C1 Consensus Engine, C2 State Machine |
| FR-0011-0020 | Staking & Validators | C3 Staking & Validator Manager |
| FR-0021-0030 | Governance | C4 Governance Engine |
| FR-0031-0040 | Fast-Path Topics | C6 Fast-Path Topic Protocol |
| FR-0041-0050 | P2P Networking | C7 P2P Networking |
| FR-0051-0060 | Artifact Availability | C8 Artifact Availability |
| FR-0061-0075 | Agent Runtime | C10 Agent Runtime |
| FR-0076-0105 | Collaboration & Inbox | C11 Collaboration & Inbox Layer |
| FR-0106-0120 | Policy Engine | C9 Policy Decision Point |
| FR-0121-0135 | Injection Defense | C9 PDP, C10 Agent Runtime |
| FR-0136-0145 | Sandbox & Telemetry | C9 PDP, C10 Agent Runtime |
| FR-0146-0160 | AGX Economics | C5 Fee Market, C12 Economics |
| FR-0161-0175 | Review Markets | C12 Economics |
| FR-0176-0193 | Incentives, Airdrop, Sybil Detection, Operator Interface | C12 Economics, C10 Agent Runtime |
| FR-0194-0200 | Task Submission, Sponsorship, Discovery, CLI, Telegram | C9 PDP, C10 Agent Runtime, C11 Collaboration, C7 P2P |
| NFR-0001-0015 | Performance | Cross-cutting |
| NFR-0016-0030 | Security/Reliability | Cross-cutting |

### Orphan Check

All 202 requirements map to at least one component. Zero orphans.

---

## Gate Status: Requirements → Architecture

| Check | Status |
|-------|--------|
| Every requirement maps to a component | PASS (202/202) |
| No orphan requirements | PASS |
| Component boundaries documented | PASS |
| Trust boundaries documented | PASS |
| Interfaces deterministic | PASS |
| ADRs recorded for all significant decisions | PASS (16 ADRs) |

**Gate result: READY FOR LAYER 4 (Specifications).**
