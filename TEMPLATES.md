# Artifact Templates

Every artifact type in the 8-layer pipeline follows a template. Use these as the single source of truth for format. Do not redefine formats inline in prompts.

---

## Research Document

Location: `docs/01-research/*/*.md`

```markdown
# 1. Title
- [Clear, technical title]

# 2. Executive Summary
- [5–10 bullets maximum: what the system is]
- [Key insight or design idea]

# 3. System Overview
- [Problem solved]
- [Core design philosophy]
- [Key constraints]

# 4. Architecture (CRITICAL SECTION)
- [Components]
- [Step-by-step data flow]

## Diagrams (REQUIRED)
Use Mermaid. Preferred types: flowchart, sequence, state machine.

## Component Responsibilities
- [Component A: responsibilities]
- [Component B: responsibilities]

## Step-by-Step Data Flow
1. [Step 1]
2. [Step 2]

# 5. Core Mechanisms
- [Mechanism 1]
- [Mechanism 2]

## Pseudocode (for complex mechanisms)
```text
function process(request):
    metadata = validate(request)
    return finalize(result)
```

# 6. Design Decisions & Tradeoffs
## Tradeoff 1
- Option A: [description]
- Option B: [description]
- Chosen: [option]
- Why chosen: [justification]
- Sacrifice: [what is lost]
- Scaling risk: [what breaks or degrades]

# 7. Failure Modes & Edge Cases
## Scenario: [Name]
- What happens: [behavior]
- Why it happens: [root cause]
- Handling/failure mode: [mitigation]

# 8. Scalability Analysis
## Small scale (10–100 nodes)
- [Expected behavior, bottlenecks]

## Medium scale (1k–10k nodes)
- [Expected behavior, bottlenecks]

## Large scale (100k+ nodes)
- [Expected behavior, critical bottlenecks, hard constraints]

# 9. Recommended Architecture
- [Final architecture choice]
- [Rejected alternatives]

# 10. Implementation Plan
1. [Components to build first]
2. [Testing strategy]

# 11. Future Improvements
- [Possible upgrades]
```

---

## Functional Requirement (FR-XXXX)

Location: `docs/02-requirements/protocol/FR-XXXX-short-title.md`

```markdown
## FR-XXXX: [Short descriptive title]

**Category:** Consensus | Networking | Agent Runtime | Governance | Economics

**Statement:** The system shall [specific testable behavior]

**Rationale:** [Why this exists, with link to source research]

**Source Research:**
- [research-file.md#section]

**Acceptance Criteria:**
- [ ] Criterion 1 (measurable)
- [ ] Criterion 2 (measurable)

**Dependencies:** FR-YYYY, NFR-YYYY, or "none"
**Tags:** must-have | should-have | nice-to-have
```

---

## Non-Functional Requirement (NFR-XXXX)

Same format as FR-XXXX, category limited to Performance | Security | Reliability | Scalability.

---

## Specification Section

Location: `docs/04-specifications/**/*.md`

Each spec is a sequence of sections following this pattern:

```markdown
## Section X: Topic

### X.1 Purpose
What this spec section defines.

### X.2 Normative Behavior
MUST/SHOULD/MAY statements defining behavior.

### X.3 Data Structures
```rust
// Pseudocode or actual type definitions
struct Example {
    field: Type,
}
```

### X.4 State Transitions
State machine or flow description.

### X.5 Failure Behavior
What happens on errors.

### X.6 Versioning and Compatibility
Upgrade rules and backwards compatibility.

### X.7 Conformance Test Hooks
How to verify this spec is implemented correctly.

### X.8 Trust-Assumption Inventory
- External dependency/oracle: [name]
  - Justification: [why it is necessary]
  - Trust-minimised alternative: [what it is, or "none — residual risk documented"]
```

Every spec **must** include section X.8.

---

## Stage Definition

Location: `docs/05-planning/stages/stage-NN-name.md`

```markdown
# Stage N: Name

## Inputs
- From previous stage: (specific artifacts)
- External: (dependencies)

## Outputs
- Artifacts to produce

## Exit Criteria
- [ ] All outputs complete
- [ ] Acceptance criteria met
- [ ] Risks documented and acceptable
- [ ] Next stage inputs prepared

## Duration Estimate
X weeks

## Dependencies
- Stage N-1 complete
- External dependency Y

## Risk Areas
- Risk 1 and mitigation
- Risk 2 and mitigation
```

---

## Checkpoint Contract

Produced at every stage boundary.

Mandatory artifacts:
1. `stage-status.md` — What is complete/incomplete
2. `artifact-manifest.json` — All files + hashes
3. `open-risks.md` — Ranked unresolved risks
4. `decision-log.md` — New ADRs and parameter choices
5. `next-stage-inputs.md` — Explicit required inputs for next stage
6. `resume-instructions.md` — How a new agent resumes without context

Handoff-proofing rules:
- No stage may rely on implicit chat context.
- All assumptions recorded in stage outputs.
- Every output includes: owner stage, version, upstream deps, downstream deps.
- New agent startup: read `docs/08-handoff/latest/*`, validate hashes, continue.

---

## Architecture Decision Record (ADR-XXXX)

Location: `docs/03-architecture/decisions/ADR-XXXX-short-title.md`

```markdown
## ADR-XXXX: [Title]

**Status:** proposed | accepted | deprecated

**Context:** [What problem does this decision address?]

**Decision:** [What was decided?]

**Consequences:**
- Positive: [benefits]
- Negative: [costs, risks]

**Alternatives considered:**
- [Option A]: [why rejected]
- [Option B]: [why rejected]

**Related:** [FR-XXXX, research-file.md]
```
