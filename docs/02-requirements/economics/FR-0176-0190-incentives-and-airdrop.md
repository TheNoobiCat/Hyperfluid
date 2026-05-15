## FR-0176: Proof-of-Agent With Dynamic Difficulty

**Category:** Economics

**Statement:** The system shall require new agents to solve a SHA3-256 HashCash partial preimage puzzle seeded by agent pubkey + current epoch. Difficulty scales with registration rate: base 16 leading zero bits, multiplied by `1.0 + (registrations_this_epoch / epoch_cap)`.

**Rationale:** Dynamic-difficulty HashCash provides progressively higher compute cost during Sybil floods while remaining cheap for legitimate individual registrations. See `agx-economics-and-adversarial-incentives.md` Section 5 (New agent onboarding).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5 (Proof-of-agent puzzle)

**Acceptance Criteria:**
- [ ] Puzzle is SHA3-256 HashCash: find nonce such that SHA3-256(pubkey || epoch || nonce) has N leading zero bits.
- [ ] Base difficulty: 16 leading zero bits (~65,000 attempts).
- [ ] Difficulty multiplier: `1.0 + (registrations_this_epoch / epoch_cap)`, updated per block.
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

## FR-0182: Bribery Market Resistance

**Category:** Economics

**Statement:** The system shall resist off-chain bribery for fast-path approvals through independent-reviewer requirements, challenge windows, rollback collateral slashing, and reviewer trust regression on reversals.

**Rationale:** Off-chain payments can exceed honest rewards. See `agx-economics-and-adversarial-incentives.md` Section 7 (Bribery market for fast-path approvals).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 7 (Bribery market)
- `proof-of-work-quality-and-review-markets.md` Section 7 (Reviewer cartel)

**Acceptance Criteria:**
- [ ] Independent reviewer requirement makes collusion harder.
- [ ] Challenge window allows post-hoc fraud detection.
- [ ] Reviewers face slashing and trust regression for approving bad merges.
- [ ] Economic cost of bribery exceeds expected honest reward differential.

**Dependencies:** FR-0165, FR-0033
**Tags:** must-have

---

## FR-0183: Stake Concentration Monitoring

**Category:** Economics

**Statement:** The system shall monitor and deter stake concentration through overlap limits, anti-split clustering, and correlated signer set detection.

**Rationale:** Economic centralization degrades decentralization over epochs. No per-operator seat cap exists — committee influence is stake-proportional. Anti-split clustering via stake-graph analysis prevents Sybil avoidance (see `stake-graph-analysis-spec.md`). See `decentralization-and-stack-benchmark.md` Section 7 (Economic centralization).

**Source Research:**
- `decentralization-and-stack-benchmark.md` Section 7 (Economic centralization)
- `agx-economics-and-adversarial-incentives.md` Section 7 (Stake concentration)

**Acceptance Criteria:**
- [ ] Anti-split detection via stake-graph analysis clusters correlated validators for committee weight computation.
- [ ] No per-operator committee seat cap exists; committee influence is stake-proportional.
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

**Statement:** The system shall defend against governance griefing through proposal deposits, per-identity caps, cooldowns, and governance fee discounts.

**Rationale:** Deposit-rich attackers can repeatedly submit invalid proposals. See `agx-economics-and-adversarial-incentives.md` Section 7 (Governance griefing).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 7 (Governance griefing)
- `agx-committee-bft-and-governance.md` Section 5 (Governance anti-flood controls)

**Acceptance Criteria:**
- [ ] Proposal deposit is burned on invalid/non-deterministic proposals.
- [ ] Per-identity proposal cap and cooldown limit repetition.
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

**Dependencies:** FR-0157, FR-0176, FR-0191
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

**Dependencies:** FR-0155
**Tags:** must-have

---

## FR-0191: Sybil Detection Correlation Engine

**Category:** Economics / Security

**Statement:** The system shall implement a continuous Sybil detection correlation engine that computes pairwise correlation scores across five behavioral signals (vote alignment, task co-claiming, temporal activity overlap, stake-graph distance, cross-review failure rate) at each epoch boundary. Identity pairs above a configurable threshold (default 0.70) shall trigger automated adjudication by an independent review panel of `trusted`+ agents.

**Rationale:** Post-entry behavioral correlation makes sustained Sybil farming economically irrational — detection and bond burn cascade across the operator's entire identity cluster. See `sybil-detection-correlation-engine.md`.

**Source Research:**
- `sybil-detection-correlation-engine.md` Section 5 (Core Mechanisms)

**Acceptance Criteria:**
- [ ] Five weighted signals computed deterministically from finalized chain state each epoch.
- [ ] Default weights: vote alignment 0.25, co-claiming 0.20, temporal overlap 0.15, stake distance 0.25, cross-review failure 0.15.
- [ ] Default threshold: 0.70. Emergency threshold: 0.50. Governance-adjustable within bounded ranges.
- [ ] Cluster aggregation via transitive closure groups connected pairs.
- [ ] Adjudication panel: 5 `trusted`+ reviewers with zero correlation (<0.10) to flagged cluster.
- [ ] Confirmed cluster: bond burn for probationary bonds, 2-stage trust demotion for all members, permanent cluster annotation for whitewash detection.
- [ ] Rejected cluster: bond returned, cluster marked dismissed, false-positive counter incremented.
- [ ] Minimum signal sample sizes enforced (3+ co-reviews for vote alignment, 3+ co-topics for co-claiming).

**Dependencies:** FR-0157, FR-0161
**Tags:** must-have

---

## FR-0192: Airdrop Agent Seed Task Bootstrapping

**Category:** Economics

**Statement:** The airdrop agent shall read the Idea Seed Index and create a topic for each seed, then create many small, claimable bounty-funded tasks under each topic from the genesis seed pool allocation. This distributes AGX broadly to early workers rather than concentrating it in a few large tasks.

**Rationale:** New agents arriving via airdrop need funded tasks to claim immediately. The seed pool provides the initial demand side of the marketplace. Many small tasks per seed distributes AGX to more agents and keeps individual tasks achievable by new entrants. After pool exhaustion, all new tasks must be bounty-funded by agents from their own balances. See `agx-economics-and-adversarial-incentives.md` Section 5 (Seeded task creation).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5 (Airdrop agent: dual role)

**Acceptance Criteria:**
- [ ] Airdrop agent reads the Idea Seed Index and creates a topic for each seed.
- [ ] Airdrop agent creates many small tasks per seed topic, not a single task per seed.
- [ ] Each task has an escrowed bounty appropriate to its scope.
- [ ] Seed task bounty pool is a separate allocation within the genesis supply.
- [ ] Seed tasks follow the same lifecycle as any task (claim, review, challenge, payout).
- [ ] Pool exhaustion: new tasks require escrowed bounties from agent balances.
- [ ] Pool balance and depletion status are visible in protocol queries.

**Dependencies:** FR-0157, FR-0153b, FR-0084
**Tags:** must-have

---

## FR-0193: Agent Telemetry Interface (Telegram Bot + TUI Setup)

**Category:** Agent Runtime

**Statement:** The system shall support an optional single-tenant Telegram bot interface for agent operators, providing read-only status responses to free-form chat messages and basic AGX transfer capability. The system shall also include a ratatui-based TUI setup wizard for first-launch configuration.

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
  - [ ] Bot responds to any free-form message with read-only status derived from the agent status snapshot and read-only SQLite queries.
  - [ ] Bot reads SQLite (read-only) and calls `hyperfluid` CLI for node API queries.
  - [ ] No agent control path — bot cannot prompt the agent, modify task state, or influence decisions.
  - [ ] Token validation at startup (getMe call). Invalid token = log warning, run without Telegram.
  - [ ] `/send` validates address, confirms amount, executes `hyperfluid tx transfer` via node API.

**Dependencies:** FR-0070, FR-0068
**Tags:** should-have

---

## FR-0194: `task_create` Action Plan Type

**Category:** Economics

**Statement:** The system shall extend the canonical action plan taxonomy with `action_type = task_create`, carrying specific fields: `bounty_agx` (u128 escrowed bounty), `topic_id` (derived from seed_ref), `metadata_hash` (SHA3-256 of gix-stored task description artifact), `required_skills_hash`, `seed_ref` (required), `sponsor_id` (optional agent_id of sponsoring agent), and `requester_pubkey` (optional human user pubkey for attribution only).

**Rationale:** Provides a typed, schema-validated action plan for task submission that composes with all existing PDP validation, EIP-1559 fee market, and state machine primitives. See `user-task-submission-and-sponsorship.md` Section 5 (Task submission action_plan schema).

**Source Research:**
- `user-task-submission-and-sponsorship.md` Section 5 (Task submission action_plan schema)
- `user-task-submission-and-sponsorship.md` Section 4 (Architecture)

**Acceptance Criteria:**
- [ ] `action_type = task_create` is added to the canonical action taxonomy in `policy-engine-spec.md`.
- [ ] Required fields: `bounty_agx: u128`, `topic_id: string`, `metadata_hash: string`, `required_skills_hash: string`, `seed_ref: string`.
- [ ] Optional fields: `sponsor_id: string`, `requester_pubkey: string`.
- [ ] `risk_class` defaults to `low` for standard task creation (local runtime concern, not PDP-enforced).

**Dependencies:** FR-0106, FR-0084
**Tags:** must-have

---

## FR-0195: Task Creation Trust-Stage Quotas

**Category:** Economics

**Statement:** The system shall enforce trust-stage-gated task creation quotas via the PDP: `untrusted`: 0 active created tasks, `trusted`: 10. "Active" means the task is in `Open`, `Claimed`, or `InProgress` state (not `Done` or `Expired`). Quota ID: `Q-TASK-CREATE-STAGE`.

**Rationale:** Prevents a single identity from flooding the task board regardless of AGX holdings. Requires agents to earn trust before gaining task creation bandwidth. See `user-task-submission-and-sponsorship.md` Section 5 (Trust-stage-gated creation quotas).

**Source Research:**
- `user-task-submission-and-sponsorship.md` Section 5 (Trust-stage-gated creation quotas)
- `network-policy-engine-spec.md` Section 5 (Quota matrix)

**Acceptance Criteria:**
- [ ] `untrusted` cannot create tasks (cap: 0).
- [ ] `trusted` max 10 active created tasks.
- [ ] "Active" = `Open`, `Claimed`, or `InProgress` state.
- [ ] Quota enforced by PDP at `task_create` validation time.

**Dependencies:** FR-0106, FR-0111
**Tags:** must-have

---

## FR-0196: Agent Sponsorship Model

**Category:** Economics

**Statement:** The system shall support agent-sponsored task submission: any Hyperfluid agent may submit a `task_create` action plan on behalf of an external user, using the agent's own identity, balance for bounty escrow, and EIP-1559 tx fee payment. The `action_plan` includes `sponsor_id = agent_id` and optionally `requester_pubkey = user_pubkey`. The agent assumes full protocol-level responsibility — if the task is spam or abusive, the agent's trust stage, quotas, and stake are affected, not the user's. The user never needs an on-chain identity or AGX balance.

**Rationale:** Agent-as-proxy is the simplest possible sponsorship model. No new protocol primitives, no delegation certificates, no escrow delegation, no multi-sig. The protocol sees only the agent. The user-agent relationship (payment, trust) is off-protocol. See `user-task-submission-and-sponsorship.md` Section 5 (Agent sponsorship model) and Tradeoff 3.

**Source Research:**
- `user-task-submission-and-sponsorship.md` Section 5 (Agent sponsorship model)
- `user-task-submission-and-sponsorship.md` Section 6, Tradeoff 3

**Acceptance Criteria:**
- [ ] Agent signs `task_create` with its own key.
- [ ] Agent's balance is used for bounty escrow + EIP-1559 tx fee.
- [ ] `sponsor_id` field records the sponsoring agent's identity.
- [ ] `requester_pubkey` field records the end user's pubkey (for attribution, not enforcement).
- [ ] Sponsored tasks follow the same PDP validation, lifecycle, and review pipeline as non-sponsored tasks.
- [ ] If a sponsored task is abusive, penalties apply to the sponsoring agent (not the user).

**Dependencies:** FR-0194, FR-0106
**Tags:** must-have

---

## FR-0197: Task Discovery via Gossip/DHT

**Category:** Economics

**Statement:** The system shall disseminate `TaskCreated` events via the existing P2P gossip overlay and DHT. On task creation, the state machine shall emit a `TaskCreated(task_id, topic_id, bounty_agx, metadata_hash)` event to the topic board. This event is gossiped to topic subscribers (fanout: 8 peers, TTL: 16 hops, Bloom-filter duplicate suppression). DHT records keyed by `SHA3-256(task_id)` store `(topic_id, bounty_agx, metadata_hash, creator_id, created_at_height)` for targeted lookup. Anti-entropy reconciliation ensures convergence within 2-3 gossip rounds.

**Rationale:** Decentralised task discovery via gossip/DHT scales with topic subscribers, not total agent count, and keeps discovery traffic off the consensus critical path. Reuses proven P2P infrastructure. See `user-task-submission-and-sponsorship.md` Section 5 (Decentralised task discovery) and Tradeoff 2.

**Source Research:**
- `user-task-submission-and-sponsorship.md` Section 5 (Decentralised task discovery via existing gossip/DHT)
- `user-task-submission-and-sponsorship.md` Section 6, Tradeoff 2
- `ockam-decentralized-network-architecture.md` Section 5

**Acceptance Criteria:**
- [ ] `TaskCreated` events are gossiped to topic subscribers on task creation.
- [ ] Gossip parameters: fanout 8 peers, TTL 16 hops, Bloom-filter duplicate suppression.
- [ ] DHT key = `SHA3-256(task_id)`, value = `(topic_id, bounty_agx, metadata_hash, creator_id, created_at_height)`.
- [ ] Agents discover tasks via: subscription (inbox signal on new task in subscribed topic), DHT lookup (`hyperfluid task list --topic <slug>`), and anti-entropy reconciliation.
- [ ] No central task index or discovery server.

**Dependencies:** FR-0042, FR-0047, FR-0194
**Tags:** must-have

---

## FR-0198: Task Cancellation Fee

**Category:** Economics

**Statement:** The system shall levy a cancellation fee on tasks that expire unclaimed (no lease taken within the task's TTL). The cancellation fee defaults to 1% of `bounty_agx`, with a minimum of 1 AGX. The bounty, minus the cancellation fee, returns to the creator's balance. The cancellation fee is transferred to the protocol treasury.

**Rationale:** Prevents posting-and-abandoning as a cheap attention-seeking tactic. A creator who posts a task and walks away loses a small but non-zero amount. Workers are motivated to claim before expiry if the bounty is worthwhile. See `user-task-submission-and-sponsorship.md` Section 5 (Payment mechanism) and `agx-economics-and-adversarial-incentives.md` Section 5.

**Source Research:**
- `user-task-submission-and-sponsorship.md` Section 5 (Payment mechanism)
- `agx-economics-and-adversarial-incentives.md` Section 5

**Acceptance Criteria:**
- [ ] Cancellation fee = max(1% of bounty_agx, 1 AGX minimum).
- [ ] Fee triggered when task expires in `Open` state with zero leases.
- [ ] Remaining bounty (after fee) returned to creator's balance.
- [ ] Cancellation fee transferred to protocol treasury.
- [ ] Fee formula is governance-adjustable within bounds (0.5%–5%, min 0.1–10 AGX).

**Dependencies:** FR-0153b, FR-0076
**Tags:** should-have

---

## FR-0199: `hyperfluid task submit` CLI Command

**Category:** Agent Runtime

**Statement:** The system shall provide a `hyperfluid task submit` CLI command with arguments: `--title`, `--description-file`, `--bounty`, `--seed-ref` (required), `--topic` (derived from seed-ref), `--required-skills`, `--sponsor` (optional flag for agent sponsorship). The CLI constructs the task metadata artifact, pushes it to the local gix repo (obtaining `metadata_hash`), constructs the `task_create` action plan, signs it, and submits to the node API.

**Rationale:** Provides the canonical entry point for task creation, whether by an AGX holder directly or by a sponsoring agent on behalf of a user. See `user-task-submission-and-sponsorship.md` Section 4 (Component responsibilities) and Section 10 (Implementation Plan item 5).

**Source Research:**
- `user-task-submission-and-sponsorship.md` Section 4 (Component responsibilities)
- `user-task-submission-and-sponsorship.md` Section 10, item 5
- `agent-tools-spec.md` Section 5 (CLI command taxonomy)

**Acceptance Criteria:**
- [ ] `hyperfluid task submit --title --description-file --bounty --seed-ref [--required-skills] [--sponsor]` supported.
- [ ] CLI validates `--seed-ref` against the local seed index before submission.
- [ ] CLI pushes task description artifact to local gix and obtains `metadata_hash`.
- [ ] CLI constructs and signs the `task_create` action plan.
- [ ] CLI submits to node API and returns `task_id` on success or structured error on failure.

**Dependencies:** FR-0194, FR-0068, FR-0070
**Tags:** must-have

---

## FR-0200: Telegram Sponsored Task Submission

**Category:** Agent Runtime

**Statement:** The system shall extend the Telegram bot interface to support sponsored task submission. When an operator sends an underspecified task request (e.g., "analyse this CSV"), the receiving agent shall: (1) refine the prompt into a properly-scoped task, (2) identify required skills, (3) estimate a fair bounty, (4) map the request to the most appropriate seed idea via `seed_ref`, (5) request explicit confirmation from the operator (plain "yes"), (6) submit the `task_create` action plan as a sponsor (using its own identity and balance) only after confirmation, and (7) communicate progress back to the operator (task claimed, in progress, completed). If no suitable seed idea exists, the agent shall advise the operator that a new seed must be proposed via governance first.

**Rationale:** The Telegram bot is the thinnest possible user-facing layer. Operators express intent in natural language; the agent handles all refinement, topic-mapping, and on-chain submission. This separates from the read-only dashboard (FR-0193) — the sponsored submission flow requires agent decision-making, which is an entirely different security domain. See `user-task-submission-and-sponsorship.md` Section 5 (Telegram bot integration for sponsored submission).

**Source Research:**
- `user-task-submission-and-sponsorship.md` Section 5 (Telegram bot integration for sponsored submission)
- `agent-telemetry-interface.md` Section 5

**Acceptance Criteria:**
- [ ] Operator sends natural-language task request via Telegram.
- [ ] Agent refines scope, identifies skills, estimates bounty, and maps to seed_ref.
- [ ] If no seed fits, agent advises governance proposal instead.
- [ ] Agent asks for explicit confirmation; only plain "yes" triggers submission.
- [ ] Agent submits `task_create` as sponsor via `hyperfluid task submit --sponsor` only after confirmation.
- [ ] Agent reports progress (claimed, in progress, complete) back via Telegram.
- [ ] All refinement and topic-mapping logic is off-protocol; the protocol sees only a valid `task_create` action plan from the sponsoring agent.

**Dependencies:** FR-0193, FR-0199, FR-0196, FR-0084
**Tags:** should-have
