# Checkpoint 2026-05-17c — Gap Fill Complete

**Date:** 2026-05-17
**Trigger:** `.opencode/commands/fill-gaps.md` execution
**Stage:** Stage 01 integration gap resolution complete; Stage 02 ready for Week 3-4

---

## Gaps Investigated

| Gap ID | Stage/Week | Component | Description |
|--------|-----------|-----------|-------------|
| G1 | Stage 01 W1-2 | Malachite BFT | `arc-malachitebft-core-*` crates loaded, no SigningScheme/Context/effect handler |
| G2 | Stage 01 W3-4 | C3 Slashing | Slashing execution, reward distribution, liveness tracking |
| G3 | Stage 01 W5-6 | C7 P2P TCP | TCP/UDP sockets (already resolved) |
| G4 | Stage 01 W5-6 | C8 Disk I/O | Disk-backed artifact storage (already resolved) |
| G5 | Stage 01 W7-8 | Node Binary | Consensus loop stub (already resolved) |
| G6 | Stage 01 W7-8 | Multi-node | No multi-node harness (already resolved) |
| G7 | Stage 02 | Integration | C4/C6/C9 not wired (already resolved) |

---

## Gaps Filled

### G1: Malachite BFT Type-Level Integration

**Status:** PARTIALLY RESOLVED — SigningScheme + Context implemented. Effect handler, network bridge, host actor deferred.

**Files changed:**
- `crates/hyperfluid-consensus/Cargo.toml` — Added `arc-malachitebft-core-types`, `arc-malachitebft-core-driver`, `ml-dsa` deps; `rand` dev-dep
- `crates/hyperfluid-consensus/src/lib.rs` — Added `pub mod malachite`
- `crates/hyperfluid-consensus/src/malachite.rs` — 588 lines new

**What was implemented:**
- `MlDsa65Scheme` — SigningScheme for ML-DSA-65 FIPS 204
- `MlDsa65Signature`, `MlDsa65PublicKey`, `MlDsa65PrivateKey` — byte-based wrappers
- `Address32` — [u8;32] newtype with Display via hex for Malachite Address trait
- `BlockHeight` — u64 wrapper implementing Height trait
- `BlockValue` — Block wrapper implementing Value trait (Ord by hash)
- `ValueHash` — [u8;32] with Display for Value::Id
- `HyperfluidValidator` — Validator trait impl
- `HyperfluidValidatorSet` — ValidatorSet trait impl (sorted by power then address)
- `HyperfluidVote` — Vote trait impl (prevote/precommit)
- `HyperfluidProposal` — Proposal trait impl
- `HyperfluidProposalPart` — ProposalPart trait impl
- `HyperfluidExtension` — Extension trait impl (empty)
- `HyperfluidContext` — Context trait impl with deterministic SHA3-256 proposer selection

**Tests added:** 13
- `signing_scheme_encode_decode_roundtrip` — Signature encode/decode roundtrip
- `signing_scheme_decode_invalid_bytes` — Invalid signature rejected
- `block_height_trait_methods` — Height trait arithmetic
- `block_height_constants` — ZERO/INITIAL constants
- `block_value_id_is_block_hash` — Value ID equals block hash
- `block_value_ord_by_hash` — BlockValue Ord determinism
- `validator_set_sorted_by_power_then_address` — Deterministic ordering
- `validator_set_lookup` — Address-based lookup
- `context_select_proposer_deterministic` — Repeatable proposer selection
- `context_create_proposal` — Proposal construction
- `context_create_prevote` — Prevote construction
- `context_create_precommit_nil` — Nil precommit
- `full_proposal_vote_cycle` — Complete proposal-vote lifecycle

**Remaining work (~1,100 lines):**
- Effect handler: route Malachite Effects to clatter network + tokio timers (~300 lines)
- Clatter network bridge: consensus message send/receive over PQ-Noise (~500 lines)
- Host actor: proposal building, block validation, vote extensions, commit (~400 lines)

---

## Gaps Verified (Already Resolved)

All previously resolved gaps verified against source code and integration gate:

| Gap | Verification | Integration Gate |
|-----|-------------|-----------------|
| G3: P2P TCP sockets | `tcp.rs` with `TcpListener`/`TcpStream`, accept_loop, connect_to_peer, clatter handshake over wire | PASS — `conforms_to_p2p_spec_1_7_actual_socket_roundtrip` + `actual_socket_lifecycle` tests |
| G4: Disk I/O | `store.rs` with `store_chunk`/`load_chunk`, SHA3-256 verification, content-addressed paths | PASS — 10 store tests including write/read/restart/hash mismatch/corruption |
| G5: Node binary | `main.rs` uses `ConsensusDriver::run_block_loop()` — real block production | PASS — `node_produces_real_blocks` test, 14 consensus_driver_tests, 6 multi_node_tests |
| G6: Multi-node harness | `multi_node_test.rs` — 6 tests across 2-5 nodes, deterministic state convergence | PASS — All 6 tests verify genesis consistency, state divergence, independent block production |
| G7: C4/C6/C9 wired | Governance/FastPath/PDP dispatched in ConsensusDriver | PASS — 17 e2e tests, governance proposal submission, fast-path merge, PDP validation |

---

## P2P Conformance Test Fix

**Bug:** `e2e_encryption_across_relay` and `tampered_ciphertext_rejected` tests failed with cache collision.
**Root cause:** Global `shim_result_cache()` shared between tests using same peer IDs `[1u8;32]` and `[2u8;32]` as `e2e_empty_message` (which runs first alphabetically).
**Fix:** Changed peer IDs to `[10u8;32]`/`[11u8;32]`/`[12u8;32]` and `[20u8;32]`/`[21u8;32]` respectively.
**Result:** All 23 P2P conformance tests pass.

---

## Determinism Sweep

| Check | Result |
|-------|--------|
| Floating-point in protocol code | CLEAN — zero hits |
| Wall-clock/random in protocol paths | EXPECTED — `SystemTime::now()` in `driver.rs:610` for block timestamps only |
| thread_local/RefCell in library code | CLEAN — zero hits in non-test paths |
| Default feature has mock shims | CLEAN — `hyperfluid-p2p` default is `clatter-secure-channel` (production) |
| HashMap/HashSet in consensus paths | ACCEPTABLE — HashMap for storage; SMT root is order-independent; HashSet only for membership checks |
| if let Some.get_mut rejecting else arms | CLEAN — all 14 matches have rejecting else arms (verified in bug audits B-24, F-01) |
| validate-then-mutate ordering | CLEAN — B-21 fix applied across all handlers |

---

## CI Mimic Results

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero warnings) |
| `cargo test --workspace` | PASS (386/386 tests) |
| `cargo doc --workspace --no-deps --document-private-items` | PASS |
| `cargo deny check` | PASS (advisories ok, bans ok, licenses ok, sources ok) |
| `cargo bench --workspace --no-run` | PASS (13 benchmark binaries compiled) |

---

## Remaining Gaps

| Gap | Status | Blocker |
|-----|--------|---------|
| Malachite effect handler + network bridge + host actor (~1,100 lines) | PARTIALLY RESOLVED — types built, runtime wiring deferred | No — ConsensusDriver produces blocks |
| Slashing execution + reward distribution | DEFERRED to Stage 03 | No |
| 24-hour soak test | DEFERRED to Stage 03 | No |
| Clatter network bridge for consensus gossip | DEFERRED — depends on BFT protocol wiring | No |

---

## Updated Files

- `docs/05-planning/stages/stage-01-protocol-core.md` — Updated GAP NOTE with 2026-05-17c resolution
- `docs/08-handoff/latest/build-status.md` — Updated Malachite gap status to PARTIALLY RESOLVED; added RESOLVED GAPS entries; last-updated date
- `PROJECT-STATUS.md` — Updated blockers, recent changes, last-updated date
- `crates/hyperfluid-consensus/src/malachite.rs` — New file (588 lines)
- `crates/hyperfluid-consensus/Cargo.toml` — Added dependencies
- `crates/hyperfluid-consensus/src/lib.rs` — Added module
- `crates/hyperfluid-p2p/tests/conformance_p2p_spec.rs` — Fixed cache collision
