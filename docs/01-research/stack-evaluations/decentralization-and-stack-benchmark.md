# 1. Title
- Hyperfluid Decentralization Benchmark: Library Choices, Committee BFT Readiness, and Comparative L1 Design Analysis

# 2. Executive Summary
- Hyperfluid can be as decentralized as leading PoS systems only if committee BFT, anti-concentration controls, and data-availability incentives are first-class from genesis.
- Ockam, Malachite, ML-DSA, SMT, and gix remain coherent choices, but the integration contracts around them matter more than the libraries alone.
- Malachite is promising but still alpha; this raises operational risk that must be offset with strict versioning, simulation, and rollback procedures.
- Post-quantum signatures improve long-horizon security posture but increase per-tx bandwidth and verification cost; batching and admission controls are mandatory.
- Sparse state commitments enable lightweight nodes, but decentralization fails if witness/history serving centralizes into a few operators.
- Hyperfluid’s `git:head` governance is novel and powerful, but deterministic execution must be enforced harder than most chains enforce contract determinism.
- Compared to common L1 patterns (Ethereum, Cosmos-family, Solana-family), Hyperfluid’s differentiation is protocol-governed code evolution plus agent-native economics; its risk is governance execution becoming consensus-critical.
- Recommended path: committee BFT day 1, slashing-enabled staking, adaptive anti-spam, hermetic governance, and decentralization SLOs.

# 3. System Overview
- Objective:
  - Evaluate whether the selected stack can deliver a chain that is lightweight and genuinely decentralized at high adoption.
  - Compare against proven L1 decentralization patterns to identify required hardening.
- Evaluation constraints:
  - Must scale from small launch network to very large participation without consensus redesign.
  - Must resist sybil, spam, and governance-capture attacks.
  - Must keep hardware and ops burden low enough for broad participation.

# 4. Architecture (CRITICAL SECTION)
- Evaluation model:
  - Each library maps to one protocol trust boundary.
  - Each boundary requires explicit decentralization controls.

```mermaid
flowchart TD
    O["Ockam<br/>Network trust boundary"]
    M["Malachite<br/>Ordering trust boundary"]
    D["ML-DSA<br/>Identity and signature boundary"]
    S["SMT<br/>State commitment boundary"]
    G["gix<br/>Governance execution boundary"]
    C["Committee Selection and Staking"]
    R["Decentralization Controls<br/>caps quotas incentives evidence"]

    O --> R
    M --> C --> R
    D --> R
    S --> R
    G --> R
```

- Component responsibilities and controls:
  - Ockam: secure routing; requires relay diversity and anti-eclipse peer rules.
  - Malachite: BFT finality; requires committee rotation and version-gated rollout.
  - ML-DSA: identity integrity; requires domain-separated signatures and DoS-resistant admission.
  - SMT: compact state; requires witness market and proof availability commitments.
  - gix: deterministic governance execution; requires hermetic and reproducible merge runtime.
  - Artifact availability implementation details (manifests, proofs-of-possession, repair) are defined in `research/networking/artifact-availability-and-retention.md`.
  - Network action policy validation details are defined in `research/agents/network-policy-engine-spec.md`.

# 5. Core Mechanisms
- **Comparative decentralization patterns from major chains**
  - Ethereum-style PoS: broad validator set with economic penalties and delayed exits.
  - Cosmos-family Tendermint chains: BFT finality with bounded validator set and governance specialization.
  - Solana-family high-throughput designs: performance-optimized but often hardware/network concentration risk.
  - Hyperfluid target: committee BFT with low hardware footprint plus deterministic protocol-level git governance.

- **Where Hyperfluid can be better**
  - Day-1 committee BFT avoids painful migration from full-set consensus.
  - On-chain `git:head` with deterministic sandbox creates a clearer upgrade authority trail than social-layer repo control.
  - Agent-native transaction design can optimize for autonomous workload profiles.
  
- **Production targets**
  - Consensus throughput: `100 tx/s` sustained, `500 tx/s` burst (with mempool buffering)
  - Block time: `10 seconds` target
  - Finality: `instant` (BFT single-block finality)
  - Committee size: `100 validators` (allows 33 Byzantine tolerance)
  - State size growth: `< 1GB per month` with pruning
  - This is designed as production-grade from genesis. No "testnet phase" - all components are built to production standards.

- **Where Hyperfluid can be worse unless fixed**
  - Fee-market economics with EIP-1559 style dynamic pricing for spam prevention and efficient resource allocation.
  - Stateless commitment model can centralize proof availability unless economically incentivized.
  - Committee sampling can centralize under stake splitting without anti-concentration controls.
  - Pseudo-agent swarms can overload collaboration and networking layers unless ingress and attention are hard-budgeted.

- **Cross-layer swarm-hardening changes (required)**
  - Consensus layer:
    - bounded mempool lanes with reserved evidence/control capacity.
    - automatic circuit-breaker under reject-depth/finality stress.
  - Networking layer:
    - staged ingress controls (IP/ASN -> identity -> gossip/topic budgets).
    - relay service classes and unknown-sender caps.
  - Collaboration layer:
    - proof-carrying soft leases with shadow takeover.
    - onboarding privilege ladder with strict bootstrap quotas.
  - Attention layer:
    - signal-only prompt injection.
    - digest-first routing for low-trust senders.
    - quarantine for abuse bursts.
  - Injection-defense boundary:
    - typed network action plans only for network mutations.
    - minimal deterministic network policy gate (no local policy scope).

```mermaid
flowchart TD
    Submit["Tx submit"]
    Admission["Admission controls<br/>pow quotas signature replay"]
    Committee["Epoch committee BFT"]
    Execute["State execution"]
    Commit["SMT root commit"]
    Witness["Witness publish"]
    Governance["Deterministic git governance"]

    Submit --> Admission --> Committee --> Execute --> Commit --> Witness
    Execute --> Governance --> Commit
```

## Pseudocode (for complex mechanisms)
```text
function decentralization_score(epoch_metrics):
    committee_entropy = shannon_entropy(epoch_metrics.operator_distribution)
    relay_hhi = herfindahl_index(epoch_metrics.relay_share)
    witness_hhi = herfindahl_index(epoch_metrics.witness_share)
    stake_capture = epoch_metrics.top_10_operator_stake
    return weighted_score(committee_entropy, 1-relay_hhi, 1-witness_hhi, 1-stake_capture)

function committee_entry_rules(candidate):
    require candidate.stake >= min_stake
    require candidate.not_jailed
    require candidate.uptime_score >= min_uptime
    require operator_effective_weight(candidate.operator) <= operator_cap
    return ELIGIBLE

function swarm_readiness_gate(metrics):
    require metrics.mempool_reject_ratio <= reject_budget
    require metrics.finality_p95 <= finality_slo
    require metrics.unknown_sender_share <= unknown_sender_budget
    require metrics.relay_queue_utilization <= relay_slo
    require metrics.inbox_drop_rate <= inbox_slo
    return READY
```

# 6. Design Decisions & Tradeoffs
## Tradeoff 1
- Option A: Full-set BFT launch and future committee migration.
- Option B: Committee BFT from genesis.
- Chosen: Option B.
- Why chosen: avoids redesign risk and keeps communication overhead bounded from day 1.
- Sacrifice: more complexity in validator sampling and fairness logic.
- Scaling risk: poor randomness/caps can still produce committee centralization.

## Tradeoff 2
- Option A: Simple lock-only staking.
- Option B: slashing + unbonding + liveness states.
- Chosen: Option B.
- Why chosen: closer to proven economic-security patterns in mature PoS systems.
- Sacrifice: higher complexity and slower capital mobility.
- Scaling risk: too-harsh penalties can reduce validator diversity.

## Tradeoff 3
- Option A: Fixed fee schedule.
- Option B: EIP-1559 style dynamic fee market.
- Chosen: Option B.
- Why chosen: proven spam prevention, predictable block sizes, efficient price discovery, deflationary pressure via fee burn.
- Sacrifice: fee volatility, users must hold AGX.
- Scaling risk: high fees may limit micro-transaction use cases.

## Tradeoff 4
- Option A: off-chain/social code governance.
- Option B: on-chain deterministic `git:head` governance.
- Chosen: Option B.
- Why chosen: explicit and auditable protocol authority.
- Sacrifice: governance pipeline becomes consensus-adjacent critical path.
- Scaling risk: if deterministic checks are brittle, governance can stall or split nodes.

# 7. Failure Modes & Edge Cases
## Scenario: Economic centralization over epochs
- What happens: same operators dominate committees repeatedly.
- Why it happens: weak operator caps or stake-splitting loopholes.
- Handling/failure mode: operator identity aggregation, effective stake caps, and diversity-aware sampling weights.

## Scenario: Relay market concentration
- What happens: network routing depends on a small relay subset.
- Why it happens: convenience defaults and no relay incentives for long tail.
- Handling/failure mode: relay rewards tied to diversity, route randomization, and client-side relay set requirements.

## Scenario: Witness/proof availability cartel
- What happens: few archive providers control state proof access.
- Why it happens: no explicit witness serving incentives.
- Handling/failure mode: witness-serving rewards, proof availability SLAs, and penalties for missing commitments.

## Scenario: Committee-targeted adaptive adversary
- What happens: attacker focuses on selected committee members after reveal.
- Why it happens: predictable committee exposure and networking metadata leakage.
- Handling/failure mode: delayed committee reveal windows, encrypted committee coordination channels, and rapid fallback proposer rotation.

## Scenario: Governance execution split
- What happens: nodes disagree on `git:head` proposal validity.
- Why it happens: non-hermetic environments or inconsistent object inputs.
- Handling/failure mode: pinned runtime hash, sealed proposal bundles, and reject-by-default on mismatch.

## Scenario: Pseudo-agent swarm saturation
- What happens: large malicious sender population saturates inboxes, relays, and mempools.
- Why it happens: open participation without strict staged budgets.
- Handling/failure mode: cross-layer budget enforcement, quarantine automation, and circuit-breaker mode spanning consensus/network/collaboration.

# 8. Scalability Analysis
## Small scale (10–100 nodes)
- Committee system can run with larger overlap for simplicity.
- Decentralization score is sensitive to operator concentration from early participants.
- Priority is policy correctness over raw throughput.

## Medium scale (1k–10k nodes)
- Committee and witness markets become dominant decentralization levers.
- Admission and verification paths need horizontal scaling.
- Governance proposal execution must be resource-isolated from block production.

## Large scale (100k+ nodes)
- Role specialization is required:
  - edge submitters,
  - committee validators,
  - relay providers,
  - witness/archive providers.
- Decentralization must be measured continuously, not assumed.
- Primary failure risk shifts from pure consensus to market concentration in supporting services.

# 9. Recommended Architecture
- Keep the selected library set, but enforce these launch requirements:
  - committee BFT from day 1 with anti-concentration sampling,
  - slashing-enabled staking with meaningful unbonding delay, `paused` state, and 1-epoch resume delay,
  - governance voting restricted to `active` validators at snapshot,
  - EIP-1559 style dynamic fee market with base fee adjustment and fee burn,
  - hermetic deterministic governance execution,
  - incentivized relay and witness diversity.
- Rejected alternatives:
  - deferring committee design,
  - lock-only staking without slashing,
  - governance execution without reproducibility guarantees.
- Decentralization benchmark targets:
  - committee operator concentration ceiling per epoch,
  - relay and witness provider HHI thresholds,
  - max top-10 stake influence threshold.

# 10. Implementation Plan
1. Define measurable decentralization SLOs and governance safety invariants.
2. Implement committee sampling with operator cap logic and auditable randomness.
3. Implement staking economics (unbonding, slashing, paused/active transitions, resume delay).
4. Implement EIP-1559 style fee market with base fee adjustment, priority fees, and fee burn mechanism.
5. Implement hermetic governance execution package format and runtime hash pinning.
6. Build relay and witness incentive modules and diversity telemetry.
7. Run adversarial drills against committee capture, service concentration, and governance divergence.
8. Add cross-layer swarm harness with 10x malicious sender simulations and SLO gating.

# 11. Future Improvements
- Add stronger distributed randomness beacons for committee selection.
- Add privacy-preserving anti-sybil identity attestation for operator aggregation.
- Add threshold PQ signature pathways as implementations mature.
- Add formal game-theoretic analysis for stake-splitting and cartel behaviors.
- Add public decentralization transparency reports per epoch.

