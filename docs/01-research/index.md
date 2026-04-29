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
- `inactive_bonded` - Validator lifecycle state (underscore)
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

### Policy and Security

| Document | Description | Key Cross-References |
|----------|-------------|---------------------|
| `agents/network-policy-engine-spec.md` | Action plan validation and policy engine | **Canonical for quotas and action plans** |
| `agents/prompt-injection-and-network-policy-boundary.md` | Prompt injection defense architecture | policy-engine |
| `agents/prompt-injection-redteam-and-evals.md` | Red team evaluation framework | policy-engine, prompt-injection-boundary |

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

### Stack Evaluation

| Document | Description | Key Cross-References |
|----------|-------------|---------------------|
| `stack-evaluations/decentralization-and-stack-benchmark.md` | Decentralization analysis and benchmarks | artifact-availability, policy-engine |

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
- **Canonical definition**: `agents/network-policy-engine-spec.md` Section 5, table
- **Used by**: inbox-anti-spam, collaboration-layer

### Challenge Window Duration
- **Canonical value**: `144 blocks` (~24 hours)
- **Defined in**: `consensus-governance/agx-economics-and-adversarial-incentives.md` Section 5
- **Referenced by**: `agents/proof-of-work-quality-and-review-markets.md`

### No-Vote Timeout Semantics
- **Canonical definition**: `consensus-governance/agx-committee-bft-and-governance.md` Section 5
- **Semantics**: Timeout = no vote (not deny), doesn't count toward quorum, no penalty
- **Used by**: topic-fastpath-protocol-spec

## Research-to-Specification Mapping

Per BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md, research documents map to specifications as follows:

| Research Document | Target Specifications |
|-------------------|----------------------|
| infinite-agent.md | runtime/agent-runtime-spec.md |
| network-policy-engine-spec.md | runtime/policy-engine-spec.md |
| proof-of-work-quality-and-review-markets.md | runtime/review-engine-spec.md |
| topic-fastpath-protocol-spec.md | protocol/fastpath-spec.md |
| identity-reputation-and-trust-ladder.md | requirements/protocol/FR-identity-*.md |
| agx-committee-bft-and-governance.md | protocol/consensus-spec.md, governance-spec.md, staking-spec.md |
| ockam-decentralized-network-architecture.md | protocol/p2p-wire-spec.md |
| artifact-availability-and-retention.md | storage/artifact-availability-spec.md |

## Implicit Knowledge Gaps

The following areas need research documents before Layer 4 (Specifications):

1. **Token budget resource model** - Formalize token limits as protocol resource
2. **Telemetry threat model** - Threat model for compromised telemetry
3. **Sandbox escape analysis** - Security analysis of execution sandboxing
4. **Content-addressing SLA** - Availability guarantees for artifacts
5. **Review independence metrics** - Quantitative independence measures

See BUILD-SYSTEM-STRUCTURE-AND-WORKFLOW.md Section "Unwritten / Implicit Knowledge" for full details.

## Traceability

Research claims must be traceable to:
- Requirements (FR-XXXX, NFR-XXXX)
- Architecture Decisions (ADR-XXXX)
- Specifications
- Test cases

Maintain bidirectional links as the build system progresses.

---

*Last updated: 2026-04-29*
