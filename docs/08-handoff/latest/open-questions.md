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

**Status:** **RESOLVED** — Option A implemented in code. See `crates/hyperfluid-consensus/src/types.rs`:
- `committee_mode()` at line 57 uses `scaled_thresholds()` for proportional bootstrap scaling.
- `can_produce()` at line 75 uses the scaled degraded threshold.
- `scaled_thresholds()` at line 86 implements `threshold_effective = ceil(threshold * total_validators / COMMITTEE_SIZE)`.
- Flagged as `SPEC_DEVIATION` in code comments.

**Remaining:** Formal spec revision of `consensus-spec.md` §1.2 to document bootstrap scaling. Code is complete — do not re-implement.

---

## Q2: git:head upgrade — process lifecycle for node and agent

**Spec sections:** `governance-spec.md` Section 1 (On-Chain git:head Governance), `agent-runtime-spec.md` Section 4 (Process Isolation), `trust-boundaries.md` Section 3 (Zone Definitions), `interfaces.md` Section 7 (Transport Guarantees)

**What is missing:** How the node process and agent process actually handle a `git:head` transition. The governance spec defines the on-chain mechanics (vote → quorum → atomic epoch-boundary flip) but says nothing about what happens at the OS process level. The node (C1-C8) and agent runtime (C10-C11) are separate OS processes per ADR-0004. When `git:head` flips:

1. Does the node hot-reload its binary, or does the operator manually restart?
2. If the operator restarts, what happens to in-flight consensus (BFT view, mempool, uncommitted blocks)?
3. The agent detects node API unavailability per `agent-runtime-spec.md §4.4` — but what if the API schema changed? Does the agent auto-reconnect or need its own restart?
4. Multiple operators restarting at different times = network split (some nodes on old `git:head`, some on new). How long is the old fork tolerated?

**Affects:** `hyperfluid-node/src/main.rs` (consensus loop + shutdown), `crates/hyperfluid-agent/` (agent runtime — not yet built), `interfaces.md` (I-01 versioning), `governance-spec.md` (missing execution section), `agent-runtime-spec.md` Section 4 (crash recovery scope), `stage-04-mainnet-prep.md` (upgrade runbook placeholder)

**Blocks current task?** No — Stage 01 (protocol core) and Stage 02 Week 1-2 (governance engine) can be built without it. But the governance engine is incomplete without a documented upgrade lifecycle. This will block Stage 04 (mainnet prep) unless addressed before.

**Proposed resolution — Option B: Coordinated epoch-boundary restart:**

**Option A (out-of-band rolling restart):** Simple — operators pull, build, restart independently after `git:head` flips. But liveness risk: fast validators running new code produce blocks; slow validators on old code reject them. No protocol-level safety mechanism for the transition window. High accidental-slash risk.

**Option B (recommended): Coordinated epoch-boundary restart with grace window:**
- Governance proposal includes `activation_epoch` field in the `GovernanceProposal` struct (current `consensus-spec.md` Section 1.3 `BlockHeader` gains `git_head_commit: Hash32` to tag each block with the active protocol commit)
- Vote window closes at `vote_end_height`. If passed, `activation_epoch = vote_end_height + 10` (10-epoch grace window ≈ 2.5 hours at 10s blocks)
- During grace window:
  - Node logs: "git:head will activate at epoch N. Protocol commit {old} → {new}"
  - Node continues producing finality under the OLD `git:head`
  - Agent continues running normally
  - Operator pulls, builds new binary, stages it
- At `activation_epoch` boundary:
  1. Node finalizes last block under old `git:head`
  2. Node broadcasts `UpgradeSignal{new_commit, activation_height}` to connected peers
  3. Node persists final state root under old code
  4. Node writes a `git_head_switch` checkpoint to disk (SMT root, last block hash, new commit)
  5. Agent receives signal via API: "node upgrading at height H+1, interface version shifting to v{N+1}. Complete in-flight tool calls, persist handoff, enter wait loop"
  6. Agent persists handoff, disconnects, enters poll loop (retry every 5s with `/health?version=v{N+1}`)
  7. Node process exits (exit code 0 — planned upgrade)
  8. Operator's orchestration (systemd supervisor, Docker restart policy, or k8s) detects exit code 0, launches new binary with staged build
  9. New binary reads checkpoint, initializes SMT from persisted root, sets `git:head` to new commit
  10. New binary begins producing blocks — agent detects API available at expected version, reconnects, resumes loop

**Key design decisions:**
- **10-epoch grace window** gives operators ~22 hours to stage builds (at 10s/block). Governance-set minimum. If you can't rebuild in 22 hours you probably shouldn't be validating.
- **Exit code 0 = planned upgrade**, exit code nonzero = crash (supervisor restarts old binary, which validates old protocol and will detect fork). No ambiguity.
- **Node never auto-updates its own binary.** The node is a dumb orchestrator — governance says "commit X is now canonical," operator supplies the binary that implements it. This keeps the security boundary clean.
- **Agent reconnects via API version negotiation.** The `/health?version=v{N+1}` poll loop means zero-config recovery. If the API is backwards-compatible, the agent never even notices. If breaking, version mismatch returns `UNSUPPORTED_VERSION` and the agent itself needs an upgrade trigger (its own `config.toml` update or a `hyperfluid agent upgrade` command — both operator-initiated).
- **Old-fork protection:** After `activation_epoch`, blocks produced under the OLD `git:head` are rejected by all nodes running the new code. Validators still on old code produce blocks that no one accepts — they stall individually, not as a network. No slash risk (they're just ignored), but strong incentive to upgrade.

**Option C (WASM hot-reload):** Protocol logic compiled to WASM blobs shipped as governance artifacts. Node hot-swaps the WASM module at epoch boundary — zero restart. Elegant but adds WASM toolchain dependency (~50MB per blob), performance overhead (~10-15% on state machine ops), and complexity for the upgrade sandbox itself. Better suited as a post-mainnet optimization. Could be documented as a future direction in `governance-spec.md` Section 11.

**Status:** OPEN — no implementation yet. Design above needs ADR before Stage 02 Week 1-2 governance C4 implementation begins.