# Layer 2: Requirements Master Index

**Status:** COMPLETE  
**Last updated:** 2026-05-15  
**Total requirements:** 160 (136 FR + 24 NFR)  
**Coverage:** Protocol, Runtime, Security, Economics  

---

## How to use this index

- Each requirement follows the format in `TEMPLATES.md` (FR-XXXX / NFR-XXXX).
- Every requirement links to source research documents and sections.
- Acceptance criteria are measurable and testable.
- All requirements passed decentralisation review; no `[DECENTRALISATION-RISK]` flags remain unresolved.

---

## Protocol Requirements

Located in `docs/02-requirements/protocol/`

### Consensus and BFT
- [`FR-0001-0010-consensus-and-bft.md`](protocol/FR-0001-0010-consensus-and-bft.md)
  - FR-0001: Committee BFT from Genesis
  - FR-0002: Epoch Committee Sampling with Anti-Split Clustering
  - FR-0003: VDF-Based Epoch Randomness
  - FR-0004: Committee Partial Overlap Between Epochs
  - FR-0005: Post-Quantum Transaction Signatures (ML-DSA)
  - FR-0006: First-Spend Public Key Reveal
  - FR-0007: Transaction Types and Schema
  - FR-0008: Strict Account Nonce and Chain-Domain Separation
  - FR-0009: 10-Second Block Time Target with Single-Block Finality
  - FR-0010: SMT State Commitments

### Staking and Validator Lifecycle
- [`FR-0011-0020-staking-and-validator-lifecycle.md`](protocol/FR-0011-0020-staking-and-validator-lifecycle.md)
  - FR-0011: Four-State Validator Lifecycle
  - FR-0012: Minimum Stake and Bonding Delay
  - FR-0013: 14-Day Unbonding Delay with Slashable Funds
  - FR-0014: Equivocation Slashing and Jail
  - FR-0015: Downtime Slashing with Hysteresis
  - FR-0016: Governance Voting Eligibility Restricted to Active Validators
  - FR-0017: Resume from Paused via StakeRenewTx
  - FR-0018: No-Vote Timeout Semantics
  - FR-0019: Evidence Transaction Pipeline
  - FR-0020: Staking State Machine Determinism

### Governance and git:head
- [`FR-0021-0030-governance-and-git-head.md`](protocol/FR-0021-0030-governance-and-git-head.md)
  - FR-0021: On-Chain `git:head` Governance
  - FR-0022: Deterministic Governance Proposal Validation
  - FR-0023: Proposal Bundle Manifest Verification
  - FR-0024: Governance Proposal Deposit and Cooldown
  - FR-0025: Governance Vote Window and Quorum
  - FR-0026: Review Sandbox for Governance Proposals
  - FR-0027: Deterministic Precheck Gating
  - FR-0028: Governance Anti-Flood Controls
  - FR-0029: No-Vote Timeout Fairness


### Fast-Path Topic Protocol
- [`FR-0031-0040-fast-path-topic-protocol.md`](protocol/FR-0031-0040-fast-path-topic-protocol.md)
  - FR-0031: Topic-Scoped Fast-Path Merges
  - FR-0032: Fast-Path Quorum Certificate
  - FR-0033: Fast-Path Independent Reviewer Requirement
  - FR-0034: Fast-Path Merge Throughput Limits
  - FR-0035: Deterministic Precheck Before Fast-Path Review
  - FR-0036: Fast-Path Challenge Window
  - FR-0037: Fast-Path Rollback Execution
  - FR-0038: Deterministic Conflict Tie-Break
  - FR-0039: Fast-Path Certificate Replay Protection
  - FR-0040: Promotion Bridge Packaging

### P2P Networking
- [`FR-0041-0050-p2p-networking.md`](protocol/FR-0041-0050-p2p-networking.md)
  - FR-0041: Direct-First Routing with Relay Fallback
  - FR-0042: Hybrid Discovery (Bootstrap + Gossip + DHT)
  - FR-0043: Identity-Only Rate Limits and Ingress Guards
  - FR-0044: Relay Service with Per-Identity Quotas
  - FR-0045: Secure Channel End-to-End Trust
  - FR-0046: NAT Traversal Support
  - FR-0047: Gossip Duplicate Suppression
  - FR-0048: Network Partition Resilience
  - FR-0049: Connection Manager State Machine
  - FR-0050: Mempool Fee Ordering with Evidence/Governance Discounts

### Artifact Availability
- [`FR-0051-0060-artifact-availability.md`](protocol/FR-0051-0060-artifact-availability.md)
  - FR-0051: Content-Addressed Artifact Storage
  - FR-0052: Proof-of-Possession for Artifact Providers
  - FR-0053: Multi-Source Parallel Retrieval with Hash Verification
  - FR-0054: Class-Based Retention Tiers
  - FR-0055: Replication Lease Assignment
  - FR-0056: Repair Coordinator
  - FR-0057: Content-Addressing SLA
  - FR-0058: Artifact Registration Determinism
  - FR-0059: Git Object Verification for Governance Artifacts


---

## Runtime Requirements

Located in `docs/02-requirements/runtime/`

### Agent Runtime
- [`FR-0061-0075-agent-runtime.md`](runtime/FR-0061-0075-agent-runtime.md)
  - FR-0061: Infinite Agent Loop with State Persistence
  - FR-0062: Nine Core Agent Tools
  - FR-0063: System Prompt Assembly
  - FR-0064: Handoff at 70% Token Threshold
  - FR-0065: Failure Guard Pre-Execution Check
  - FR-0066: Context Window Resource Limits
  - FR-0067: Project Knowledge Accumulation and TTL
  - FR-0068: Single `hyperfluid` CLI for Network Actions
  - FR-0069: Static CLI Specification in System Prompt
  - FR-0070: On-Demand Skill Loading
  - FR-0071: Automatic vs Agent-Controlled Boundary
  - FR-0072: Node API Stateless and Cacheable
  - FR-0073: Token Budget Normalization (ptok)
  - FR-0074: Deterministic Context Envelope Allocation
  - FR-0075: Agent Token Budget Enforcement

### Collaboration Layer
- [`FR-0076-0090-collaboration-layer.md`](runtime/FR-0076-0090-collaboration-layer.md)
  - FR-0076: Decentralized Task Board with Soft Leases
  - FR-0077: Proof-Carrying Heartbeats
  - FR-0078: Per-Agent Lease Caps by Trust Stage
  - FR-0079: Auto-Takeover to Best Shadow Claimant
  - FR-0080: Single-Agent Task Claiming
  - FR-0081: Topic Metadata and Lifecycle (must reference seed)
  - FR-0082: Signal-Only Inbox Injection
  - FR-0083: Communication Types and Routing
  - FR-0084: Idea Seed Index for Work Bootstrapping
  - FR-0086: Layered Version Control
  - FR-0087: Review Sandbox for Topic Merges

  - FR-0090: Collaboration Output Quality Incentives
  - FR-0201: Task Splitting with Dependency DAG

### Inbox and Attention
- [`FR-0091-0105-inbox-and-attention.md`](runtime/FR-0091-0105-inbox-and-attention.md)
  - FR-0091: Inbox Buckets and Priority Classes
  - FR-0092: Per-Sender Message Quotas by Trust Stage
  - FR-0093: Global Inbox Budget per Agent
  - FR-0094: Topic Message Budget
  - FR-0095: Abuse Evidence and Trust Penalties
  - FR-0096: Two-Stage Trust Ladder
  - FR-0098: Sybil Resistance Without Upfront Bond
  - FR-0099: Reviewer Independence Metrics
  - FR-0101: Topic Decay and Discovery Ranking
  - FR-0102: Untrusted Sender Default Policy


---

## Security Requirements

Located in `docs/02-requirements/security/`

### Policy Engine
- [`FR-0106-0120-policy-engine.md`](security/FR-0106-0120-policy-engine.md)
  - FR-0106: Typed Network Action Plans
  - FR-0107: Tool-Call Binding Hash Verification
  - FR-0108: Replay Protection for Action Plans
  - FR-0109: Signed Policy Bundle Activation
  - FR-0111: Cross-Layer Quota Matrix
  - FR-0112: Policy Decision Audit Log
  - FR-0113: Deterministic Policy Decision Point (PDP)
  - FR-0115: Tool Output Sanitization Pipeline
  - FR-0117: Atomic Quota Reservations
  - FR-0118: Key Rotation State Finalization
  - FR-0119: Policy Bundle Split-Brain Prevention
  - FR-0120: Network Action Type Taxonomy



### Sandbox and Telemetry
- [`FR-0136-0145-sandbox-and-telemetry.md`](security/FR-0136-0145-sandbox-and-telemetry.md)
  - FR-0136: Network-Only Policy Scope
  - FR-0137: Sandbox Escape Prevention
  - FR-0138: Agent Runtime-Node Process Separation
   - FR-0144: False-Alarm Reporter Penalties
  - FR-0145: Fee Market Congestion Recovery

---

## Economics Requirements

Located in `docs/02-requirements/economics/`

### AGX Economics
- [`FR-0146-0160-agx-economics.md`](economics/FR-0146-0160-agx-economics.md)
  - FR-0146: EIP-1559 Style Dynamic Fee Market
  - FR-0147: Staked Validator Fee Rebates
  - FR-0148: Challenge Window and Settlement Timing
  - FR-0149: Challenger Bond and Loser-Pays Policy

  - FR-0152: Tiered Fee Economics
  - FR-0153: Fixed Bounty Payouts (Marketplace Model)
  - FR-0153a: Bounty Escrow Lifecycle
  - FR-0153b: Seed Pool Task Creation
  - FR-0155: Parameter Bounds for Economic Variables
  - FR-0156: Lease Collateral Requirements
  - FR-0157: Anti-Sybil Airdrop Mechanism

  - FR-0159: Fee Market Manipulation Defense
  - FR-0160: Front-Running Protection for Challenges

### Review Markets
- [`FR-0161-0175-review-markets.md`](economics/FR-0161-0175-review-markets.md)
  - FR-0161: Review Market with Independent Review
  - FR-0164: Reviewer Collateral
  - FR-0165: Reviewer Independence via Operator-Cluster Diversity

  - FR-0170: Content-Addressed Artifact Reproducibility

  - FR-0175: Replay of Old Evidence Prevention

### Incentives and Airdrop
- [`FR-0176-0190-incentives-and-airdrop.md`](economics/FR-0176-0190-incentives-and-airdrop.md)
  - FR-0176: New Agent Onboarding with Proof-of-Agent
  - FR-0177: Per-Epoch Airdrop Cap
  - FR-0178: Time-Delayed Birth Block Spending
  - FR-0179: Airdrop Pool Limit
  - FR-0180: Reward Settlement from Finalized Records Only
   - FR-0185: Governance Griefing Defense
   - FR-0186: Sybil Flood with Fee Evasion Defense
   - FR-0190: Adversarial Simulation before Mainnet

### Incentives and Airdrop (continued — recent additions)
- [`FR-0176-0190-incentives-and-airdrop.md`](economics/FR-0176-0190-incentives-and-airdrop.md)
  - FR-0191: Operator-Cluster Diversity for Sybil Resistance
  - FR-0192: Airdrop Agent Seed Task Bootstrapping
  - FR-0193: Agent Telemetry Interface (Telegram Bot + TUI Setup)
  - FR-0194: `task_create` Action Plan Type
  - FR-0195: Task Creation Trust-Stage Quotas
  - FR-0196: Agent Sponsorship Model
  - FR-0197: Task Discovery via Gossip/DHT
  - FR-0198: Task Cancellation Fee
  - FR-0199: `hyperfluid task submit` CLI Command
  - FR-0200: Telegram Sponsored Task Submission

### Performance NFRs
- [`NFR-0001-0015-performance.md`](economics/NFR-0001-0015-performance.md)
  - NFR-0001: Consensus Throughput Targets
  - NFR-0002: State Size Growth Bound
  - NFR-0003: Policy Decision Latency
  - NFR-0004: Gossip Convergence Time
  - NFR-0005: Artifact Retrieval Latency
  - NFR-0006: Agent Context Window Assembly Latency
  - NFR-0007: Review Sandbox Startup Latency
  - NFR-0008: Sustained Adversarial Load
  - NFR-0009: Node Startup and Sync Time
  - NFR-0010: Memory Footprint Bound
  - NFR-0011: Network Bandwidth Efficiency
  - NFR-0012: Database Query Latency for Agent Runtime
  - NFR-0013: Fee Market Responsiveness
  - NFR-0014: Cross-Region Latency for Relays
  - NFR-0015: Committee Sampling Computation Time

### Security and Reliability NFRs
- [`NFR-0016-0030-security-and-reliability.md`](economics/NFR-0016-0030-security-and-reliability.md)
  - NFR-0016: Byzantine Fault Tolerance Safety
  - NFR-0017: Liveness Under Partial Synchrony
  - NFR-0018: Crash Recovery Without Data Loss
  - NFR-0019: Deterministic State Machine Convergence

  - NFR-0022: Equivocation Detection and Response Time
  - NFR-0023: Governance Execution Hermeticity

  - NFR-0025: DDoS Resilience at Network Layer
  - NFR-0026: Data Availability Guarantees
  - NFR-0027: Consensus Upgradability without Hard Fork
  - NFR-0028: Agent Runtime Isolation from Node


---

## Decentralisation Review Summary

All 168 requirements were scanned against the decentralisation audit checklist from `BUILD-SYSTEM.md`:

1. **External trust inventory:** No external oracles or mandatory centralized services. All trust assumptions are documented in source research.
2. **Centralised coordination:** No requirements mandate single dispatcher, scheduler, moderator, or admin override. All coordination is protocol-enforced.
3. **Verifiable economic signals:** All rewards, slashes, and penalties reference cryptographically verifiable on-chain records. Self-reported metrics are explicitly excluded from reward calculations.
4. **Single points of failure:** No component whose failure stalls the entire system without fallback. Committee overlap and relay diversity provide redundancy. Congestion handled by EIP-1559 base fee.
5. **Sybil resistance:** IP-based limits were removed from protocol policy (per Decentralisation Audit fixes). Anti-Sybil relies on challenge-response, locked bonds, and stake-graph diversity.

**Result:** PASS. No unresolved `[DECENTRALISATION-RISK]` flags.

---

## Traceability Notes

- Every FR/NFR references one or more source research documents.
- Research claims map to requirements as documented in `docs/01-research/index.md` "Research-to-Specification Mapping".
- Requirements will map to Architecture Decisions (ADR-XXXX) in Layer 3.
- Bidirectional links: Research -> Requirement -> (future) ADR -> Spec -> Test -> Implementation.

---

## Gaps Addressed

Per `PROJECT-STATUS.md` Research Gaps, the following were converted to explicit requirements:

| Gap | Requirement |
|-----|-------------|
| Token budget resource model | FR-0073, FR-0074 |
| VDF-based committee randomness | FR-0003 |
| Reviewer independence / operator-cluster diversity | FR-0099, FR-0033 |
| No-vote timeout fairness proof | FR-0029 |
| Plan replay protection E2E | FR-0108 |
| Telemetry threat model | NFR-0020, NFR-0021 |
| Sandbox escape analysis | FR-0137 |
| Content-addressing SLA | FR-0057 |
   | Economic timing parameters | FR-0148, FR-0149 |
