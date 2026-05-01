# Hyperfluid Build System: Structure and Workflow

## Purpose

This document defines the **layered documentation structure** for converting research into production-ready specifications and implementation artifacts. It is the master plan for building Hyperfluid—not a description of Hyperfluid itself.

Use this document when:
- Starting a new implementation stage
- Creating new specification documents from research
- Handing off work between agents
- Determining where a new document belongs

---

## Layer Structure

Research flows through 7 layers, from exploration to operations. Each layer has a specific purpose and produces specific artifacts.

```
Layer 1: RESEARCH            ← Current state: exploratory documents
Layer 2: REQUIREMENTS        ← What the system must do (testable)
Layer 3: ARCHITECTURE        ← High-level design and component boundaries
Layer 4: SPECIFICATIONS      ← Normative technical specs
Layer 5: PLANNING            ← Stages, checkpoints, sequencing
Layer 6: VALIDATION          ← Verification strategy and test plans
Layer 7: OPERATIONS          ← Live system procedures
Layer 8: HANDOFF             ← Zero-context continuation packages
```

---

## Layer 1: Research

**Purpose:** Exploratory analysis, comparative evaluation, and design exploration.

**Current location:** `research/` (at project root)

**Target structure:**
```
docs/01-research/
  index.md                     # Research inventory and cross-references
  agents/                      # Agent runtime, coordination, identity
  consensus-governance/        # BFT, staking, governance mechanisms
  networking/                  # P2P, transport, availability
  security/                    # Threat models, attacks, mitigations
  economics/                   # Tokenomics, incentives, market design
  evaluations/                 # Benchmarks, comparisons, stack evaluations
```

**Research document format:**
Must follow `_template.md` (research document template inside the research folder):
1. Title
2. Executive Summary (5-10 bullets)
3. System Overview
4. Architecture (with Mermaid diagrams)
5. Core Mechanisms
6. Design Decisions & Tradeoffs (min 3)
7. Failure Modes & Edge Cases
8. Scalability Analysis
9. Recommended Architecture
10. Implementation Plan
11. Future Improvements

**Output to next layer:**
- Core claims and constraints → Requirements
- Component boundaries → Architecture
- Mechanism specifications → Specifications

---

## Layer 2: Requirements

**Purpose:** Explicit, testable statements of what the system must do.

**Artifacts:**
```
docs/02-requirements/
  index.md                     # Requirements inventory and traceability
  product/
    PRD-system-objectives.md   # Product requirements document
  protocol/
    FR-*.md                    # Functional requirements (numbered)
    NFR-*.md                   # Non-functional requirements (performance, security)
  acceptance/
    acceptance-matrix.md       # Testable acceptance criteria per requirement
```

**Requirement format:**
```markdown
## FR-0001: Requirement Title

**Category:** Consensus | Networking | Agent Runtime | Governance | Economics

**Statement:** The system shall...

**Rationale:** Why this requirement exists (link to research)

**Source Research:**
- research/consensus-governance/agx-committee-bft-and-governance.md

**Acceptance Criteria:**
- [ ] Criterion 1 (measurable)
- [ ] Criterion 2 (measurable)

**Dependencies:** FR-XXXX, NFR-XXXX
**Tags:** must-have, should-have, nice-to-have
```

**Process:**
1. Extract claims from research documents
2. Convert to functional requirements (FR-XXXX)
3. Convert to non-functional requirements (NFR-XXXX)
4. Define acceptance criteria (testable, measurable)
5. Tag with source research links

---

## Layer 3: Architecture

**Purpose:** System decomposition, component boundaries, trust boundaries, invariant-level design.

**Artifacts:**
```
docs/03-architecture/
  index.md                     # Architecture overview and navigation
  system-context.md            # System boundaries and external interfaces
  component-model/
    components.md              # Component definitions and responsibilities
    interfaces.md              # Inter-component contracts
  data-model/
    state-model.md             # Core state entities and relationships
    message-model.md           # Message types and flows
  trust-boundaries.md          # Security domains and trust assumptions
  failure-model.md             # System-level failure scenarios
```

**Architecture format:**
- Component diagrams (Mermaid)
- Interface definitions (inputs, outputs, error conditions)
- State transition diagrams where applicable
- Explicit trust boundaries and threat model references

**Process:**
1. Map requirements to component boundaries
2. Define interfaces and data ownership
3. Identify trust boundaries
4. Record decisions as ADRs
5. Update traceability links

---

## Layer 4: Specifications

**Purpose:** Canonical normative behavior and interfaces. Implementation-independent truth source.

**Artifacts:**
```
docs/04-specifications/
  index.md                     # Spec inventory and status
  protocol/
    p2p-wire-spec.md           # Network protocol, message serialization
    consensus-spec.md          # BFT consensus rules, state machine
    staking-spec.md            # Staking lifecycle, slashing conditions
    governance-spec.md         # Proposal/voting, git:head transitions
    fastpath-spec.md           # Topic coordination fast path
  runtime/
    agent-runtime-spec.md      # Agent execution environment
    policy-engine-spec.md      # Action plan validation and quotas
    review-engine-spec.md      # Quality review and scoring
  storage/
    artifact-availability-spec.md  # Content-addressed storage guarantees
    state-sync-spec.md         # State synchronization protocol
  security/
    key-management-spec.md     # Key derivation, signing, rotation
    incident-response-spec.md  # Emergency procedures and controls
```

**Specification format:**
```markdown
## Section X: Topic

### X.1 Purpose
What this spec section defines

### X.2 Normative Behavior
MUST/SHOULD/MAY statements defining behavior

### X.3 Data Structures
```rust
// Pseudocode or actual type definitions
struct Example {
    field: Type,
}
```

### X.4 State Transitions
State machine or flow description

### X.5 Failure Behavior
What happens on errors

### X.6 Versioning and Compatibility
Upgrade rules and backwards compatibility

### X.7 Conformance Test Hooks
How to verify this spec is implemented correctly
```

**Process:**
1. Translate architecture components into specs
2. Define deterministic inputs/outputs
3. Specify failure behavior explicitly
4. Include conformance test hooks
5. Update traceability (Architecture → Specification)

---

## Layer 5: Planning

**Purpose:** Sequence, stages, checkpoints, ownership, and delivery orchestration.

**Artifacts:**
```
docs/05-planning/
  index.md                     # Planning overview and current status
  roadmap/
    release-tracks.md          # Major release themes and timelines
  stages/
    stage-00-foundation.md     # Requirements + ADR baseline
    stage-01-protocol-core.md  # Consensus/staking/wire/storage specs
    stage-02-agent-runtime.md  # Runtime, policy, review, coordination
    stage-03-hardening.md      # Validation, adversarial testing
    stage-04-mainnet-ready.md  # Operations, monitoring, incident response
  checkpoints/
    checkpoint-template.md     # Template for stage checkpoints
    stage-00/                  # Per-stage checkpoint artifacts
      checkpoint-00-baseline.md
      artifact-manifest.json
      open-risks.md
      decision-log.md
      next-stage-inputs.md
      resume-instructions.md
```

**Stage format:**
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

**Checkpoint contract (mandatory at each stage end):**
1. `stage-status.md` - What is complete/incomplete
2. `artifact-manifest.json` - All files + hashes
3. `open-risks.md` - Ranked unresolved risks
4. `decision-log.md` - New ADRs and parameter choices
5. `next-stage-inputs.md` - Explicit required inputs for next stage
6. `resume-instructions.md` - How a new agent resumes without context

**Handoff-proofing rules:**
- No stage may rely on implicit chat context
- All assumptions recorded in stage outputs
- Every output includes: owner stage, version, upstream deps, downstream deps
- New agent startup: read `docs/08-handoff/latest/*`, validate hashes, continue

---

## Layer 6: Validation

**Purpose:** Verification strategy and evidence that implementation matches specs.

**Artifacts:**
```
docs/06-validation/
  index.md                     # Validation strategy overview
  test-strategy.md             # Testing pyramid and approach
  simulation/
    adversarial-scenarios.md   # Attack scenarios and simulations
  evals/
    prompt-injection-eval-plan.md  # Agent security evaluations
    redteam-findings.md        # Red team results and mitigations
  conformance/
    spec-conformance-matrix.md # Spec section → test case mapping
    test-results/              # Test execution results
```

**Process:**
1. Define test strategy per spec section
2. Create adversarial scenarios from failure modes in research
3. Build conformance matrix (Spec → Test)
4. Execute and record results

---

## Layer 7: Operations

**Purpose:** Live-system procedures and reliability/security operations.

**Artifacts:**
```
docs/07-operations/
  index.md                     # Operations overview
  runbooks/
    deployment.md              # Deployment procedures
    upgrade.md                 # Upgrade procedures
    incident-response.md       # Incident response playbooks
    key-rotation.md            # Key management procedures
  incident-postmortems/
    YYYY-MM-DD-incident-name.md
  SLOs.md                      # Service level objectives
  monitoring.md                # Metrics, alerts, dashboards
```

**Process:**
1. Document operational procedures from specs
2. Define SLOs from NFRs
3. Create incident response playbooks from failure modes
4. Record postmortems and feed back to research

---

## Layer 8: Handoff

**Purpose:** Zero-context continuation package for new agents/operators.

**Artifacts:**
```
docs/08-handoff/
  handoff-contract.md          # How to use handoff packages
  artifact-manifest-template.md
  latest/                      # Current handoff package
    stage-status.md            # Current stage completion status
    open-risks.md              # Active risks and blockers
    next-actions.md            # What to do next
    artifact-manifest.json     # All current artifacts + hashes
  archive/                     # Historical handoff packages
    stage-00-baseline/
    stage-01-protocol-core/
```

**Process:**
1. At each stage checkpoint, create complete handoff package
2. Include all artifacts, their hashes, and status
3. Document next actions for resuming agent
4. Archive when stage completes

---

## Traceability Requirements

Every artifact must maintain bidirectional traceability:

```
Research Claim → Requirement → Architecture Decision → Specification → Test Case → Implementation
```

**Traceability matrix format:**
```markdown
| Research | Requirement | ADR | Spec | Test | Status |
|----------|-------------|-----|------|------|--------|
| consensus-governance/agx-committee-bft-and-governance.md#staking | FR-0021 | ADR-0003 | staking-spec.md#lifecycle | test-staking-lifecycle.rs | ✓ |
```

**Drift prevention:**
- Block merges when spec changes lack conformance updates
- Block merges when requirement changes lack ADR/spec impact analysis
- Run periodic contradiction linting:
  - Terminology consistency checks
  - Parameter consistency checks
  - Duplicated normative rule detection

---

## Document Workflow

### Converting Research to Implementation

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│   Research  │────▶│ Requirements│────▶│  Architecture│────▶│Specifications│
│   (Layer 1) │     │  (Layer 2)  │     │  (Layer 3)  │     │  (Layer 4)  │
└─────────────┘     └─────────────┘     └─────────────┘     └─────────────┘
         │                  │                  │                  │
         ▼                  ▼                  ▼                  ▼
    [Claims]           [Testable]         [Components]       [Normative]
    [Exploration]      [Constraints]      [Interfaces]       [Behavior]
```

### Research → Requirements
1. Extract core claims from research (what the system must do)
2. Convert to functional requirements (FR-XXXX)
3. Convert to non-functional requirements (NFR-XXXX)
4. Define acceptance criteria (testable, measurable)
5. Tag with source research links

### Requirements → Architecture
1. Map FR/NFR set to component boundaries
2. Define interfaces between components
3. Identify trust boundaries
4. Record decisions as ADRs
5. Update traceability links

### Architecture → Specifications
1. Produce protocol/runtime/storage/security specs
2. For each spec section require:
   - Deterministic inputs/outputs
   - Failure behavior
   - Compatibility/version rules
   - Conformance test hooks

### Specifications → Planning
1. Group specs into implementation stages
2. Define stage inputs/outputs/exit criteria
3. Create checkpoint artifacts
4. Generate handoff packages

### Specifications → Validation
1. Create test cases from spec conformance hooks
2. Design adversarial scenarios from research failure modes
3. Build eval plans for agent behaviors
4. Record results in conformance matrix

### Validation → Operations
1. Convert test scenarios to operational runbooks
2. Define SLOs from NFRs
3. Create incident response procedures
4. Feed operational findings back to research

---

## Research-to-Spec Mapping

Map research documents to their primary target specification layers:

| Research Document | Primary Layer | Target Specs |
|-------------------|---------------|--------------|
| `research/agents/infinite-agent.md` | Layer 4 | `runtime/agent-runtime-spec.md` |
| `research/agents/token-budget-resource-model.md` | Layer 4 | `runtime/agent-runtime-spec.md` |
| `research/agents/network-policy-engine-spec.md` | Layer 4 | `runtime/policy-engine-spec.md` |
| `research/agents/proof-of-work-quality-and-review-markets.md` | Layer 4 | `runtime/review-engine-spec.md` |
| `research/agents/topic-fastpath-protocol-spec.md` | Layer 4 | `protocol/fastpath-spec.md` |
| `research/agents/identity-reputation-and-trust-ladder.md` | Layers 2-3 | `requirements/protocol/FR-identity-*.md`, `trust-boundaries.md` |
| `research/agents/collaboration-layer-parallel-teams.md` | Layers 2-3 | `requirements/protocol/FR-collaboration-*.md` |
| `research/agents/inbox-attention-control-and-anti-spam.md` | Layers 2-4 | `protocol/p2p-wire-spec.md`, `runtime/policy-engine-spec.md` |
| `research/agents/prompt-injection-and-network-policy-boundary.md` | Layers 2, 6 | `requirements/security/`, `evals/prompt-injection-eval-plan.md` |
| `research/consensus-governance/agx-committee-bft-and-governance.md` | Layer 4 | `protocol/consensus-spec.md`, `protocol/governance-spec.md`, `protocol/staking-spec.md` |
| `research/consensus-governance/agx-economics-and-adversarial-incentives.md` | Layers 2-3 | `requirements/protocol/FR-economics-*.md`, `data-model/state-model.md` |
| `research/networking/ockam-decentralized-network-architecture.md` | Layer 4 | `protocol/p2p-wire-spec.md` |
| `research/networking/artifact-availability-and-retention.md` | Layer 4 | `storage/artifact-availability-spec.md` |
| `research/networking/decentralized-incident-response-and-recovery.md` | Layers 4, 7 | `security/incident-response-spec.md`, `07-operations/runbooks/` |
| `research/security/telemetry-threat-model.md` | Layers 2, 6 | `security/telemetry-spec.md`, `evals/prompt-injection-eval-plan.md` |
| `research/stack-evaluations/decentralization-and-stack-benchmark.md` | Layer 1 | Informs all specs |

---

## Terminology

**Canonical terms** (use consistently across all layers):
- `active` / `paused` / `unbonding` / `withdrawn` - Validator lifecycle states (4-state model; `inactive_bonded` has been merged into `paused`)
- `untrusted_joiner` - Initial trust stage
- `sandboxed_contributor` - Trust stage after initial work
- `trusted_contributor` - Established contributor
- `coordinator_eligible` - Can coordinate topics
- `action_plan` - Network mutation intent
- `plan_signature` - Cryptographic authorization
- `git:head` - On-chain code state reference

---

## File Naming Conventions

**Requirements:** `FR-XXXX-short-title.md`, `NFR-XXXX-short-title.md`

**ADRs:** `ADR-XXXX-short-title.md`

**Specs:** `subsystem-topic-spec.md` (e.g., `consensus-spec.md`, `staking-spec.md`)

**Stages:** `stage-NN-descriptive-name.md`

**Checkpoints:** `checkpoint-NN-description.md`

---

## Migration from Current State

**Current:** Research lives in `research/` at project root

**Target:** Research moves to `docs/01-research/`

**Migration steps:**
1. Create `docs/` structure
2. Move `research/` → `docs/01-research/`
3. Create layer indices
4. Begin extracting requirements from existing research
5. Build traceability matrix incrementally

---

## Unwritten / Implicit Knowledge

The following assumptions, constraints, and contextual knowledge exist but are not yet explicitly captured in the research corpus. These must be surfaced and documented as the build system progresses.

### System-Level Assumptions

1. **Canonical Document Authority**
   - Policy semantics have a single source of truth: `policy-engine-spec.md`
   - Fast-path semantics live in `fastpath-spec.md`
   - Artifact availability semantics live in `artifact-availability-spec.md`
   - Cross-document references must point to canonical owners, never duplicate definitions

2. **Token Budget as System Resource**
   - Token limits are not just LLM runtime concerns
   - Must be formalized in runtime specs and validation criteria
   - Ingress budgets, context windows, and handoff triggers need explicit treatment

3. **Content-Addressed Artifact Reliability**
   - Governance determinism assumes artifact retrievability
   - Research assumes IPFS/content-addressing "just works" under churn
   - This coupling must be explicit in requirements and conformance tests

4. **Policy Bundle Propagation Speed**
   - Action plans assume policy bundles propagate "fast enough"
   - Split-brain scenarios from slow propagation are not fully analyzed
   - Network delay bounds need specification

### Protocol Semantics

5. **No-Vote Timeout Semantics**
   - Fairness/safety invariant for review subagents
   - Must be codified across governance + fast-path + runtime specs
   - Currently implied but not explicitly stated

6. **Single-Use Action Plan Execution**
   - Replay-safety invariant requiring end-to-end enforcement
   - Policy gate + executor + audit trail must coordinate
   - Consumed plan state transitions need explicit specification

7. **Local Operator Freedom Boundary**
   - Local actions (non-network-mutating) are out of protocol scope
   - Sandboxed execution containment assumptions
   - Must be consistently stated in runtime/security docs to prevent overreach

### Economic and Incentive Assumptions

8. **Reviewer Independence Scoring**
   - Quality review assumes robust identity correlation signals
   - Sybil resistance in trust ladder depends on unproven signals
   - Anti-collusion mechanisms assume independence can be measured

9. **Telemetry Integrity**
   - Incident triggers assume honest telemetry
   - Red-team eval trustworthiness assumes uncorrupted metrics
   - Compromised telemetry pathways not fully threat-modeled

10. **Economic Finality Timing**
    - Review reward settlement timing relative to challenge windows
    - Slashing execution timing relative to evidence submission
    - These time bounds exist in research but need formal specification

### Operational Assumptions

11. **Operator Sandbox Quality**
    - Risk containment assumes proper sandboxing
    - Escape vectors not fully enumerated
    - Host compromise scenarios under-specified

12. **Emergency Control Loop Stability**
    - Incident response assumes anti-oscillation mechanisms work
    - Hysteresis thresholds, dwell timers, cooldown gates need verification
    - Multiple simultaneous incidents not fully analyzed

13. **Deterministic Execution Environment**
    - `git:head` governance assumes hermetic execution
    - Reproducible input bundles requirement
    - Build environment assumptions need documentation

### Gaps to Fill in Specification

| Implicit Knowledge | Should Live In | Current State |
|--------------------|----------------|---------------|
| Token budget resource model | `runtime/agent-runtime-spec.md` | Written: `research/agents/token-budget-resource-model.md` |
| No-vote timeout fairness proof | `protocol/governance-spec.md` | Implied only |
| Plan replay protection E2E | `runtime/policy-engine-spec.md` + storage specs | Partial |
| Telemetry threat model | `security/` research → specs | Written: `research/security/telemetry-threat-model.md` |
| Sandbox escape analysis | `security/` research → specs | Unwritten |
| Content-addressing SLA | `storage/artifact-availability-spec.md` | Assumed |
| Review independence metrics | `runtime/review-engine-spec.md` | Implied only |
| Economic timing parameters | `protocol/` specs | Partial in research |

### Action Items

1. **Before Layer 2 (Requirements):** Surface all implicit assumptions in research review
2. **During Layer 3 (Architecture):** Document trust boundaries around assumed components
3. **During Layer 4 (Specifications):** Convert all "assumed" behaviors to explicit normative requirements
4. **During Layer 6 (Validation):** Create test cases that fail if assumptions are violated

---

## Summary

This build system provides:

1. **Clear layering:** Research → Requirements → Architecture → Specs → Planning → Validation → Operations
2. **Traceability:** Every claim can be traced to implementation
3. **Handoff-proofing:** Zero-context resumption at any stage
4. **Checkpointing:** Stage boundaries with explicit artifacts
5. **Scalability:** Multiple agents can work in parallel on different layers

Use this structure when creating new documents. Always know which layer you're writing for and what the inputs/outputs should be.
