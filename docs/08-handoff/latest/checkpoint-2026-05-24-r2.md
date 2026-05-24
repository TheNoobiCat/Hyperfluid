# Checkpoint 2026-05-24-r2 — Bug Audit Round 8

**Summary:** Comprehensive full-project audit across all 13 crates, 15 specs, architecture, and requirements. 8 bugs fixed across 7 crates. 4 new generic guards added to execute-build.md checkpoint. CI all-green.

## Crates Changed

| Crate | Changes |
|-------|---------|
| `hyperfluid-consensus/src/driver.rs` | Block timestamp determinism: SystemTime::now() → block height. Removed unused imports. |
| `hyperfluid-governance/src/proposal.rs` | Cooldown: renamed `block_time_s` → `epoch_length_blocks`. Added status guard to `mark_invalid`. |
| `hyperfluid-state/src/state_sync.rs` | snapshot_state: added leases, trust_stages, topic_records, consumed_nonces (4 missing collections). |
| `hyperfluid-state/src/state_machine.rs` | Added `consumed_nonces_iter()`. Fixed get_mut unwrap in slashing handlers. |
| `hyperfluid-state/src/smt.rs` | Replaced `debug_assert!` on SMT insert with explicit `let _ =`. |
| `hyperfluid-pdp/src/quota.rs` | Added `canonical_quota_entry()` as single source of truth. Added division-by-zero guard. |
| `hyperfluid-pdp/src/rule_chain.rs` | Removed 137-line duplicate `get_quota_entry()`. Delegates to `canonical_quota_entry()`. |
| `hyperfluid-fastpath/src/lifecycle.rs` | Quorum threshold: floor division → `.div_ceil(100)`. Test adjusted weight=5→4. |
| `.opencode/commands/execute-build/checkpoint.md` | Added 4 new generic guards. |

## Key Findings

1. **CRITICAL:** Consensus block timestamp used `SystemTime::now()` — validators would produce divergent block hashes for the same height in BFT consensus.
2. **CRITICAL:** PDP quota matrix duplicated 137 lines in two modules — any divergence would be consensus-breaking.
3. **HIGH:** Governance cooldown was 30 blocks (not 3 epochs = ~15,120 blocks) — rejected proposers could resubmit almost immediately.
4. **HIGH:** `snapshot_state()` missed 4 of 9 SMT collections — crash recovery would silently lose leases, trust stages, topics, and consumed nonces.
5. **HIGH:** Fast-path quorum used floor division — off-by-one at small validator weights.
6. **HIGH:** SMT insert errors silently swallowed in release builds via `debug_assert!`.
7. **HIGH:** `.unwrap()` on `get_mut` after `get` check in slashing handlers — maintenance hazard.
8. **HIGH:** `mark_invalid()` could overwrite Passed/Executed proposals without status guard.

## Systemic Patterns

- Determinism in block timestamps (SystemTime::now in consensus paths)
- Duplicate canonical data (quota matrix in two modules)
- Parameter-unit semantic mismatch (block_time_s × epochs = wrong unit)
- Integer floor division for supermajority (should be ceil)
- debug_assert swallowing in release builds on state-critical operations

## CI Mimic

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS (13 crates, zero warnings) |
| `cargo test --workspace` | PASS (0 failures) |
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero) |
| `cargo doc --workspace --no-deps --document-private-items` | PASS |
| `cargo deny check` | PASS |
| `cargo bench --workspace --no-run` | PASS |

## Known Issues (unchanged from prior checkpoints)

- Malachite BFT effect handler + Clatter network bridge: DEFERRED to Stage 03
- Multi-node BFT soak test: DEFERRED to Stage 03
- P2P identity confusion (G-02): ML-DSA-65 not bound to Noise handshake
- P2P TOCTOU race (H-04): connect_to_peer dual-lock pattern
- Agent trust_stage never advanced from 0 (D-01)
- CLI tx_type semantic mismatches (D-05)

See `docs/01-research/_audit-bugs-2026-05-24.md` for full details.
