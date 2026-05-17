# Hyperfluid Build System

Defines the 8-layer documentation pipeline and the gates between layers.

## When to use this document

- Starting a new implementation stage
- Creating new specification documents from research
- Handing off work between agents
- Determining where a new document belongs

---

## Layer Structure

Research flows through 8 layers, from exploration to operations. Each layer has a specific purpose and produces specific artifacts.

```
Layer 1: RESEARCH        ← Exploratory documents
Layer 2: REQUIREMENTS    ← What the system must do (testable)
Layer 3: ARCHITECTURE    ← High-level design and component boundaries
Layer 4: SPECIFICATIONS  ← Normative technical specs
Layer 5: PLANNING        ← Stages, checkpoints, sequencing
Layer 6: VALIDATION      ← Verification strategy and test plans
Layer 7: OPERATIONS      ← Live system procedures
Layer 8: HANDOFF         ← Zero-context continuation packages
```

**Flow direction:** Research → Requirements → Architecture → Specifications → Planning → Validation → Operations. Handoff artifacts are produced at every stage boundary.

**Parallel work:** Multiple agents can work on different layers simultaneously, provided traceability links are maintained.

---

## Layer Definitions

### Layer 1: Research
**Purpose:** Exploratory analysis, comparative evaluation, and design exploration.
**Location:** `docs/01-research/`
**Output to next layer:** Core claims and constraints → Requirements; Component boundaries → Architecture; Mechanism specifications → Specifications

### Layer 2: Requirements
**Purpose:** Explicit, testable statements of what the system must do.
**Location:** `docs/02-requirements/`
**Artifacts:** FR-*.md (functional), NFR-*.md (non-functional), PRD, acceptance matrix
**Output to next layer:** Requirements → Architecture component boundaries

### Layer 3: Architecture
**Purpose:** System decomposition, component boundaries, trust boundaries, invariant-level design.
**Location:** `docs/03-architecture/`
**Artifacts:** Component definitions, interfaces, data model, trust boundaries, failure model, ADRs
**Output to next layer:** Architecture → Spec sections

### Layer 4: Specifications
**Purpose:** Canonical normative behavior and interfaces. Implementation-independent truth source.
**Location:** `docs/04-specifications/`
**Artifacts:** Protocol specs, runtime specs, storage specs, security specs
**Output to next layer:** Specs → Planning stages

### Layer 5: Planning
**Purpose:** Sequence, stages, checkpoints, ownership, and delivery orchestration.
**Location:** `docs/05-planning/`
**Artifacts:** Roadmap, stage definitions, week-by-week breakdowns, checkpoints

### Layer 6: Validation
**Purpose:** Verification strategy and evidence that implementation matches specs.
**Location:** `docs/06-validation/`
**Artifacts:** Test strategy, adversarial scenarios, conformance matrix, eval plans

### Layer 7: Operations
**Purpose:** Live-system procedures and reliability/security operations.
**Location:** `docs/07-operations/`
**Artifacts:** Runbooks, SLOs, monitoring, incident postmortems

### Layer 8: Handoff
**Purpose:** Zero-context continuation package for new agents/operators.
**Location:** `docs/08-handoff/`
**Artifacts:** Stage status, risk register, next actions, artifact manifest with hashes

---

## Gates (hard approval checkpoints)

No artifact promotes to the next layer without passing the gate for its current layer.

| Gate | What passes | What is checked |
|------|------------|----------------|
| Research → Requirements | Research documents | Decentralisation audit, terminology consistency, cross-references valid |
| Requirements → Architecture | FR/NFR set | Every requirement maps to a component; no orphans |
| Architecture → Specifications | Component definitions, ADRs | Trust boundaries documented; interfaces deterministic |
| Specifications → Planning | Spec inventory | Every spec has conformance test hooks; trust-assumption inventory complete |
| Planning → Implementation | Stage plan for current stage | All upstream specs for this stage are frozen |
| Implementation → Integration | Component code | Component actually functions end-to-end (see Integration Gate below) |
| Any stage → Validation | Implementation artifacts | Conformance matrix updated; tests pass; integration tests pass |

### Gate rules
- A gate blocks promotion until all items in its checklist are satisfied.
- Gate review outputs are recorded in a checkpoint artifact.
- If a gate finds a defect in an upstream layer, the defect is fixed in the upstream layer and the gate is re-run.
- No "temporary exceptions" or "we'll fix it later."

---

## Traceability

Every artifact must maintain bidirectional traceability:

```
Research Claim → Requirement → Architecture Decision → Specification → Test Case → Implementation
```

### Traceability matrix
- Format: tabular, one row per claim.
- Stored in: `docs/08-handoff/latest/traceability-matrix.md` (updated at each checkpoint).
- Updated by: whoever changes any artifact in the chain.

### Drift prevention
- Spec changes without conformance updates = blocked.
- Requirement changes without ADR/spec impact analysis = blocked.
- Terminology, parameter, or duplicated normative rule changes without reconciliation = blocked.

---

## Decentralisation Audit Gate

This gate runs between Research → Requirements and again between Specifications → Planning. It is a hard gate.

### Checklist

1. **External trust inventory**
   - List every external service, oracle, or infrastructure assumption.
   - For each: explain why it is necessary and what the trust-minimised alternative is.
   - If no alternative exists, document the residual risk.

2. **Centralised coordination assumptions**
   - Scan for language implying a single dispatcher, scheduler, moderator, or admin override.
   - Flag any "manual assignment," "human review," or "admin approval" triggers.
   - Replace with deterministic protocol fallbacks.

3. **Verifiability of economic signals**
   - Any parameter tied to rewards or slashing must be cryptographically verifiable by the network.
   - Self-reported local metrics cannot be protocol-enforced economics.

4. **Single points of failure**
   - Identify any component whose failure stalls the entire system.
   - Require redundancy, fallback, or graceful degradation.

5. **Sybil-resistance mechanism review**
   - Every anti-Sybil control must be evaluated against pseudonymous, permissionless participation.
   - IP-based limits, geo-IP checks, and social-graph assumptions must be flagged and replaced.

### Audit output

If issues are found, create `docs/01-research/_overengineered.md` (or a per-spec equivalent) containing:
- Source file
- One-line issue summary
- Explanation of why it contradicts a decentralised model
- Proposed fix

Fix issues before promotion. Do not create an empty audit file if no issues are found.

---

## Integration Gate (hard)

This gate runs before any stage week can be marked complete. It ensures components actually function end-to-end, not just pass isolated unit tests.

### Checklist

For each component claimed as "complete" this week, verify:

1. **Network components** (P2P, transport, discovery):
   - Actual socket connections established (TCP/UDP), not just state machine transitions
   - Messages sent and received between two independent processes or threads
   - Connection lifecycle (connect → exchange → disconnect) demonstrated

2. **Storage components** (artifact storage, state persistence):
   - Actual disk I/O: write data, restart, read it back
   - Content-addressed verification: hash of stored data matches expected hash
   - Not just in-memory data structures with types defined

3. **Consensus/protocol components** (block production, voting, state transitions):
   - Actual processing loop: input transactions → execute → produce output (block/state update)
   - State changes are observable and verifiable after processing
   - Not just type definitions + pure function unit tests

4. **Runtime components** (agent runtime, governance engine, PDP):
   - Component runs its main loop and processes real input
   - Output is observable (state changes, messages sent, decisions made)
   - Not just types + rule definitions with unit tests of individual rules

5. **Node binary / integration**:
   - Node binary demonstrates the component in action (not just boot and sleep)
   - At least one integration test exercises the component through its public interface with real I/O

### Failure criteria

A component FAILS the integration gate if:
- It only has unit tests of internal/pure functions with no end-to-end demonstration
- It defines types, enums, and structs but has no behavior that produces observable output
- It uses mocks, shims, or in-memory simulations where real I/O is required by the spec
- The node binary does not exercise the component (e.g., consensus loop is a `sleep()` timer)

### Gate output

If a component fails the integration gate:
1. Do NOT mark the week as complete.
2. Document the gap in `docs/08-handoff/latest/open-questions.md` with:
   - Component name
   - What behavior is missing (e.g., "no actual socket connections", "no disk I/O", "consensus loop is stub")
   - What is needed to pass (e.g., "implement TCP listener + connector", "wire state machine into block production loop")
3. The component remains in progress until the integration gate passes.

---

## Gap Discovery

Before promoting research to requirements, run a gap analysis to surface implicit assumptions. Document findings in `PROJECT-STATUS.md` — not in this build system file.

Required output: a list of implicit assumptions, unwritten requirements, and unproven claims. Every gap must be either resolved in research or converted to an explicit requirement before Layer 2.

For the current project's active gaps, see `PROJECT-STATUS.md` "Research Gaps."

---

## File Naming Conventions

- **Requirements:** `FR-XXXX-short-title.md`, `NFR-XXXX-short-title.md`
- **ADRs:** `ADR-XXXX-short-title.md`
- **Specs:** `subsystem-topic-spec.md`
- **Stages:** `stage-NN-descriptive-name.md`
- **Checkpoints:** `checkpoint-NN-description.md`

---

## Supporting Documents

- `TEMPLATES.md` — Format templates for every artifact type.
- `GLOSSARY.md` — Canonical terminology (use consistently across all layers).
- `PROJECT-STATUS.md` — Current project state, active gaps, blockers, next actions.
- `PROMPT-BOOK.md` — Executable prompts for building each layer.
