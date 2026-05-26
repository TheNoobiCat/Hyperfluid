# Checkpoint 2026-05-26 — Stage 01 Multi-Node BFT Networking

**Summary:** Multi-validator BFT consensus infrastructure wired into node binary. `--multi-validator` flag starts BFT loop with dynamic peer management. Persistent TCP connections carry encrypted consensus messages (votes/proposals) after Clatter handshake. External `NetworkBridge` supports dynamically added peers.

## Crates Changed

| Crate | Lines +/- | Files Changed |
|-------|-----------|---------------|
| `hyperfluid-p2p` | ~+260 | `tcp.rs`, `Cargo.toml` |
| `hyperfluid-consensus` | ~+15 | `driver.rs`, `network_bridge.rs` |
| `hyperfluid-node` | ~+250 | `main.rs`, `Cargo.toml` |
| **Total** | **~+525** | **6 files** |

## Tasks Completed

### Task 1: `--multi-validator` flag + BFT loop wiring
- **File:** `crates/hyperfluid-node/src/main.rs`
- `--multi-validator` and `--peers <addr1,addr2,...>` CLI flags
- Builds `HyperfluidValidatorSet` from genesis validators with u64-capped voting power
- Creates `Address32` node_addr from node identity's peer_id
- Generates proposer_seed via SHA3-256(peer_id || genesis_timestamp)
- Creates external `NetworkBridge` with empty peers (dynamic management)
- Consensus handler decodes wire-format votes/proposals from TCP and routes to BFT incoming channel
- Starts `run_bft_loop` with external bridge; connects to configured peers via `connect_and_maintain`
- Default mode (`run_block_loop`) unchanged for backward compatibility

### Task 2: TCP transport consensus message routing
- **File:** `crates/hyperfluid-p2p/src/tcp.rs`
- `ConsensusMessageHandler` type alias: `Arc<dyn Fn(Hash32, Vec<u8>) + Send + Sync>`
- `peer_message_loop()`: post-handshake persistent connection using `tokio::select!` for concurrent read (decrypt → handler) and write (mpsc → encrypt → TCP frame). Exits when either side closes.
- `connect_and_maintain()`: outbound TCP connect + initiator handshake + `peer_message_loop`. Returns mpsc sender for outgoing consensus messages.
- `accept_loop` now accepts optional `ConsensusMessageHandler` — when present, inbound connections enter persistent messaging; otherwise legacy one-shot handshake.
- Changed `perform_initiator_handshake` to borrow stream (`&mut TcpStream`) instead of consuming it, enabling post-handshake reuse.
- `handle_inbound` now returns `(Hash32, ClatterSecureChannel)` instead of `()`.
- Added `tracing` + `hex` deps; `tracing::info!/warn!` replace `eprintln!`.

### Task 3: BFT loop external bridge
- **File:** `crates/hyperfluid-consensus/src/driver.rs`
- `run_bft_loop` now accepts optional `external_bridge: Option<Arc<Mutex<NetworkBridge>>>`
- When provided, `run_sender` reads from bridge_rx and broadcasts to dynamically managed peers
- Inbound messages handled by TCP consensus handler → `incoming_tx` directly (no `run_receiver` needed)

### Task 4: Public decode
- **File:** `crates/hyperfluid-consensus/src/network_bridge.rs`
- Made `decode_vote` public for use in consensus handler

## Deferred

| Task | Reason |
|------|--------|
| Multi-node BFT integration test (3+ nodes over TCP) | Requires peer key resolution (currently hardcoded `[0u8;32]` for `remote_peer_id` in outbound connections) |
| 24h soak test | Blocked on multi-node test infrastructure |
| ClatterHandshake ML-DSA-65 identity binding | Pre-existing: `_identity` parameter unused for crypto binding |

## CI Mimic

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS |
| `cargo test --workspace` | PASS (all passing, 0 failures) |
| `cargo fmt --all` | applied |
| `cargo clippy --workspace --all-targets` | PASS (pre-existing warnings only) |
| Determinism sweep (floating-point in protocol) | PASS |
| Determinism sweep (wall-clock in protocol) | PASS |
| `if let Some.get_mut` guard | PASS (no new violations) |
| `panic!/assert!` in library src/ | PASS (no new violations) |

## Known Gaps

1. **Outbound peer identity resolution:** `connect_and_maintain` hardcodes `remote_peer_id: [0u8; 32]` and empty DH/KEM keys. Peers specified via `--peers` will fail the Clatter handshake. Needs peer configuration format (peer_id + DH/KEM keys per peer) or dynamic key discovery.

2. **Multi-node exit criterion not met:** "3+ validators reach consensus, gossip transactions, finalise blocks" still unchecked. Infrastructure built but not yet demonstrated with real multi-node TCP.

## Architecture Decisions

- Persistent peer connections use a single `tokio::select!` task per connection (one read arm, one write arm) to avoid `Arc<Mutex<ClatterSecureChannel>>` on TransportState's nonce counters.
- Network bridge supports dynamic peer management via `Arc<Mutex<NetworkBridge>>` with mutable `.peers` vec, decoupled from BFT loop startup.
- Consensus messages carry a 1-byte tag prefix (0x01=Vote, 0x02=Proposal) followed by wire-encoded payload, matching `network_bridge.rs` format.
