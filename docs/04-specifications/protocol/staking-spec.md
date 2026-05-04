# Protocol Spec: Staking & Validator Manager

**Components:** C3 Staking & Validator Manager
**Source ADRs:** ADR-0007 (Committee BFT with VDF), ADR-0010 (Four-Stage Trust Ladder)
**Covered FRs:** FR-0011, FR-0012, FR-0013, FR-0014, FR-0015, FR-0016, FR-0017, FR-0018, FR-0019, FR-0020, FR-0151
**Dependencies:** C1 Consensus Engine, C2 State Machine & SMT

---

## Section 1: Validator Lifecycle

### 1.1 Purpose

Define the four-state validator lifecycle, state transitions, bonding/unbonding mechanics, and committee eligibility rules.

### 1.2 Normative Behavior

- The system MUST enforce exactly four validator states: `active`, `paused`, `unbonding`, `withdrawn`.
- A validator MUST be in `active` state to be eligible for committee selection or governance voting.
- State transitions MUST be deterministic and irreversible in the withdrawal direction.
- Slashing MUST apply to any validator state (except `withdrawn`) and only deducts stake without changing state (except as specified for jailing).
- The system MUST record all staking state transitions at the block of transaction inclusion.
- Governance voting eligibility MUST be restricted to `active` validators at the proposal snapshot height.

### 1.3 Data Structures

```rust
struct ValidatorRecord {
    validator_id: [u8; 32],         // same as account_id
    state: ValidatorState,
    bonded_stake: u64,              // total bonded AGX in atto-AGX
    bonding_height: u64,
    unbonding_height: u64,
    jail_until_height: u64,         // 0 if not jailed
    liveness_bitmap: [u8; 1024],    // 8192 bits for ~1 day window
    slash_count: u32,
    missed_blocks: u32,
    last_renew_height: u64,
}

enum ValidatorState {
    Active,
    Paused,
    Unbonding,
    Withdrawn,
}

struct SlashRecord {
    slash_id: [u8; 32],        // SHA3-256 of evidence || validator_id
    validator_id: [u8; 32],
    fault_type: FaultType,
    slash_amount: u64,
    slash_height: u64,
    evidence_ref: [u8; 32],    // content hash of evidence
}

enum FaultType {
    Equivocation,
    LivenessFailure,
    Other,
}

struct SystemParameters {
    epoch_length: u64,                // 8192 blocks
    committee_size: u64,              // 100
    min_stake: u128,                  // 1,000 AGX = 1_000_000_000_000_000_000_000 atto-AGX (10^21)
    bond_delay: u64,                  // 8640 blocks (~24 hrs at 10s/block)
    unbond_delay: u64,                // 120,960 blocks (14 days)
    max_governance_proposals: u64,    // 32
    proposal_deposit: u128,           // 500 AGX
    liveness_window_blocks: u64,      // 8192
    liveness_miss_threshold_pct: u8,  // 20 (20%)
}
```

### 1.4 State Transitions

**Canonical transition graph:**

```
StakeBondTx (+1 epoch wait) ──► active
active ──(miss >20% in window)──► paused [0.1% slash]
paused ──(StakeRenewTx + 1 epoch wait)──► active
active ──(UnbondRequestTx)──► unbonding [14-day timer]
paused ──(UnbondRequestTx)──► unbonding [14-day timer]
unbonding ──(unbond_delay expires + WithdrawUnbondedTx)──► withdrawn
active ──(equivocation proof)──► paused [10% slash + 30-day jail]
```

**Transition details:**

| Trigger | From | To | Conditions |
|---------|------|----|------------|
| StakeBondTx >= 1,000 AGX | (new) | active | After bond_delay (8,640 blocks) |
| Miss >20% blocks in liveness window | active | paused | 0.1% slash; repeated within 3 windows escalates to 1% |
| StakeRenewTx | paused | active | 1 epoch wait (8,192 blocks) after tx inclusion |
| UnbondRequestTx | active/paused | unbonding | 14-day timer starts at inclusion height |
| WithdrawUnbondedTx | unbonding | withdrawn | unbond_delay expired; funds released |
| Equivocation evidence | active | paused | 10% slash; jailed 30 days (259,200 blocks) |

**Liveness tracking:** An 8,192-bit bitmap per validator tracks participation per block in the liveness window. A missed block sets the bit at position (height % 8192) to 1. The count of 1-bits is the missed_block counter. At each epoch boundary, the window slides forward.

**StakeRenewTx:** This is the exclusive mechanism to resume from paused. No separate ResumeTx exists. The tx contains the validator's signature and takes effect after 1 epoch.

### 1.5 Failure Behavior

- **Insufficient stake:** StakeBondTx with amount < 1,000 AGX is rejected at admission.
- **Double-bind:** StakeBondTx from an account that already has a VALIDATOR in non-withdrawn state is rejected.
- **Premature withdrawal:** WithdrawUnbondedTx before unbond_delay expiry is rejected.
- **Delayed evidence:** Equivocation evidence submitted more than 8,640 blocks after the event cancels the slash but marks the validator for governance review.
- **Invalid evidence:** EvidenceTx with non-verifiable proof is rejected; reporter may be penalized for repeated false submissions.
- **Committee eligibility during unbonding:** Unbonding validators remain committee-eligible until the epoch boundary after unbond initiation, then are excluded.

### 1.6 Versioning and Compatibility

- ValidatorRecord schema version tracked in system parameters.
- Breaking changes to lifecycle states require governance proposal.
- Liveness window size and slashing parameters are governance-adjustable within defined bounds.

### 1.7 Conformance Test Hooks

- Verify only four states are valid; any other state transition is rejected.
- Verify StakeBondTx with < 1,000 AGX is rejected.
- Verify newly bonded validators are not committee-eligible until bond_delay expires.
- Verify 14-day unbonding delay is enforced and funds remain slashable during unbonding.
- Verify equivocation triggers 10% slash and 30-day jail.
- Verify liveness window breach at 20% missed blocks triggers 0.1% slash and move to paused.
- Verify repeated liveness breach within 3 windows escalates to 1% slash.
- Verify StakeRenewTx restores active state after 1 epoch wait.
- Verify GovernanceVoteTx from paused/unbonding/withdrawn validators is rejected.
- Verify all staking transitions produce deterministic state across all nodes.

### 1.8 Trust-Assumption Inventory

- Validator honesty in signing
  - Justification: Slashing disincentivizes but cannot prevent. BFT safety requires <33% Byzantine.
  - Trust-minimised alternative: None — economic security is the standard trust model for PoS.
- Liveness window accuracy
  - Justification: Validator downtime is self-reported through chain participation; external monitoring is not protocol-enforced.
  - Trust-minimised alternative: Multi-party observation of block production seen on-chain; consensus itself is the ground truth.
- Evidence submission incentives
  - Justification: EvidenceTx reporters are not directly rewarded; reliance on protocol-aligned validators and agents.
  - Trust-minimised alternative: Bounty-based evidence submission with reward from slashed funds.

---

## Section 2: Governance Voting Eligibility

### 2.1 Purpose

Define validator eligibility for governance voting including snapshot mechanics and vote weight computation.

### 2.2 Normative Behavior

- Only validators in `active` state at the proposal snapshot height MAY vote on governance proposals.
- Vote weight MUST be proportional to bonded_stake at snapshot height.
- Snapshot MUST be taken at the block of GovernanceProposeTx inclusion.
- Validators activated after snapshot height MUST NOT be eligible for that proposal.
- Non-voting (timeout) MUST NOT affect quorum calculation or penalize the validator.

### 2.3 Data Structures

```rust
struct GovernanceVoteTx {
    proposal_id: [u8; 32],
    voter_id: [u8; 32],
    vote: VoteOption,
    reason_hash: [u8; 32],
    vote_weight: u64,          // bonded_stake at snapshot
    signature: Vec<u8>,
}

enum VoteOption {
    Yes,
    No,
}
```

### 2.4 State Transitions

**Voting eligibility validation flow:**
1. On GovernanceProposeTx receipt, snapshot all active validators at inclusion height.
2. For each GovernanceVoteTx received: verify validator_id is in snapshot, verify vote weight equals bonded_stake at snapshot height, verify signature.
3. Vote recorded against (proposal_id, voter_id). Later votes overwrite earlier ones.
4. At proposal close height: compute quorum from total bonded stake in snapshot; compare total yes/no votes. Proposal passes if yes_weight > no_weight AND total_votes > quorum.

### 2.5 Failure Behavior

- Vote from ineligible validator (not active at snapshot) is rejected.
- Vote weight computed by validator but verified by state machine against snapshot bonded_stake.
- Double-vote: the later vote overwrites the earlier one for the same (proposal_id, voter_id).

### 2.6 Versioning and Compatibility

- Governance vote snapshot mechanics versioned in system parameters.
- Vote weight computation is deterministic and tied to state root at snapshot height.
- Breaking changes to eligibility rules require governance proposal.

### 2.7 Conformance Test Hooks

- Verify vote weight exactly equals bonded_stake at snapshot height.
- Verify paused, unbonding, and withdrawn validator votes are rejected.
- Verify no-vote (timeout) does not count toward quorum and does not penalize.

### 2.8 Trust-Assumption Inventory

- Snapshot data availability
  - Justification: All nodes recompute vote eligibility from their own SMT state; no external snapshot required.
  - Trust-minimised alternative: None — on-chain deterministic snapshots.
