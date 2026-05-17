## ADR-0018: Malachite Core-Library Integration (No Fork, No Engine Crate)

**Status:** accepted

**Context:** Hyperfluid's consensus-spec.md mandates committee BFT with rotating epoch committees of 100 validators, stake-weighted sampling, VDF-based randomness, and ML-DSA-65 signatures. The original plan (stage-01-protocol-core.md Week 1-2) called for integrating the full Malachite BFT engine. However:

1. Malachite's `engine` and `network` crates are **hardcoded to libp2p** (GossipSub, Kademlia, Noise/Yamux). Hyperfluid's spec mandates clatter PQ-Noise secure channels (ADR-0016). The network crate (`arc-malachitebft-network`, ~2,000 lines) has no trait abstraction — it directly spawns a libp2p swarm with concrete `PeerId`, `Multiaddr`, and `Config` types throughout.

2. Malachite's default signing scheme is Ed25519. While the `SigningScheme` trait in `core-types` is designed for pluggable crypto, the `engine` crate couples signing to libp2p validator proofs (ADR-006), which assume Ed25519 keypairs.

3. Malachite rotates validator sets per-height. Hyperfluid rotates per-epoch (many blocks per committee). The `HeightParams` reply in `Finalized` supports this, but requires adapter logic in the Host actor.

4. Three alternative integration approaches were evaluated:
   - **Fork Malachite, replace network crate:** ~1-2 weeks, ongoing merge conflict risk with upstream.
   - **Run both libp2p + clatter:** ~2-3 days, but ships two P2P stacks, doubles attack surface.
   - **Use Malachite `core-*` crates as pure libraries, bring your own network:** ~1 week, no fork, no libp2p dependency, full control over transport.

See the Malachite source analysis at `circlefin/malachite` commit `9109e96e`: `core-types/src/signing.rs` (39-line `SigningScheme` trait), `engine/src/network.rs` (685 lines, hardcoded libp2p), and the DeepWiki architecture documentation.

**Decision:** Integrate Malachite using only its `core-*` crates as pure libraries. Do NOT use the `engine`, `network`, `app`, `app-channel`, `discovery`, or `sync` crates. Build a custom integration layer that:

1. **Uses `SigningScheme` trait** — implement for ML-DSA-65 in a new `hyperfluid-signing-ml-dsa` module (~50 lines). No fork needed.

2. **Implements `Context` trait** — defines Hyperfluid's `Value` (Block), `Height`, `ValidatorSet` (Committee), `SigningScheme` (MlDsa65Scheme), and `proposer()` logic (~200 lines).

3. **Wraps `core-consensus` effect system** — Malachite's `process!` macro yields `Effect`s (SignVote, PublishConsensusMsg, GetValue, ScheduleTimeout, Decide, VerifyCommitCertificate). The effect handler routes each to the appropriate Hyperfluid component (~300 lines).

4. **Bridges to clatter network** — consensus messages are serialized and sent/received via clatter PQ-Noise secure channels. Peer discovery, validator proof exchange, and message routing are Hyperfluid-owned code (~500 lines).

5. **Implements Host actor** — handles proposal building, block validation, vote extensions, and commit logic (~400 lines).

**Crates used from Malachite:**

| Crate | Purpose | Why |
|-------|---------|-----|
| `arc-malachitebft-core-types` | `Context`, `SigningScheme`, `ValidatorSet`, `Vote`, `Proposal` | Generic type/trait definitions — no I/O |
| `arc-malachitebft-core-state-machine` | Tendermint round state machine (Propose→Prevote→Precommit→Commit) | Pure consensus logic — no I/O |
| `arc-malachitebft-core-votekeeper` | Vote accumulation, 2f+1 threshold detection, equivocation tracking | Pure consensus logic — no I/O |
| `arc-malachitebft-core-driver` | Orchestrates state machine + votekeeper + proposal keeper | Pure consensus logic — no I/O |
| `arc-malachitebft-core-consensus` | Top-level effect system: `process!` macro, Effect/Resume pattern | Pure consensus logic — no I/O |

**Crates NOT used from Malachite:**

| Crate | Reason | Lines Avoided |
|-------|--------|---------------|
| `arc-malachitebft-network` | Hardcoded libp2p (GossipSub, Kademlia, Noise) | ~2,000 |
| `arc-malachitebft-engine` | Spawns libp2p swarm, ractor actors | ~1,500 |
| `arc-malachitebft-app` | Host actor pattern too coupled to Malachite's types | ~800 |
| `arc-malachitebft-app-channel` | Channel bridge — we build our own | ~400 |
| `arc-malachitebft-discovery` | Kademlia DHT — we use clatter peer discovery | ~1,000 |
| `arc-malachitebft-sync` | ValueSync — we'll adapt for Hyperfluid's state sync | ~800 |
| `arc-malachitebft-wal` | Write-ahead log — may add later | ~500 |

**Integration architecture:**

```
┌─────────────────────────────────────────────────────┐
│  hyperfluid-consensus (your crate)                  │
│                                                      │
│  ConsensusDriver                                     │
│  (wraps malachite core-consensus)                   │
│    Input:  votes, proposals, timeouts               │
│    Output: Effects (Sign, Broadcast, Timeout)       │
│         │                                           │
│  Effect Handler (your code)                         │
│    Sign(MlDsa65) ──► hyperfluid-signing-ml-dsa      │
│    Broadcast(msg) ──► clatter network               │
│    ScheduleTimeout ──► tokio timer                  │
│    GetValue ──► hyperfluid-host / state machine     │
└─────────────────────────────────────────────────────┘
```

**Consequences:**
- Positive: No fork required. Malachite updates can be pulled via `Cargo.toml` version bump. ~2,300 lines of proven consensus logic reused (formally specified in Quint, model-checked). ~7,000 lines of libp2p coupling avoided. Full control over transport (clatter PQ-Noise), signing (ML-DSA-65), and epoch rotation logic. Clean separation: Malachite handles "how to agree," Hyperfluid handles "who agrees" and "how to talk."
- Negative: More integration glue to write (~1,500 lines) vs using Malachite's engine out of the box (~500 lines of config). Malachite's WAL, sync, and discovery crates are not used — equivalent functionality must be built. The `core-consensus` effect system is lower-level than the engine's actor API — more manual wiring.
- Mitigation: The effect handler is a single `match` statement — straightforward to test. The clatter network bridge reuses existing `SecureChannel` trait from ADR-0016. WAL can be added later if crash recovery proves necessary.

**Alternatives considered:**
- **Full Malachite engine (original plan):** Rejected — libp2p is hardcoded, incompatible with clatter PQ-Noise mandate (ADR-0016). Would require forking and replacing the network crate.
- **Fork Malachite, replace network crate:** Rejected — ongoing merge conflict risk with upstream. Circle/Malachite team is actively developing; a fork would drift. The `core-*` crates provide the same consensus logic without the coupling.
- **Commonware Simplex:** Considered — pluggable crypto and network by design, but no formal specs, no production users, indie project sustainability risk. Malachite has Quint specs, Circle backing, and production use (Arc, Starknet, Farcaster).
- **HotStuff-rs:** Considered — stable v0.4.0, but no activity since Dec 2024 (likely abandoned), no network layer, smaller community.
- **Write our own BFT:** Rejected — Tendermint/HotStuff consensus is subtle (view change, locked value, polka certificates). Reinventing is a multi-month effort with high correctness risk.

**Related:** ADR-0007 (Committee BFT with VDF), ADR-0016 (clatter+ml-dsa secure channel), `docs/04-specifications/protocol/consensus-spec.md`, `docs/05-planning/stages/stage-01-protocol-core.md`
