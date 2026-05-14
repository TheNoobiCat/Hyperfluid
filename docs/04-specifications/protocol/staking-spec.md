# Protocol Spec: Staking & Validator Manager

**Components:** C3 Staking & Validator Manager
**Source ADRs:** ADR-0007 (Committee BFT with VDF), ADR-0010 (Two-Stage Trust Ladder)
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
    self_bond: u128,                // validator's own bonded AGX (atto-AGX)
    total_delegated: u128,          // total AGX delegated by others (atto-AGX)
    commission_rate: u8,            // 0-100, percent commission on delegation rewards
    bonding_height: u64,
    unbonding_height: u64,
    jail_until_height: u64,         // 0 if not jailed
    liveness_counter: u32,            // rolling missed-block counter
    slash_count: u32,
    missed_blocks: u32,
    last_renew_height: u64,
}

struct DelegationRecord {
    delegator_id: [u8; 32],         // account delegating AGX
    validator_id: [u8; 32],         // validator receiving delegation
    amount: u128,                   // atto-AGX delegated
    unbonding_at_height: u64,       // 0 if not unbonding, height if unbond init
    status: DelegationStatus,
}

enum DelegationStatus {
    Active,
    Unbonding,
    Withdrawn,
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
    slash_amount: u128,
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
    min_self_bond: u128,              // 1,000 AGX = 10^21 atto-AGX (validator's own stake)
    min_delegation: u128,             // 1 AGX = 10^18 atto-AGX (minimum delegation amount)
    max_commission_rate: u8,          // 20 (20% max; governance-adjustable 5-50%)
    delegation_unbond_delay: u64,     // 60,480 blocks (7 days)
    bond_delay: u64,                  // 8640 blocks (~24 hrs at 10s/block)
    unbond_delay: u64,                // 120,960 blocks (14 days)
    max_governance_proposals: u64,    // 32
    proposal_deposit: u128,           // 500 AGX
    liveness_window_blocks: u64,      // 8192
    liveness_miss_threshold_pct: u8,  // 20 (20%)
}
```

### 1.4 State Transitions

**Canonical transition graph (using generalized StakingTx and DelegationTx with action sub-enums):**

```
StakingTx(Bond) (+1 epoch wait) ──► active
active ──(miss >20% in window)──► paused [0.1% slash]
paused ──(StakingTx(Renew) + 1 epoch wait)──► active
active ──(StakingTx(Unbond))──► unbonding [14-day timer]
paused ──(StakingTx(Unbond))──► unbonding [14-day timer]
unbonding ──(unbond_delay expires + StakingTx(Withdraw))──► withdrawn
active ──(equivocation proof)──► paused [10% slash + 30-day jail]

--- Delegation transitions ---
DelegationTx(Delegate) ──► DelegationRecord.active (if validator is active, amount <= delegator balance)
DelegationTx(Undelegate) ──► DelegationRecord.unbonding (7-day timer)
DelegationTx(WithdrawDelegation) (after unbonding) ──► DelegationRecord.withdrawn + funds returned
DelegationTx(SetCommission) ──► ValidatorRecord.commission_rate updated (after 2 epochs)
```

**Transition details:**

| Trigger | From | To | Conditions |
|---------|------|----|------------|
| StakingTx(Bond) >= 1,000 AGX (self_bond) | (new) | active | After bond_delay (8,640 blocks) |
| Miss >20% blocks in liveness window | active | paused | 0.1% slash; repeated within 3 windows escalates to 1% |
| StakingTx(Renew) | paused | active | 1 epoch wait (8,192 blocks) after tx inclusion |
| StakingTx(Unbond) | active/paused | unbonding | 14-day timer starts at inclusion height |
| StakingTx(Withdraw) | unbonding | withdrawn | unbond_delay expired; funds released |
| Equivocation evidence | active | paused | 10% slash; jailed 30 days (259,200 blocks) |
| DelegationTx(Delegate) >= 1 AGX | delegator | DelegationRecord.active | Validator must be active; amount deducted from delegator balance |
| DelegationTx(Undelegate) (any amount) | delegator | DelegationRecord.unbonding | 7-day timer starts; amount still slashable |
| DelegationTx(WithdrawDelegation) | delegator | DelegationRecord.withdrawn | After delegation_unbond_delay (7 days); funds returned |
| DelegationTx(SetCommission) (0-20%) | validator | ValidatorRecord.commission_rate | Effective after 2 epochs (buffer for delegator reaction) |

**Liveness tracking:** A rolling counter of missed blocks per validator. The counter increments when a validator misses a block in the liveness window and resets on renewal. At >20% miss rate within the window, the validator transitions to paused.

**StakingTx(Renew):** This is the exclusive mechanism to resume from paused. No separate ResumeTx exists. The tx contains the validator's signature and takes effect after 1 epoch.

### 1.5 Failure Behavior

- **Insufficient stake (self-bond):** StakingTx(Bond) with self_bond amount < 1,000 AGX is rejected at admission.
- **Insufficient delegation:** DelegationTx(Delegate) with amount < 1 AGX is rejected at admission.
- **Double-bind:** StakingTx(Bond) from an account that already has a VALIDATOR in non-withdrawn state is rejected.
- **Delegation to inactive validator:** DelegationTx(Delegate) to a validator not in `active` state is rejected.
- **Self-delegation:** An account cannot delegate to itself; StakingTx(Bond) is the mechanism for self-staking.
- **Premature withdrawal:** StakingTx(Withdraw) before unbond_delay expiry is rejected.
- **Premature delegation withdrawal:** WithdrawDelegationTx before delegation_unbond_delay expiry is rejected.
- **Delayed evidence:** Equivocation evidence submitted more than 8,640 blocks after the event cancels the slash but marks the validator for governance review.
- **Invalid evidence:** EvidenceTx with non-verifiable proof is rejected; reporter may be penalized for repeated false submissions.
- **Committee eligibility during unbonding:** Unbonding validators remain committee-eligible until the epoch boundary after unbond initiation, then are excluded.
- **Slashing propagation:** On validator slash, each delegator's stake is reduced by `delegated_amount * (slash_pct / 100)`. The slash is applied proportionally across all delegators and the self-bond. The slash fires even if the delegator is in unbonding status (same as validator unbonding slashing).
- **Commission abuse:** A validator setting commission rate > max_commission_rate (20%) is rejected at admission. Rate changes take 2 epochs to allow delegators to undelegate before the new rate applies.

### 1.6 Versioning and Compatibility

- ValidatorRecord schema version tracked in system parameters.
- Breaking changes to lifecycle states require governance proposal.
- Liveness window size and slashing parameters are governance-adjustable within defined bounds.

### 1.7 Conformance Test Hooks

- Verify only four states are valid; any other state transition is rejected.
- Verify StakingTx(Bond) with < 1,000 AGX is rejected.
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
    vote_weight: u128,          // bonded_stake at snapshot in atto-AGX
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
