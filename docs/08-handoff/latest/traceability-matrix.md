# Traceability Matrix

**Last updated:** 2026-05-05

Every claim is traceable: Research → Requirement → Architecture Decision → Specification → Test Case → Implementation.

## FR Traceability

| FR | Research Source | Requirement File | Architecture Component | Spec Document | Spec Section | Status |
|----|----------------|-----------------|-----------------------|---------------|--------------|--------|
| FR-0001 | consensus-governance/agx-committee-bft-and-governance.md | FR-0001-0010 | C1 | consensus-spec.md | 1 | complete |
| FR-0002 | consensus-governance/agx-committee-bft-and-governance.md, stake-graph-analysis-spec.md | FR-0001-0010 | C1, C3 | consensus-spec.md, stake-graph-analysis-spec.md | 1, 1 | complete |
| FR-0003 | consensus-governance/agx-committee-bft-and-governance.md | FR-0001-0010 | C1 | consensus-spec.md | 1 | complete |
| FR-0004 | consensus-governance/agx-committee-bft-and-governance.md | FR-0001-0010 | C1 | consensus-spec.md | 1 | complete |
| FR-0005 | consensus-governance/agx-committee-bft-and-governance.md | FR-0001-0010 | C2 | consensus-spec.md | 2 | complete |
| FR-0006 | consensus-governance/agx-committee-bft-and-governance.md | FR-0001-0010 | C2 | consensus-spec.md | 2 | complete |
| FR-0007 | consensus-governance/agx-committee-bft-and-governance.md | FR-0001-0010 | C2 | consensus-spec.md | 2 | complete |
| FR-0008 | consensus-governance/agx-committee-bft-and-governance.md | FR-0001-0010 | C2 | consensus-spec.md | 1 | complete |
| FR-0009 | consensus-governance/agx-committee-bft-and-governance.md | FR-0001-0010 | C1 | consensus-spec.md | 1 | complete |
| FR-0010 | consensus-governance/agx-committee-bft-and-governance.md | FR-0001-0010 | C2 | consensus-spec.md, state-sync-spec.md | 2, 1 | complete |
| FR-0011–0020 | consensus-governance/agx-committee-bft-and-governance.md | FR-0011-0020 | C3 | staking-spec.md | 1, 2 | complete |
| FR-0020a | ADR-0015-stake-delegation.md | FR-0011-0020 | C3 | staking-spec.md | 1 | complete (pending code implementation) |
| FR-0021–0030 | consensus-governance/agx-committee-bft-and-governance.md | FR-0021-0030 | C4 | governance-spec.md | 1, 2 | complete |
| FR-0031–0040 | agents/collaboration-layer-parallel-teams.md | FR-0031-0040 | C6 | fastpath-spec.md | 1 | complete |
| FR-0041–0050 | networking/p2p-wire-protocol.md | FR-0041-0050 | C7 | p2p-wire-spec.md | 1, 2 | complete |
| FR-0051–0060 | networking/content-addressed-storage-and-availability.md | FR-0051-0060 | C8 | artifact-availability-spec.md | 1, 2 | complete |
| FR-0061–0075 | agents/infinite-agent.md, agents/agent-tools-spec.md | FR-0061-0075 | C10 | agent-runtime-spec.md | 1, 2, 3, 4 | complete |
| FR-0076–0105 | agents/collaboration-layer-parallel-teams.md | FR-0076-0090, FR-0091-0105 | C11 | collaboration-spec.md | 1, 2, 3 | complete |
| FR-0106–0120 | security/prompt-injection-and-network-policy-boundary.md | FR-0106-0120 | C9 | policy-engine-spec.md | 1, 2, 3 | complete |
| FR-0121–0135 | security/prompt-injection-and-network-policy-boundary.md | FR-0121-0135 | C9 | policy-engine-spec.md | 4 | complete |
| FR-0136–0138 | security/agent-sandbox-and-telemetry.md | FR-0136-0145 | C10 | agent-runtime-spec.md | 4 | complete |
| FR-0139–0141 | security/agent-sandbox-and-telemetry.md | FR-0136-0145 | C1, C2 | telemetry-spec.md | 1, 2 | complete |
| FR-0142–0145 | security/agent-sandbox-and-telemetry.md | FR-0136-0145 | C12, C4 | incident-response-spec.md | 1, 2 | complete |
| FR-0146–0147 | consensus-governance/agx-economics-and-adversarial-incentives.md | FR-0146-0160 | C5 | fee-market-spec.md | 1 | complete |
| FR-0148–0150, 0153 | consensus-governance/agx-economics-and-adversarial-incentives.md | FR-0146-0160 | C12 | review-engine-spec.md | 1 | complete |
| FR-0151 | consensus-governance/agx-economics-and-adversarial-incentives.md | FR-0011-0020 | C3 | staking-spec.md | 1 | complete |
| FR-0152 | networking/p2p-wire-protocol.md | FR-0041-0050 | C7 | p2p-wire-spec.md | 2 | complete |
| FR-0154 | security/agent-sandbox-and-telemetry.md | FR-0136-0145 | C12 | incident-response-spec.md | 1 | complete |
| FR-0155 | consensus-governance/agx-committee-bft-and-governance.md | FR-0021-0030 | C4 | governance-spec.md | 1 | complete |
| FR-0156–0158 | consensus-governance/agx-economics-and-adversarial-incentives.md | FR-0146-0160 | C11 | collaboration-spec.md | 3 | complete |
| FR-0159–0160 | consensus-governance/agx-economics-and-adversarial-incentives.md | FR-0146-0160 | C5 | fee-market-spec.md | 2 | complete |
| FR-0161–0175 | consensus-governance/agx-economics-and-adversarial-incentives.md | FR-0161-0175 | C12 | review-engine-spec.md | 1 | complete |
| FR-0176–0190 | consensus-governance/agx-economics-and-adversarial-incentives.md | FR-0176-0190 | C12, C10 | collaboration-spec.md | 3 | complete |
| FR-0191 | agents/sybil-detection-correlation-engine.md | FR-0176-0190 | C12 | collaboration-spec.md | 3 | complete |
| FR-0192 | consensus-governance/agx-economics-and-adversarial-incentives.md | FR-0176-0190 | C12 | collaboration-spec.md | 3 | complete |
| FR-0193 | agents/agent-telemetry-interface.md | FR-0176-0190 | C10 | agent-runtime-spec.md | 2 | complete |
| FR-0194 | user-task-submission-and-sponsorship.md | FR-0176-0190 | C9, C1 | policy-engine-spec.md, consensus-spec.md | 1, 2 | complete |
| FR-0195 | user-task-submission-and-sponsorship.md | FR-0176-0190 | C9 | policy-engine-spec.md | 1 | complete |
| FR-0196 | user-task-submission-and-sponsorship.md | FR-0176-0190 | C10 | agent-runtime-spec.md | 5 | complete |
| FR-0197 | user-task-submission-and-sponsorship.md | FR-0176-0190 | C7 | p2p-wire-spec.md | 2 | complete |
| FR-0198 | user-task-submission-and-sponsorship.md | FR-0176-0190 | C11 | collaboration-spec.md | 1 | complete |
| FR-0199 | user-task-submission-and-sponsorship.md | FR-0176-0190 | C10 | agent-runtime-spec.md | 3 | complete |
| FR-0200 | user-task-submission-and-sponsorship.md | FR-0176-0190 | C10 | agent-runtime-spec.md | 5 | complete |

## NFR Traceability

NFRs are cross-cutting and apply to multiple components.

| NFR | Research Source | Primary Components | Spec Coverage |
|-----|----------------|-------------------|---------------|
| NFR-0001–0015 | Various | Cross-cutting | Referenced per spec |
| NFR-0016–0030 | Various | Cross-cutting | Referenced per spec |

## ADR Traceability

| ADR | Subject | Spec(s) | Status |
|-----|---------|---------|--------|
| ADR-0001 | 12-Component Architecture | All specs | accepted |
| ADR-0002 | Three-Layer Trust | agent-runtime-spec.md, policy-engine-spec.md | accepted |
| ADR-0003 | PDP Deterministic Rule Chain | policy-engine-spec.md, governance-spec.md | accepted |
| ADR-0004 | Agent Process Separation | agent-runtime-spec.md | accepted |
| ADR-0005 | Content-Addressed SMT | consensus-spec.md, state-sync-spec.md | accepted |
| ADR-0006 | Dual-Lane Economics | fee-market-spec.md, collaboration-spec.md | accepted |
| ADR-0007 | Committee BFT VDF | consensus-spec.md, staking-spec.md, stake-graph-analysis-spec.md | accepted (amended 2026-05-06: 15% cap removed, overlap 33%→20%, VDF fallback hardened) |
| ADR-0008 | Three-Phase Quality | review-engine-spec.md | accepted |
| ADR-0009 | EIP-1559 Fee Market | fee-market-spec.md | accepted |
| ADR-0010 | Four-Stage Trust Ladder | collaboration-spec.md | accepted |
| ADR-0011 | Review Sandbox Isolation | governance-spec.md, fastpath-spec.md | accepted |
| ADR-0012 | Circuit-Breaker Hierarchy | incident-response-spec.md | accepted |
| ADR-0013 | Expanded Agent Tools and Seed Index | agent-runtime-spec.md, collaboration-spec.md | accepted |
| ADR-0014 | User Task Submission and Sponsorship | policy-engine-spec.md, consensus-spec.md, collaboration-spec.md, agent-runtime-spec.md | accepted |
| ADR-0015 | Stake Delegation | staking-spec.md, consensus-spec.md, stake-graph-analysis-spec.md | accepted |
