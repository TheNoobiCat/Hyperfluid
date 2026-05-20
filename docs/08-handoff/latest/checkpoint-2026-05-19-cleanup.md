# Checkpoint: Overengineering Cleanup + Build Process Hardening

**Date:** 2026-05-19
**Trigger:** Bug audit / architecture review — identified vaporware patterns in code and spec.

## Summary

This session removed approximately 1,300 lines of overengineered stub code, 7 deleted spec/research docs, and hardened the build process to prevent future vaporware.

## Changes Made

### Deleted Code

| File | Lines | Feature | Reason |
|------|-------|---------|--------|
| `staking/src/graph.rs` | 455 | Stake-graph clustering (ClusterRecord, FundingEdge, N-hop walk, detect_clusters) | Overengineered — graph traversal in consensus at every epoch boundary. False positives from exchange wallets. Does not detect deeply hidden chains. |
| `pdp/src/key_rotation.rs` | 296 | Dual-key state model (initiate_key_rotation, KEY_ROTATION_GRACE_WINDOW) | Agents can create new accounts. 100-block grace window for no reason. |
| `consensus/src/types.rs` | ~100 | CommitteeMode, committee_mode, can_produce, scaled_thresholds, emergency_transition, compute_vdf_fallback, EMERGENCY_IDLE_BLOCKS, ShadowClaimTx | Three-tier liveness was never enforced. VDF was just SHA3-256. Shadow claims added a priority queue + state machine for zero benefit. |
| `state/src/lib.rs` | ~60 | ShadowClaim, LeasePenaltyLevel, TaskStatus::Decomposed, EscrowStatus::BountyRedistributed, parent_task_id, depends_on, timeout_count | Dead types with no behavioral code paths. Restored later as implemented features (split + review). |
| `state/src/state_machine.rs` | ~130 | execute_register_shadow, shadow promotion in run_lease_expiry, penalty schedule | Shadow claim priority queue + promotion algorithm + 3-tier penalty ladder in consensus state. |
| `economics/src/lib.rs` | 228 | compute_decentralization_metrics (depended on graph.rs types) | Empty after stake-graph deletion. |

### Deleted Docs

| File | Reason |
|------|--------|
| `sybil-detection-correlation-engine.md` | 5-signal pairwise O(n^2) engine with 30% detection rate. Academic wankery. |
| `proof-of-work-quality-and-review-markets.md` | Review pipeline spec (never had code). Rewritten as simpler review-as-task model. |
| `inbox-attention-control-and-anti-spam.md` | Priority scoring recommender system for agent inboxes. |
| `stake-graph-analysis-spec.md` | Canonical spec for deleted stake-graph clustering. |
| `ADR-0007-committee-bft-vdf.md` | ADR for VDF that was just SHA3-256. |
| `ADR-0017-ninety-ten-payout-split.md` | ADR for review payout (spec still exists, ADR was the only record of the decision). |

### Restored + Reimplemented as Real Code

| Feature | Previous State | Current State |
|---------|---------------|---------------|
| Task splitting | Dead types only (Decomposed, BountyRedistributed, parent_task_id, depends_on always zero/empty). No SplitTaskTx. | Full implementation: `execute_split_task()` with auth check, share sum validation, cycle detection (DFS), atomic child creation. Wired into driver. |
| Agent reviews | Pure spec-ware. `submit_completion` just flipped to Done. | Real: InReview status, 2 review tasks in pool, trusted-only claim, binary verdict tally, 90/10 payout settlement, review expiry. |
| Commit-reveal seed | Named "VDF" but was just SHA3-256. Three-tier liveness was enforced on nothing. | `compute_committee_seed()` — commit-reveal with <33% fallback. No VDF theater. Committee selection just uses SHA3-256(epoch \|\| seed). |
| Agent handoff prompt | `trigger_handoff()` saved JSON blob of internal state. Useless. | Now injects HANDOFF_REFLECTION_PROMPT into message log. LLM response becomes summary. |
| CLI spec in prompt | Only documented 3 commands. Agent had no idea tx/query/review/governance existed. | Full command tree with explanations for every group. |
| First-run onboarding | Nothing. Agent woke up with zero context. | 50-line FIRST_RUN_ONBOARDING block that appears once, explaining Hyperfluid, run-forever, economy-building, and first steps. |
| LLM provider | Hardcoded stub returning empty response. | Real OpenAI-compatible + Ollama providers via reqwest. Config-based switching. |

### Build Process Hardening

| File | Change |
|------|--------|
| `checkpoint.md` | Added vaporware guard: grep new types for non-test usage. Zero matches = block completion. |
| `week-completion.md` | Added stub audit step before CI. |
| `fill-gaps.md` | Added Phase 0 step h: type-to-behavior vaporware scan. |

## What is NOT a Gap Anymore

The following items were previously tracked as open gaps or work items. They are no longer relevant:

| Previous Gap | Status | Reason |
|-------------|--------|--------|
| FR-0060 Signed Telemetry | CLOSED — not needed | Telemetry is overengineered. Deleted from all plans. The telemetry-spec.md never existed as a file. |
| Sybil detection correlation engine | CLOSED — not needed | 5-signal O(n^2) engine with 30% detection rate. Anti-Sybil via Proof-of-Agent puzzle + bond + trust ladder is sufficient. |
| Operator-cluster diversity for reviews | CLOSED — not needed | Reviews use a simple trust-stage gate + abuse flag system. No stake-graph analysis. |
| Review challenge windows + arbiter panels | CLOSED — not needed | Reviews settle immediately on majority verdict. Fast-path has its own challenge mechanism for topic merges. |
| VDF integration | CLOSED — replaced | Commit-reveal seed via SHA3-256 is sufficient. No Wesolowski VDF crate needed. |
| Three-tier liveness (Degraded/Emergency) | CLOSED — not needed | Binary: validators produce blocks or they don't. No critical-tx-only filtering. |
| Consensus liveness tiers | CLOSED — not needed | Same as above. BFT consensus handles liveness via standard timeout mechanisms. |
| Shadow claims + penalty schedule | CLOSED — not needed | Lease expiry returns task to pool. No priority queue or 3-tier penalty schedule needed. |
| Key rotation with 100-block grace window | CLOSED — not needed | Agents can create new accounts. No dual-key state machine in consensus. |
| Mempool lanes | CLOSED — removed | Single fee-ordered pool with evidence/governance fee discounts. |
| Circuit breaker hierarchy | CLOSED — removed | EIP-1559 base fee is the sole congestion mechanism. |
| Quality-weighted scoring | CLOSED — removed | Fixed 90/10 payout. No ML weights or scoring curves. |

## What REMAINS to Build (in priority order)

The stage-02-agent-runtime.md has been updated with the real remaining work:

1. **Week 5-6: Wire P2P + Mempool + PDP**
   - TCP listener + peer connections from `main.rs`
   - Mempool into `produce_block()` (real tx selection)
   - PDP context state (agent key refs, nonces, quotas)

2. **Week 7-8: BFT Consensus Integration**
   - Malachite effect handler, Clatter network bridge, host actor (~1,200 lines)
   - Replace local block production with BFT propose/vote/commit
   - Equivocation detection, state sync

3. **Week 9-10: Real PDP Sig Verify + CLI + Slashing**
   - ML-DSA-65 signature verification in PDP step 2
   - `hyperfluid` CLI binary with 43 subcommands
   - Slashing/reward distribution

## CI Status

`cargo fmt --check` — PASS
`cargo clippy` — zero warnings (clean)
`cargo test --workspace` — all tests pass (0 failures)
`cargo doc` — PASS
`cargo deny check` — PASS
