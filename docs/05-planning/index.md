# Layer 5: Planning

**Status:** COMPLETE
**Last updated:** 2026-05-05
**Total stages:** 5 (00–04)
**Estimated total duration:** 28–30 weeks

---

## How to use this index

This directory defines the implementation sequence, stage definitions, checkpoints, and delivery orchestration for Hyperfluid. Each stage follows the format defined in `TEMPLATES.md` (Inputs, Outputs, Exit Criteria, Duration Estimate, Dependencies, Risk Areas). Every stage is traceable to Layer 4 specifications and Layer 3 architecture components.

**Upstream traceability:** Layer 4 (14 specs across 4 domains) → Layer 5 (planning stages).
**Downstream:** Layer 6 (Validation) consumes stage definitions for test strategy and conformance evidence.

---

## Sequencing Rationale

The stage ordering follows a bottom-up dependency graph: infrastructure before applications, protocol before agents, core before periphery, testing before launch.

```
stage-00: Foundation             ← Build tooling, repo, CI
    ↓
stage-01: Protocol Core          ← Chain (C1, C2), staking (C3), P2P (C7), fees (C5), storage (C8)
    ↓
stage-02: Agent Runtime          ← PDP (C9), agents (C10), collaboration (C11), review (C12), governance (C4), fast-path (C6)
    ↓
stage-03: Validation             ← Conformance, adversarial, load, security hardening
    ↓
stage-04: Mainnet Prep           ← Operations, monitoring, incident response, launch
```

Protocol Core must precede Agent Runtime because agents submit action plans to the PDP, whose decisions land on the chain via the Consensus Engine. Validation must follow both, since end-to-end correctness depends on the full stack. Mainnet Prep is the final hardening stage.

---

## Stage Summary

| Stage | Name | Duration | Components | Specs | Key Deliverable |
|-------|------|----------|------------|-------|-----------------|
| 00 | Foundation | 1–2 weeks | — | — | Repo, tooling, CI, local testnet scaffold |
| 01 | Protocol Core | 6–8 weeks | C1, C2, C3, C5, C7, C8 | 6 (protocol + storage) | Single-node chain with staking, fees, P2P, artifact storage |
| 02 | Agent Runtime | 6–8 weeks | C4, C6, C9, C10, C11, C12 | 8 (runtime + remaining protocol) | Agent-driven task board with PDP gating, review pipeline, governance |
| 03 | Validation | 4–6 weeks | All | All 14 | Conformance matrix, adversarial test suite, load results, security audit |
| 04 | Mainnet Prep | 4–6 weeks | All | All 14 | Runbooks, SLOs, monitoring, incident playbooks, launch checklist |

---

## Status Tracker

| Stage | Status | Started | Completed | Gate Result |
|-------|--------|---------|-----------|-------------|
| Stage 00: Foundation | COMPLETE | 2026-05-02 | 2026-05-04 | Gate result: PASS |
| Stage 01: Protocol Core | NOT STARTED | — | — | — |
| Stage 02: Agent Runtime | NOT STARTED | — | — | — |
| Stage 03: Validation | NOT STARTED | — | — | — |
| Stage 04: Mainnet Prep | NOT STARTED | — | — | — |

---

## Spec-to-Stage Mapping

| Stage | Specs Included |
|-------|---------------|
| 01 | consensus-spec.md, staking-spec.md, p2p-wire-spec.md, fee-market-spec.md, state-sync-spec.md, artifact-availability-spec.md |
| 02 | governance-spec.md, fastpath-spec.md, agent-runtime-spec.md, policy-engine-spec.md, review-engine-spec.md, collaboration-spec.md, telemetry-spec.md, incident-response-spec.md |
| 03 | All 14 specs (conformance) |
| 04 | All 14 specs (operations) |

---

## Component-to-Stage Mapping

| Component | Stage 01 | Stage 02 | Stage 03 | Stage 04 |
|-----------|----------|----------|----------|----------|
| C1 Consensus Engine | Build | — | Test | Operate |
| C2 State Machine & SMT | Build | — | Test | Operate |
| C3 Staking & Validator Manager | Build | — | Test | Operate |
| C4 Governance Engine | — | Build | Test | Operate |
| C5 Fee Market | Build | — | Test | Operate |
| C6 Fast-Path Topic Protocol | — | Build | Test | Operate |
| C7 P2P Networking | Build | — | Test | Operate |
| C8 Artifact Availability | Build | — | Test | Operate |
| C9 Policy Decision Point | — | Build | Test | Operate |
| C10 Agent Runtime | — | Build | Test | Operate |
| C11 Collaboration & Inbox | — | Build | Test | Operate |
| C12 Economics & Incentives | — | Build | Test | Operate |

---

## Gate Status: Specifications → Planning

| Check | Status |
|-------|--------|
| Every spec has conformance test hooks (Section X.7) | PASS (14/14, verified in Phase 3) |
| Trust-assumption inventory complete | PASS (14/14, verified in Phase 3) |
| All FRs mapped to spec sections | PASS (verified in Phase 3) |
| Decentralisation audit complete (no issues) | PASS (verified in Phase 3) |
| Stage definitions reference all 14 specs | PASS |
| Stage definitions account for all 12 components | PASS |

**Gate result: READY FOR LAYER 6 (Validation).**

---

## Risk Register (Layer 5)

| Risk | Severity | Stage Affected | Mitigation |
|------|----------|----------------|------------|
| Malachite BFT integration complexity | Medium | 01 | Stage 01 reserves 2 weeks for integration. Fallback: embed Malachite as vendored dependency with adaptation layer. |
| [TUNE] parameters calibrated without testnet data | Medium | 01–02 | Stage 01 ships with defaults from spec; Stage 03 produces calibration data. Tuning happens in Stage 03. |
| Agent runtime LLM provider availability | Low | 02 | Runtime abstracted behind provider interface; support multiple providers (Anthropic, OpenAI, local via Ollama). |
| Full integration surface too large for small team | Medium | 01–02 | Stage definitions use 6–8 week windows with explicit scope borders. Each stage is independently testable. |
| P2P connectivity issues in heterogeneous networks | Medium | 01, 03 | Ockam transport abstracts transport; Stage 03 includes NAT traversal and relay stress testing. |
| Governance upgrade mechanism untested before mainnet | Low | 02, 04 | Stage 02 builds governance engine; Stage 04 validates upgrade via private testnet fork. |

---

## Next Actions

1. Begin Stage 00 (Foundation): create repo structure, Cargo workspace, CI, local testnet scaffold.
2. Stage 01 (Protocol Core): build Minimum Viable Chain — single-node first, then multi-node gossip.
3. Stage 02 (Agent Runtime): layer agent behavior, PDP, collaboration, and governance on top of the chain.
4. Stage 03 (Validation): full conformance, adversarial load, and security testing.
5. After Stage 04 (Mainnet Prep), proceed to Layer 6 (Validation — the formal test strategy layer, distinct from Stage 03 which is implementation-stage validation).

---

## Traceability

| Layer | Artifact | Link |
|-------|----------|------|
| L4 Specs | 14 specs across 4 domains | `docs/04-specifications/index.md` |
| L3 Architecture | 12 components, 14 ADRs | `docs/03-architecture/index.md` |
| L2 Requirements | 202 (172 FR + 30 NFR) | `docs/02-requirements/index.md` |
| L8 Handoff | Phase 03 handoff | `docs/08-handoff/latest/phase-03-status.md` |
