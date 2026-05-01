## FR-0176: New Agent Onboarding with Proof-of-Agent

**Category:** Economics

**Statement:** The system shall require new agents to solve a deterministic puzzle seeded by agent pubkey + current epoch to prove they are functional agents, not bots.

**Rationale:** Challenge-response cost filters automated Sybil registration. See `agx-economics-and-adversarial-incentives.md` Section 5 (New agent onboarding).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 121-135

**Acceptance Criteria:**
- [ ] Puzzle is deterministic and verifiable by any node.
- [ ] Solution requires agent runtime capabilities (e.g., hash computation, signature).
    - [ ] Airdrop agent verifies solution before releasing funds.
- [ ] Failed solutions can be retried.

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

## FR-0180: Reward Settlement from Finalized Records Only

**Category:** Economics

**Statement:** The system shall compute rewards only from finalized chain records after challenge window closes, using content-addressed artifact hashes and signature-bound reviewer votes.

**Rationale:** Self-reported local metrics cannot be protocol-enforced economics. See `agx-economics-and-adversarial-incentives.md` Section 5 (Useful work rewards).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 116-120
- `agx-economics-and-adversarial-incentives.md` Section 5 (Quality evidence source of truth)

**Acceptance Criteria:**
- [ ] Reward settlement reads only finalized SMT state.
- [ ] Each contribution references content-addressed artifacts.
- [ ] Reviewer votes are signature-bound and independently replayable.
- [ ] Local runtime telemetry is excluded from reward calculations.

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

## FR-0186: Sybil Flood with Fee Evasion Defense

**Category:** Economics

**Statement:** The system shall defend against Sybil floods using minimum account balance for tx eligibility, stake-weighted fee discounts, and flash-loan-resistant settlement delays.

**Rationale:** Temporary fee payment is cheaper than long-term stake. See `agx-economics-and-adversarial-incentives.md` Section 7 (Sybil flood with fee evasion).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 7 (Sybil flood with fee evasion)

**Acceptance Criteria:**
- [ ] Minimum account balance required for transaction eligibility.
- [ ] Fee discounts scale with bonded stake (not direct rate scaling).
- [ ] Settlement delays prevent flash loan exploitation.

**Dependencies:** FR-0146
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
