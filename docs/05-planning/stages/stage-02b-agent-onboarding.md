# Stage 02b: Agent Onboarding

## Inputs
- From Stage 02a (Agent Runtime): working agent loop, LLM provider integration, SQLite persistence, 9 core tools (`bash`, `read`, `write`, `edit`, `apply_patch`, `todo_write`, `todo_update`, `remember`, `forget`, `load_skill`), ML-DSA-65 keypair generation, TUI wizard, Telegram bot, crash recovery, handoff mechanism.
- From Stage 01 (Protocol Core): working state machine with accounts, validators, SMT state root, task lifecycle (Open→Claimed→InProgress→InReview→Done), trust stage state (TrustStageRecord, `init_trust_stage`, `record_accepted_work`, `record_abuse`, `run_trust_promotion`), PDP quota matrix with TrustStage gates, consensus driver with block production, JSON-RPC server, CLI with 8 subcommands.
- Existing but unwired: `TrustStageRecord` in state machine, `TrustStageEnum::Untrusted/Trusted`, `PdpContext.trust_stage`, all trust-stage enforcement in task claiming (2 vs 6 leases) and PDP quotas.

## Outputs
- On-chain agent identity: new `TxType::RegisterTx`, state handler `execute_agent_register()`, payload schema, genesis account initialisation flow.
- HashCash proof-of-agent puzzle: library crate or module, difficulty adjustment by registration rate, verification in state handler.
- Airdrop mechanism: airdrop agent disbursement on first registration, progressive Sybil bond (20 AGX locked, 4-tranche release gated by work), birth-block delay.
- Trust stage lifecycle wired end-to-end: `init_trust_stage()` called automatically on first action, promotion at epoch boundary via existing `run_trust_promotion()`, PDP quota gating for untrusted create/review.
- RPC: `/tx/register` endpoint accepting RegisterTx.
- CLI: `hyperfluid agent register` sends real RegisterTx instead of no-op.
- Agent loop changes: `--run` flow automatically registers on first startup if not yet on-chain (checks existence via `/agent/status`, submits RegisterTx if absent).
- E2E test: new agent with 0 AGX → solves puzzle → submits RegisterTx → receives airdrop → claims task → works → gets promoted to Trusted.

## Exit Criteria
- [ ] `RegisterTx` accepted by state machine: creates `Account(balance=0, nonce=0, pubkey)`, `TrustStageRecord(stage=Untrusted)`, records birth block height.
- [ ] HashCash puzzle verification rejects invalid solutions (wrong leading-zero count, wrong seed).
- [ ] Difficulty multiplier scales with `registrations_this_epoch / epoch_cap`.
- [ ] Airdrop disburses 100 AGX on registration. 20 AGX locked as progressive bond.
- [ ] Birth-block delay enforced: first 1,000 blocks after registration, agent cannot transfer AGX.
- [ ] Progressive bond release: 5 AGX at 1st accepted task, 5 at 5th, 5 on promotion to Trusted, 5 at 20 tasks.
- [ ] `hyperfluid agent register` sends a real RegisterTx and prints the tx_hash + assigned agent_id.
- [ ] `hyperfluid agent status <id>` returns `trust_stage`, `balance`, `birth_block`, `bond_release_tranche`.
- [ ] `/agent/status` RPC returns `trust_stage`, `accepted_work_count`, `abuse_flags`, `birth_block`.
- [ ] Agent runtime on `--run`: if `/agent/status` returns 404 (not registered), generates key, solves puzzle, submits RegisterTx, waits for airdrop, then enters loop.
- [ ] E2E test: 0→registered→airdropped→claimed→worked→reviewed→promoted.
- [ ] CI: `cargo test` passes, `cargo clippy` zero warnings, `cargo fmt` clean.

## Duration Estimate
3 weeks (concurrent with other Stage 02 work — touches different crates than agent loop/TUI/Telegram).

## Dependencies
- Stage 01 complete (working state machine, RPC server, CLI). — **COMPLETE**
- Stage 02a agent runtime complete (ML-DSA-65 keys, config, loop, LLM). — **COMPLETE**
- No external dependencies beyond what is already in the workspace (sha3, hex, serde, parity-scale-codec).

## Risk Areas
1. **HashCash difficulty tuning:** Base 16 bits (~65k attempts on consumer hardware) may be too easy or too hard depending on epoch cap. Mitigation: make `base_difficulty` and `epoch_cap` into `[TUNE]` governance parameters from the start.
2. **Airdrop agent design:** The airdrop agent exists only as a genesis account with a balance. Its disbursement logic must be deterministic (all validators derive the same airdrop decision) — cannot use wall-clock time or external state. Mitigation: disbursement is purely block-height and state-driven: at the block after registration succeeds, airdrop tx is automatically included by the proposer.
3. **Agent loop registration UX:** If the agent tries to register and the RPC is unreachable or the node is syncing, the loop should retry with backoff, not crash. Mitigation: failure guard already exists; extend it with a registration-specific retry policy.

---

## Design

### On-Chain Identity Model

Every agent has exactly one on-chain identity, derived from their ML-DSA-65 public key:

```
agent_id = SHA3-256(ml_dsa_65_public_key)  # 32 bytes
```

The `Account` struct (already in state machine) gains no new fields — `pubkey` already exists. Registration creates the account if absent.

### Transaction Flow

```
Agent keygen → solve HashCash puzzle → sign RegisterTx → submit via CLI or agent loop
    → node validates puzzle → creates Account + TrustStageRecord → emits event
    → proposer includes airdrop tx in next block → AGX arrives
```

### `RegisterTx` Payload

```
RegisterPayload {
    pubkey: Vec<u8>,           // ML-DSA-65 public key (encoded)
    puzzle_nonce: u64,         // HashCash nonce satisfying difficulty
    puzzle_difficulty: u8,     // difficulty target at time of solving
    agent_name: Vec<u8>,       // human-readable name (max 64 bytes)
    capability_tags: Vec<Vec<u8>>,  // optional skill tags
    nonce: u64,                // always 0 for first registration
}
```

The state handler:
1. Verifies `pubkey` is valid ML-DSA-65 (correct length).
2. Computes `agent_id = SHA3-256(pubkey)`.
3. Checks `agent_id` does not already have an account (rejects duplicate).
4. Verifies HashCash puzzle: `SHA3-256(pubkey || puzzle_nonce)` has `>= puzzle_difficulty` leading zero bits.
5. Verifies `puzzle_difficulty` matches current on-chain difficulty target (computed from `registrations_this_epoch / epoch_cap`).
6. Creates `Account(agent_id, balance=0, nonce=0, pubkey)`.
7. Calls `init_trust_stage(agent_id)` — creates `TrustStageRecord(stage=Untrusted)`.
8. Records `birth_block = current_height` on a new `AgentMetadata` record.
9. Increments `registrations_this_epoch` counter.
10. Returns `Accepted`.

### HashCash Puzzle

Simple partial-preimage SHA3-256 search:

```
difficulty_target = base_difficulty + floor(registrations_this_epoch / epoch_cap * difficulty_scale)
```

The agent searches for `nonce` such that:
```
leading_zero_bits(SHA3-256(pubkey || nonce)) >= difficulty_target
```

Verification is O(1) — just hash and count bits. No state needed beyond the current `registrations_this_epoch`.

### Airdrop Disbursement

When a `RegisterTx` is accepted at height H, the proposer of block H+1 automatically includes an `AirdropDisbursementTx` (not a user-submittable tx type — it is protocol-injected): sends 100 AGX from the genesis airdrop agent account to the new agent's account, with 20 AGX marked as `locked_bond`.

The locked bond is tracked in a new `BondRecord`:
```
BondRecord {
    agent_id: Hash32,
    total_locked: u128,
    released_tranches: u8,       // bitmask: bit 0 = 1st task, bit 1 = 5th, etc.
    birth_block: u64,
}
```

Release triggers:
- Tranche 1 (5 AGX): on first accepted task completion (review verdict reaches Done).
- Tranche 2 (5 AGX): on 5th accepted task.
- Tranche 3 (5 AGX): on promotion to Trusted.
- Tranche 4 (5 AGX): on 20th accepted task.

Birth-block delay: all `TransferTx` from an agent whose `current_height - birth_block < 1000` are rejected.

### Trust Stage Wiring (Already Exists)

| Piece | Location | Status |
|-------|----------|--------|
| `TrustStageRecord` + `TrustStageEnum` | `state/src/lib.rs:180-191` | ✅ |
| `init_trust_stage()` | `state/src/state_machine.rs:1336` | ✅ |
| `record_accepted_work()` | `state/src/state_machine.rs:1346` | ✅ |
| `record_abuse()` + demotion | `state/src/state_machine.rs:1353` | ✅ |
| `run_trust_promotion()` | `state/src/state_machine.rs:1369` | ✅ |
| Trust stage in SMT root | `state/src/state_machine.rs:1523` | ✅ |
| Trust stage in snapshot | `state/src/state_sync.rs:98` | ✅ |
| Trust stage enforced in claim (2/6 leases) | `state/src/state_machine.rs:857` | ✅ |
| PDP quota matrix per trust stage | `pdp/src/quota.rs` | ✅ |
| `init_trust_stage()` called automatically | **NOWHERE** — only in test code | ❌ |

The only missing wiring is calling `init_trust_stage()` from `ConsensusDriver` when a transaction from a new agent is first processed.

---

## Work Breakdown

### Week 1: RegisterTx + State Handler

#### Day 1-2: TxType + Payload

Files to create/modify:
- `consensus/src/types.rs` — add `TxType::RegisterTx` variant
- `consensus/src/driver.rs` — add `RegisterPayload` struct (Encode + Decode)
- `state/src/state_machine.rs` — add `execute_agent_register()` handler
- `consensus/src/driver.rs` — wire `RegisterTx` dispatch in `execute_tx()`
- `node/src/rpc.rs` — add `/tx/register` endpoint

`execute_agent_register()` logic:
1. Decode `RegisterPayload` from `tx_payload`.
2. Verify ML-DSA-65 pubkey length.
3. Compute `agent_id = SHA3-256(pubkey)`.
4. Check `agent_id` not already in `self.accounts`.
5. Verify HashCash puzzle (stub for now — puzzle added Week 2).
6. Create `Account(agent_id, 0, 0, pubkey)`.
7. Call `self.init_trust_stage(agent_id)`.
8. Store agent metadata (birth block, name, tags).
9. Increment `registrations_this_epoch` on `ConsensusDriver`.
10. Return `Accepted`.

Tests:
- RegisterTx accepted for new agent with valid pubkey.
- RegisterTx rejected if agent_id already exists.
- RegisterTx rejected with invalid pubkey length.
- Account created with balance=0, nonce=0.
- `TrustStageRecord` created with `Untrusted`.
- State root changes deterministically after registration.

#### Day 3-4: RPC + CLI + Agent Loop Wiring

- `rpc.rs`: Add `/tx/register` — parse `pubkey`, `puzzle_nonce`, `agent_name`, `capability_tags`; construct RegisterPayload; submit to driver; return tx_hash + agent_id.
- `cli/commands/agent.rs`: Replace `Register` no-op with real RPC call to `/tx/register`. Remove fake `"tx_type": "task_create"` hack.
- `cli/commands/mod.rs`: Add `register_tx_hex()` helper that builds and signs a RegisterPayload.
- `agent/src/loop_.rs`: In `load_or_create()`, after generating identity, query `/agent/status` for the derived `agent_id`. If 404, generate and submit RegisterTx via CLI or direct RPC. Wait for airdrop before entering main loop.

Tests:
- CLI `hyperfluid agent register --name foo --tags bar` returns tx_hash + agent_id hex.
- CLI register accepted by running node (integration test: start node, register, query status, see `trust_stage: Untrusted`).
- Agent loop auto-registers on first `--run` if not on chain.

#### Day 5: Airdrop Disbursement

- `consensus/src/driver.rs`: In `produce_block()`, after processing all mempool transactions, check if any newly-registered agents need airdrop disbursement. If yes, inject a protocol-internal transfer from genesis airdrop agent to the new agent.
- `state/src/state_machine.rs`: Add `execute_airdrop_disbursement()` that transfers AGX and creates the initial `BondRecord`.
- `state/src/lib.rs`: Add `BondRecord` struct and `KeyPrefix::Bond = 0x15`.
- `state/src/state_machine.rs`: Add `bond_records: HashMap<Hash32, BondRecord>`, `get_bond()`, `release_bond_tranche()`, `release_bond_on_accepted_work()` called from task completion path.
- Enforce birth-block delay in `execute_transfer()`: reject if `current_height - birth_block < 1000`.

Tests:
- Airdrop disburses 100 AGX to new agent at block H+1.
- Balance reads 100 AGX, 20 locked.
- Transfer rejected before birth-block delay expires.
- Transfer accepted after 1,000 blocks.
- Bond release: each accepted task at milestones triggers tranche release.
- `BondRecord` included in SMT root + snapshot.

### Week 2: HashCash Puzzle

#### Day 1-2: Puzzle Library

New crate or module: `consensus/src/hashcash.rs` (lightweight, no new crate needed).

```
pub fn solve(pubkey: &[u8], difficulty: u8, max_attempts: u64) -> Option<u64>
pub fn verify(pubkey: &[u8], nonce: u64, difficulty: u8) -> bool
pub fn current_difficulty(registrations_this_epoch: u64, epoch_cap: u64, base_difficulty: u8) -> u8
```

Puzzle: find `nonce` where `leading_zero_bits(SHA3-256(pubkey || nonce.to_le_bytes())) >= difficulty`.

Tests:
- `solve` returns valid nonce for difficulty 8 (fast).
- `verify` accepts correct nonce, rejects wrong nonce.
- `verify` fails on wrong pubkey.
- `current_difficulty` scales with registration rate: 0 regs → base, epoch_cap regs → base + scale, 2x epoch_cap → base + 2*scale (linear cap, never exceeds 32).

#### Day 3: Wire puzzle into RegisterTx

- `state/src/state_machine.rs`: In `execute_agent_register()`, before creating account, call `hashcash::verify(pubkey, puzzle_nonce, puzzle_difficulty)`. Reject if invalid.
- `state/src/state_machine.rs`: Verify `puzzle_difficulty == hashcash::current_difficulty(registrations_this_epoch, epoch_cap, base_difficulty)`. Reject if mismatch.
- `consensus/src/driver.rs`: Add `registrations_this_epoch` counter, reset at epoch boundary.
- `consensus/src/driver.rs`: Add `epoch_cap` and `base_difficulty` to `GenesisConfig` as `[TUNE]` parameters.

Tests:
- RegisterTx with valid puzzle is accepted.
- RegisterTx with wrong nonce is rejected with `InvalidPuzzle` reason.
- RegisterTx with wrong difficulty target is rejected.
- RegisterTx accepted during high registration rate uses higher difficulty.
- After epoch boundary, `registrations_this_epoch` resets.

#### Day 4: Integration into agent loop

- `agent/src/loop_.rs`: In auto-registration path, before submitting RegisterTx, solve the HashCash puzzle. Query current difficulty from `/query/registration-difficulty` RPC endpoint.
- `node/src/rpc.rs`: Add `/query/registration-difficulty` — returns current difficulty target and `registrations_this_epoch`.
- `agent/src/loop_.rs`: Show progress indicator during puzzle solving (important for UX — may take seconds to minutes).

Tests:
- Agent loop with `StubProvider` successfully auto-registers (stub can't solve real puzzles, so test with a low-difficulty override or mock the solver).
- Integration: start node with low base_difficulty (1 bit), run agent with `--auto-register`, verify agent appears on chain.

#### Day 5: CLI puzzle integration

- `cli/commands/agent.rs`: `hyperfluid agent register` optionally accepts `--auto-solve` (default true) which solves the puzzle locally before submission. `--puzzle-nonce` for external solvers.
- `cli/commands/agent.rs`: Print estimated solve time based on current difficulty. Print progress dots.

Tests:
- `hyperfluid agent register --auto-solve` produces valid RegisterTx.
- `hyperfluid agent register --puzzle-nonce 12345` uses specified nonce (for testing).

### Week 3: Wiring, Promotion, E2E

#### Day 1-2: Trust Stage Wiring + Promotion

- `consensus/src/driver.rs`: In `execute_tx()` dispatch, for any tx from an agent_id not in `trust_stages`, call `state_machine.init_trust_stage(agent_id)` lazily (but only for non-RegisterTx — RegisterTx already does it).
- `consensus/src/driver.rs`: At epoch boundary in `produce_block()`, call `state_machine.run_trust_promotion()`. Log promoted agents.
- `consensus/src/driver.rs`: At epoch boundary, call `state_machine.run_trust_demotion()` if abuse flags > 0 (already exists in `record_abuse`).
- Wire `record_accepted_work()` into the task completion path in the state machine (when review verdict reaches Done, call `record_accepted_work()` on the task's `primary_owner`).

Tests:
- Agent starts Untrusted, completes 10 tasks, gets promoted to Trusted at next epoch boundary.
- Agent with abuse flags does not get promoted (even with 10+ tasks).
- High-severity abuse demotes Trusted back to Untrusted.
- New agent submitting a ClaimTaskTx without prior RegisterTx automatically gets `init_trust_stage` called.

#### Day 3-4: PDP Quota Wiring for Registration

- `pdp/src/types.rs`: Add `ActionType::AgentRegister` if not present.
- `pdp/src/rule_chain.rs`: In `evaluate()`, handle `ActionType::AgentRegister` — skip signature verification if account doesn't exist yet (registration is the first action). Apply a special `register_quota` (1 per identity, ever).
- `pdp/src/quota.rs`: Add `register_quota` entry with `(1, 1)` for all stages (one registration per identity, lifetime).

Tests:
- RegisterTx passes PDP with correct action type.
- Duplicate RegisterTx for same agent_id is rejected by PDP quota (1 lifetime).
- Signature verification for RegisterTx uses the embedded pubkey, not a pre-existing key binding.

#### Day 5: E2E Integration Test + Documentation

- `node/tests/`: New `e2e_agent_onboarding.rs` — full lifecycle:
  1. Start single-validator node with low puzzle difficulty.
  2. Generate fresh ML-DSA-65 identity.
  3. Query `/agent/status` — expect 404.
  4. Solve puzzle, sign RegisterTx, submit via RPC.
  5. Query `/agent/status` — expect `trust_stage: Untrusted`, `balance: 100 AGX`, 20 locked.
  6. Attempt transfer — expect rejection (birth-block delay).
  7. Fast-forward 1000 blocks (via `produce_block` loop in test).
  8. Transfer succeeds.
  9. Create task, claim, work, submit, get reviewed → accepted.
  10. Verify bond release at milestones.
  11. Run 10 accepted tasks, trigger epoch promotion, verify Trusted.
  12. Verify Trusted agent can create tasks, review, has 6 lease capacity.
- Update `docs/04-specifications/runtime/agent-runtime-spec.md` Section 1 to describe auto-registration flow.
- Update `PROJECT-STATUS.md` with completion status.
- Update `docs/08-handoff/latest/build-status.md` with new stage entry.

---

## Concurrent Track: Multi-Validator BFT Productionisation

Agent onboarding (above) runs on the existing single-validator node. Multi-validator BFT has been wired and tested (2-3 node integration tests pass) but has deferred items that block production use. These are independent from onboarding — they touch different crates (`consensus`, `p2p`) and can proceed in parallel.

### Deferred Items (from Stage 01/02)

| Item | Est. Lines | Location | Blocked By |
|------|------------|----------|------------|
| Malachite effect handler | ~300 | `consensus/src/effect_handler.rs` (new) | Nothing — pure library code |
| Clatter network bridge wired to TCP | ~500 | `consensus/src/network_bridge.rs` + `p2p/src/tcp.rs` | Effect handler |
| ClatterHandshake ML-DSA-64 identity binding | ~150 | `p2p/src/secure_channel.rs` | Nothing |
| BFT wire encoding: hand-rolled binary → SCALE | ~250 | `consensus/src/network_bridge.rs` | Nothing |
| Multi-node BFT soak test (24h) | ~200 | `node/tests/multi_node_bft_soak.rs` (new) | All above |

### Week 1: Effect Handler + SCALE Wire Encoding

#### Day 1-2: Malachite Effect Handler

The effect handler bridges Malachite's consensus events to Hyperfluid's state machine. It currently does not exist — `run_bft_loop` receives consensus events but has no handler for them.

`consensus/src/effect_handler.rs`:

```rust
pub struct HyperfluidEffectHandler {
    driver: Arc<Mutex<ConsensusDriver>>,
    state: Arc<Mutex<BftState>>,
}

impl EffectHandler<HyperfluidContext> for HyperfluidEffectHandler {
    // Handle propose: build block from mempool, sign, return
    // Handle process_block: apply to state machine
    // Handle commit: finalize block in block store
    // Handle timeout: advance round
    // Handle gossip: relay to peers
    // Handle validator_set_change: update committee
}
```

Each handler maps a Malachite effect to a `ConsensusDriver` operation. The key ones:

- **`Propose`**: Lock driver, drain mempool, build `Block`, SCALE-encode, sign with ML-DSA-65, return `ProposedBlock`.
- **`ProcessProposal`**: SCALE-decode, verify proposer signature, run state machine, return `Valid`/`Invalid`.
- **`Commit`**: Append block to `block_store`, update `height`/`epoch`, emit events.
- **`Timeout`**: Advance round, trigger new proposal.
- **`ValidatorSetUpdate`**: At epoch boundary, compute new committee via `Committee::sample()`.

Tests:
- Effect handler produces valid block from mempool.
- Effect handler processes proposal and returns correct validity.
- Effect handler commits block and advances state.
- Timeout triggers round advancement.
- Validator set update produces deterministic committee.

#### Day 3: SCALE Wire Encoding

The current `network_bridge.rs` uses hand-rolled binary encoding (`0x01` tag for votes, `0x02` for proposals, followed by raw byte fields). This causes partition with any third-party BFT reimplementation.

Replace with SCALE-encoded messages:

```rust
#[derive(Encode, Decode)]
struct BftNetworkMessage {
    msg_type: BftMessageType,  // Vote, Proposal, WALEntry
    round: u64,
    validator_id: Hash32,
    signature: Vec<u8>,         // ML-DSA-65
    payload: Vec<u8>,           // SCALE-encoded Vote or Proposal
}
```

Changes:
- `network_bridge.rs`: Replace `encode_vote`/`decode_vote`/`encode_proposal`/`decode_proposal` with `encode_bft_message`/`decode_bft_message` using SCALE.
- `main.rs` consensus handler: Match on decoded `BftMessageType` instead of raw `0x01`/`0x02` bytes.
- Remove the `GAP-N03` entry from `build-status.md`.

Tests:
- Roundtrip: encode vote → decode → same vote.
- Roundtrip: encode proposal → decode → same proposal.
- Malformed SCALE bytes return decode error (no panic).
- Wire format is identical across all platforms (SCALE determinism).

#### Day 4: ClatterHandshake ML-DSA-65 Identity Binding

The current `ClatterHandshake::initiator()` takes `_identity: &Identity` but never uses it. The `remote_id` returned to the caller has zero cryptographic binding to the DH/KEM key exchange — an active MITM can claim any peer ID.

Fix: Sign the DH/KEM handshake output with the claimed identity's ML-DSA-65 key.

```
Initiator:
    handshake_output = Noise_XX_hybrid(local_dh, local_kem, remote_dh, remote_kem)
    signature = ML-DSA-65.sign(identity_secret_key, handshake_output)
    send(handshake_output ++ signature ++ pubkey)

Responder:
    receive(data)
    handshake_output = parse(data[..handshake_len])
    signature = parse(data[handshake_len..handshake_len+3309])
    pubkey = parse(data[handshake_len+3309..])
    agent_id = SHA3-256(pubkey)
    verify ML-DSA-65.verify(pubkey, handshake_output, signature)
    if valid: remote_peer_id = agent_id
    else: reject
```

Changes:
- `p2p/src/secure_channel.rs`: Extend `ClatterHandshake::initiator()` and `responder()` to sign and verify the handshake output.
- `p2p/src/identity.rs`: Add `sign_handshake()` and `verify_handshake()` methods.
- `p2p/src/tcp.rs`: Thread identity through `connect_and_maintain()` and `accept_loop()`.
- Update `main.rs` callers.

Tests:
- Honest handshake: initiator and responder agree on remote peer ID.
- MITM with wrong key: signature verification fails, handshake rejected.
- Tampered handshake output: signature mismatch, handshake rejected.
- Empty/zero-length key material returns error (not panic).

#### Day 5: Wire effect handler into run_bft_loop

- `consensus/src/malachite_consensus.rs`: Instantiate `HyperfluidEffectHandler` and pass to `run_bft_loop()`.
- `consensus/src/malachite_consensus.rs`: Route Malachite effects through the handler instead of the current stub dispatch.
- Remove `SPEC_DEVIATION` entries in `build-status.md` related to effect handler.
- Verify `run_bft_loop()` processes a full consensus round (propose → process → commit) in test.

### Week 2: Network Bridge + Multi-Node BFT

#### Day 1-2: Wire network bridge to real TCP sockets

The current network bridge has `outgoing: mpsc::Sender<ConsensusNetworkMsg>` and `peers: Vec<mpsc::Sender<Vec<u8>>>` but is not connected to the TCP transport layer. Consensus messages are never actually sent over the wire in production.

Changes:
- `consensus/src/network_bridge.rs`: Replace `mpsc::Sender`-based peer management with TCP sender handles. Each peer gets a `tokio::sync::mpsc::Sender<Vec<u8>>` that feeds into the TCP write task.
- `p2p/src/tcp.rs`: In `connect_and_maintain()`, return a sender handle that the network bridge can use to send SCALE-encoded BFT messages to that peer.
- `consensus/src/network_bridge.rs`: `broadcast()` iterates all connected peers and sends via TCP. `send_to(validator_id)` sends to a specific peer.
- `consensus/src/malachite_consensus.rs`: Wire network bridge into effect handler so `broadcast`/`send_to` are called on propose/commit/gossip effects.

Tests:
- Two nodes connect via TCP, exchange BFT messages (votes, proposals) through the network bridge.
- Disconnected peer: send fails gracefully (log warning, remove from peer list).
- Proposer broadcasts proposal to all peers; peers receive and process.

#### Day 3: Multi-node BFT convergence test

- `node/tests/multi_node_bft_networking.rs`: Extend existing multi-node harness to test:
  1. 3 nodes, all connect via TCP, reach consensus on block 1.
  2. All 3 nodes have identical state root after 10 blocks.
  3. Network partition: isolate node 3, 2 nodes continue. Reconnect node 3, it catches up via state sync.
  4. Byzantine node: a 4th node sends conflicting proposals. 3 honest nodes slash it.
- Measure block propagation time, finality latency, state sync duration.

#### Day 4: Identity binding validation in multi-node tests

- All multi-node BFT tests must use real ML-DSA-65 identities (not the mock key provider).
- Verify that `ClatterHandshake` with identity binding prevents peer ID spoofing in multi-node setup.
- Test that a node with a corrupted `node_key` file cannot join the network (handshake rejected).

#### Day 5: CI hardening + documentation

- Remove all `SPEC_DEVIATION` entries related to BFT networking from `build-status.md`.
- Remove `DEFERRED to Stage 03` entries for effect handler, network bridge, identity binding.
- Verify `cargo test --workspace` passes with multi-node BFT networking tests.
- Verify `cargo clippy --workspace --all-targets -- -D warnings` passes.
- Update `PROJECT-STATUS.md` blockers section.

### Multi-Validator Concrete Changes Summary

| File | Change | Est. Lines |
|------|--------|------------|
| `consensus/src/effect_handler.rs` | NEW — propose, process, commit, timeout, validator_set handlers | +300 |
| `consensus/src/network_bridge.rs` | SCALE wire encoding + TCP sender integration | +250 |
| `consensus/src/malachite_consensus.rs` | Wire effect handler, route effects, remove stub dispatch | +80 |
| `p2p/src/secure_channel.rs` | ML-DSA-65 handshake signing + verification | +100 |
| `p2p/src/identity.rs` | `sign_handshake()` / `verify_handshake()` methods | +50 |
| `p2p/src/tcp.rs` | Return sender handle from connect_and_maintain | +30 |
| `node/src/main.rs` | Simplify multi-validator path, use effect handler | +20 |
| `node/tests/multi_node_bft_networking.rs` | NEW — TCP-connected multi-node BFT tests with identity | +300 |
| **Total new code** | | **~1,130** |

---

## Dependency Graph

```
Agent Onboarding Track              Multi-Validator Track
──────────────────────              ──────────────────────
Week 1: RegisterTx          │       Week 1: Effect Handler
        RPC + CLI           │               SCALE Encoding
        Airdrop             │               Identity Binding
                            │
Week 2: HashCash            │       Week 2: TCP Network Bridge
        Agent loop wiring   │               Multi-Node Tests
                            │               CI Hardening
Week 3: Trust promotion     │
        PDP gates           │
        E2E tests           │
                            │
Both tracks converge at:    │
  • Multi-validator node with registered agents
  • Agents submit transactions via BFT consensus
  • Trust promotion works under multi-validator
  • E2E: agent registers → works → promoted on 3-node BFT network
```

The two tracks are independent until the final integration step. Agent onboarding works on single-validator mode (the default today). Multi-validator productionisation works independently. They can be built in parallel, then validated together.

---

## Concrete File Changes Summary (Onboarding Track)

| File | Change | Est. Lines |
|------|--------|------------|
| `consensus/src/types.rs` | Add `TxType::RegisterTx` | +1 |
| `consensus/src/driver.rs` | Add `RegisterPayload`, dispatch in `execute_tx()`, airdrop injection in `produce_block()`, `registrations_this_epoch` counter, epoch-reset logic, lazy `init_trust_stage()` for first-time agents | +120 |
| `consensus/src/hashcash.rs` | NEW — `solve()`, `verify()`, `current_difficulty()` | +80 |
| `state/src/lib.rs` | Add `BondRecord`, `AgentMetadata` structs, `KeyPrefix::Bond = 0x15` | +30 |
| `state/src/state_machine.rs` | Add `execute_agent_register()`, `execute_airdrop_disbursement()`, `bond_records` collection, `release_bond_tranche()`, birth-block enforcement in `execute_transfer()`, `record_accepted_work()` call in task completion path, `agent_metadata` collection, SMT root inclusion for bonds + metadata | +200 |
| `state/src/state_sync.rs` | Add bonds + agent metadata to `snapshot_state()` + `build_smt_from_keys()` | +20 |
| `node/src/rpc.rs` | Add `/tx/register`, `/agent/status` (full: trust_stage, balance, bond, birth_block), `/query/registration-difficulty` | +80 |
| `cli/commands/agent.rs` | Replace register no-op with real RPC call; add `--auto-solve` flag | +50 |
| `cli/commands/mod.rs` | Add `register_payload_hex()` helper | +20 |
| `agent/src/loop_.rs` | Auto-registration flow in `load_or_create()`: query status → if absent, solve puzzle → submit RegisterTx → wait for airdrop | +60 |
| `pdp/src/types.rs` | Add `ActionType::AgentRegister` if absent | +1 |
| `pdp/src/rule_chain.rs` | Handle `AgentRegister` action, skip key-binding check for new agents | +15 |
| `pdp/src/quota.rs` | Add `register_quota` entry (1 lifetime) | +3 |
| `node/tests/e2e_agent_onboarding.rs` | NEW — full lifecycle E2E test | +200 |
| `consensus/src/genesis.rs` | Add base_difficulty, epoch_cap to GenesisConfig | +4 |
| **Total new code** | | **~884** |

Plus 15-20 new unit tests across the state machine, consensus driver, and PDP modules, and 1 new integration test file.

---

## Appendix: Status Honesty — What "Complete" Actually Means

The project uses "COMPLETE" to mean "the code compiles and unit tests pass." It does NOT mean "production-ready" or "the feature works in a real deployment." Below is the honest delta between claimed status and reality across every major subsystem.

### Multi-Validator BFT — CLAIMED: Complete | REALITY: Single-Validator Only

| Required Piece | Status | Lines |
|----------------|--------|-------|
| Malachite types + Context (Address32, ValidatorSet, Vote, Proposal) | ✅ Done | ~410 |
| ML-DSA-65 SigningScheme | ✅ Done | ~100 |
| BftDriver wrapping core-driver::Driver | ✅ Done | ~280 |
| `run_bft_loop()` wired in main.rs for `--multi-validator` | ✅ Done | ~140 |
| TCP accept loop with consensus message handler | ✅ Done | ~80 |
| Outbound peer key resolution (`--peers` with DH/KEM keys) | ✅ Done | ~150 |
| Multi-node BFT integration test (2-3 nodes) | ✅ Done | ~200 |
| **Malachite effect handler** (Propose → build block, ProcessProposal → validate, Commit → persist) | ❌ Not built | ~300 |
| **Network bridge wired to TCP sockets** (broadcasts to real peers) | ❌ Not built | ~500 |
| **ClatterHandshake ML-DSA-65 identity binding** (MITM prevention) | ❌ Not built | ~150 |
| **SCALE wire encoding** (currently hand-rolled binary — incompatible with third-party BFT impls) | ❌ Not built | ~250 |
| **24-hour soak test** (never validated beyond minutes) | ❌ Deferred | ~200 |
| **Total missing for production multi-validator** | | **~1,400** |

**Impact:** The default `hyperfluid-node` runs in single-validator mode. The `--multi-validator` flag exists and has been tested with 2-3 nodes, but the effect handler + network bridge + identity binding are all stubs. This is not a BFT blockchain today — it is a single-node sequencer with BFT scaffolding.

### Agent Onboarding — CLAIMED: Minor no-op | REALITY: Full Pipeline Missing

| Required Piece | Status | Lines |
|----------------|--------|-------|
| `TxType::RegisterTx` variant | ❌ Not built | - |
| `execute_agent_register()` state handler | ❌ Not built | - |
| HashCash proof-of-agent puzzle (FR-0176) | ❌ Not built | ~80 |
| Puzzle difficulty scaling by registration rate | ❌ Not built | ~20 |
| Airdrop disbursement (100 AGX, 20 locked) | ❌ Not built | ~60 |
| Progressive Sybil bond release (4 tranches) | ❌ Not built | ~80 |
| Birth-block delay (1,000 blocks before transfers) | ❌ Not built | ~20 |
| `/tx/register` RPC endpoint | ❌ Not built | - |
| `/agent/status` returning trust_stage + bond + birth_block | ❌ Not built | - |
| CLI `hyperfluid agent register` real tx (currently returns "not_implemented") | ❌ Not built | - |
| Agent loop auto-registration on first `--run` | ❌ Not built | - |
| `init_trust_stage()` called automatically for first-time agents (currently only called in test code) | ❌ Not wired | - |
| **Total missing** | | **~1,100** |

**Impact:** An agent with 0 AGX cannot autonomously join the network today. The trust stage state machine (`TrustStageRecord`, `run_trust_promotion()`, PDP quota gating) works once populated, but nothing creates the initial record. Agents exist only if manually added to genesis config.

### OS-Level Agent Sandbox — CLAIMED: Logic built | REALITY: No Isolation

| Required Piece | Status | Lines |
|----------------|--------|-------|
| Disk quota enforcement | ✅ Done | ~40 |
| File descriptor limit check | ✅ Done | ~30 |
| **cgroups CPU/memory isolation** | ❌ Deferred (Linux-only) | ~200 |
| **seccomp system call filtering** | ❌ Deferred (Linux-only) | ~300 |
| **namespace process isolation** | ❌ Deferred (Linux-only) | ~150 |
| **Total missing** | | **~650** |

**Impact:** The agent's `bash` tool runs with no sandboxing. `rm -rf /`, crypto miners, network exfiltration — all trivially possible. The only protection is "the agent is assumed to be honest" which defeats the entire purpose of untrusted-agent infrastructure.

### State Sync — CLAIMED: Complete | REALITY: Dead Code

| Required Piece | Status | Lines |
|----------------|--------|-------|
| `snapshot_state()` | ✅ Exists but `#[allow(dead_code)]`, never called | ~100 |
| `build_smt_from_keys()` | ✅ Exists but `#[allow(dead_code)]`, never called | ~30 |
| `compute_state_checksum()` | ✅ Exists but `#[allow(dead_code)]`, never called | ~20 |
| `verify_snapshot_checksum()` | ✅ Exists but `#[allow(dead_code)]`, never called | ~10 |
| Wire into RPC for peer-to-peer state sync | ❌ Not built | ~200 |
| **Total missing** | | **~200** |

**Impact:** A node that falls behind cannot catch up. All nodes must start from genesis. There is no sync protocol, no checkpoint serving, no snapshot download. Works in tests because every test starts a fresh node at genesis.

### Artifact Storage — CLAIMED: Complete | REALITY: Orphan Crate

| Required Piece | Status |
|----------------|--------|
| `hyperfluid-artifact` crate code | ✅ Compiles, tested |
| **Imported or used by `hyperfluid-node`** | ❌ Never imported. Zero `use hyperfluid_artifact` in any production crate. Listed in `Cargo.toml` but dead weight at link time. |

**Impact:** Content-addressed blob storage, chunking, Merkle proofs, proof-of-possession — ~1,500 lines of code that compiles and has its own unit tests, but is never linked into the node binary. The feature literally does not exist at runtime.

### Governance Deposits — CLAIMED: Complete | REALITY: Field Exists, Lifecycle Unwired

| Required Piece | Status |
|----------------|--------|
| `GovernanceProposal.deposit_amount` field | ✅ Defined |
| Deposit deducted from proposer on submission | ❌ Not wired |
| Deposit returned on successful execution | ❌ Not wired |
| Deposit burned on frivolous proposal | ❌ Not wired |

**Impact:** Anti-flood economic security for governance relies on deposits. Without them, proposing is free — a (hypothetical) Sybil attacker could spam governance proposals at no cost.

### Challenge Bonds — CLAIMED: Complete | REALITY: Same as deposits

| Required Piece | Status |
|----------------|--------|
| `FastPathChallengeTx.challenger_bond` field | ✅ Defined |
| Bond deducted from challenger on submission | ❌ Not wired |
| Bond returned on successful challenge | ❌ Not wired |
| Bond slashed on frivolous challenge | ❌ Not wired |

**Impact:** Fast-path challenges are free. A (hypothetical) Sybil attacker could challenge every merge at no cost, halting the collaboration protocol.

### Commit-Reveal Seed — CLAIMED: Staged | REALITY: Not Built

```rust
#[allow(dead_code, reason = "staged for commit-reveal in Stage 03")]
pub fn compute_committee_seed(...)
```

**Impact:** Committee randomness uses `SHA3-256(previous_seed || epoch)` — a deterministic fallback with no reveal mechanism. An attacker who can predict the committee selection 1-2 epochs ahead can target specific validators for corruption or censorship. The commit-reveal scheme exists on paper only.

### Cross-Platform Determinism — CLAIMED: Not tracked | REALITY: Validated on Windows Only

| Platform | Tested |
|----------|--------|
| Windows (x86-64) | ✅ Primary dev platform |
| Linux (x86-64) | ❌ Never validated |
| macOS (aarch64) | ❌ Never validated |

**Impact:** Integer overflow behaviour is specified, but `HashMap` iteration order (even with `BTreeMap` fixes), file system semantics, thread scheduling for timeouts, and SCALE encoding consistency have never been verified outside Windows. A single determinism bug under BFT would cause a consensus split (chain fork) that requires manual recovery.

### Sybil Resistance — CLAIMED: Three-Layer Defense | REALITY: Zero Layers Implemented

| Layer | Status |
|-------|--------|
| HashCash proof-of-agent puzzle | ❌ Not built |
| Progressive Sybil bond (20 AGX, 4 tranches) | ❌ Not built |
| Behavioural correlation detection | ❌ Removed as overengineering |

**Impact:** The only thing preventing Sybil attacks is that the registration tx type doesn't exist. Once it's built (this plan), there will be no Sybil resistance at all until the puzzle and bond are also built.

### Summary Table

| Subsystem | Claimed Status | Actual Status | Missing Lines | Blocks Production? |
|-----------|---------------|---------------|---------------|-------------------|
| Multi-Validator BFT | Complete | Single-validator only, ~1,400 lines missing | ~1,400 | **YES** — not a blockchain |
| Agent Onboarding | Minor no-op | Full pipeline missing | ~1,100 | **YES** — agents cannot join |
| OS Sandbox | Logic built | No isolation, vulnerable | ~650 | **YES** — agent tooling unsafe |
| State Sync | Complete | Dead code, never called | ~200 | **YES** — cannot catch up after disconnect |
| Artifact Storage | Complete | Orphan crate, not linked | ~1,500* | Partially — task system works without it |
| Governance Deposits | Complete | Field only, lifecycle unwired | ~80 | **YES** — no anti-flood for governance |
| Challenge Bonds | Complete | Field only, lifecycle unwired | ~60 | **YES** — no anti-flood for fast-path |
| Commit-Reveal Seed | Staged | Not built | ~100 | Risk — predictable committees |
| Cross-Platform Testing | Not tracked | Windows-only | ~100 (test infra) | Risk — consensus split on divergence |
| Sybil Resistance | Three-layer | Zero layers | ~150 | Risk — no defense after registration ships |
| **Totals** | | | **~5,540** | 5 blockers, 3 risks |

*Lines that exist but are dead code, counted in the repo but not in production.

This plan (Stage 02b) addresses the agent onboarding pipeline (~1,100 lines) and multi-validator BFT (~1,400 lines). The remaining ~3,000 lines of critical missing infrastructure must be addressed in subsequent stages before the system can be called production-ready.
