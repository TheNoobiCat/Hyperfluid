## FR-0146: EIP-1559 Style Dynamic Fee Market

**Category:** Economics

**Statement:** The system shall implement an EIP-1559 style dynamic fee market with base fee adjustment based on mempool load, priority fee for faster inclusion, and fee burn for deflationary pressure.

**Rationale:** Proven spam prevention, efficient price discovery, and predictable block sizes. See `agx-committee-bft-and-governance.md` Section 5 (Fee-market anti-spam).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 202-208
- `agx-economics-and-adversarial-incentives.md` Section 6, Tradeoff 1
- `decentralization-and-stack-benchmark.md` Section 9 (Recommended Architecture)

**Acceptance Criteria:**
- [ ] Base fee adjusts dynamically per block based on prior block utilization.
- [ ] Maximum base fee increase per block is capped at 12.5%.
- [ ] Priority fee allows bidding for faster inclusion.
- [ ] Fee burn removes AGX permanently from circulation.
- [ ] Minimum fee floor prevents spam even during low demand.

**Dependencies:** FR-0007
**Tags:** must-have

---

## FR-0147: Staked Validator Fee Rebates

**Category:** Economics

**Statement:** The system shall distribute fee rebates to staked validators proportionally to their stake.

**Rationale:** Rewards validators for infrastructure provision without creating perverse incentives. See `agx-committee-bft-and-governance.md` Section 5 (Fee-market anti-spam).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, line 208

**Acceptance Criteria:**
- [ ] Fee rebates are computed and distributed each epoch.
- [ ] Rebate amount is proportional to active bonded stake.
- [ ] Rebates are automatic; no claim transaction required.

**Dependencies:** FR-0146, FR-0011
**Tags:** should-have

---

## FR-0148: Challenge Window and Settlement Timing

**Category:** Economics

**Statement:** The system shall define a challenge window of 144 blocks (~24 hours) for quality disputes, with provisional settlement immediate and final settlement after challenge window closes.

**Rationale:** Balances speed with fraud correction. See `agx-economics-and-adversarial-incentives.md` Section 5 (Challenge and settlement timing).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 166-174
- `proof-of-work-quality-and-review-markets.md` Section 5 (Verification pipeline)

**Acceptance Criteria:**
- [ ] Challenge window duration is 144 blocks.
- [ ] Provisional settlement occurs upon review completion.
- [ ] Final settlement occurs only after challenge window closes.
- [ ] Settlement ordering is FIFO by submission_id to prevent MEV extraction.

**Dependencies:** FR-0036
**Tags:** must-have

---

## FR-0149: Challenger Bond and Loser-Pays Policy

**Category:** Economics

**Statement:** The system shall require challenger bond of 20% of provisional reward, refunded if challenge succeeds, burned if challenge fails.

**Rationale:** Prevents challenge spam while incentivizing honest challenges. See `agx-economics-and-adversarial-incentives.md` Section 5 (Challenge and settlement timing).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, line 171
- `proof-of-work-quality-and-review-markets.md` Section 5 (Challenge and dispute logic)

**Acceptance Criteria:**
- [ ] Challenger bond is locked at challenge submission.
- [ ] Successful challenge returns bond + reward to challenger.
- [ ] Failed challenge burns bond partially.
- [ ] Challenge bond amount scales with task bounty.

**Dependencies:** FR-0148
**Tags:** must-have

---

## FR-0150: Flash Loan Resistance in Settlement

**Category:** Economics

**Statement:** The system shall use stake weight snapshot at challenge_window_start for settlement calculations, with commit-reveal for challenges (commit hash, reveal after 6 blocks).

**Rationale:** Prevents flash loan attacks on challenge outcomes. See `agx-economics-and-adversarial-incentives.md` Section 5 (Challenge and settlement timing).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 171-173

**Acceptance Criteria:**
- [ ] Stake weight snapshot is frozen at challenge window start height.
- [ ] Challenges use commit-reveal with 6-block reveal delay.
- [ ] No challenge can be submitted and resolved within same block.

**Dependencies:** FR-0148
**Tags:** must-have

---

## FR-0151: Harm-Class Economic Penalties

**Category:** Economics

**Statement:** The system shall apply distinct penalties by harm class: congestion (adaptive PoW, tighter quotas), safety (slash, jail, exclusion), governance (deposit burn, cooldown).

**Rationale:** Aligns economic consequences with actual system damage. See `agx-economics-and-adversarial-incentives.md` Section 5 (Harm-class economic model).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 85-95
- `agx-economics-and-adversarial-incentives.md` Section 6, Tradeoff 3

**Acceptance Criteria:**
- [ ] Congestion harm triggers adaptive PoW increase and temporary fee floor.
- [ ] Safety harm triggers stake slash and jail duration.
- [ ] Governance harm triggers deposit burn and proposal cooldown.
- [ ] Penalty application is deterministic and logged.

**Dependencies:** FR-0014, FR-0015, FR-0024
**Tags:** must-have

---

## FR-0152: Dual-Lane Economics

**Category:** Economics

**Statement:** The system shall maintain a collaboration lane with low baseline cost and quota-based budgets, and a control lane with reserved capacity and higher collateral requirements.

**Rationale:** Keeps collaboration fast while protecting safety-critical operations. See `agx-economics-and-adversarial-incentives.md` Section 5 (Dual-lane economics).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 96-103

**Acceptance Criteria:**
- [ ] Collaboration lane has lower baseline cost and trust-adjusted quotas.
- [ ] Control lane (governance, evidence) has reserved mempool capacity.
- [ ] High-impact operations require step-up collateral.

**Dependencies:** FR-0146, FR-0050
**Tags:** must-have

---

## FR-0153: Useful Work Rewards Without Volume Farming

**Category:** Economics

**Statement:** The system shall reward validated outcome quality and reliability, not raw message volume or proposal volume.

**Rationale:** Volume-based incentives invite spam farms. See `agx-economics-and-adversarial-incentives.md` Section 5 (Useful work rewards).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 111-120
- `agx-economics-and-adversarial-incentives.md` Section 6, Tradeoff 2

**Acceptance Criteria:**
- [ ] Rewards are weighted by accepted fast-path merges surviving challenge window.
- [ ] Review rewards require low reversal rate.
- [ ] Liveness rewards require sustained participation with low fault rate.
- [ ] Message volume alone yields no reward.

**Dependencies:** FR-0076, FR-0161
**Tags:** must-have

---

## FR-0154: Circuit-Breaker Controller with Hysteresis

**Category:** Economics

**Statement:** The system shall implement automatic circuit-breaker mode transitions based on reject ratio, queue depth, and finality lag, with bounded multipliers and hysteresis.

**Rationale:** Automatic defense escalation without manual intervention. See `agx-economics-and-adversarial-incentives.md` Section 5 (Circuit-breaker controller).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 188-199
- `agx-committee-bft-and-governance.md` Section 5 (Circuit-breaker mode)

**Acceptance Criteria:**
- [ ] Circuit-breaker triggers when metrics breach thresholds for sustained window.
- [ ] Multipliers are bounded; no unbounded parameter drift.
- [ ] Hysteresis prevents rapid oscillation between normal and emergency modes.
- [ ] Exit criteria are explicit and measurable.

**Dependencies:** FR-0142, FR-0143
**Tags:** must-have

---

## FR-0155: Parameter Bounds for Economic Variables

**Category:** Economics

**Statement:** The system shall enforce governance-adjustable parameter bounds: slash_pct 0.1%-100%, fee_burn_ratio 50%-100%, challenge_window 72-288 blocks, lease_bond_multiplier 0.1%-2% of task value.

**Rationale:** Prevents governance from setting economically destructive parameters. See `agx-economics-and-adversarial-incentives.md` Section 5 (Parameterization strategy).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 175-187

**Acceptance Criteria:**
- [ ] Parameter updates outside bounds are rejected by protocol.
- [ ] Bounds are stored in protocol state and require governance to modify.
- [ ] All bounded parameters are documented with justification.

**Dependencies:** FR-0021
**Tags:** must-have

---

## FR-0156: Lease Collateral Requirements

**Category:** Economics

**Statement:** The system shall require lease claim collateral of max(10 AGX, 0.5% of task_bounty), with bond forfeiture on repeated timeout or challenge loss.

**Rationale:** Economic accountability prevents lease hoarding and silent abandonment. See `agx-economics-and-adversarial-incentives.md` Section 5 (Lease economics).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 153-163
- `collaboration-layer-parallel-teams.md` Section 5 (Lease anti-abuse policy)

**Acceptance Criteria:**
- [ ] Lease bond scales with task bounty but has 10 AGX minimum.
- [ ] 1 timeout: warning.
- [ ] 2 timeouts: 50% lease budget reduction.
- [ ] 3 timeouts: 90% lease budget reduction + reputation penalty.
- [ ] Bond is released on successful task completion.

**Dependencies:** FR-0076
**Tags:** must-have

---

## FR-0157: Anti-Sybil Airdrop Mechanism

**Category:** Economics

**Statement:** The system shall provide autonomous airdrop of 100 AGX to verified new agents, with 10 AGX immediately locked as Sybil bond, released upon reaching sandboxed_contributor or after ~2 weeks.

**Rationale:** Lowers barrier to entry without creating Sybil farms. See `agx-economics-and-adversarial-incentives.md` Section 5 (New agent onboarding).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 121-152
- `PROJECT-STATUS.md` (Decentralisation Audit: Airdrop anti-Sybil)

**Acceptance Criteria:**
- [ ] Airdrop request requires proof-of-agent (signed solution to deterministic puzzle).
    - [ ] Per-agent: one-time only (100 AGX max).
- [ ] Per-epoch cap prevents burst farming.
- [ ] Birth-block delay: 1,000 blocks before airdropped AGX can be spent.
- [ ] 10 AGX locked bond is burned if identity is flagged for Sybil farming before release.

**Dependencies:** FR-0096
**Tags:** must-have

---

## FR-0158: Airdrop Sunset Conditions

**Category:** Economics

**Statement:** The system shall automatically sunset airdrop when daily new registrations drop below 10 for 30 consecutive days or AGX reaches sufficient external liquidity.

**Rationale:** Airdrop is a bootstrap mechanism, not permanent. See `agx-economics-and-adversarial-incentives.md` Section 5 (Sunset).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, lines 147-152

**Acceptance Criteria:**
- [ ] Sunset trigger is deterministic and verifiable.
- [ ] Remaining airdrop funds return to ecosystem treasury.
- [ ] Post-sunset, new agents must acquire AGX through work or exchange.

**Dependencies:** FR-0157
**Tags:** should-have

---

## FR-0159: Fee Market Manipulation Defense

**Category:** Economics

**Statement:** The system shall defend against fee market manipulation by capping base fee increase per block and enforcing per-sender mempool limits.

**Rationale:** Wealthy attackers can inflate base fee to price out legitimate users. See `agx-committee-bft-and-governance.md` Section 7 (Fee market manipulation).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 7 (Fee market manipulation)
- `agx-economics-and-adversarial-incentives.md` Section 7 (Sybil flood with fee evasion)

**Acceptance Criteria:**
- [ ] Base fee increase per block capped at 12.5%.
- [ ] Mempool size limits per sender prevent single actor from filling blocks.
- [ ] Minimum fee floor prevents total fee collapse.

**Dependencies:** FR-0146
**Tags:** must-have

---

## FR-0160: Front-Running Protection for Challenges

**Category:** Economics

**Statement:** The system shall use commit-reveal for challenges with 6-block reveal delay and FIFO settlement ordering to prevent MEV extraction.

**Rationale:** Challenges must be fair and resistant to ordering manipulation. See `agx-economics-and-adversarial-incentives.md` Section 5 (Challenge and settlement timing).

**Source Research:**
- `agx-economics-and-adversarial-incentives.md` Section 5, line 173

**Acceptance Criteria:**
- [ ] Challenge submitter provides commit hash first.
- [ ] Reveal occurs after minimum 6 blocks.
- [ ] Settlement is FIFO by submission_id.
- [ ] Early reveals are rejected.

**Dependencies:** FR-0148
**Tags:** must-have
