## ADR-0015: Stake Delegation

**Status:** accepted

**Context:** The staking model is pure self-staking: validators bond their own 1,000+ AGX with no delegation abstraction. The airdrop gives 100 AGX, meaning new agents must earn 10x their airdrop before they can become validators. This produces a tiny validator pool, making committee diversity meaningless and the anti-split clustering (see `stake-graph-analysis-spec.md`) ineffective against concentration. Real PoS systems (Cosmos, Cardano, Polkadot, Ethereum) succeed because delegation allows small holders to participate in security collectively.

**Decision:** Add on-chain stake delegation. Validators self-bond minimum 1,000 AGX (skin in the game). Any agent can delegate AGX to an active validator. Total effective stake (self-bond + delegated) determines committee selection weight. Delegation uses a proportional slashing model — delegators lose stake proportionally when the validator is slashed.

**Key parameters:**
- Minimum self-bond: 1,000 AGX (unchanged)
- Minimum delegation: 1 AGX
- Delegation unbonding period: 7 days (shorter than validator 14-day unbonding)
- Validator commission rate: max 20%, governance-adjustable within 5-50% bounds
- Commission change: rate changes apply after 2 epochs (buffer for delegators to react)
- Slashing propagation: delegator loses `delegated_amount * (slash_pct / 100)` proportional to their share

**Transaction types:**
- `DelegateTx(delegator, validator, amount)`: delegate AGX to validator
- `UndelegateTx(delegator, validator, amount)`: begin unbonding timer
- `WithdrawDelegationTx(delegator, validator)`: withdraw after unbonding delay
- `SetCommissionTx(validator, new_rate)`: set commission rate (max 20%)

**State:**
- `ValidatorRecord` gains: `commission_rate: u8` (0-100, percent), `total_delegated: u128`, `self_bond: u128`
- NEW: `DelegationRecord(delegator, validator, amount, unbonding_at_height, status, commission_share)`
- NEW: key prefix `0x0E` for DELEGATION in SMT

**Consequences:**
- Positive: Drastically larger validator pool. Small holders participate in security. Committee diversity improves. 15% cap removal compensated by broader natural stake distribution. Anti-split clustering still catches Sybil-funded validators.
- Negative: Added state complexity. Delegators must monitor validator behavior. Commission rate management adds governance surface. Slashing propagation may deter small delegators.
- Neutral: Default delegation strategy (stake-weighted random) for automated agents who don't want to choose manually.

**Alternatives considered:**
- **Pure self-staking (status quo):** Rejected because produces tiny validator pool, defeating committee diversity.
- **Liquid staking (Lido-style):** Rejected because adds systemic risk from derivative tokens and centralization pressure on the liquid staking provider.
- **Lower minimum stake (100 AGX):** Rejected because weakens economic security — 100 AGX is insufficient slash deterrent. Self-bond must be meaningful.
- **Delegation without commission cap:** Rejected because uncapped commission creates race-to-the-bottom extraction behavior.

**Related:** FR-0011, FR-0012, FR-0020
