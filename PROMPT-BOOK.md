# Prompt Book: Build Hyperfluid

Step-by-step prompts to build the system. Use this in a fresh chat.

## Overview

This document contains executable prompts for building Hyperfluid. It works alongside `BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md` which defines the 8-layer documentation structure:

- Layer 1: Research (exploratory documents)
- Layer 2: Requirements (testable FR-XXXX/NFR-XXXX)
- Layer 3: Architecture (components, interfaces, data model)
- Layer 4: Specifications (normative technical specs)
- Layer 5: Planning (build stages with week-by-week breakdown)
- Layer 6: Validation (test strategy, conformance)
- Layer 7: Operations (runbooks, SLOs)
- Layer 8: Handoff (status files, checkpoints)

Each phase in this Prompt Book maps to creating one or more layers. Read BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md for detailed structure definitions before executing prompts.

---

## Phase 0: Research Audit

Use this prompt when starting fresh or before creating requirements.

### Prompt:

Audit all research documents. Read BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md "Layer 1: Research" for format context, then read every file in `research/` recursively and check for:

1. Contradictions between documents (especially around validator states, action plans, trust stages, slashing conditions)
2. Terminology inconsistencies (is it inactive_bonded or inactive-bonded? action_plan or actionPlan?)
3. Missing cross-references (documents claiming to cite others but links broken)
4. Unresolved TODOs or obvious gaps

Create `research-audit-report.md` with:
- Summary of issues found (categorized by severity: blocking, warning, minor)
- Specific file locations and line references for each issue
- Recommended fixes
- GO/NO-GO recommendation for proceeding to Phase 1

If blocking issues exist, list them clearly. If minor only, note them but recommend proceeding.

---

## Phase 1: Extract Requirements

Use this prompt to create all requirements from research.

### Prompt:

Create Layer 2 (Requirements). Read BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md "Layer 2: Requirements" for format, then read all files in `research/` and extract requirements:
1. Identify every "shall", "must", "should", or implied requirement
2. Convert to numbered functional requirements (FR-XXXX) or non-functional (NFR-XXXX)
3. Tag with source research file and section
4. Define 1-3 acceptance criteria per requirement (measurable/testable)

Create directory structure:
- docs/02-requirements/index.md (master list with links to all requirements)
- docs/02-requirements/protocol/FR-XXXX-*.md (consensus, staking, networking, policy)
- docs/02-requirements/runtime/FR-XXXX-*.md (agent runtime, review engine)
- docs/02-requirements/security/FR-XXXX-*.md (injection protection, key management)
- docs/02-requirements/economics/FR-XXXX-*.md (tokenomics, incentives)

Each requirement file should follow this structure:

FR-XXXX: [Short descriptive title]
Category: [Consensus | Runtime | Networking | Security | Economics]
Statement: The system shall [specific testable behavior]
Rationale: [Why this exists, with link to source research]
Source Research: [file.md#section]
Acceptance Criteria:
  - [Criterion 1 - measurable]
  - [Criterion 2 - measurable]
Dependencies: [FR-YYYY or "none"]
Tags: [must-have | should-have | nice-to-have]

Create 40-60 total requirements covering all domains. Group related requirements into single files where logical (e.g., all validator lifecycle requirements in one file).

When complete, update docs/08-handoff/latest/phase-01-status.md with count of requirements created and any gaps identified.

---

## Phase 2: Architecture Definition

Use this prompt to create architecture from requirements.

### Prompt:

Create Layer 3 (Architecture). Read BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md "Layer 3: Architecture" for structure, then read all files in docs/02-requirements/ and create architecture documents:

Create these files:

1. docs/03-architecture/index.md
   - Overview of architecture approach
   - List of all components with one-line descriptions
   - Navigation to detailed docs

2. docs/03-architecture/component-model/components.md
   - Define each major component (Consensus, Staking, Agent Runtime, Policy Engine, P2P Networking, Storage)
   - For each: responsibilities, owned state, interfaces (inputs/outputs), dependencies on other components
   - Include Mermaid diagram showing component interactions

3. docs/03-architecture/component-model/interfaces.md
   - Define contracts between components
   - Message formats, error handling, versioning strategy

4. docs/03-architecture/data-model/state-model.md
   - Core state entities (Validator, ActionPlan, Identity, Topic, Block, etc.)
   - Fields and types for each entity
   - Relationships between entities
   - Include entity relationship diagram

5. docs/03-architecture/trust-boundaries.md
   - Security zones and boundaries
   - What's in-protocol vs local-only
   - Sandboxed vs unsandboxed execution
   - Network-mutating vs read-only operations

6. docs/03-architecture/failure-model.md
   - System-level failure scenarios
   - How components handle failures
   - Cascading failure prevention

Map every requirement to a component. If a requirement doesn't fit, flag it for requirement revision.

When complete, update docs/08-handoff/latest/phase-02-status.md with component list and any architecture decisions that need documentation.

---

## Phase 3: Write Specifications

Use this prompt to create detailed technical specifications.

### Prompt:

Create Layer 4 (Specifications). Read BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md "Layer 4: Specifications" for format, then read docs/03-architecture/ and docs/02-requirements/ and write all specs:

Create these specification files:

1. docs/04-specifications/protocol/consensus-spec.md
   From: research/consensus-governance/agx-committee-bft-and-governance.md + consensus requirements
   Include: validator lifecycle, committee selection, block finalization, epoch transitions, BFT rules
   Define all parameters (timeouts, thresholds, percentages) with specific values

2. docs/04-specifications/protocol/staking-spec.md
   From: staking requirements + governance research
   Include: bonding/unbonding, slashing conditions, reward distribution, stake delegation

3. docs/04-specifications/protocol/governance-spec.md
   From: governance requirements
   Include: proposal flow, voting rules, git:head transitions, deterministic merge policy

4. docs/04-specifications/protocol/p2p-wire-spec.md
   From: networking research + networking requirements
   Include: message serialization, handshake protocol, message types, encryption

5. docs/04-specifications/protocol/fastpath-spec.md
   From: research/agents/topic-fastpath-protocol-spec.md
   Include: topic coordination, quorum rules, challenge windows, rollback

6. docs/04-specifications/runtime/agent-runtime-spec.md
   From: research/agents/infinite-agent.md + runtime requirements
   Include: infinite loop, handoff mechanism, memory tools, token budget management

7. docs/04-specifications/runtime/policy-engine-spec.md
   From: research/agents/network-policy-engine-spec.md + policy requirements
   Include: action plan validation, quota enforcement, ACLs, replay protection

8. docs/04-specifications/runtime/review-engine-spec.md
   From: research/agents/proof-of-work-quality-and-review-markets.md
   Include: quality scoring, reviewer selection, challenge mechanism

9. docs/04-specifications/storage/artifact-availability-spec.md
   From: research/networking/artifact-availability-and-retention.md
   Include: content addressing, replication, retention guarantees, retrieval under churn

10. docs/04-specifications/storage/state-sync-spec.md
    Include: state synchronization between nodes, catch-up protocol

11. docs/04-specifications/security/key-management-spec.md
    Include: key derivation, signing, rotation, HD wallet support

12. docs/04-specifications/security/incident-response-spec.md
    Include: emergency procedures, circuit breakers, recovery flows

Each specification must have:
- Purpose section
- Normative behavior (MUST/SHOULD/MAY statements)
- Data structures with types
- State transition diagrams where applicable
- Failure behavior definitions
- Versioning and compatibility rules
- Conformance test hooks (how to verify implementation)

Use exact numbers, not placeholders. If a parameter is uncertain, mark it [TUNE] but provide a reasonable default.

When complete, update docs/08-handoff/latest/phase-03-status.md with spec inventory and any [TUNE] parameters identified.

---

## Phase 4: Create Build Stages

Use this prompt to create the implementation roadmap.

### Prompt:

Create Layer 5 (Planning). Read BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md "Layer 5: Planning" for stage format, then read docs/04-specifications/ and create week-by-week build stages:

Create these files:

1. docs/05-planning/index.md
   - Overview of build approach
   - Stage summary table
   - Current status tracker

2. docs/05-planning/stages/stage-00-foundation.md
   Pre-coding stage. 1-2 weeks.
   - Week 1: Requirements review and finalization
   - Week 2: Architecture review and spec prioritization
   - Exit criteria: All blockers resolved, ready to code

3. docs/05-planning/stages/stage-01-protocol-core.md
   First coding stage. 6-8 weeks.
   Break down by week with:
   - Which spec sections to implement
   - Specific deliverables
   - Tests to write
   - Checkpoint at end of each week
   
   Cover: wire protocol, state model, consensus core, staking, governance

4. docs/05-planning/stages/stage-02-agent-runtime.md
   Second coding stage. 6-8 weeks.
   Week-by-week breakdown for:
   - Runtime core (infinite loop, handoff)
   - Policy engine (validation, quotas)
   - Review engine (scoring, selection)
   - Integration with protocol

5. docs/05-planning/stages/stage-03-validation.md
   4-6 weeks.
   - Conformance testing
   - Adversarial scenarios
   - Load testing
   - Security audits

6. docs/05-planning/stages/stage-04-mainnet-prep.md
   4-6 weeks.
   - Operations runbooks
   - Monitoring setup
   - Incident response procedures
   - Mainnet deployment plan

Each stage file must include:
- Duration estimate
- Week-by-week task breakdown
- Inputs (what's needed from previous stage)
- Outputs (what this stage produces)
- Exit criteria (when is this stage done)
- Dependencies (what must complete first)
- Risk areas and mitigations

Be realistic about timelines. If uncertain, estimate on the longer side.

When complete, update docs/08-handoff/latest/phase-04-status.md with stage summaries and total estimated timeline.

---

## Phase 5: Execute Build

Use these prompts when actually implementing.

### Prompt Template for Implementation:

Execute current build task. Read BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md "Layer 5: Planning" and "Layer 8: Handoff" for checkpoint format, then read docs/05-planning/stages/ to find current week and implement:

Read:
1. docs/05-planning/stages/stage-XX-[current-stage].md (find the current week)
2. The specification referenced for that week
3. docs/08-handoff/latest/build-status.md (to see what's already done)

Implement the specified functionality in the appropriate crate/directory.

Follow the specification exactly. If the spec is unclear or contradictory:
- Make a reasonable decision
- Document the deviation in a code comment
- Flag it in docs/08-handoff/latest/open-questions.md

Write tests alongside implementation. Aim for test coverage of the specified behavior.

When the week's tasks are complete:
1. Update docs/05-planning/stages/stage-XX-[current-stage].md (mark week complete)
2. Update docs/08-handoff/latest/build-status.md (add completed work)
3. Create brief checkpoint: docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md (what works, what's next)

Then stop and wait for next prompt.

---

## Quick Start Guide

In a fresh chat, determine current state:

1. Does research-audit-report.md exist?
   NO -> Run Phase 0
   YES -> Continue

2. Does docs/02-requirements/ have FR-*.md files?
   NO -> Run Phase 1
   YES -> Continue

3. Does docs/03-architecture/ have component docs?
   NO -> Run Phase 2
   YES -> Continue

4. Does docs/04-specifications/ have spec files?
   NO -> Run Phase 3
   YES -> Continue

5. Does docs/05-planning/stages/ have stage files?
   NO -> Run Phase 4
   YES -> Run Phase 5 (Implementation)

---

## Status Tracking

Keep these files updated in docs/08-handoff/latest/:

- phase-0X-status.md - What's complete in each phase
- build-status.md - What's currently implemented (code)
- next-tasks.md - What to do next
- open-questions.md - Uncertainties or blockers
- checkpoint-YYYY-MM-DD.md - Snapshot after each major chunk

Update these after every significant work session.

---

## Current State (Update After Each Session)

Last Updated: [DATE]

Phase 0 Research Audit: [NOT STARTED | IN PROGRESS | COMPLETE]
Phase 1 Requirements: [NOT STARTED | IN PROGRESS | COMPLETE]
Phase 2 Architecture: [NOT STARTED | IN PROGRESS | COMPLETE]
Phase 3 Specifications: [NOT STARTED | IN PROGRESS | COMPLETE]
Phase 4 Planning: [NOT STARTED | IN PROGRESS | COMPLETE]
Phase 5+ Building: [NOT STARTED | IN PROGRESS | COMPLETE]

Current Stage: [If in Phase 5+, which stage and week]
Blockers: [Any blockers]
Notes: [Any notes for next agent]
