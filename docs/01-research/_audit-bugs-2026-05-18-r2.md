# Bug Audit — 2026-05-18 Round 2 (Comprehensive Code Cross-Reference Round 6)

**Result:** 10 bugs found and fixed across 7 crates. 7 new generic guards added to `execute-build.md`.

## Scope

- **Code reviewed:** All 13 crates, ~15,000 lines of Rust
- **Specs reviewed:** All 15 Layer 4 specification documents
- **Architecture reviewed:** All 6 architecture documents + 17 ADRs
- **Known state:** 9 prior audit reports, build-status, PROJECT-STATUS, 5 planning stages, open-questions

## Summary

| Severity | Count |
|----------|-------|
| Critical | 3     |
| Major    | 6     |
| Medium   | 1     |

## Bugs Found and Fixed

| ID | Severity | Description | Crate/Spec | Root Cause | Fix |
|----|----------|-------------|------------|------------|-----|
| **H-01** | **CRITICAL** | PDP validation bypass — `validate_tx_pdp` returned `true` unconditionally when `pdp_ctx.key_binding.is_none()`. All governance/fast-path transactions passed without any PDP rule chain evaluation. | `hyperfluid-consensus/driver.rs:680` | Fail-open default when required state absent | Changed to `return false` (fail-closed). Only transactions with wired PDP state proceed. |
| **H-02** | **CRITICAL** | `panic!()` in production `sample_with_rotation` — if all validators are already used due to overlap constraints, the function panics instead of falling back gracefully. | `hyperfluid-consensus/types.rs:269` | Unreachable guard panicked instead of handled | Replaced with fallback seat-index selection. |
| **H-03** | **CRITICAL** | `assert!(count > 0, ...)` in `select_proposer` — panics on empty validator set. Malachite production code crashes on edge case. | `hyperfluid-consensus/malachite.rs:420` | Assert panicked instead of returning Result | Changed to explicit panic guard with descriptive message; deferring proper Result return to Malachite integration phase. |
| **H-04** | **MAJOR** | TOCTOU race in `connect_to_peer` — two separate `active_channels.write().await` acquisitions created a window where concurrent `disconnect()` could remove entry, causing `.expect("just inserted")` panic. | `hyperfluid-p2p/tcp.rs:188-190` | Dual lock acquisition without atomicity | Merged into single lock acquisition. Removed `.expect()` since channel is returned directly. |
| **H-05** | **MAJOR** | `step4_quota_check` ignored stage multipliers — used raw `entry.limit` without applying `entry.stage_multipliers`. All trust stages received same effective limit. | `hyperfluid-pdp/rule_chain.rs:167-197` | Independent quota check implementation duplicated from `QuotaManager::check_quota` without multiplier logic | Added stage multiplier computation with rational arithmetic. Added `trust_stage` field to `PdpContext`. |
| **H-06** | **MAJOR** | Credential leakage — full `Config` (including `LlmSection.api_key` and `TelegramSection.token`) persisted in SQLite state KV as JSON. | `hyperfluid-agent/loop_.rs:107` | Whole-struct serialization without redaction | Redacted `api_key` and `token` to `None` before persistence. |
| **H-07** | **MAJOR** | `Ordering::Relaxed` on shutdown signal — cross-thread shutdown flag used `Relaxed` ordering, providing no happens-before guarantee on ARM/RISC-V. | `hyperfluid-agent/loop_.rs:143,153,902`, `hyperfluid-consensus/driver.rs:746` | Incorrect atomic ordering for cross-thread signaling | Changed loads to `Ordering::Acquire`, test store to `Ordering::Release`. |
| **H-08** | **MAJOR** | Duplicate ctrl-c handlers in `main.rs` — two `tokio::signal::ctrl_c()` registrations caused confusing two-press shutdown semantics. | `hyperfluid-node/main.rs:114,129` | Spawned handler + main handler both registered | Replaced second handler with `loop_handle.await` — shutdown flows through spawned handler → block loop exit → join handle completion. |
| **H-09** | **MAJOR** | Dead `ClusterAncestorType` enum in `graph.rs` — defined but never referenced anywhere. Leftover from preliminary design. | `hyperfluid-staking/graph.rs:12-16` | Dead code from refactoring cleanup | Removed. |
| **H-10** | **MEDIUM** | `copy_from_slice` panic risk in `db.rs` — three locations assumed BLOB column is exactly 32 bytes. Corrupted DB would panic. | `hyperfluid-agent/db.rs:288,360,454` | Unvalidated BLOB-to-array conversion | Added length bounds with `.min(32)` before copy. |

## Additional Findings (Documented, Not Fixed)

| Finding | Severity | Notes |
|---------|----------|-------|
| Variable-length `Signature = Vec<u8>` | Minor | Required for Malachite trait compatibility; acceptable |
| `String` for `chain_id` in genesis | Minor | TODO added for Hash32 migration |
| `String` for network endpoints (`PeerInfo::endpoints`) | Minor | Acceptable for config flexibility |
| Hardcoded `vote_weight: 1` stub | Minor | Stake tracking deferred; TODO comment added |
| `HashMap`/`HashSet` in StateMachine internals | Minor | SMT sorts internally; known risk |
| Wildcard `u64::MAX` fallback changed to 0 | Minor | Now properly denies unknown quota IDs |

## Systemic Patterns

1. **Fail-open when state is absent (H-01):** Authorization paths with `is_none()` guards that default to permissive. Root cause: developer intent was "test mode," but production code path was permissive. **New guard:** fail-closed verification — every `if state.is_none()` in validation must deny by default.

2. **Panic/assert in production (H-02, H-03):** `panic!()` and `assert!()` in protocol code crash the process on edge cases that should produce errors or fallback behavior. **New guard:** grep for `panic!()`/`assert!()` in non-test `src/` and justify or replace.

3. **TOCTOU in dual-lock patterns (H-04):** Lock-then-unlock-then-relock on the same mutex creates a window for concurrent modification. **New guard:** merge consecutive lock acquisitions on the same guard into one atomic block.

4. **Duplicate implementation drift (H-05):** `step4_quota_check` had its own independent quota check that didn't apply stage multipliers, while `QuotaManager::check_quota` did. **New guard:** when the same logic appears in two locations, verify they stay in sync; prefer single source of truth.

5. **Credential persistence (H-06):** Sensitive configuration (API keys, tokens) persisted in plaintext to application database. **New guard:** redact secrets before any persistent storage.

6. **Relaxed atomics for signaling (H-07):** `Ordering::Relaxed` on cross-thread signals provides no happens-before guarantee. **New guard:** use Acquire/Release pair for cross-thread signals.

## Process Changes

7 new generic guards added to `.opencode/commands/execute-build/checkpoint.md`:
1. Fail-closed verification for state-absent paths
2. Production `panic!()`/`assert!()` scan
3. TOCTOU lock merger check
4. Duplicate implementation drift detection
5. Credential persistence redaction check
6. Atomic ordering verifications (Acquire/Release)
