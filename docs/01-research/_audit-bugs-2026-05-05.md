# Bug Audit — 2026-05-05

## Summary

| Metric | Value |
|--------|-------|
| Total bugs found | 8 |
| Major | 4 |
| Medium | 2 |
| Minor | 2 |
| Total fixed | 8 |

## Systemic Patterns

1. **Incomplete B-01 migration**: The 2026-05-04 bug audit correctly identified the u64→u128 monetary type migration and fixed it in code and spec structs, but missed 11 monetary fields in `docs/03-architecture/data-model/state-model.md` that remained as `uint64`. **Root cause:** The architecture data model document was partially updated — Account.balance, Validator.bonded_stake, and SlashRecord.slash_amount were fixed, but GOVERNANCE_PROPOSAL, GOVERNANCE_VOTE, REPLICATION_LEASE, AIRDROP_POOL, and SYSTEM_PARAMETERS monetary fields were overlooked.

2. **f64 in deterministic context**: Two independent uses of `f64` were found in spec data structures that must be deterministic across all nodes. In `policy-engine-spec.md`, `QuotaEntry.stage_multipliers` used `[(TrustStage, f64); 4]` in the PDP quota matrix — the PDP rule chain explicitly forbids floating point. In `collaboration-spec.md`, `ReputationVector` used `f64` fields — this state lives in the SMT and must produce identical roots on all nodes. **Root cause:** f64 is convenient for documentation but is non-deterministic across platforms. Should use rational pairs or u8 scaling for on-chain state.

3. **Requirement count drift after amendments**: The 2026-05-05 amendments added FR-0194–FR-0200 (7 new FRs), bringing the total to 202. Architecture documents (`index.md`, `components.md`) still referenced the pre-amendment count of 195. **Root cause:** The architecture layer was not re-audited after the Layer 2 amendments were propagated through Layers 4 and 5.

---

## Bug Details

### DB-01: MAJOR — state-model.md monetary fields still uint64 after B-01 fix

**Files:** `docs/03-architecture/data-model/state-model.md`

**What the doc had:** 11 monetary fields across 5 entities remained as `uint64` after the B-01 migration:
- GOVERNANCE_PROPOSAL.deposit_amount: uint64
- GOVERNANCE_PROPOSAL.yes_weight: uint64
- GOVERNANCE_PROPOSAL.no_weight: uint64
- GOVERNANCE_VOTE.vote_weight: uint64
- REPLICATION_LEASE.collateral: uint64
- AIRDROP_POOL.total_allocated: uint64
- AIRDROP_POOL.remaining: uint64
- SYSTEM_PARAMETERS.min_stake: uint64
- SYSTEM_PARAMETERS.proposal_deposit: uint64
- SYSTEM_PARAMETERS.airdrop_amount: uint64
- SYSTEM_PARAMETERS.airdrop_pool_total: uint64

**What it should be:** All monetary fields in atto-AGX must use `uint128` for consistency and to prevent overflow at 10^25 atto-AGX total supply.

**Spec reference:** B-01 audit, consensus-spec.md Section 2.3 (Account struct), staking-spec.md Section 1.3 (SystemParameters)

**Root cause category:** Type/representation error — incomplete migration from prior fix.

**Fix:** Changed all 11 fields from `uint64` to `uint128`.

---

### DB-02: MAJOR — Missing traceability-matrix.md

**Files:** `docs/08-handoff/latest/traceability-matrix.md` (does not exist)

**What should exist:** BUILD-SYSTEM.md §Traceability requires `docs/08-handoff/latest/traceability-matrix.md` (updated at each checkpoint). No such file exists anywhere in the project.

**What it should contain:** Tabular traceability matrix with one row per claim: Research → Requirement → Architecture Decision → Specification → Test Case → Implementation. Bidirectional links between all layers.

**Spec reference:** BUILD-SYSTEM.md lines 107-114.

**Root cause category:** Implementation gap — required artifact not created.

**Fix:** Created `docs/08-handoff/latest/traceability-matrix.md` with FR → spec, NFR, and ADR traceability.

---

### DB-03: MAJOR — f64 in PDP QuotaEntry (determinism violation)

**Files:** `docs/04-specifications/runtime/policy-engine-spec.md`

**What the doc had:** `QuotaEntry.stage_multipliers` defined as `[(TrustStage, f64); 4]`. Floating-point arithmetic is non-deterministic across platforms and compilers. The PDP spec (Section 1.2) explicitly mandates: "The PDP MUST NOT contain probabilistic logic in the root authorization path" and "The PDP MUST produce identical decisions for identical inputs on all nodes."

**What it should be:** Use a rational pair `(u64, u64)` representing numerator/denominator for deterministic stage multiplier computation.

**Spec reference:** policy-engine-spec.md Section 1.2 (PDP determinism), Section 2.3 (QuotaEntry).

**Root cause category:** Type/representation error — f64 in deterministic context.

**Fix:** Changed `[(TrustStage, f64); 4]` to `[(TrustStage, (u64, u64)); 4]` with comment documenting rational pair convention.

---

### DB-04: MAJOR — f64 in ReputationVector (on-chain state non-determinism)

**Files:** `docs/04-specifications/runtime/collaboration-spec.md`

**What the doc had:** `ReputationVector` with four `f64` fields for delivery_quality, review_reliability, liveness, and safety. These values are stored on-chain in the SMT as `reputation_vector` per TRUST_STAGE entity (state-model.md §3). Floating-point representation would cause SMT root divergence across nodes.

**What it should be:** Use `u8` scaled 0-255 (where 0=0%, 255=100%) for deterministic cross-platform representation. Conversion at computation boundaries uses rational arithmetic.

**Spec reference:** collaboration-spec.md Section 3.3 (ReputationVector), state-model.md TRUST_STAGE entity.

**Root cause category:** Type/representation error — f64 in on-chain deterministic state.

**Fix:** Changed all four `f64` fields to `u8` with scaling documentation.

---

### DB-05: MINOR — policy-engine-spec.md section ordering error

**Files:** `docs/04-specifications/runtime/policy-engine-spec.md`

**What the doc had:** Section 2 subsections in wrong order: 2.5 Failure Behavior → 2.7 Conformance Test Hooks → 2.6 Versioning and Compatibility → 2.8 Trust-Assumption Inventory. A duplicate 2.6 section also existed after 2.7.

**What it should be:** Canonical order per TEMPLATES.md: 2.5 Failure Behavior → 2.6 Versioning and Compatibility → 2.7 Conformance Test Hooks → 2.8 Trust-Assumption Inventory. No duplicate sections.

**Spec reference:** TEMPLATES.md spec format (8 sections in canonical order).

**Root cause category:** Documentation error — structural misordering.

**Fix:** Reordered sections to canonical sequence. Removed duplicate 2.6 block.

---

### DB-06: MINOR — architecture/index.md requirement count out of date

**Files:** `docs/03-architecture/index.md`

**What the doc had:** "All 195 requirements map to at least one component. Zero orphans." (165 FR + 30 NFR). FR mapping table only went up to FR-0176-0193.

**What it should be:** 202 requirements (172 FR + 30 NFR) per the 2026-05-05 amendments (FR-0194–FR-0200 added). FR mapping table should include FR-0194-0200.

**Spec reference:** PROJECT-STATUS.md (202 requirements), checkpoint-2026-05-05c.md.

**Root cause category:** Documentation drift — architecture layer not updated after Layer 2 amendments.

**Fix:** Updated count from 195 to 202, added FR-0194-0200 mapping row, updated gate check from 195/195 to 202/202.

---

### DB-07: MINOR — components.md requirement count out of date

**Files:** `docs/03-architecture/component-model/components.md`

**What the doc had:** "All 195 FR/NFR requirements mapped to components".

**What it should be:** 202 FR/NFR.

**Spec reference:** Same as DB-06.

**Root cause category:** Documentation drift — same root cause as DB-06.

**Fix:** Updated count from 195 to 202.

---

### DB-08: MINOR — Spec headers missing FR-0194–0200 coverage

**Files:**
- `docs/04-specifications/protocol/consensus-spec.md` — missing FR-0194 (TaskCreateTx)
- `docs/04-specifications/runtime/agent-runtime-spec.md` — missing FR-0196, FR-0199, FR-0200
- `docs/04-specifications/runtime/collaboration-spec.md` — missing FR-0194, FR-0195, FR-0198

**What the doc had:** Header line listing Covered FRs did not include FRs added in the 2026-05-05c amendment, even though the spec body references them.

**What it should be:** Header line should list all covered FRs for completeness.

**Spec reference:** checkpoint-2026-05-05c.md (additions to consensus-spec.md, agent-runtime-spec.md, collaboration-spec.md).

**Root cause category:** Documentation drift — spec headers not updated when body sections were amended.

**Fix:** Added missing FR numbers to each spec's Covered FRs header line.

---

## Spec/Architecture Updates

| Document | Change |
|----------|--------|
| `docs/03-architecture/data-model/state-model.md` | 11 monetary fields: uint64 → uint128 (GOVERNANCE_PROPOSAL, GOVERNANCE_VOTE, REPLICATION_LEASE, AIRDROP_POOL, SYSTEM_PARAMETERS) |
| `docs/03-architecture/index.md` | Requirement count 195→202; FR mapping extended; gate check updated |
| `docs/03-architecture/component-model/components.md` | Requirement count 195→202; tool list updated for 9 tools |
| `docs/04-specifications/runtime/policy-engine-spec.md` | QuotaEntry.stage_multipliers f64→rational pair; section ordering fixed; duplicate section removed |
| `docs/04-specifications/runtime/collaboration-spec.md` | ReputationVector f64 fields→u8 scaled; FR coverage header updated |
| `docs/04-specifications/protocol/consensus-spec.md` | FR coverage header updated (FR-0194 added) |
| `docs/04-specifications/runtime/agent-runtime-spec.md` | FR coverage header updated (FR-0196, FR-0199, FR-0200 added) |
| `docs/08-handoff/latest/traceability-matrix.md` | Created (was missing) |
