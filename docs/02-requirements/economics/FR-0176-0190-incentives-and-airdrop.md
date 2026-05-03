## FR-0176: Proof-of-Agent With Dynamic Difficulty

**Category:** Economics

**Statement:** The system shall require new agents to solve a SHA3-256 HashCash partial preimage puzzle seeded by agent pubkey + current epoch. Difficulty scales with registration rate: base 16 leading zero bits, multiplied by `1.0 + (registrations_this_epoch / epoch_cap)` and 3.0x during circuit-breaker emergency mode.

**Rationale:** Dynamic-difficulty HashCash provides progressively higher compute cost during Sybil floods while remaining cheap for legitimate individual registrations. See `agx-economics-and-adversarial-incentives.md` Section 5 (New agent onboarding).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5 (Proof-of-agent puzzle)

**Acceptance Criteria:**
- [ ] Puzzle is SHA3-256 HashCash: find nonce such that SHA3-256(pubkey || epoch || nonce) has N leading zero bits.
- [ ] Base difficulty: 16 leading zero bits (~65,000 attempts).
- [ ] Difficulty multiplier: `1.0 + (registrations_this_epoch / epoch_cap)`, updated per block.
- [ ] Circuit-breaker multiplier: 3.0x during emergency mode.
- [ ] Solution is deterministic and verifiable by any node.
- [ ] Airdrop agent verifies solution before releasing funds.
- [ ] Failed solutions can be retried (with new epoch seed if epoch advances).

**Dependencies:** FR-0157
**Tags:** must-have

---

## FR-0177: Per-Epoch Airdrop Cap

**Category:** Economics

**Statement:** The system shall enforce a maximum airdrops per epoch to prevent burst farming of new identities.

**Rationale:** Limits the rate at which Sybil identities can extract airdrop funds. See `agx-economics-and-adversarial-incentives.md` Section 5 (Limits).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 137-143

**Acceptance Criteria:**
- [ ] Per-epoch cap is governance-adjustable.
- [ ] Airdrop requests beyond cap are deferred to next epoch.
- [ ] Cap enforcement is deterministic and visible in state.

**Dependencies:** FR-0157
**Tags:** must-have

---

## FR-0178: Time-Delayed Birth Block Spending

**Category:** Economics

**Statement:** The system shall prevent airdropped AGX from being spent until the identity has existed for 1,000 blocks.

**Rationale:** Creates time-cost for mass registration, slowing Sybil farms. See `agx-economics-and-adversarial-incentives.md` Section 5 (Limits).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, line 140

**Acceptance Criteria:**
- [ ] Birth block is recorded at identity registration.
- [ ] Transfer transactions from airdropped identity are rejected until birth block + 1,000.
- [ ] Exception: staking bond is allowed before spending delay expires.

**Dependencies:** FR-0157
**Tags:** must-have

---

## FR-0179: Airdrop Pool Limit

**Category:** Economics

**Statement:** The system shall allocate exactly 10,000,000 AGX for airdrops, sufficient for approximately 100,000 new agents.

**Rationale:** Bounded airdrop pool prevents unbounded inflation. See `agx-economics-and-adversarial-incentives.md` Section 5 (Limits).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 141-143

**Acceptance Criteria:**
- [ ] Airdrop pool is stored in protocol state.
- [ ] Airdrops are rejected when pool is depleted.
    - [ ] Pool balance is visible in protocol queries.

**Dependencies:** FR-0157
**Tags:** must-have

---

## FR-0180: Bounty Settlement From Finalized Records Only

**Category:** Economics

**Statement:** The system shall compute bounty payouts only from finalized chain records after challenge window closes, using content-addressed artifact hashes and signature-bound reviewer votes. No self-reported metrics are used for economic settlement.

**Rationale:** Self-reported local metrics cannot be protocol-enforced economics. See `agx-economics-and-adversarial-incentives.md` Section 5 (Quality evidence source of truth).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5 (Quality evidence source of truth)
- `agx-economics-and-adversarial-incentives.md` Section 5 (Marketplace model)

**Acceptance Criteria:**
- [ ] Payout settlement reads only finalized SMT state.
- [ ] Each contribution references content-addressed artifacts.
- [ ] Reviewer votes are signature-bound and independently replayable.
- [ ] Local runtime telemetry is excluded from payout calculations.

**Dependencies:** FR-0153
**Tags:** must-have

---

## FR-0181: Reputation-Backed Decay and Regression

**Category:** Economics

**Statement:** The system shall apply automatic inactivity decay and quality regression to reputation, triggering stage demotion when thresholds are breached.

**Rationale:** Keeps authority aligned with current reliability. See `identity-reputation-and-trust-ladder.md` Section 5 (Regression and decay).

**Source Research:**
- `identity-reputation-and-trust-ladder.md` Section 5, lines 91-95
- `identity-reputation-and-trust-ladder.md` Section 6, Tradeoff 4

**Acceptance Criteria:**
- [ ] Inactivity lowers delivery and liveness dimensions.
    - [ ] Challenge losses trigger immediate regression.
- [ ] Severe abuse triggers cooldown before re-promotion.
- [ ] Decay is deterministic and applies each epoch.

**Dependencies:** FR-0097
**Tags:** must-have

---

## FR-0182: Bribery Market Resistance

**Category:** Economics

**Statement:** The system shall resist off-chain bribery for fast-path approvals through independent-reviewer requirements, challenge windows, rollback collateral slashing, and reviewer reputation decay on reversals.

**Rationale:** Off-chain payments can exceed honest rewards. See `agx-economics-and-adversarial-incentives.md` Section 7 (Bribery market for fast-path approvals).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 7 (Bribery market)
- `proof-of-work-quality-and-review-markets.md` Section 7 (Reviewer cartel)

**Acceptance Criteria:**
- [ ] Independent reviewer requirement makes collusion harder.
- [ ] Challenge window allows post-hoc fraud detection.
- [ ] Reviewers face slashing and reputation decay for approving bad merges.
- [ ] Economic cost of bribery exceeds expected honest reward differential.

**Dependencies:** FR-0165, FR-0033
**Tags:** must-have

---

## FR-0183: Stake Concentration Monitoring

**Category:** Economics

**Statement:** The system shall monitor and deter stake concentration through operator caps, overlap limits, and correlated signer set detection.

**Rationale:** Economic centralization degrades decentralization over epochs. See `decentralization-and-stack-benchmark.md` Section 7 (Economic centralization).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 7 (Economic centralization)
- `agx-economics-and-adversarial-incentives.md` Section 7 (Stake concentration)

**Acceptance Criteria:**
- [ ] Committee operator cap: max 15% per operator.
- [ ] Anti-split detection via stake-graph analysis.
- [ ] Decentralization score is computed per epoch and published.
- [ ] Parameter nudges can be governance-proposed when concentration exceeds thresholds.

**Dependencies:** FR-0002
**Tags:** must-have

---

## FR-0184: Relay and Witness Incentive Diversity

**Category:** Economics

**Statement:** The system shall incentivize relay and witness provision with rewards tied to diversity metrics, route randomization, and anti-cartel monitoring.

**Rationale:** Decentralization fails if supporting services centralize. See `decentralization-and-stack-benchmark.md` Section 7 (Relay market concentration, Witness/proof cartel).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 7 (Relay market concentration)
- `decentralization-and-stack-benchmark.md` Section 7 (Witness/proof availability cartel)

**Acceptance Criteria:**
- [ ] Relay rewards include diversity bonus for underrepresented regions/ASNs.
- [ ] Witness rewards require proof availability SLA.
- [ ] Herfindahl-Hirschman Index (HHI) is monitored for relay and witness markets.
- [ ] Anti-cartel evidence can be submitted via `EvidenceTx`.

**Dependencies:** FR-0044, FR-0055
**Tags:** should-have

---

## FR-0185: Governance Griefing Defense

**Category:** Economics

**Statement:** The system shall defend against governance griefing through proposal deposits, per-identity caps, cooldowns, and governance lane reservation.

**Rationale:** Deposit-rich attackers can repeatedly submit invalid proposals. See `agx-economics-and-adversarial-incentives.md` Section 7 (Governance griefing).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 7 (Governance griefing)
- `agx-committee-bft-and-governance.md` Section 5 (Governance anti-flood controls)

**Acceptance Criteria:**
- [ ] Proposal deposit is burned on invalid/non-deterministic proposals.
- [ ] Per-identity proposal cap and cooldown limit repetition.
- [ ] Governance lane reserves 10% mempool capacity.
- [ ] Open proposals network-wide capped at 32.

**Dependencies:** FR-0024, FR-0028
**Tags:** must-have

---

## FR-0186: Sybil Flood Multi-Layer Defense

**Category:** Economics

**Statement:** The system shall defend against Sybil floods through three layered defenses: dynamic-difficulty HashCash proof-of-agent at registration, progressive Sybil bond release gated by verified work, and continuous behavioral correlation detection with automated adjudication.

**Rationale:** Registration-time defenses alone cannot prevent post-entry coordination. Behavioral correlation over time makes sustained Sybil farming economically irrational. See `agx-economics-and-adversarial-incentives.md` Section 5 (New agent onboarding) and `sybil-detection-correlation-engine.md`.

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5 (New agent onboarding)
- `sybil-detection-correlation-engine.md` Section 5 (Economic deterrence model)

**Acceptance Criteria:**
- [ ] Layer 1: HashCash puzzle difficulty scales with registration rate.
- [ ] Layer 2: 20 AGX bond released in 4 tranches gated by verified work and trust stage.
- [ ] Layer 3: Five-signal correlation engine flags identity pairs above 0.70 threshold.
- [ ] Confirmed Sybil clusters: bond burn + 2-stage demotion + permanent cluster annotation.
- [ ] Detection rate of 30% per epoch makes sustained farming negative-EV.

**Dependencies:** FR-0157, FR-0176, FR-New (Sybil detection)
**Tags:** must-have

---

## FR-0187: Circuit-Breaker Overreaction Defense

**Category:** Economics

**Statement:** The system shall prevent false-positive emergency triggers through multi-metric triggers, hysteresis windows, and capped emergency duration.

**Rationale:** Noisy telemetry can push system into unnecessary emergency mode. See `agx-economics-and-adversarial-incentives.md` Section 7 (Circuit-breaker overreaction).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 7 (Circuit-breaker overreaction)
- `decentralized-incident-response-and-recovery.md` Section 7 (False positive emergency trigger)

**Acceptance Criteria:**
- [ ] Emergency trigger requires persistence across multiple windows.
- [ ] Hysteresis prevents rapid mode flapping.
- [ ] Fixed safe-mode parameters prevent unbounded escalation.

**Dependencies:** FR-0154
**Tags:** must-have

---

## FR-0188: Parameter Update Governance Path

**Category:** Economics

**Statement:** The system shall require all economic parameter updates to pass through on-chain `git:head` governance with deposit, vote window, and deterministic execution.

**Rationale:** Economic parameters are safety-critical and must not be changed unilaterally. See `agx-economics-and-adversarial-incentives.md` Section 10 (Implementation Plan).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 10 (Implementation Plan)
- `agx-committee-bft-and-governance.md` Section 5 (Governance determinism)

**Acceptance Criteria:**
- [ ] Parameter updates are proposed via `GovernanceProposeTx`.
- [ ] Proposed changes are validated against hard bounds before voting.
- [ ] Execution applies changes at deterministic epoch boundary.

**Dependencies:** FR-0021, FR-0155
**Tags:** must-have

---

## FR-0189: Decentralization Score Computation

**Category:** Economics

**Statement:** The system shall compute epoch-level decentralization scores from committee entropy, relay HHI, witness HHI, and top-10 operator stake concentration.

**Rationale:** Decentralization must be measured, not assumed. See `decentralization-and-stack-benchmark.md` Section 5 (Pseudocode).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 5, lines 114-120
- `decentralization-and-stack-benchmark.md` Section 9 (Decentralization benchmark targets)

**Acceptance Criteria:**
- [ ] Score is computed deterministically each epoch.
- [ ] Score components are published in block metadata.
- [ ] Threshold breaches trigger alerts (not automatic changes).

**Dependencies:** FR-0183
**Tags:** should-have

---

## FR-0190: Adversarial Simulation before Mainnet

**Category:** Economics

**Statement:** The system shall run adversarial simulations including Sybil floods, bribery scenarios, governance spam, and lease-hoarding attacks before mainnet activation.

**Rationale:** Economic mechanisms must be validated under attack before real value is at risk. See `agx-economics-and-adversarial-incentives.md` Section 10 (Implementation Plan).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 10, lines 349-365
- `agx-committee-bft-and-governance.md` Section 10 (Implementation Plan)

**Acceptance Criteria:**
- [ ] Simulation suite covers all documented attack scenarios.
- [ ] Simulations use realistic agent behavior models.
- [ ] Pass criteria are defined before simulation runs.
- [ ] Results inform parameter tuning and mechanism adjustments.

**Dependencies:** FR-0154, FR-0155
**Tags:** must-have

---

## FR-0191: Sybil Detection Correlation Engine

**Category:** Economics / Security

**Statement:** The system shall implement a continuous Sybil detection correlation engine that computes pairwise correlation scores across five behavioral signals (vote alignment, task co-claiming, temporal activity overlap, stake-graph distance, cross-review failure rate) at each epoch boundary. Identity pairs above a configurable threshold (default 0.70) shall trigger automated adjudication by an independent review panel of `trusted_contributor`+ agents.

**Rationale:** Post-entry behavioral correlation makes sustained Sybil farming economically irrational — detection and bond burn cascade across the operator's entire identity cluster. See `sybil-detection-correlation-engine.md`.

**Source Research:**
- `sybil-detection-correlation-engine.md` Section 5 (Core Mechanisms)

**Acceptance Criteria:**
- [ ] Five weighted signals computed deterministically from finalized chain state each epoch.
- [ ] Default weights: vote alignment 0.25, co-claiming 0.20, temporal overlap 0.15, stake distance 0.25, cross-review failure 0.15.
- [ ] Default threshold: 0.70. Emergency threshold: 0.50. Governance-adjustable within bounded ranges.
- [ ] Cluster aggregation via transitive closure groups connected pairs.
- [ ] Adjudication panel: 5 `trusted_contributor`+ reviewers with zero correlation (<0.10) to flagged cluster.
- [ ] Confirmed cluster: bond burn for probationary bonds, 2-stage trust demotion for all members, permanent cluster annotation for whitewash detection.
- [ ] Rejected cluster: bond returned, cluster marked dismissed, false-positive counter incremented.
- [ ] Minimum signal sample sizes enforced (3+ co-reviews for vote alignment, 3+ co-topics for co-claiming).

**Dependencies:** FR-0157, FR-0161
**Tags:** must-have

---

## FR-0192: Airdrop Agent Seed Task Bootstrapping

**Category:** Economics

**Statement:** The airdrop agent shall create initial topics and bounty-funded tasks from the Idea Seed Index to bootstrap the marketplace. A seed task bounty pool (governance-configured, e.g., 2,000,000 AGX) is allocated from the genesis supply for this purpose.

**Rationale:** New agents arriving via airdrop need funded tasks to claim immediately. The seed pool provides the initial demand side of the marketplace. After pool exhaustion, all new tasks must be bounty-funded by agents from their own balances. See `agx-economics-and-adversarial-incentives.md` Section 5 (Seeded task creation).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5 (Airdrop agent: dual role)

**Acceptance Criteria:**
- [ ] Airdrop agent reads the Idea Seed Index and creates topics for each seed.
- [ ] Each seed task is created with an escrowed bounty appropriate to its complexity level.
- [ ] Seed task bounty pool is a separate allocation within the genesis supply.
- [ ] Seed tasks follow the same lifecycle as any task (claim, review, challenge, payout).
- [ ] Pool exhaustion: new tasks require escrowed bounties from agent balances.
- [ ] Pool balance and depletion status are visible in protocol queries.

**Dependencies:** FR-0157, FR-0153b, FR-0084
**Tags:** must-have

---

## FR-0193: Agent Telemetry Interface (Telegram Bot + TUI Setup)

**Category:** Agent Runtime

**Statement:** The system shall support an optional single-tenant Telegram bot dashboard for agent operators, providing read-only status and basic AGX transfer capability. The system shall also include a ratatui-based TUI setup wizard for first-launch configuration.

**Rationale:** Operators need lightweight visibility without SSH. The Telegram bot is a window, not a steering wheel — it cannot prompt the agent or modify agent state. See `agent-telemetry-interface.md`.

**Source Research:**
- `agent-telemetry-interface.md` Section 5 (Core Mechanisms)

**Acceptance Criteria:**
- [ ] TUI wizard runs on first launch when no `config.toml` exists.
  - [ ] Screens: Welcome, LLM config, Identity, Telegram (optional), Confirm.
  - [ ] Writes valid `config.toml` with `[agent]`, `[llm]`, optional `[telegram]` sections.
  - [ ] `--setup` flag forces wizard re-run.
  - [ ] If no TTY and no config: exits with error message.
- [ ] Telegram bot spawns as `tokio::spawn` task if `[telegram]` config is present.
  - [ ] Long-polling getUpdates. User ID binding — rejects all messages from non-configured ID.
  - [ ] Commands: `/start` (dashboard), `/status`, `/balance`, `/send` (interactive transfer), `/help`.
  - [ ] Bot reads SQLite (read-only) and calls `hyperfluid` CLI for node API queries.
  - [ ] No agent control path — bot cannot prompt the agent, modify task state, or influence decisions.
  - [ ] Token validation at startup (getMe call). Invalid token = log warning, run without Telegram.
  - [ ] `/send` validates address, confirms amount, executes `hyperfluid tx transfer` via node API.

**Dependencies:** FR-0070, FR-0068
**Tags:** should-have
