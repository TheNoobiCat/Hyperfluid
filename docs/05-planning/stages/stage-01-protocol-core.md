# Stage 01: Protocol Core

## Inputs
- From Stage 00: Cargo workspace, CI pipeline, local testnet scaffold.
- From Layer 4 specs: consensus-spec.md, staking-spec.md, p2p-wire-spec.md, fee-market-spec.md, state-sync-spec.md, artifact-availability-spec.md.
- External: Malachite BFT library, Ockam P2P library, ML-DSA-65 crate, Blake3 crate, SQLite via `rusqlite`, content-addressed storage via gix.

## Outputs
- C1 Consensus Engine: committee BFT integration (Malachite), VDF-based committee rotation, block production, SMT root commitment.
- C2 State Machine & SMT: sparse Merkle tree state, transaction execution, block finalisation, deterministic state transitions.
- C3 Staking & Validator Manager: four-state validator lifecycle (active, paused, unbonding, withdrawn), bonding/unbonding, slashing conditions, downtime tracking.
- C5 Fee Market: EIP-1559 base fee, validator rebates, front-running protection, fee-burning logic (off-chain only — see FR-0073 for runtime burn), fee adjustment formula.
- C7 P2P Networking: peer discovery, connection state machine, gossip protocol, mempool transaction relay, fee-ordered priority queue.
- C8 Artifact Availability & Storage: content-addressed storage via gix, proof-of-possession, retention tiers, repair coordinator.
- Single-node chain fully functional; multi-node network booting with gossip consensus between 3+ validators.
- State sync implementation (snap sync and full sync).

## Exit Criteria
- [ ] Single-node chain: produces blocks, processes transactions, finalises state, commits SMT roots.
- [ ] Multi-node network: 3+ validators reach consensus, gossip transactions, finalise blocks.
- [ ] Staking: validators bond, pause, unbond, withdraw per four-state lifecycle. Slashing fires on equivocation and >20% downtime.
- [ ] Fee market: EIP-1559 base fee adjusts per target block gas usage. Validators receive correct rebates.
- [ ] P2P: peer discovery finds nodes, connections establish, gossip delivers transactions within expected latency.
- [ ] Artifact storage: content-addressed blobs written, retrieved, replicated. Proof-of-possession verified on read.
- [ ] State sync: snap-sync brings a lagging node to tip; full-sync replays from genesis.
- [ ] All 6 specs pass their conformance test hooks (Section X.7) at unit and integration level.
- [ ] No crash on 24-hour soak test with 3 validators, steady transaction load.
- [ ] Risks documented and acceptable.
- [ ] Next stage inputs prepared.

## Duration Estimate
6–8 weeks. Extend to 8 weeks if consensus integration requires custom Malachite adaptation. Extend beyond 8 weeks only if P2P connectivity across NAT/firewall topologies requires protocol-level changes.

## Dependencies
- Stage 00 complete (workspace, CI, testnet scaffold).
- Malachite BFT v0.x stable release (or vendored commit with known-good hash).
- Ockam P2P library stable release.
- gix (Gitoxide) for content-addressed storage operations.

## Week-by-Week Breakdown

### Week 1–2: Consensus + State Machine (C1 + C2) — **COMPLETE (2026-05-05)**
1. Integrate Malachite BFT: define validator identity, epoch structure, block proposal pipeline. (deferred: Malachite integration pending external dep)
2. Implement SMT-backed state: key-value state store, transaction execution, block finalisation, SMT root hash computation. — **DONE**
3. VDF-based committee rotation: deterministic committee from epoch seed (initially seeded from genesis; full VDF integration and tuning in Stage 03 (Validation)). — **DONE (deterministic SHA3-256 sampling; full VDF deferred)**
4. Transaction types: `TransferTx`, `StakeBondTx`, `UnbondRequestTx`, `WithdrawUnbondedTx`, `TaskCreateTx`, `GovernanceProposeTx`, `GovernanceVoteTx`, `EvidenceTx`, etc. — **DONE (12 tx types in TxType enum)**
5. Unit tests for state transitions; integration test for single-node block production. — **DONE (56 workspace tests)**
6. Exit checkpoint: `cargo test` passes for C1 and C2 crates; single-node testnet produces blocks. — **DONE (see checkpoint-2026-05-05d.md)**

### Week 3–4: Staking + Fee Market (C3 + C5)
1. Validator lifecycle state machine: `active` → `paused` (downtime trigger) → `unbonding` (user request, 14-day window) → `withdrawn`.
2. Stake-weighted committee sampling using `self_bond + total_delegated` as effective weight; operator identity deduplication via stake-graph anti-split clustering (see `stake-graph-analysis-spec.md`).
3. Delegation: `DelegateTx`, `UndelegateTx`, `WithdrawDelegationTx`, `SetCommissionTx`; DelegationRecord management; proportional slash propagation; commission rate constraints.
4. Slashing evidence pipeline: equivocation proof, downtime proof (signed headers missing >20% in window).
4. EIP-1559 fee market: base fee per block, priority tip, fee adjustment denominator (8), block weight targets.
5. Validator rebate distribution: proportional to signed blocks in epoch.
6. Integration test: 3-validator network with staking lifecycle and fee market.
7. Exit checkpoint: validators bond/unbond correctly; fees adjust to load; slashing fires on detected byzantine behavior.

### Week 5–6: P2P Networking + Artifact Storage (C7 + C8)
1. Peer discovery: bootstrap nodes, Kademlia DHT for validator discovery, connection state machine (outbound/inbound, keepalive, backoff).
2. Gossip protocol: transaction gossip (push), block gossip (push), mempool fee-ordered priority queue.
3. Relay mechanism: nodes behind NAT connect via relay nodes; relay transmits consensus messages.
4. Content-addressed storage: gix-based blob store, hash verification on write, proof-of-possession challenge on read.
5. Retention tiers: hot (all nodes, 30 days), warm (sample of nodes, 180 days), cold (archive nodes, indefinite).
6. Repair coordinator: periodic sweep detects missing blobs via hash tree comparison; schedules replication.
7. State sync: snap-sync (download SMT snapshot + recent blocks), full-sync (replay from genesis).
8. Exit checkpoint: 5-node network achieves consensus with 2 relay nodes. Artifact write/read/repair lifecycle works.

### Week 7–8: Integration, Soak, Polish
1. End-to-end integration: single-node boot → add validators → stake tokens → submit transactions → verify fee adjustment → unbond → withdraw.
2. 24-hour soak test: 3 validators, steady 1-tx-per-second load. No crashes, no memory leaks, no unbounded disk growth.
3. Parameter audit: all [TUNE] parameters from specs recorded with default values; calibration log created for Stage 03.
4. Conformance self-check: run each relevant spec's Section X.7 test hooks; document results.
5. Bug fixes and polish from soak test findings.
6. Exit checkpoint: all exit criteria met; conformance log written.

## Risk Areas
- **Malachite BFT mismatch with Hyperfluid specs:** Hyperfluid's committee model (exactly 100, anti-split clustering, VDF rotation) may not be a direct fit for Malachite's internals. Mitigation: first verify Malachite supports custom validator set changes at epoch boundaries. If not, adapt Hyperfluid's consensus-spec or compile a custom Malachite fork.
- **Ockam P2P version compatibility:** Ockam APIs may change between versions. Mitigation: pin Ockam dependency to a specific git commit hash. Re-evaluate at the start of each week.
- **State sync correctness:** Snap-sync reconstructed state must match SMT root exactly; divergence = network split risk. Mitigation: automated differential fuzzing: replay same transaction stream via full-sync and snap-sync; assert identical state roots.
- **gix repository-scale performance:** Content-addressed blob store may become a bottleneck at scale. Mitigation: benchmark with 100k+ blobs in Week 5; if performance degrades, add RocksDB-backed blob index.
- **Mempool congestion:** Cheaper transactions must not starve governance/evidence transactions. Mitigation: evidence/governance fee discounts per `p2p-wire-spec.md` Section 2; test with adversarial cheap-transaction flood.
- **Validator set churn instability:** Rapid bonding/unbonding of many validators could cause committee oscillation. Mitigation: epoch-boundary-only validator set updates (stake changes apply at epoch+2); test with 50% churn simulation.

## Spec References

| Spec | Sections | Key Requirements |
|------|----------|-----------------|
| consensus-spec.md | 1 (Committee BFT), 2 (SMT State) | FR-0001–0010 |
| staking-spec.md | 1 (Lifecycle), 2 (Slashing) | FR-0011–0020 |
| p2p-wire-spec.md | 1 (Discovery), 2 (Gossip) | FR-0041–0050 |
| fee-market-spec.md | 1 (EIP-1559), 2 (Rebates) | FR-0146, FR-0147, FR-0159, FR-0160 |
| state-sync-spec.md | 1 (Sync) | FR-0010, NFR-0009, NFR-0018, NFR-0019 |
| artifact-availability-spec.md | 1 (Storage), 2 (Availability) | FR-0051–0060 |

## Upstream Dependencies for Next Stage
- Chain must be stable: blocks finalise, SMT roots are deterministic, multi-node consensus works.
- P2P network must be functional: agents in Stage 02 submit action plans via the chain RPC (gRPC or JSON-RPC).
- Token balances must be queryable: PDP in Stage 02 checks stake and fee balances for quota enforcement.
- Artifact storage must accept blobs: agents produce review artifacts and collaboration evidence stored as content-addressed blobs.
