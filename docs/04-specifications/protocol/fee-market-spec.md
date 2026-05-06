# Protocol Spec: Fee Market

**Component:** C5 Fee Market
**Source ADRs:** ADR-0009 (EIP-1559 Fee Market), ADR-0012 (Circuit-Breaker Hierarchy)
**Covered FRs:** FR-0146, FR-0147, FR-0159, FR-0160
**Dependencies:** C1 Consensus Engine, C3 Staking & Validator Manager

---

## Section 1: EIP-1559 Dynamic Fee Market

### 1.1 Purpose

Define the EIP-1559 style dynamic fee market for transaction pricing, spam prevention, and validator compensation.

### 1.2 Normative Behavior

- The system MUST implement a dynamic fee market with base fee adjustment per block based on prior block utilization.
- The system MUST support priority fees for ordering preference (higher priority fee = earlier inclusion).
- The base fee portion of each transaction MUST be burned (permanently removed from circulation).
- The priority fee portion MUST be distributed to the block proposer.
- Base fee adjustment per block MUST be capped at +/- 12.5% of the previous base fee.
- The block gas target MUST be set such that normal utilization is 50% of block capacity.
- Base fee MUST increase when the prior block is above target (>50% full) and decrease when below target.
- A minimum fee floor MUST prevent spam during zero or low demand periods.
- Per-sender mempool transaction limits MUST prevent a single actor from filling blocks.
- Fee burns SHALL NOT be used as protocol reward mechanisms; they are deflationary pressure only.

### 1.3 Data Structures

```rust
struct FeeMarketState {
    base_fee: u128,                  // atto-AGX per unit
    block_utilization_history: Vec<u8>,  // rolling window of utilization percentages
    fee_burn_accumulator: u128,      // total AGX burned in atto-AGX
    min_fee_floor: u128,             // absolute minimum fee in atto-AGX
}

struct FeeConfig {
    target_utilization_pct: u8,      // 50 (%)
    max_adjustment_pct: u8,          // 12.5 (%)
    min_base_fee: u128,              // floor in atto-AGX — [TUNE] default 1_000_000 atto-AGX
    max_per_sender_tx: u32,          // 100 per block
}
```

### 1.4 State Transitions

**Base fee adjustment algorithm (per block):**

```
if current_block.utilization > target_utilization:
    delta = base_fee * (utilization - target) / target / adjustment_denominator
    new_base_fee = min(base_fee + delta, base_fee * (1 + max_adjustment_pct))
elif current_block.utilization < target_utilization:
    delta = base_fee * (target - utilization) / target / adjustment_denominator
    new_base_fee = max(base_fee - delta, base_fee * (1 - max_adjustment_pct), min_fee_floor)
else:
    new_base_fee = base_fee
```

Where `adjustment_denominator = 8` (smooth adjustment rate).

**Transaction admission:**
1. Transaction must pay `(base_fee + priority_fee) * tx_size` in total.
2. Transactions with `max_fee < base_fee` are rejected at admission.
3. Mempool ordered by priority_fee descending.
4. Block proposer selects transactions up to block gas target.
5. Excess priority_fee (max_fee - base_fee - minimum_inclusion_bid) is refunded.

### 1.5 Failure Behavior

- **Fee manipulation:** Base fee capped at 12.5% increase per block prevents rapid price inflation by wealthy actors.
- **Fee collapse:** Minimum fee floor prevents zero-fee spam even during zero demand.
- **Mempool saturation:** Per-sender transaction limits prevent single actor from filling blocks.
- **Starvation:** Evidence and governance fee discounts prevent critical transaction starvation (see p2p-wire-spec.md §2 for fee discount mechanism).

### 1.6 Versioning and Compatibility

- Fee algorithm parameters are governance-adjustable via `git:head` update.
- FeeConfig stored in SMT system parameters (key prefix 0x08).
- Base fee adjustment formula versioned in policy bundle.

### 1.7 Conformance Test Hooks

- Verify base fee increases when block utilization exceeds target and decreases when below.
- Verify base fee change per block never exceeds 12.5%.
- Verify minimum fee floor is enforced.
- Verify base fee portion of transaction is burned.
- Verify priority fee goes to block proposer.
- Verify per-sender mempool limit is enforced.
- Verify basic fee adjustments produce expected direction and magnitude.

### 1.8 Trust-Assumption Inventory

- Fee parameter bounds integrity
  - Justification: Governance could set destructive fee parameters if bounds are wrong.
  - Trust-minimised alternative: Hard-coded absolute maxima for fee parameters; governance can only adjust within bounds.
- Fee market responsiveness
  - Justification: Fee adjustment denominator of 8 may be too slow or too fast; requires testnet calibration.
  - Trust-minimised alternative: Governance-adjustable adjustment rate with bounded range.

---

## Section 2: Validator Fee Rebates

### 2.1 Purpose

Define the validator fee rebate distribution mechanism.

### 2.2 Normative Behavior

- Fee rebates MUST be distributed to staked validators proportionally to their bonded stake.
- Rebates MUST be computed and distributed each epoch.
- Rebate distribution MUST be automatic; no claim transaction required.
- Only validators in `active` state during the epoch SHALL receive rebates.

### 2.3 Data Structures

```rust
struct FeeRebateBatch {
    epoch: u64,
    total_priority_fees: u128,          // total priority fees collected during epoch in atto-AGX
    distributions: Vec<FeeRebateEntry>,
}

struct FeeRebateEntry {
    validator_id: [u8; 32],
    bonded_stake: u128,
    rebate_amount: u128,                // proportional to stake share in atto-AGX
}
```

### 2.4 State Transitions

At each epoch boundary:
1. Compute total priority fees collected during the epoch.
2. For each active validator, compute rebate = total_priority_fees * (validator.bonded_stake / total_bonded_stake).
3. Credit rebate to validator account balance.
4. Record rebate distribution in FeeRebateBatch.
5. Reset epoch fee counters.

### 2.5 Failure Behavior

- Validator in paused/unbonding state during epoch receives no rebate.
- Validator slashed during epoch still receives rebate for blocks validated before slash.
- Fractional atto-AGX: any remainder from integer division is added to the fee_burn_accumulator.

### 2.6 Versioning and Compatibility

- Fee rebate computation formula versioned in system parameters.
- Rebate distribution epoch boundary execution is deterministic; all nodes compute identical rebates.
- Breaking changes to rebate formula require governance proposal.

### 2.7 Conformance Test Hooks

- Verify rebate amount proportional to bonded_stake / total_bonded_stake.
- Verify rebate credited automatically at epoch boundary.
- Verify paused validators receive no rebate.

### 2.8 Trust-Assumption Inventory

- Validator rebate fairness
  - Justification: Fee rebates are computed from on-chain priority fee totals. Validators cannot inflate own rebates without consensus.
  - Trust-minimised alternative: On-chain fee accounting with Merkle proof of per-block fee totals.
- Stake-proportional reward liveness
  - Justification: Validators receive rebates proportionally; large stakers earn proportionally more. This is by design, not a trust assumption.
  - Trust-minimised alternative: Flat rebate with stake-capped multiplier to prevent concentration.
