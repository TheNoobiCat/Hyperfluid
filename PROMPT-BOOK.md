# Prompt Book: Build Hyperfluid

Step-by-step prompts to build the system. Use this in a fresh chat.

note: these prompts are a good baseline but i pretty much always need to add additional specific details to them etc

---

## Where to read first

Before executing any phase:
1. Read `BUILD-SYSTEM.md` for layer definitions, gates, and traceability rules.
2. Read `TEMPLATES.md` for all artifact formats (research, FR-XXXX, spec, stage, checkpoint).
3. Read `GLOSSARY.md` for canonical terminology.
4. Read `PROJECT-STATUS.md` for current project state, phase status, blockers, and spec inventory. **Always update `PROJECT-STATUS.md` after completing any phase.**
5. **Read the latest handoff** (`docs/08-handoff/latest/checkpoint-*.md` and `docs/08-handoff/latest/build-status.md` if they exist). If the previous agent left unfinished work, complete it before starting the current phase. Handoff files are the canonical state of what has been done and what is next.

This document contains only what is *unique to each phase*. Do not redefine formats that already live in `TEMPLATES.md`.

---

## Phase 0: Research Audit

Use this prompt when starting fresh or before promoting research to requirements.

### Prompt:

Audit all research documents. Read `BUILD-SYSTEM.md` (Decentralisation Audit Gate) and `GLOSSARY.md` for context, then read the latest handoff (`docs/08-handoff/latest/`) if it exists, then read every file in `docs/01-research/` recursively and check for:

1. Contradictions between documents (validator states, action plans, trust stages, slashing conditions)
2. Terminology inconsistencies (check against `GLOSSARY.md`)
3. Missing or broken cross-references
4. Unresolved TODOs or obvious gaps
5. **Decentralisation audit:** Run the checklist defined in `BUILD-SYSTEM.md`. Do not re-state the checklist here — execute it. If issues are found, create `docs/01-research/_overengineered.md` with source file, issue summary, explanation, and proposed fix. Do not create this file if no issues are found.

Create `research-audit-report.md` with:
- Summary of issues found (severity: blocking, warning, minor)
- Specific file locations and line references
- Recommended fixes
- Decentralisation audit findings (reference `_overengineered.md` if created)
- GO/NO-GO recommendation for proceeding to Phase 1

When complete, update `PROJECT-STATUS.md`: set Phase 0 status, record any blockers found, update "Next Actions" and "Last updated" date.

---

## Phase 1: Extract Requirements

Use this prompt to create all requirements from research.

### Prompt:

Create Layer 2 (Requirements). Read `BUILD-SYSTEM.md` (Layer 2), `TEMPLATES.md` (FR-XXXX format), and the latest handoff (`docs/08-handoff/latest/`) if it exists, then read all files in `docs/01-research/` and extract requirements:

1. Identify every "shall", "must", "should", or implied requirement.
2. Convert to numbered FR-XXXX or NFR-XXXX. Follow the format in `TEMPLATES.md` exactly.
3. Tag with source research file and section.
4. Define 1-3 acceptance criteria per requirement (measurable/testable).
5. **Decentralisation review:** Scan every requirement for language that introduces centralised coordination, external trust, or unverifiable economic enforcement. Flag as `[DECENTRALISATION-RISK]` and require a trust-minimised rewrite before spec creation.

Create directory structure:
- `docs/02-requirements/index.md` (master list with links)
- `docs/02-requirements/protocol/FR-XXXX-*.md`
- `docs/02-requirements/runtime/FR-XXXX-*.md`
- `docs/02-requirements/security/FR-XXXX-*.md`
- `docs/02-requirements/economics/FR-XXXX-*.md`

Create 60-200 total requirements covering all domains. Group related requirements into single files where logical.

When complete, update `docs/08-handoff/latest/phase-01-status.md` with count and gaps, and update `PROJECT-STATUS.md`: set Phase 1 status, record any new gaps, update "Next Actions" and "Last updated" date.

---

## Phase 2: Architecture Definition

Use this prompt to create architecture from requirements.

### Prompt:

Create Layer 3 (Architecture). Read `BUILD-SYSTEM.md` (Layer 3), `TEMPLATES.md` (ADR format), and the latest handoff (`docs/08-handoff/latest/`) if it exists, then read all files in `docs/02-requirements/` and create architecture documents:

Create these files:

1. `docs/03-architecture/index.md` — Overview, component list, navigation.
2. `docs/03-architecture/component-model/components.md` — Component definitions, responsibilities, owned state, interfaces, dependencies. Include Mermaid diagram.
3. `docs/03-architecture/component-model/interfaces.md` — Inter-component contracts, message formats, error handling, versioning.
4. `docs/03-architecture/data-model/state-model.md` — Core entities, fields, types, relationships. Include entity relationship diagram.
5. `docs/03-architecture/trust-boundaries.md` — Security zones, in-protocol vs local-only, sandboxed vs unsandboxed.
6. `docs/03-architecture/failure-model.md` — System-level failure scenarios and cascading failure prevention.

Map every requirement to a component. If a requirement doesn't fit, flag it for requirement revision.

Record every significant decision as an ADR in `docs/03-architecture/decisions/ADR-XXXX-*.md`.

When complete, update `docs/08-handoff/latest/phase-02-status.md`, and update `PROJECT-STATUS.md`: set Phase 2 status, record component/ADR counts, update "Next Actions" and "Last updated" date.

---

## Phase 3: Write Specifications

Use this prompt to create detailed technical specifications.

### Prompt:

Create Layer 4 (Specifications). Read `BUILD-SYSTEM.md` (Layer 4), `TEMPLATES.md` (Specification Section format), `GLOSSARY.md`, and the latest handoff (`docs/08-handoff/latest/`) if it exists, then read `docs/03-architecture/` and `docs/02-requirements/` and write all specs.

Target specs (read `PROJECT-STATUS.md` section "Layer 4 Spec Inventory" for the canonical list):
- `protocol/consensus-spec.md`
- `protocol/staking-spec.md`
- `protocol/governance-spec.md`
- `protocol/p2p-wire-spec.md`
- `protocol/fastpath-spec.md`
- `protocol/fee-market-spec.md`
- `storage/state-sync-spec.md`
- `storage/artifact-availability-spec.md`
- `runtime/agent-runtime-spec.md`
- `runtime/policy-engine-spec.md`
- `runtime/review-engine-spec.md`
- `runtime/collaboration-spec.md`
- `security/telemetry-spec.md`
- `security/incident-response-spec.md`

Every spec **must** include a trust-assumption inventory (section X.8 per `TEMPLATES.md`). Use exact numbers, not placeholders. Uncertain parameters: mark `[TUNE]` with a reasonable default.

When complete, update `docs/08-handoff/latest/phase-03-status.md` with spec inventory and any `[TUNE]` parameters, and update `PROJECT-STATUS.md`: set Phase 3 status, mark specs as complete in the inventory table, update "Next Actions" and "Last updated" date.

---

## Phase 4: Create Build Stages

Use this prompt to create the implementation roadmap.

### Prompt:

Create Layer 5 (Planning). Read `BUILD-SYSTEM.md` (Layer 5), `TEMPLATES.md` (Stage format), and the latest handoff (`docs/08-handoff/latest/`) if it exists, then read `docs/04-specifications/` and create week-by-week build stages:

Create these files:

1. `docs/05-planning/index.md` — Overview, stage summary table, status tracker.
2. `docs/05-planning/stages/stage-00-foundation.md` — Pre-coding, 1-2 weeks.
3. `docs/05-planning/stages/stage-01-protocol-core.md` — 6-8 weeks. Consensus, staking, wire, storage.
4. `docs/05-planning/stages/stage-02-agent-runtime.md` — 6-8 weeks. Runtime, policy, review.
5. `docs/05-planning/stages/stage-03-validation.md` — 4-6 weeks. Conformance, adversarial, load, security.
6. `docs/05-planning/stages/stage-04-mainnet-prep.md` — 4-6 weeks. Operations, monitoring, incident response.

Be realistic about timelines. If uncertain, estimate on the longer side.

When complete, update `docs/08-handoff/latest/phase-04-status.md`, and update `PROJECT-STATUS.md`: set Phase 4 status, update "Next Actions" and "Last updated" date.

---

## Phase 5: Execute Build

Use the prompt in `.opencode/commands/execute-build.md`. It is the single source of truth for the build execution workflow. This section exists only to reference it — do not redefine the prompt here.

---

## Quick Start Guide

In a fresh chat, determine current state:

1. Does `research-audit-report.md` exist?
   NO -> Run Phase 0
   YES -> Continue

2. Has the decentralisation audit passed (`_overengineered.md` reviewed and fixed if it exists)?
   NO -> Run Phase 0 decentralisation audit
   YES -> Continue

3. Does `docs/02-requirements/` have FR-*.md files?
   NO -> Run Phase 1
   YES -> Continue

4. Does `docs/03-architecture/` have component docs?
   NO -> Run Phase 2
   YES -> Continue

5. Does `docs/04-specifications/` have spec files?
   NO -> Run Phase 3
   YES -> Continue

6. Does `docs/05-planning/stages/` have stage files?
   NO -> Run Phase 4
   YES -> Run Phase 5 (Implementation via `.opencode/commands/execute-build.md`)

---

## Utility A: Fix-and-Update

Use this when encountering a build error, runtime crash, test failure, or when a new feature/capability is needed mid-build. Paste the error message, stack trace, test output, or feature description as your message, then run this prompt. It will fix the code AND synchronise all related documentation in one cycle.

### Prompt:

Use the prompt in `.opencode/commands/execute-build.md`. It is the single source of truth for the build execution workflow. This section exists only to reference it — do not redefine the prompt here.


---

## Utility B: Code Audit (Silent Bugs)

Use the prompt in `.opencode/commands/audit.md`. It is the single source of truth for the code audit workflow. This section exists only to reference it — do not redefine the audit instructions here.
