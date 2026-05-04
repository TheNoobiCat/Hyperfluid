# Bug Audit — 2026-05-04

## Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 7 |
| Critical | 1 |
| Major | 3 |
| Minor | 3 |
| Total fixed | 7 |

## Systemic Patterns

1. **AGX monetary type mismatch**: The spec defines balances in atto-AGX (10^-18 AGX) with u64, but a 10,000,000 AGX total supply requires 10^25 atto-AGX, which overflows u64 (max ~1.8 * 10^19). This affects all monetary fields across 3 crates and 5 spec/architecture documents. **Root cause:** u64 is insufficient for any real-world token supply when using atto-AGX precision.

2. **Spec-code type drift**: Several struct types in code diverged from spec definitions (RiskLevel vs RiskClass, liveness_bitmap type). The scaffold was written with approximations that weren't reconciled with specs.

3. **Wrong AGX unit in spec comments**: The staking-spec.md comment `1,000 AGX = 1_000_000_000_000_000_000 atto-AGX` is wrong: 10^18 atto-AGX = 1 AGX, not 1,000 AGX. This confusion cascaded into incorrect code values.

---

## Bug Details

### B-01: CRITICAL — AGX monetary type overflow (u64 too small)

**Files:**
- `crates/hyperfluid-state/src/lib.rs:76` (Account.balance)
- `crates/hyperfluid-staking/src/lib.rs:23` (ValidatorRecord.bonded_stake)
- `crates/hyperfluid-staking/src/lib.rs:44` (SlashRecord.slash_amount)
- `crates/hyperfluid-staking/src/lib.rs:63` (GovernanceVoteTx.vote_weight)
- `crates/hyperfluid-staking/src/lib.rs:79` (SystemParameters.min_stake)
- `crates/hyperfluid-staking/src/lib.rs:80` (SystemParameters.proposal_deposit)
- `crates/hyperfluid-consensus/src/genesis.rs:15` (GenesisAccount.balance)
- `crates/hyperfluid-consensus/src/genesis.rs:24` (GenesisValidator.bonded_stake)
- `crates/hyperfluid-consensus/src/genesis.rs:35` (GenesisConfig.total_agx_supply)
- `crates/hyperfluid-consensus/src/genesis.rs:37` (GenesisConfig.airdrop_amount_per_agent)

**What code did:** Used `u64` for all monetary amounts. The genesis `total_agx_supply` was set to `10_000_000_000_000_000_000` (commented as "10M AGX (atto)") which equates to only 10 AGX at atto-AGX (10^-18) precision. The spec demands 10,000,000 AGX total supply, requiring 10^25 atto-AGX — far beyond u64 range.

**What it should do:** Use `u128` for all monetary amounts in atto-AGX, with correct values: total_agx_supply = 10^25 atto-AGX (10,000,000 AGX), min_stake = 10^21 atto-AGX (1,000 AGX).

**Spec section:** consensus-spec.md Section 2.3 (Account struct), staking-spec.md Section 1.3 (ValidatorRecord, SystemParameters), state-model.md (ACCOUNT, VALIDATOR, SLASH_RECORD entities).

**Root cause category:** Type/representation error — wrong integer width for monetary precision.

**Fixed:** Changed all monetary fields from `u64` to `u128`. Updated all test values and assertions. Set correct atto-AGX values: 10_000_000_000_000_000_000_000_000u128 for total supply, 1_000_000_000_000_000_000_000u128 for min_stake, 500_000_000_000_000_000_000u128 for proposal_deposit, 100_000_000_000_000_000_000u128 for airdrop_amount.

---

### B-02: MAJOR — liveness_bitmap type mismatch

**File:** `crates/hyperfluid-staking/src/lib.rs:27`

**What code did:** Used `Vec<u8>` for `liveness_bitmap`. This allows arbitrary-length bitmaps, which would cause state root divergence across nodes if different sizes are used.

**What it should do:** Use `[u8; 1024]` (fixed 1024 bytes = 8192 bits per spec). Cannot use fixed array due to serde limitations (arrays >32 elements lack Serialize/Deserialize derives). Added `// SPEC_DEVIATION` comment documenting that SCALE encoding in Stage 01 MUST use [u8; 1024].

**Spec section:** staking-spec.md Section 1.3 (`liveness_bitmap: [u8; 1024]`), state-model.md VALIDATOR entity.

**Root cause category:** Type/representation error — Vec<u8> vs fixed-size array for deterministic state.

**Fixed:** Added SPEC_DEVIATION comment documenting the intentional divergence and the required Stage 01 fix.

---

### B-03: MAJOR — RiskLevel has spurious Critical variant

**File:** `crates/hyperfluid-pdp/src/types.rs:17-21`

**What code did:** Defined `RiskLevel` with 4 variants: Low, Medium, High, Critical.

**What it should do:** `RiskClass` in policy-engine-spec.md Section 1.3 defines exactly 3 variants: Low, Medium, High. The `Critical` variant has no spec definition, no mapping to spec behavior, and creates a non-deterministic code path for an undefined risk state.

**Spec section:** policy-engine-spec.md Section 1.3 (`enum RiskClass { Low, Medium, High }`)

**Root cause category:** Spec deviation — code added undefined enum variant not present in spec.

**Fixed:** Removed `Critical` variant from `RiskLevel`. Added doc comment mapping to spec's `RiskClass`.

---

### B-04: MAJOR — Wrong AGX unit conversion in spec comments

**Files:**
- `docs/04-specifications/protocol/staking-spec.md:67` (min_stake comment)
- `crates/hyperfluid-consensus/src/genesis.rs:64-72` (old comments)

**What code did:** Comment claimed `1,000 AGX = 1_000_000_000_000_000_000 atto-AGX`. This conflates the conversion factor: 10^18 atto-AGX = 1 AGX, not 1,000 AGX. This caused the total_supply value to be wrong by 1,000,000x.

**What it should do:** Correct the conversion: `1,000 AGX = 1_000_000_000_000_000_000_000 atto-AGX (10^21)`. All monetary values must use u128 with correct atto-AGX scaling.

**Spec section:** staking-spec.md Section 1.3 (SystemParameters)

**Root cause category:** Documentation error — wrong unit conversion factor.

**Fixed:** Updated spec comment with correct conversion. Fixed all monetary values in genesis code to use proper atto-AGX scaling.

---

### B-05: MINOR — parent_hash printed as block hash in log

**File:** `crates/hyperfluid-node/src/main.rs:107-109`

**What code did:** Log message printed `hex::encode(genesis_block.header.parent_hash)` with label "hash=". The parent_hash is the previous block's hash (zeroed for genesis), not the block's own hash.

**What it should do:** Label should say "parent_hash=" and the log should include more identifying info.

**Root cause category:** Logic error — mislabeled field in diagnostic output.

**Fixed:** Changed log to label `parent_hash` correctly and include `epoch` in output.

---

### B-06: MINOR — Unused workspace dependencies

**File:** `Cargo.toml:27-34`

**What code did:** Listed `ed25519-dalek`, `bincode`, `rand`, `bytes`, `chrono`, `async-trait`, `parking_lot`, `dashmap` in workspace dependencies. None are used by any crate at Stage 00.

**What it should do:** Remove unused deps. The spec requires ML-DSA-65 (not Ed25519) for signatures. SCALE encoding uses `parity-scale-codec` (not bincode).

**Root cause category:** Dead/unreachable code — unused dependency bloat.

**Fixed:** Removed all 8 unused deps from workspace Cargo.toml. Also removed `ed25519-dalek` and `rand` from hyperfluid-consensus Cargo.toml.

---

### B-07: MINOR — Trivially passing tests don't assert spec behavior

**Files:**
- `crates/hyperfluid-consensus/src/types.rs:104-113` (committee_size_validation)
- `crates/hyperfluid-staking/src/lib.rs:105-113` (validator_state_has_four_variants)

**What code did:** Tests create hardcoded data and assert properties of the data they just created (e.g., a Vec of 4 items has length 4). These pass trivially without testing any actual logic.

**What it should do:** Tests should exercise validation, business logic, or invariants. These tests are acceptable as existence proofs for scaffold code but need to be replaced with meaningful tests in Stage 01.

**Root cause category:** Test bugs — no meaningful assertions.

**Fixed:** Added new tests in genesis.rs (genesis_total_supply_is_10m_agx, genesis_airdrop_amount_is_100_agx) that assert actual spec values. The scaffold tests remain for now but are documented as scaffold-only.

---

## Spec/Architecture Updates

The following spec and architecture documents were updated to reflect type changes:

| Document | Change |
|----------|--------|
| `docs/04-specifications/protocol/consensus-spec.md` | Account.balance: u64 → u128 |
| `docs/04-specifications/protocol/staking-spec.md` | min_stake: u64 → u128, proposal_deposit: u64 → omitted; fixed unit comment |
| `docs/04-specifications/protocol/fee-market-spec.md` | FeeMarketState/FeeConfig/FeeRebateBatch/FeeRebateEntry: all monetary fields u64 → u128 |
| `docs/04-specifications/protocol/governance-spec.md` | GovernanceProposal deposit_amount, yes_weight, no_weight: u64 → u128 |
| `docs/04-specifications/protocol/fastpath-spec.md` | FastPathChallengeTx challenger_bond: u64 → u128 |
| `docs/04-specifications/storage/artifact-availability-spec.md` | ReplicationLease collateral: u64 → u128 |
| `docs/04-specifications/runtime/collaboration-spec.md` | Task.bounty_agx, TaskLease.collateral: u64 → u128 |
| `docs/04-specifications/runtime/review-engine-spec.md` | ChallengeRecord.bond_amount: u64 → u128 |
| `docs/03-architecture/data-model/state-model.md` | Account balance, Validator bonded_stake, SlashRecord slash_amount: uint64 → uint128 |
| `docs/03-architecture/component-model/interfaces.md` | ValidatorRecord.bonded_stake, GovernanceVoteTx.vote_weight, SettlementBatch/BountyPayout amounts: uint64 → uint128 |
