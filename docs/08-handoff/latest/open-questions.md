# Session 2026-05-06 — Open Questions

## Q1: Bootstrap committee size / three-tier stall at genesis

**Spec section:** `consensus-spec.md` Section 1.2 (three-tier liveness) + Section 1.4 (committee sampling)

**What is missing:** The three-tier model (Normal/Degraded/Emergency) requires 67+ validators for Normal and 50+ for Degraded. At genesis, the chain starts with 1 validator (testnet-single) or a handful of validators. With <50 validators, the chain enters Emergency mode immediately and block production halts — the chain cannot bootstrap.

**Affects:** `hyperfluid-consensus/src/types.rs` (`committee_mode`, `can_produce`), `hyperfluid-node/src/main.rs` (genesis consensus loop), `genesis.rs` (committee_size = 100 vs actual validator count of 1)

**Blocks current task?** Yes — a single-node testnet cannot produce blocks. This blocks Week 1-2 verification ("single-node testnet produces blocks") and any integration work on Week 3-4.

**Proposed resolution options:**
- **Option A: Bootstrap mode.** Epoch 0 (genesis) uses `min(committee_size, validator_count)` as the effective committee size. Thresholds scale proportionally: `threshold_effective = ceil(threshold * active_validators / committee_size)`. After epoch 0, full thresholds apply and new validators must bond to reach Normal mode. This is the least-invasive fix and matches how real PoS chains (Cosmos, Polkadot) bootstrap.
- **Option B: Genesis validator minimum.** Require genesis configs to have at least 67 validators. Reject smaller configs. This breaks the single-node testnet scaffold (which is essential for development).
- **Option C: Override Emergency at genesis.** Hardcode an exception: Emergency mode does not halt blocks if `epoch == 0`. This is trivial but doesn't scale to the 3-5 validator early days.

**Recommendation:** Option A. Add `effective_committee_size` and scaled thresholds to `Committee`, with a `SPEC_DEVIATION` flag noting that this is a bootstrap accommodation pending formal spec revision.

**Status:** PENDING spec revision. To be implemented as SPEC_DEVIATION in the next session.