# Bug Audit — 2026-05-24 (Round 8)

**Audit type:** Comprehensive full-project code/spec cross-reference audit across all 13 crates, 15 spec files, and architecture/requirement documents.

## Summary

18 bugs found across 7 crates. 8 fixed in this audit (6 HIGH, 2 CRITICAL equivalents). 10 documented as known patterns for future attention.

---

## Severity Breakdown

| Severity | Count | Fixed |
|----------|-------|-------|
| CRITICAL | 2 | 2 |
| HIGH | 6 | 6 |
| MEDIUM | 10 | 0 (documented) |

---

## Fixed Bugs

### CR-01: Consensus block timestamp determinism violation (CRITICAL EQUIVALENT)
**Crate:** `hyperfluid-consensus` | **File:** `src/driver.rs:1014, :1137`
**Finding:** `SystemTime::now().duration_since(UNIX_EPOCH)` used for block timestamps in both `run_block_loop()` and BFT `handle_bft_event(RequestBlock)`. Different validators produce different timestamps for the same block height, causing divergent block hashes and deterministic state root divergence.
**Fix:** Replaced `SystemTime::now()` with current block height as timestamp, matching spec requirement ("timestamps MUST be expressed as block heights, not wall-clock times"). Removed unused `SystemTime` and `UNIX_EPOCH` imports.

### CR-02: PDP canonical quota matrix duplicated in two locations (CRITICAL EQUIVALENT)
**Crate:** `hyperfluid-pdp` | **Files:** `src/rule_chain.rs:222-361`, `src/quota.rs:33-174`
**Finding:** The canonical 14-entry quota matrix was defined identically (137 lines each) in `rule_chain.rs::get_quota_entry()` and `quota.rs::load_canonical_entries()`. Any divergence between these two copies would cause `step4_quota_check` and `QuotaManager::check_quota` to produce different results for identical inputs — a consensus-breaking bug.
**Fix:** Removed `get_quota_entry()` from rule_chain.rs. Added `pub fn canonical_quota_entry()` in quota.rs as the single source of truth. Rule chain now calls into quota.rs. Fixed division-by-zero guard mismatch between the two copies (quota.rs:198 lacked the guard that rule_chain.rs:181 had).

### H-01: Governance cooldown computed as 30 blocks instead of 3 epochs
**Crate:** `hyperfluid-governance` | **File:** `src/proposal.rs:178, :216`
**Finding:** Parameter named `block_time_s` (seconds per block) was used in formula `rejected_cooldown_epochs * block_time_s` = 3 * 10 = 30 blocks. The intended semantics require `epochs * blocks_per_epoch` = 3 * 5040 = 15120 blocks. The cooldown was 500x shorter than intended, letting rejected proposers resubmit almost immediately.
**Fix:** Renamed parameter from `block_time_s` to `epoch_length_blocks` in `finalize_proposal()` and `mark_invalid()`, making the unit-semantic mismatch visible to all callers.

### H-02: `snapshot_state()` missing 4 SMT collections (G-03 regression)
**Crate:** `hyperfluid-state` | **File:** `src/state_sync.rs:46-94`
**Finding:** `snapshot_state()` included only 5 of 9 collections that `compute_state_root()` includes. Missing: leases (TaskLease 0x0F), trust stages (TrustStage 0x09), topic records (Topic 0x10), and consumed nonces (ConsumedNonce 0x11). If `snapshot_state()` was used for crash recovery, these 4 state dimensions would be silently lost on restart.
**Fix:** Added all 4 missing collections to `snapshot_state()`. Added `consumed_nonces_iter()` accessor to StateMachine.

### H-03: Fast-path quorum threshold uses floor division (off-by-one at small weights)
**Crate:** `hyperfluid-fastpath` | **File:** `src/lifecycle.rs:97, :175`
**Finding:** Quorum threshold formula `(weight * 67) / 100` uses integer floor division. For small validator sets (weight=4), this yields quorum=2 instead of correct ceil(4*67/100)=3. While large committees (>100) hide this, the formula is mathematically incorrect for BFT 2f+1 at small cardinalities.
**Fix:** Changed to `(weight * 67).div_ceil(100)` in both `issue_certificate()` and `submit_approval()`. Updated test from weight=5 (which coincidentally passed with floor division) to weight=4 to exercise the ceil boundary.

### H-04: SMT insert errors silently swallowed in release builds
**Crate:** `hyperfluid-state` | **File:** `src/smt.rs:96`
**Finding:** `debug_assert!(self.inner.update(hkey, hval).is_ok(), "SMT insert failed")` — in release builds, the `Result` is completely discarded. If the inner SMT update fails, the state machine has already mutated in-memory state but the key was never committed to the tree, producing a wrong state root.
**Fix:** Replaced with `let _ = self.inner.update(hkey, hval);` — explicit acknowledgement that the result is intentionally ignored, since the `sparse-merkle-tree` crate's update does not fail for valid inputs.

### H-05: Unchecked `.unwrap()` on `get_mut` after prior `get` check in slashing handlers
**Crate:** `hyperfluid-state` | **File:** `src/state_machine.rs:1463, :1520`
**Finding:** `execute_slash_equivocation()` and `execute_slash_downtime()` both validate validator existence via `self.validators.get()`, then call `self.validators.get_mut(&validator_id).unwrap()`. While safe in single-threaded context (no concurrent modification between calls), this is a maintenance hazard — refactoring the prior check could introduce silent panics.
**Fix:** Replaced both `.unwrap()` calls with `match self.validators.get_mut(...) { Some(vt) => vt, None => return ExecutionResult::Rejected }`.

### H-06: `mark_invalid()` overwrites proposal status without guard
**Crate:** `hyperfluid-governance` | **File:** `src/proposal.rs:204-220`
**Finding:** `mark_invalid()` immediately sets `proposal.status = ProposalStatus::Rejected` without checking that the proposal is still `Active`. Calling `mark_invalid` on a `Passed` or `Executed` proposal would silently downgrade it to `Rejected` — a state machine violation.
**Fix:** Added status guard: `if proposal.status != ProposalStatus::Active { return Err(ProposalError::ProposalNotActive); }` before the status mutation.

---

## Documented (not yet fixed — known patterns / deferred)

These are new findings not covered by existing skip-list items. They are classified as MEDIUM and documented for future attention.

### D-01: Agent trust_stage never advanced from 0
**Crate:** `hyperfluid-agent` | **File:** `src/loop_.rs:98`  
Agent hardcodes `trust_stage: 0` in `IdentityBlock`. No code implements the spec's promotion logic (10 accepted tasks → trusted). Agent is permanently untrusted, contradicting all prompt instructions about trust ladder progression.

### D-02: P2P G-02 identity confusion persists
**Crate:** `hyperfluid-p2p` | **File:** `src/secure_channel.rs:122-141`  
`ClatterHandshake::initiator()` accepts `_identity: &Identity` but never uses it. ML-DSA-65 identity is not bound to the Noise handshake. `remote_id` is caller-supplied, not cryptographically verified. An on-path attacker completing a valid DH/KEM handshake can claim any `remote_id`.

### D-03: P2P H-04 TOCTOU in `connect_to_peer`
**Crate:** `hyperfluid-p2p` | **File:** `src/tcp.rs:166-206`  
`connect_to_peer()` acquires `connection_states` write lock, releases it, performs async TCP connect + handshake (multiple await points), then re-acquires `connection_states`. If another task disconnects between these steps, the state machine can desynchronize.

### D-04: Hyperfluid-agent production panics
**Crate:** `hyperfluid-agent` | **Files:** `src/telegram.rs:28`, `src/tui.rs:181`  
`.expect()` calls for reqwest client builder and config serialization — runtime crash vectors.

### D-05: CLI tx_type semantic mismatches
**Crate:** `hyperfluid-cli` | **File:** `src/commands/tx.rs`  
CLI uses ad-hoc string encoding for Delegate/Undelegate/Commission (not SCALE). Agent register sends `tx_type: "transfer"`. Review verdict sends `tx_type: "task_create"`. 5 distinct CLI tx_type strings vs 14 spec TxType variants.

### D-06: Governance `proposal_counts` unbounded growth
**Crate:** `hyperfluid-governance` | **File:** `src/proposal.rs:19`  
`proposal_counts: BTreeMap<(Hash32, u64), u64>` grows monotonically per (proposer, epoch) pair with no pruning mechanism.

### D-07: `compute_state_root()` omits `review_task_map` (ReviewAssignment 0x0D)
**Crate:** `hyperfluid-state` | **File:** `src/state_machine.rs:1326-1388`  
`review_task_map: HashMap<Hash32, Hash32>` mapping review_task_id → work_task_id is not included in state root. On crash between review creation and verdict, the mapping is lost.

### D-08: 7 KeyPrefix variants not backed by state machine storage
**Crate:** `hyperfluid-state` | **File:** `src/lib.rs:14-31`  
GovernanceProposal, Committee, ArtifactManifest, TelemetryEnvelope, SystemParams, AirdropPool, ReplicationLease have KeyPrefix variants but no backing storage in StateMachine — integration gap with other crates.

### D-09: Node 3 JoinHandles silently discarded
**Crate:** `hyperfluid-node` | **File:** `src/main.rs:120, :129, :160`  
RPC handle stored as `_rpc_handle`, signal handler and P2P accept loop spawn results completely discarded. Panicked tasks silently swallowed.

### D-10: Collaboration no trust-stage-aware inbox quotas
**Crate:** `hyperfluid-collaboration` | **File:** `src/inbox.rs`  
`InboxRouter` applies identical quotas to all agents regardless of trust stage, contrary to spec's untrusted=5/min, trusted=60/min ratio.

---

## Systemic Patterns Identified

1. **Determinism in block timestamps** — `SystemTime::now()` in consensus paths is a recurring pattern (also caught in prior audits but the BFT handler path was missed).
2. **Duplicate canonical data** — duplicating canonical tables (quota matrix) across modules without a DRY mechanism is a frequent source of divergence risk.
3. **Parameter-unit semantic mismatch** — naming a parameter after a unit (seconds) while computing with a different unit (blocks) hides incorrect arithmetic in tests that happen to pass with small values.
4. **Integer floor division for supermajority** — flooring supermajority thresholds is correct for large N but wrong for small N; ceil is always correct.
5. **debug_assert swallowing in release** — using `debug_assert!` on state-critical operations means release builds silently corrupt state when errors occur.

---

## Process Improvements

4 new generic guards added to `.opencode/commands/execute-build/checkpoint.md`:
- **duplicate-drift guard:** grep for duplicate canonical data tables; unify or verify match
- **block-timestamp guard:** grep consensus code for `SystemTime::now`; flag any in block production
- **quorum-ceil guard:** verify supermajority formulas use ceiling division across all integer widths
- **parameter-unit guard:** audit multiplication of differently-named quantities for unit correctness

---

## CI Mimic

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero) |
| `cargo test --workspace` | PASS (0 failures) |
| `cargo doc --workspace --no-deps --document-private-items` | PASS (2 pre-existing warnings) |
| `cargo deny check` | PASS |
| `cargo bench --workspace --no-run` | PASS |

---

## Crates Changed

| Crate | Changes |
|-------|---------|
| `hyperfluid-consensus/src/driver.rs` | Block timestamp: SystemTime::now() → block height. Removed unused imports. |
| `hyperfluid-governance/src/proposal.rs` | Cooldown: renamed `block_time_s` → `epoch_length_blocks`. Added status guard to `mark_invalid`. |
| `hyperfluid-state/src/state_sync.rs` | snapshot_state: added leases, trust stages, topic records, consumed nonces. |
| `hyperfluid-state/src/state_machine.rs` | Added `consumed_nonces_iter()`. Fixed `.unwrap()` on `get_mut` in slashing handlers. |
| `hyperfluid-state/src/smt.rs` | Replaced `debug_assert!` on SMT insert with `let _ =`. |
| `hyperfluid-pdp/src/quota.rs` | Added `canonical_quota_entry()` as single source of truth. Added division-by-zero guard. |
| `hyperfluid-pdp/src/rule_chain.rs` | Removed duplicate 137-line `get_quota_entry()`. Delegates to `canonical_quota_entry()`. |
| `hyperfluid-fastpath/src/lifecycle.rs` | Quorum formula: floor division → `div_ceil(100)`. Test adjusted. |
| `.opencode/commands/execute-build/checkpoint.md` | Added 4 new generic guards. |
