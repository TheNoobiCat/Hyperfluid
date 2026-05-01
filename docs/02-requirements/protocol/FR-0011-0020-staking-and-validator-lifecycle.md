## FR-0011: Four-State Validator Lifecycle

**Category:** Consensus

**Statement:** The system shall implement a simplified validator lifecycle with exactly four states: `active`, `paused`, `unbonding`, and `withdrawn`.

**Rationale:** Simplified state machine reduces implementation complexity and test surface while preserving economic accountability. See `agx-committee-bft-and-governance.md` Section 5 (Staking and validator lifecycle).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 92-111
- `index.md` (Canonical Terminology)

**Acceptance Criteria:**
- [ ] Only four states are valid for validators; any other state transition is rejected.
- [ ] State transitions follow the deterministic graph: Active <-> Paused, Active/Paused -> Unbonding -> Withdrawn.
- [ ] Slashing can apply to any state and only deducts stake without changing state.

**Dependencies:** FR-0007
**Tags:** must-have

---

## FR-0012: Minimum Stake and Bonding Delay

**Category:** Consensus

**Statement:** The system shall require a minimum validator stake of 1,000 AGX, with a 24-hour bonding delay before committee eligibility.

**Rationale:** Prevents instant influence buys and Sybil validator creation. See `agx-committee-bft-and-governance.md` Section 5 (Default protocol parameters).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 113-115

**Acceptance Criteria:**
- [ ] `StakeBondTx` with amount < 1,000 AGX is rejected.
- [ ] Newly bonded validators are not eligible for committee selection until 24 hours (8,640 blocks) after bonding height.
- [ ] Bonding delay is enforced at committee sampling time, not at stake acceptance time.

**Dependencies:** FR-0011
**Tags:** must-have

---

## FR-0013: 14-Day Unbonding Delay with Slashable Funds

**Category:** Consensus

**Statement:** The system shall enforce a 14-day unbonding delay during which funds remain locked and slashable.

**Rationale:** Stronger economic safety by keeping stake accountable after exit intent. See `agx-committee-bft-and-governance.md` Section 5 (Default protocol parameters).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 113-116
- `agx-economics-and-adversarial-incentives.md` Section 5 (Stake lifecycle as economic commitment)

**Acceptance Criteria:**
- [ ] `UnbondRequestTx` initiates a 14-day timer; funds cannot be withdrawn before timer completion.
- [ ] During unbonding, validator remains slashable for equivocation or liveness faults discovered within the unbonding window.
- [ ] Withdrawal before timer completion is rejected.

**Dependencies:** FR-0011, FR-0012
**Tags:** must-have

---

## FR-0014: Equivocation Slashing and Jail

**Category:** Consensus

**Statement:** The system shall slash 10% of bonded stake per proven equivocation event and jail the validator for 30 days minimum before re-entry eligibility.

**Rationale:** Strong deterrence against double-signing, the most dangerous Byzantine behavior. See `agx-committee-bft-and-governance.md` Section 5 (Default protocol parameters).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 117-119
- `agx-committee-bft-and-governance.md` Section 5, lines 131-139

**Acceptance Criteria:**
- [ ] Proven equivocation (two conflicting votes for same height/round) triggers immediate 10% stake slash.
- [ ] Equivocating validator is moved to `paused` and jailed for 30 days.
- [ ] Evidence must be submitted within 24 hours (8,640 blocks) of the equivocation event; late evidence cancels slash but marks validator for review.

**Dependencies:** FR-0011, FR-0013
**Tags:** must-have

---

## FR-0015: Downtime Slashing with Hysteresis

**Category:** Consensus

**Statement:** The system shall slash validators for repeated liveness failures: 0.1% per liveness window breach, escalating to 1% on repeated breaches, with movement to `paused` state.

**Rationale:** Incentivizes reliable uptime without excessive punishment for transient issues. See `agx-committee-bft-and-governance.md` Section 5 (Default protocol parameters).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 118-119
- `agx-committee-bft-and-governance.md` Section 5, lines 131-135

**Acceptance Criteria:**
- [ ] Liveness window is 8,192 blocks (~1 day) with a 20% missed-block threshold.
- [ ] First breach: 0.1% slash + move to `paused`.
- [ ] Repeated breaches within 3 liveness windows: escalate to 1% slash.
- [ ] Hysteresis prevents flapping: validator must submit `StakeRenewTx` and wait 1 epoch before returning to `active`.

**Dependencies:** FR-0011, FR-0014
**Tags:** must-have

---

## FR-0016: Governance Voting Eligibility Restricted to Active Validators

**Category:** Governance

**Statement:** The system shall restrict governance vote submission to validators in `active` state at the governance snapshot block.

**Rationale:** Ensures governance decisions are made by currently participating validators with live economic stake. See `agx-committee-bft-and-governance.md` Section 5, lines 111-112.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, line 111

**Acceptance Criteria:**
- [ ] `GovernanceVoteTx` from `paused`, `unbonding`, or `withdrawn` validators is rejected.
- [ ] Snapshot is taken at proposal height; validators activated after snapshot cannot vote.
- [ ] Vote weight is proportional to bonded stake at snapshot.

**Dependencies:** FR-0011
**Tags:** must-have

---

## FR-0017: Resume from Paused via StakeRenewTx

**Category:** Consensus

**Statement:** The system shall allow validators to resume from `paused` to `active` only by submitting `StakeRenewTx` and waiting 1 epoch.

**Rationale:** Prevents rapid state flapping and ensures commitment before rejoining. See `agx-committee-bft-and-governance.md` Section 5, lines 105-108.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 105-108
- `agx-committee-bft-and-governance.md` Section 5, lines 131-139

**Acceptance Criteria:**
- [ ] `StakeRenewTx` is the only mechanism to resume from `paused`.
- [ ] Resume takes effect after 1 full epoch (8,192 blocks) from transaction inclusion.
- [ ] No separate `ResumeTx` exists.

**Dependencies:** FR-0011, FR-0015
**Tags:** must-have

---

## FR-0018: No-Vote Timeout Semantics

**Category:** Governance

**Statement:** The system shall treat timeout in any review or governance context as a `no vote` (not deny, not abstain), which does not count toward quorum and incurs no penalty.

**Rationale:** Distinguishes active denial from unavailability, preserving fairness under transient delays. See `agx-committee-bft-and-governance.md` Section 5, lines 140-144.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 140-144
- `index.md` (No-Vote Timeout Semantics)

**Acceptance Criteria:**
- [ ] Non-responding reviewers/voters emit no vote.
- [ ] No-vote does not count toward quorum threshold.
- [ ] No-vote does not penalize the validator.
- [ ] Height-based timeouts preferred over wall-clock; wall-clock maps to approximate block height with ±2 block tolerance.

**Dependencies:** FR-0016
**Tags:** must-have

---

## FR-0019: Evidence Transaction Pipeline

**Category:** Consensus

**Statement:** The system shall accept `EvidenceTx` containing cryptographic proof of equivocation or severe liveness failure, automatically applying slashing and jail when proof validates.

**Rationale:** Enables decentralized fault reporting without central authority. See `agx-committee-bft-and-governance.md` Section 5 (Transaction types, EvidenceTx).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 74-75
- `agx-committee-bft-and-governance.md` Section 5, lines 131-139

**Acceptance Criteria:**
- [ ] EvidenceTx schema includes fault type, conflicting signatures or liveness proof, and reporter signature.
- [ ] Valid equivocation evidence triggers automatic slash and jail without governance vote.
- [ ] Invalid evidence is rejected and reporter may be penalized for spam.

**Dependencies:** FR-0014, FR-0015
**Tags:** must-have

---

## FR-0020: Staking State Machine Determinism

**Category:** Consensus

**Statement:** The system shall apply all staking state transitions (bond, renew, unbond, withdraw, slash, jail) in deterministic order within each block execution.

**Rationale:** Ensures all nodes converge to identical staking state. See `agx-committee-bft-and-governance.md` Section 5 (Staking and validator lifecycle).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 92-111

**Acceptance Criteria:**
- [ ] Staking transitions are ordered by transaction inclusion index within the block.
- [ ] All nodes produce identical post-block staking state given the same transactions.
- [ ] Staking state root is included in block header SMT root.

**Dependencies:** FR-0011, FR-0007
**Tags:** must-have
