# Checkpoint 2026-05-20 — Stage 02 Week 5-6 P2P + Mempool + PDP Wire-Up

## Summary

Wired P2P TCP transport, fee-ordered mempool, and PDP context state into the running node. Replaced stub `vec![]` block production with real mempool transaction selection. Added `submit_tx()`, `extract_sender_id()`, and PDP state tracking (key bindings, agent nonces, quota states, consumed plan IDs) to `ConsensusDriver`.

## Changes

| File | Change |
|------|--------|
| `crates/hyperfluid-p2p/src/mempool.rs` | Added `tx_data: Vec<u8>` field to `MempoolTx` for serialized transaction storage |
| `crates/hyperfluid-p2p/tests/conformance_p2p_spec.rs` | Updated `tx()` test helper for new `tx_data` field |
| `crates/hyperfluid-consensus/Cargo.toml` | Added `hyperfluid-p2p` as dependency |
| `crates/hyperfluid-consensus/src/driver.rs` | Added `mempool`, `tx_store`, `key_bindings`, `agent_nonces`, `quota_states`, `consumed_plan_ids` fields. Added `submit_tx()`, `extract_sender_id()`, `apply_quota_consumption()`. `produce_block()` now selects from mempool when called with empty txs. `validate_tx_pdp()` now populates `PdpContext` from live driver state (balance, nonce, trust stage, quotas). |
| `crates/hyperfluid-node/Cargo.toml` | Moved `hyperfluid-p2p` from dev-dependencies to dependencies |
| `crates/hyperfluid-node/src/main.rs` | Added P2P TCP transport startup: creates `TcpTransport`, generates local `Identity`, starts `accept_loop` on ephemeral port |

## CI Mimic

| Check | Result |
|-------|--------|
| `cargo fmt --all -- --check` | PASS |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero warnings) |
| `cargo test --workspace` | PASS (467/467) |
| `cargo doc --workspace --no-deps --document-private-items` | PASS (2 pre-existing rustdoc warnings in hyperfluid-state) |
| `cargo deny check` | PASS |  
| `cargo bench --workspace --no-run` | PASS |

## Determinism Sweep

| Check | Result |
|-------|--------|
| Floating-point in protocol code | PASS (zero hits) |
| Wall-clock/Random in protocol code | PASS (only `SystemTime::now()` in async block loop, not in deterministic paths) |
| Test shims in library code | PASS (zero hits) |
| `panic!`/`assert!` in production code | PASS |

## Integration Verification

| Component | Process Data? | Communicate? | Persist? | Run Loop? | Wired to Node? |
|-----------|--------------|--------------|----------|-----------|----------------|
| Mempool | Yes — `submit_tx()`, `produce_block()` selects from mempool | N/A (in-process) | N/A (in-memory, tx_store is BTreeMap) | Yes — block loop pulls from mempool | Yes — `ConsensusDriver::produce_block()` uses mempool |
| P2P TCP | N/A | Yes — `accept_loop` with Clatter handshake over TCP | N/A | Yes — accept_loop runs indefinitely | Yes — spawned as tokio task in `main()` |
| PDP Context | Yes — real balances, nonces, trust stages, quotas used | N/A | N/A | Yes — PDP called per-tx in block production | Yes — `validate_tx_pdp()` in `execute_tx()` path |

## Open Items

| Item | Status |
|------|--------|
| P2P TCP listener only (no outbound connections) | Production nodes need `connect_to_peer` for peer discovery; deferred to Week 7-8 multi-node BFT |
| Mempool transactions only come from local `submit_tx()` | No gossiped transactions from peers; deferred to Week 7-8 P2P gossip |
| PDP `pdp_bypass` still `true` by default | Key bindings are tracked but not populated (ML-DSA deferred to Week 9-10); changing to false would deny all governance/fast-path txs (fail-closed) |
| No integration test for P2P TCP in node binary | `accept_loop` requires real socket + external client; existing `tcp::socket_integration` tests cover the TCP path in isolation |
