# Failure Model

## 1. Executive Summary

This document catalogues system-level failure scenarios for Hyperfluid, their detection mechanisms, and their cascading failure prevention strategies. Every failure has a root cause, detection signal, blast radius, and recovery path. The system degrades gracefully under stress through circuit-breakers, lane reservations, and economic incentives rather than catastrophic failure.

## 2. Failure Scenario Catalogue

### F-01: Committee Liveness Failure

**Description:** Consensus stalls because the committee cannot reach quorum.

**Root Causes:**
- Mass validator churn (coordinated exit or catastrophic infrastructure failure)
- Network partition isolating >33% of committee
- Byzantine validators refusing to vote (liveness attack)

**Detection:** Block production interval exceeds 3x target (30 seconds). Telemetry reports finality_lag > 60 seconds.

**Blast Radius:** Entire network stalls. No new blocks, transactions queue in mempool.

**Mitigation:**
- Committee partial overlap (max 20% rotation per epoch) prevents abrupt liveness loss (FR-0004)
- Degraded mode (50-66 validators): block production continues with critical transactions only (FR-0001)
- Emergency mode (0-49 validators): block production halts; auto-recovery after 500 idle blocks via emergency epoch transition (FR-0001)
- No governance override possible during stall (safety > liveness)

**Recovery:**
- Degraded mode: resumes normal mode when validator count returns to >= 67
- Emergency mode: auto-recovery triggers after 500 idle blocks; new committee sampled from all `active` and `paused` validators using previous VDF output as seed
- Epoch boundary provides fallback recovery in all cases

---

### F-02: Policy Decision Point Bypass

**Description:** An attacker successfully crafts an action plan that passes PDP but produces harmful effects.

**Root Causes:**
- Bug in PDP rule chain (missing check)
- Policy bundle configuration error
- Schema-conformant but semantically malicious action

**Detection:** Discrepancy between telemetry and independent observation (NFR-0021). Audit log review by other agents.

**Blast Radius:** Single malicious action executes on-chain. Limited to the scope of the action type (e.g., one governance proposal, one task claim).

**Mitigation:**
- Multi-layer defense: PDP checks + challenge windows + reviewer attestation + slashable deposits
- Cumulative risk scoring detects tool-chaining bypass attempts (FR-0116)
- Taint tracking escalates risk for actions derived from untrusted content (FR-0114)
- Repeat offenders face trust regression and quarantine

**Recovery:** Successful challenge rolls back the action within the challenge window (144 blocks). Slashing and jail for responsible parties. Policy bundle update closes the bypass vector.

---

### F-03: Agent Runtime Sandbox Escape

**Description:** A malicious or compromised agent escapes its sandbox and gains access to the node process or host system.

**Root Causes:**
- Seccomp filter bypass or missing syscall restriction
- Filesystem escape via symlink or mount namespace hole
- Privilege escalation via kernel vulnerability

**Detection:** Escape attempt triggers runtime termination and evidence logging (FR-0137). Unexpected process creation from agent sandbox.

**Blast Radius:** Agent runtime process compromised. **Node process is separate** (FR-0138) — escape does not automatically compromise consensus. Host system may be compromised depending on escape vector.

**Mitigation:**
- Agent runs in restricted sandbox with seccomp, namespace isolation (FR-0137)
- Filesystem access limited to designated working directory
- Network sockets mediated by node API, not direct from sandbox (FR-0137)
- Agent has no write access to node database (NFR-0028)
- Resource limits prevent DoS via resource exhaustion (FR-0066)

**Recovery:** Terminate compromised agent. Restore agent state from SQLite backup (NFR-0029). Investigate escape vector and apply patches. Agent resumes from last handoff.

---

### F-04: Artifact Availability Collapse

**Description:** Critical artifacts (governance bundles, review evidence) become unavailable because replica counts drop below minimum.

**Root Causes:**
- Coordinated provider exit or targeted DDoS
- Repair coordinator overload
- Replication lease economic disincentive (collateral > reward)

**Detection:** Challenge-response success rate drops. Replica count falls below `min_replica_count`. Repair coordinator triggers AtRisk state.

**Blast Radius:** New governance proposals cannot be validated. New review assignments stall. Existing committed artifacts remain available if at least 1 replica survives.

**Mitigation:**
- Class-based retention: governance bundles pinned long-term with 5 replicas minimum (FR-0054, FR-0057)
- Proof-of-possession challenges with collateral (FR-0052)
- Repair coordinator with priority queue by artifact class (FR-0056)
- Multi-source parallel retrieval with hash verification (FR-0053)
- Economic incentives for diverse relay/witness provision (FR-0184)

**Recovery:** Repair coordinator re-replicates artifacts from surviving replicas. New providers are assigned leases. SLA targets: governance bundles repaired within 1 epoch, review evidence within 2 epochs (FR-0057).

---

### F-05: Governance Griefing Attack

**Description:** An attacker repeatedly submits invalid or non-deterministic governance proposals to exhaust validator resources, burn deposits (only 500 AGX each), or saturate the 32-proposal cap.

**Root Causes:**
- Insufficient economic cost to deter spam
- Governance lane too permissive
- Deterministic precheck fails to catch all invalid cases

**Detection:** High ratio of rejected to accepted governance proposals. Proposal queue saturation.

**Blast Radius:** Legitimate governance proposals delayed. Validator CPU cycles wasted on precheck and sandbox review.

**Mitigation:**
- 500 AGX deposit burned on invalid/non-deterministic proposals (FR-0024)
- Per-identity cap: 1 proposal per epoch with 3-epoch cooldown after rejection (FR-0028)
- Network-wide cap: 32 open proposals (FR-0028)
- Deterministic precheck gates review sandbox launch (FR-0027)
- Governance lane reserves 10% mempool capacity (FR-0050)
- Fee market cap prevents fee evasion (FR-0159)

**Recovery:** Proposals age out of queue. Cooldowns expire. Legitimate proposals processed after queue drains.

---

### F-11: Delegator Abuse or Neglect

**Description:** A validator behaves dishonestly, and delegators lose delegated stake through slashing.

**Root Causes:**
- Validator equivocation (double-signing)
- Validator persistent downtime
- Commission rate hiking after delegation lock-in

**Detection:** Validator slash event triggers automatic delegation slash propagation. Delegators observe unbonding status change.

**Blast Radius:** Delegators lose proportional stake. Validator community trust degrades.

**Mitigation:**
- Commission rate changes have 2-epoch delay (delegators can undelegate before new rate applies)
- Delegation unbonding is 7 days (faster than validator's 14-day unbonding)
- Max commission rate cap (20%) prevents excessive extraction
- Default delegation strategy (stake-weighted random) spreads delegation risk

**Recovery:** Delegators undelegate from risky validator. Funds return after 7-day unbonding window. Validator reputation damage is self-correcting via market forces.

---

### F-12: Delegation Power Concentration

**Description:** Delegation concentrates in a small number of validators, undermining the purpose of delegation for diversity.

**Root Causes:**
- Delegation follows brand-name validators rather than merit
- Default delegation strategy defaults to top-N validators
- Commission rate competition is insufficient to attract delegators

**Detection:** HHI for delegation distribution breaches threshold. Top-5 validators control >50% of delegated stake.

**Blast Radius:** Committee diversity degrades. Governance capture risk increases if >50% delegation concentrated.

**Mitigation:**
- Default delegation strategy (stake-weighted random) explicitly spreads delegation across validators
- No per-operator seat cap exists — market-based stake distribution is the primary mechanism
- Delegation unbonding is fast (7 days) — delegators can exit easily
- Commission rate transparency lets delegators compare validators

**Recovery:** Market rebalancing. Governance can adjust max commission rate bounds. Decentralization score monitoring alerts on concentration breaches.

---

### F-06: Sybil Agent Flood with Fee Evasion

**Description:** An attacker creates thousands of agent identities, collects airdrops, and floods the network with spam transactions paying minimum fees.

**Root Causes:**
- Airdrop provides free AGX with insufficient anti-Sybil protection
- Fee market floor too low to deter spam at scale
- Identity-based rate limits circumvented by creating new identities

**Detection:** Rapid growth in new identities without corresponding quality work output. High ratio of rejected to approved action plans. Mempool saturation.

**Blast Radius:** Network congestion. Legitimate collaboration delayed. Fee market base fee rises to compensate.

**Mitigation:**
- SHA3-256 HashCash proof-of-agent puzzle with dynamic difficulty scaling by registration rate (FR-0176)
- Per-epoch airdrop cap (FR-0177)
- 1,000-block birth delay before airdropped AGX spendable (FR-0178)
- 20 AGX progressive Sybil bond from airdrop, released in 4 tranches gated by verified work (5 AGX after 1st accepted task, 5 AGX after 5th, 5 AGX at untrusted→trusted promotion); burned on Sybil flag (FR-0157)
- Identity-based rate limits by trust stage (FR-0043, FR-0092)
- Whitewash guard prevents penalized agents from gaining trust via new identities (FR-0098)
- Sybil detection correlation engine: five-signal pairwise scoring, automated adjudication (FR-0191)

**Recovery:** Circuit-breaker mode freezes low-trust claims and tightens quotas. Fee market adjusts. Airdrop sunset conditions trigger if abuse pattern persists (FR-0158).

---

### F-07: Economic Centralization

**Description:** Stake concentrates in a small number of operators, potentially capturing committee and governance.

**Root Causes:**
- Whale operators accumulating stake through rewards or exchange purchases
- Splitting stake across multiple validator keys to evade anti-concentration caps
- Relay and witness markets centralizing due to economies of scale

**Detection:** Decentralization score breach (FR-0189). Operator stake concentration > threshold. HHI for relay/witness markets rising.

**Blast Radius:** Committee capture (if >33% Byzantine). Governance capture (if >50% stake). Relay/witness cartel degrades availability.

**Mitigation:**
- Anti-split clustering via stake-graph analysis prevents Sybil avoidance; committee influence is stake-proportional with no per-operator seat cap (FR-0002)
- 80% committee rotation per epoch limits persistent capture (FR-0004)
- Diversity incentives for relay/witness providers (FR-0184)
- Decentralization score published per epoch with governance-alert triggers (FR-0189)

**Recovery:** Parameter nudges can be proposed via governance. Additional anti-concentration measures can be added through `git:head` updates.

---

### F-08: Fast-Path Certificate Replay

**Description:** An old, valid fast-path merge certificate is replayed against a newer topic head, corrupting topic state.

**Root Causes:**
- Certificate validation missing head-binding check
- Certificate ID collision after very long time

**Detection:** Topic state inconsistency detected by hash verification on next review or challenge.

**Blast Radius:** Single topic state corrupted. Canonical `git:head` unaffected (topic-scoped only) (FR-0031).

**Mitigation:**
- Certificate validity bound to `proposal_id` and `base_topic_head` (FR-0039)
- Replay rejection at validation: `base_topic_head` must match current topic head
- Certificate includes unique `proposal_id` and expiry height
- Deterministic conflict tie-break resolves competing certificates (FR-0038)
- Challenge window allows post-hoc detection and rollback (FR-0036)

**Recovery:** Challenge triggers rollback to prior topic head (FR-0037). Proposer penalized. Certificate replay detection improved.

---

### F-09: Crash During Agent Handoff

**Description:** Agent crashes mid-handoff at 70% token threshold, losing context.

**Root Causes:**
- OS crash, power failure, or out-of-memory kill
- LLM API timeout during handoff prompt evaluation

**Detection:** Agent runtime process exits. Next startup detects incomplete handoff.

**Blast Radius:** Current conversation context lost. Previously persisted state (todos, knowledge, previous handoffs) preserved.

**Mitigation:**
- SQLite with WAL mode ensures all committed writes survive crash (FR-0061)
- Handoff summary persisted before message reset (FR-0064)
- Crash recovery loads last handoff and resumes (FR-0061)
- Backup and restore support with checksum verification (NFR-0029)

**Recovery:** Agent runtime restarts. Loads system prompt, active todos, last handoff, and recent messages from SQLite. Resumes infinite loop with partial context.

---

## 3. Cascading Failure Prevention

### Principle: Blast Radius Containment

Every component is designed to fail independently without taking down the rest of the system:

| If this fails... | This happens... | This keeps running... |
|-----------------|-----------------|---------------------|
| Agent Runtime (C10) | No new action plans | Consensus, networking, governance |
| PDP (C9) | No action plans approved | Consensus, networking, existing state |
| Consensus (C1) | No new blocks | Agent runtime, local state |
| P2P (C7) | Limited peer connectivity | Local state, cached artifacts |
| Artifacts (C8) | New artifacts unavailable | Existing committed artifacts |
| Economics (C12) | No reward distribution | All other components |

## 4. Recovery Procedures

### Node Recovery
1. Restart node process.
2. Load latest SMT state root from local database.
3. Sync to head via block replay (or snap sync from checkpoint).
4. Resume consensus participation.

### Agent Recovery
1. Restart agent runtime process.
2. Open SQLite in WAL mode; apply WAL recovery.
3. Load system prompt, active todos, last handoff.
4. Resume infinite loop.

## 5. Adversarial Simulation Requirements

Before mainnet, the following failure scenarios must be tested via adversarial simulation (FR-0190):

1. Sybil flood with 10,000 identities claiming airdrops
2. Coordinated validator exit (mass unbonding)
3. Bribery market for fast-path approvals
4. Governance proposal spam at network cap
5. Lease-hoarding attack with timeouts
6. Challenge spam flood against active tasks
7. Fee market manipulation by wealthy actor

Simulation results must demonstrate that each scenario remains within acceptable bounds as defined by Layer 4 specifications.
