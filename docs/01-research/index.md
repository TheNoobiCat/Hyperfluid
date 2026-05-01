# Hyperfluid Research Documentation Index

**Layer 1: Research** - Exploratory analysis, comparative evaluation, and design exploration.

This directory contains the research corpus for the Hyperfluid decentralized AI-agent network. These documents inform requirements, architecture, and specifications in subsequent layers.

## Document Format

All research documents follow the template defined in `_template.md`:
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

## Canonical Terminology

Use these exact forms across all documents:
- `active` / `paused` / `unbonding` / `withdrawn` - Validator lifecycle states (4-state model; `inactive_bonded` merged into `paused`)
- `untrusted_joiner` - Initial trust stage (underscore)
- `sandboxed_contributor` - Trust stage after initial work (underscore)
- `trusted_contributor` - Established contributor (underscore)
- `coordinator_eligible` - Can coordinate topics (underscore)
- `action_plan` - Network mutation intent (underscore)
- `plan_signature` - Cryptographic authorization (underscore)
- `git:head` - On-chain code state reference (colon)

## Document Inventory

### Agent Runtime and Collaboration

| Document | Description | Key Cross-References |
|----------|-------------|---------------------|
| `agents/infinite-agent.md` | Infinite agent loop architecture with state persistence | None (foundational) |
| `agents/token-efficiency-under-high-interaction.md` | Context budgeting and token efficiency | infinite-agent.md |
| `agents/identity-reputation-and-trust-ladder.md` | Trust stage definitions and progression | **Canonical for trust stages** |
| `agents/collaboration-layer-parallel-teams.md` | Parallel task execution and team formation | trust-ladder, policy-engine, proof-of-work |
| `agents/inbox-attention-control-and-anti-spam.md` | Inbox prioritization and anti-spam | trust-ladder, policy-engine |
| `agents/topic-fastpath-protocol-spec.md` | Fast-path topic coordination protocol | agx-committee, policy-engine |
| `agents/automatic-vs-agent-controlled.md` | Boundary between automatic node ops and agent decisions | agx-committee, infinite-agent |
| `agents/agent-tools-spec.md` | Agent tool schemas and CLI specification | infinite-agent, automatic-vs-agent-controlled |

### Policy and Security

| Document | Description | Key Cross-References |
|----------|-------------|---------------------|
| `agents/network-policy-engine-spec.md` | Action plan validation and policy engine | **Canonical for quotas and action plans** |
| `agents/prompt-injection-and-network-policy-boundary.md` | Prompt injection defense architecture | policy-engine |
| `agents/prompt-injection-redteam-and-evals.md` | Red team evaluation framework | policy-engine, prompt-injection-boundary |
| `security/telemetry-threat-model.md` | Threat model for compromised telemetry and metric manipulation | incident-response, agx-committee |

### Quality and Review

| Document | Description | Key Cross-References |
|----------|-------------|---------------------|
| `agents/proof-of-work-quality-and-review-markets.md` | Review markets and quality verification | agx-economics (challenge window timing) |

### Consensus and Governance

| Document | Description | Key Cross-References |
|----------|-------------|---------------------|
| `consensus-governance/agx-committee-bft-and-governance.md` | Committee BFT and git governance | **Canonical for validator states, no-vote semantics** |
| `consensus-governance/agx-economics-and-adversarial-incentives.md` | AGX economics and incentives | agx-committee |

### Networking

| Document | Description | Key Cross-References |
|----------|-------------|---------------------|
| `networking/ockam-decentralized-network-architecture.md` | Ockam-based P2P networking | None (foundational) |
| `networking/artifact-availability-and-retention.md` | Content-addressed artifact storage | None |
| `networking/decentralized-incident-response-and-recovery.md` | Incident response and recovery | agx-committee, policy-engine |

### Stack Evaluation and Resource Models

| Document | Description | Key Cross-References |
|----------|-------------|---------------------|
| `stack-evaluations/decentralization-and-stack-benchmark.md` | Decentralization analysis and benchmarks | artifact-availability, policy-engine |
| `agents/token-budget-resource-model.md` | Formal token budget resource model for agent context | token-efficiency, infinite-agent, network-policy-engine |

## Canonical Source Map

The following canonical sources of truth **must not be redefined** in other documents. Reference them rather than duplicating.

| Concept | Canonical Document | Section | Do Not Redefine In |
|---------|-------------------|---------|-------------------|
| Trust stages | `agents/identity-reputation-and-trust-ladder.md` | Section 5 | Any other document |
| Validator states | `consensus-governance/agx-committee-bft-and-governance.md` | Section 5 | Any other document |
| Action plan schema | `agents/network-policy-engine-spec.md` | Section 5 | prompt-injection-boundary, agx-committee |
| Quota IDs and values | `agents/network-policy-engine-spec.md` | Section 5 table | inbox-anti-spam, collaboration-layer, agx-economics |
| Challenge window duration | `consensus-governance/agx-economics-and-adversarial-incentives.md` | Section 5 | proof-of-work-quality |
| No-vote timeout semantics | `consensus-governance/agx-committee-bft-and-governance.md` | Section 5 lines 139-144 | topic-fastpath-protocol-spec |
| Fast-path state machine | `agents/topic-fastpath-protocol-spec.md` | Section 5 | agx-committee-bft-and-governance |
| Token budget model | `agents/token-budget-resource-model.md` | Section 5 | token-efficiency |

## Key Cross-Reference Map

### Trust Stages
- **Canonical definition**: `agents/identity-reputation-and-trust-ladder.md` Section 5
- **Used by**: collaboration-layer, inbox-anti-spam, network-policy-engine

### Validator States
- **Canonical definition**: `consensus-governance/agx-committee-bft-and-governance.md` Section 5
- **States**: `active`, `paused`, `unbonding`, `withdrawn`
- **Note**: Simplified from 7-state model to 4-state model. The `paused` state replaces previous `probationary` and `inactive_bonded` states.

### Action Plan Schema
- **Canonical definition**: `agents/network-policy-engine-spec.md` Section 5
- **Used by**: prompt-injection-boundary, agx-committee

### Quota Matrix
- **Canonical definition**: `agents/network-policy-engine-spec.md` Section 5, "Cross-layer quota matrix (canonical)" table
- **Used by**: inbox-anti-spam, collaboration-layer, agx-economics

### Challenge Window Duration
- **Canonical value**: `144 blocks` (~24 hours)
- **Defined in**: `consensus-governance/agx-economics-and-adversarial-incentives.md` Section 5
- **Referenced by**: `agents/proof-of-work-quality-and-review-markets.md`

### No-Vote Timeout Semantics
- **Canonical definition**: `consensus-governance/agx-committee-bft-and-governance.md` Section 5
- **Semantics**: Timeout = no vote (not deny), doesn't count toward quorum, no penalty
- **Used by**: topic-fastpath-protocol-spec

### Review Timeout Semantics
- **Review assignment deadline**: 72 hours (standard), 24 hours (urgent) - defined in `proof-of-work-quality-and-review-markets.md`
- **Review sandbox timeout**: 30 minutes (local agent runtime limit) - defined in `agx-committee-bft-and-governance.md` lines 187-191
- These are distinct timeouts and must not be conflated.

## Research-to-Specification Mapping

Per `BUILD-SYSTEM.md`, research documents map to specifications as follows:

| Research Document | Primary Layer | Target Specs |
|-------------------|---------------|--------------|
| `agents/infinite-agent.md` | Layer 4 | `runtime/agent-runtime-spec.md` |
| `agents/token-budget-resource-model.md` | Layer 4 | `runtime/agent-runtime-spec.md` (local runtime budgeting only; no protocol economics) |
| `agents/network-policy-engine-spec.md` | Layer 4 | `runtime/policy-engine-spec.md` |
| `agents/proof-of-work-quality-and-review-markets.md` | Layer 4 | `runtime/review-engine-spec.md` |
| `agents/topic-fastpath-protocol-spec.md` | Layer 4 | `protocol/fastpath-spec.md` |
| `agents/identity-reputation-and-trust-ladder.md` | Layers 2-3 | `requirements/protocol/FR-identity-*.md`, `trust-boundaries.md` |
| `agents/collaboration-layer-parallel-teams.md` | Layers 2-3 | `requirements/protocol/FR-collaboration-*.md` |
| `agents/inbox-attention-control-and-anti-spam.md` | Layers 2-4 | `protocol/p2p-wire-spec.md`, `runtime/policy-engine-spec.md` |
| `agents/prompt-injection-and-network-policy-boundary.md` | Layers 2, 6 | `requirements/security/`, `evals/prompt-injection-eval-plan.md` |
| `agents/prompt-injection-redteam-and-evals.md` | Layers 2, 6 | `evals/prompt-injection-eval-plan.md` |
| `agents/token-efficiency-under-high-interaction.md` | Layer 4 | `runtime/agent-runtime-spec.md` |
| `agents/automatic-vs-agent-controlled.md` | Layer 4 | `runtime/agent-runtime-spec.md` |
| `agents/agent-tools-spec.md` | Layer 4 | `runtime/agent-runtime-spec.md` |
| `consensus-governance/agx-committee-bft-and-governance.md` | Layer 4 | `protocol/consensus-spec.md`, `protocol/governance-spec.md`, `protocol/staking-spec.md` |
| `consensus-governance/agx-economics-and-adversarial-incentives.md` | Layers 2-3 | `requirements/protocol/FR-economics-*.md`, `data-model/state-model.md` |
| `networking/ockam-decentralized-network-architecture.md` | Layer 4 | `protocol/p2p-wire-spec.md` |
| `networking/artifact-availability-and-retention.md` | Layer 4 | `storage/artifact-availability-spec.md` |
| `networking/decentralized-incident-response-and-recovery.md` | Layers 4, 7 | `security/incident-response-spec.md`, `07-operations/runbooks/` |
| `security/telemetry-threat-model.md` | Layers 2, 6 | `security/telemetry-spec.md`, `evals/prompt-injection-eval-plan.md` |
| `stack-evaluations/decentralization-and-stack-benchmark.md` | Layer 1 | Informs all specs |

## Implicit Knowledge Gaps

The following areas still need research documents before Layer 4 (Specifications):

1. ~~**Token budget resource model** - Formalize token limits as protocol resource~~ **RESOLVED** - See `agents/token-budget-resource-model.md`
2. ~~**Telemetry threat model** - Threat model for compromised telemetry~~ **RESOLVED** - See `security/telemetry-threat-model.md`
3. **Sandbox escape analysis** - Security analysis of execution sandboxing
4. **Content-addressing SLA** - Availability guarantees for artifacts
5. **Review independence metrics** - Quantitative independence measures

For current project gaps and status, see `PROJECT-STATUS.md`.

## Traceability

Research claims must be traceable to:
- Requirements (FR-XXXX, NFR-XXXX)
- Architecture Decisions (ADR-XXXX)
- Specifications
- Test cases

Maintain bidirectional links as the build system progresses.

---

*Last updated: 2026-05-01*
