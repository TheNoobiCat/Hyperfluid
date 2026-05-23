# Stage 02: Agent Runtime

## Inputs
- From Stage 01: chain state machine (C2) functional, staking types (C3) defined, fee market algorithms (C5) implemented, P2P types + crypto (C7) defined, artifact Merkle logic (C8) implemented.
- **GAP NOTE (Resolved 2026-05-17):** Stage 01 does NOT yet have: actual BFT consensus (C1 is types + committee math only), actual P2P sockets (C7 has no TCP/UDP), actual disk storage (C8 has no file I/O), or a working node binary (consensus loop is a stub timer). These must be built BEFORE Stage 02's agent runtime can function end-to-end. See Integration Gate in BUILD-SYSTEM.md. **Resolution:** All 4 integration gaps filled — P2P TCP sockets (`hyperfluid-p2p/src/tcp.rs`), disk-backed storage (`hyperfluid-artifact/src/store.rs`), consensus driver with real block production (`hyperfluid-consensus/src/driver.rs`), node binary wired (block production loop replacing sleep stub). C4/C6/C9 wired into consensus driver. Multi-node test harness created. Stage 02 can now proceed to Week 3-4 (Agent Runtime + Sandbox + Operator Interface).
- From Layer 4 specs: governance-spec.md, fastpath-spec.md, agent-runtime-spec.md, policy-engine-spec.md, review-engine-spec.md, collaboration-spec.md, incident-response-spec.md.
- External: LLM provider SDKs (Anthropic, OpenAI, Ollama), ML-DSA-65 for agent key operations, `rusqlite` for agent-local state, sandbox runtime (WASM or Firecracker microVM).

## Outputs
- C4 Governance Engine: git:head on-chain state, proposal lifecycle (submit → vote → execute), sandbox review period, anti-flood deposit.
- C6 Fast-Path Topic Protocol: topic-scoped consensus merges, quorum certificates, challenge windows, rollback, promotion bridge.
- C9 Policy Decision Point: 5-step deterministic rule chain (schema → signature → replay → quota → fee), append-only audit log.
- C10 Agent Runtime: infinite agent loop, system prompt loader, core tool set, handoff mechanism, resource limits, process isolation, crash recovery via handoff replay, Telegram bot dashboard (optional), TUI setup wizard.
- C11 Collaboration & Inbox: task board, soft leases, single-agent task execution with dependency DAG and split, task submission with sponsorship (task_create action plan, PDP validation, gossip/DHT discovery), inbox routing, trust ladder (2 stages, promotion thresholds), bounty escrow mechanism, idea seed index with airdrop agent bootstrapping.
- C12 Economics & Review: review-as-task pipeline — workers submit, trusted agents claim review tasks from open pool, 2-reviewer majority verdict, 90/10 payout settlement.
- Incident Response: basic incident logging only. No circuit-breaker hierarchy — EIP-1559 base fee is sole congestion mechanism.
- Full agent lifecycle: join (0 AGX) → claim task → execute with tools → submit action_plan → PDP evaluates → reviewers attest → settlement → trust promotion.

## Exit Criteria
- [ ] Governance: proposal submitted, passes quorum vote, executes state change. No-vote timeout functions correctly (not counted toward quorum). Anti-flood deposit returned or slashed.
- [ ] Fast-Path: topic merge committed, quorum certificate verified, challenge window expires without challenge, promotion bridge activates.
- [ ] PDP: 5-step rule chain produces identical decisions on all nodes for identical inputs. Audit log is append-only and content-addressed.
- [ ] Agent Runtime: agent joins as `untrusted`, runs infinite loop, claims up to 2 tasks, submits action plans, progresses to `trusted` after 10 accepted tasks. TUI setup wizard writes valid config.toml. Telegram bot serves dashboard and /send commands (when configured).
- [ ] Collaboration: task board visible across nodes, soft leases prevent double-claim, bounty escrow locks funds on task creation and releases on completion after challenge window, all tasks reference valid seed_ref, `hyperfluid task submit` CLI creates tasks through PDP → state machine → gossip pipeline, `TaskCreated` events propagate via gossip/DHT to subscribed agents, inbox routes messages correctly.
- [ ] Review: review-as-task pipeline — completed work creates 2 review tasks in the pool, trusted agents claim review tasks, binary verdict (accept/reject), majority acceptance releases 90/10 payout, rejection returns task to Open. Reviewers paid regardless of verdict.
- [ ] EIP-1559 base fee: verified to adjust correctly under load, no protocol-level circuit-breaker.
- [ ] End-to-end agent workflow: 3 agents complete tasks with review, bounty payout, and trust-stage update. Sponsoring agent submits task on behalf of user. Telegram bot delivers dashboard status and (for sponsoring agents) processes task submission requests.
- [ ] All 14 specs pass their conformance test hooks (Section X.7).
- [ ] Risks documented and acceptable.
- [ ] Next stage inputs prepared.

## Duration Estimate
10–14 weeks. Telegram bot and TUI wizard are estimated at ~3 days combined. CLI crate estimated at ~1 week. Protocol catch-up items (slashing, rewards, commit-reveal seed, quota matrix) estimated at ~2 weeks collective. Soak testing estimated at ~1 week including bug fixes.

## Dependencies
- Stage 01 complete (chain, P2P, artifact storage functional).
- LLM provider API access (Anthropic/OpenAI keys or local Ollama deployment).
- Sandbox runtime decision: WASM (via Wasmtime) or Firecracker microVM. Decision must be made in Week 1.

## Week-by-Week Breakdown

### Pre-Flight: clatter+ml-dsa Secure Channel Implementation
1. Add `clatter` v2.2.0 (Noise hybrid XX) and `ml-dsa` v0.1.0-rc.11 (FIPS 204) as workspace dependencies.
2. Replace the mock `SecureChannel` in `crates/hyperfluid-p2p/src/transport.rs` with a real clatter-backed implementation. See ADR-0016 and `docs/01-research/stack-evaluations/clatter-vs-ockam-secure-channel.md`.
3. Integration shim: wrap clatter `HybridHandshake` → `TransportState` behind the existing `SecureChannel` trait (`establish()`, `seal()`, `open()`).
4. ML-DSA identity provider: keypair generation, signing, verification. Map pubkey → PeerId via SHA3-256.
5. Conformance tests: replace mock SHA3-256 XOR tests with real cryptographic roundtrip, wrong-key rejection, tampered-ciphertext rejection, and nonce advancement tests.
6. Feature-gate the mock behind `mock-secure-channel` (for fast unit tests); production code uses `clatter-secure-channel`.
7. Exit checkpoint: `cargo test -p hyperfluid-p2p` passes with real crypto; conformance hooks p2p-spec 1.7 hooks 7-8 verified with clatter.

### Week 1–2: Governance + Fast-Path + PDP (C4, C6, C9)
1. Governance engine: git:head state representation, proposal submission (target hash, proposed branch, sandbox period), vote window (5,040 blocks = ~7 days at 2s blocks), no-vote timeout, anti-flood deposit.
2. Fast-Path topic protocol: topic scope definition, merge proposal with quorum certificate (67/100), challenge window (1,440 blocks = ~48 min), rollback on successful challenge, promotion bridge to governance for permanent codification.
3. PDP rule chain: implement 5 steps in order (schema → signature → replay → quota → fee). Ensure determinism — no `HashMap` iteration, no floating-point in root authorization path, no time-based decisions. Structured deny reason codes.
4. Audit log: append-only, content-addressed (each entry hashes to previous entry). Queryable by plan_id and agent_id.
5. **INTEGRATION:** Wire PDP into the state machine's transaction validation path. Governance proposals must be processable by the node's transaction handler. Fast-Path merge decisions must produce observable state changes.
6. Exit checkpoint: governance proposal lifecycle works end-to-end; PDP rule chain rejects invalid action plans with correct reason codes; integration test demonstrates PDP + state machine + node binary processing a real governance transaction.

### Week 3–4: Agent Runtime + Sandbox + Operator Interface (C10)
1. Infinite agent loop: `load_system_prompt() → call_llm() → parse_action() → execute_tool() → check_token_count() → handoff_if_needed() → repeat`.
2. Core tool set: `claim_task`, `publish_output`, `request_review`, `read_state`, `submit_action_plan`. Agent tools defined in agent-runtime-spec.md §2 (bash, todo, remember, forget, read, edit, write, apply_patch) extended with on-chain interaction primitives.
2a. `hyperfluid task submit` CLI: `--title`, `--description-file`, `--bounty`, `--seed-ref` (required), `--required-skills`, `--sponsor` (optional). Constructs metadata artifact in gix, builds `task_create` action plan, signs, submits via PDP.
3. System prompt loader: loads base prompt from chain (governance-managed), merges with agent-local overrides, formats with state context.
4. Handoff mechanism: compress context at 70% token threshold or 50-message trigger → persist handoff record → next iteration loads resume context.
5. Resource limits: 4 GB RAM ceiling, 120-second tool timeout, max 128 concurrent tool calls, disk quota for local state.
6. Process isolation: agent runs in separate OS process or WASM sandbox. Network access restricted to chain RPC only. Filesystem access limited to designated working directory. No outbound internet (except LLM provider calls via proxy).
7. Crash recovery: on restart, load last handoff record, verify state consistency, resume loop.
8. Local SQLite for agent state: todos, knowledge base, failure log, inbox messages.
9. TUI setup wizard (ratatui): first-launch config flow (project name, agent name, LLM provider/URL/key, capability tags, optional Telegram config). Writes `config.toml`.
10. Telegram bot client (optional, tokio task): long-polling getUpdates, user ID binding, commands (/start, /status, /balance, /send), read-only dashboard from SQLite, AGX transfer via CLI command construction. Single-tenant, no agent control path.
10a. Telegram chat agent (FR-0193, FR-0200): dedicated telegram_chat_agent instance with read-only access to status snapshot + chat log. Responds to any message with read-only status. Only tool allowed: `hyperfluid task submit`.
10b. Telegram sponsored submission (FR-0200): operator sends natural-language task request; telegram_chat_agent refines scope, maps to seed_ref, estimates bounty, requests explicit confirmation (plain "yes"), submits `hyperfluid task submit --sponsor` only after confirmation. All refinement logic is off-protocol.
11. Config file (`config.toml`): serde-deserialized with `[agent]`, `[llm]`, `[telegram]` sections.
12. Exit checkpoint: single agent runs stable loop for 1 hour; survives tool timeout, token limit handoff, and process restart; TUI wizard writes valid config; Telegram bot responds to commands from configured user ID.

**Status: COMPLETE (2026-05-18)** — C10 Agent Runtime library built. All spec sections 1-4 implemented. 87 tests (64 unit + 23 conformance). Core tools (bash, todo, remember, forget, read, edit, write, apply_patch), infinite loop, handoff/crash recovery, system prompt assembly, resource limits/sandbox validation. Deferred: LLM provider integration (stub), TUI wizard (Section 5 optional), Telegram bot (Section 5 optional), OS-level sandbox enforcement (Linux-only), agent binary (library crate).



**GAP NOTE (Resolved 2026-05-18 — inbox budgets, topic decay, abuse evidence, replay prevention):** FR-0093 (global inbox budget 2000/hr), FR-0094 (topic budget 500/5min), FR-0095 (abuse evidence + quarantine), FR-0101 (topic decay lifecycle), and FR-0175 (freshness nonce for artifact replay prevention) are must-have requirements with no explicit build task in this week or any other. They are small but required by specs. If not absorbed into W5-6 implementation, must be added to W9-10.

### Week 5–6: P2P + Mempool + PDP Wire-Up (Critical Path)

The P2P networking, secure channels, discovery, and mempool are all independently implemented and tested. They are NOT wired into the running node binary. Week 5-6 connects them.

1. **Node P2P bootstrap:** Start `accept_loop()` (TCP listener) and `connect_to_peer()` on node boot in `main.rs`. Wire `run_connection_loop()` handlers into the node event loop. Peer discovery via DHT gossip.
2. **Mempool integration:** Replace the empty `vec![]` block production with real mempool transaction selection. `produce_block()` pulls from mempool, orders by fee, includes top N. Mempool rejects known-duplicate transactions.
3. **PDP state wiring:** Add agent key reference map, nonce tracking, and `QuotaState` tracking to `ConsensusDriver`. Enable `pdp_bypass = false`. Step 2 (signature verification) is still a stub — real ML-DSA verification deferred to Week 7-8.
4. **Networked state machine:** Two or more nodes connect via P2P, discover each other, and exchange transactions through the mempool/gossip layer. Single-validator block production per node (no BFT consensus — each node still produces its own chain).
5. **Multi-node test harness:** A 3-node network on localhost exchanging transactions and producing independent chains. Verify state root determinism across nodes receiving identical transactions.
6. **Exit checkpoint:** 3 `hyperfluid-node` processes connect via TCP, gossip transactions, produce blocks independently. PDP validates non-signature steps (schema, replay, quota, fee). `pdp_bypass` disabled in test.

### Week 7–8: BFT Consensus Integration

The Malachite type adapters exist (697 lines) but there is no propose/vote/commit loop. This week builds the remaining ~1,200 lines to make it a real multi-validator BFT network.

1. **Effect handler (~300 lines):** Route Malachite protocol effects (SendMessage, ScheduleTimer, RequestBlock, CommitBlock) to the Clatter network bridge, tokio timer, and state machine respectively. Implement the `EffectHandler` trait.
2. **Clatter network bridge (~500 lines):** Consensus message serialization/deserialization over Clatter secure channels. Topic-based routing (propose/vote/commit messages). Message timeouts and retransmission.
3. **Host actor (~400 lines):** Proposal building (pull from mempool), block validation, vote extensions. Integrates with existing `ConsensusDriver`.
4. **Disable local block production:** Replace `produce_block()` auto-loop with Malachite-driven block production triggered by leader proposal. BFT finality replaces single-node finality.
5. **Byzantine validation tests:** Equivocation detection, censorship resistance, proposal verification. Network of 4 validators with 1 byzantine.
6. **State sync integration:** Snapshot serving on catch-up. Sync from genesis for new nodes. Proof verification on received state.
7. **Exit checkpoint:** 4-validator BFT network produces agreed blocks. State roots converge. A byzantine validator (equivocating) is detected, slashed, and jailed.

### Week 9–10: Real PDP + CLI + TUI + Telegram + Inbox + Soak

1. **PDP signature verification (step 2):** Wire ML-DSA-65 signature checking into the PDP rule chain. `key_bindings` maintained in `ConsensusDriver` with key rotation support. Set `pdp_bypass = false`.
2. **`hyperfluid` CLI crate:** Implement all 7 top-level subcommands (tx, query, task, review, governance, agent, idea) with clap argument parsing. Machine-parseable JSON output. All mutating commands route through PDP.
3. **CLI → PDP → state machine pipeline:** End-to-end test of `hyperfluid tx transfer` constructing an `ActionPlanRequest`, PDP evaluating it, state machine executing it.
4. **TUI setup wizard (ratatui):** First-launch config flow (project name, agent name, LLM provider/URL/key, capability tags, optional Telegram config). Writes `config.toml`. Reads existing config for re-entry.
5. **Telegram bot client:** Long-polling getUpdates, user ID binding, commands (`/start`, `/status`, `/balance`, `/send`), read-only dashboard from SQLite. Single-tenant, no agent control path. Sponsored task submission via confirmation flow.
6. **Inbox router + agent messaging (off-chain):** Define `InboxMessage` type (sender_id, recipient_id, topic_id, body_bytes, nonce, signature). Build inbox router in C11 collaboration crate that validates PDP quotas (`inbox_msg_per_sender`, `inbox_global_per_agent`) at the `inbox_router` enforcement point. Route `GossipMessage` payloads through Bloom-filter deduplication → PDP inbox validation → agent inbox subscription. Wire agent loop with `check_inbox` tool and gossip subscription for topic-scoped and direct messages.
7. **Review sandbox subagent (real OS isolation):** When a task enters `InReview` OR a governance proposal enters its sandbox review period, spawn a stripped-down agent runtime with: all exploration tools (bash, read, edit, write) to inspect the submitted work artifact, evidence, or proposed protocol change (code artifacts, manifest bundles, diff bundles). A `review` tool for the verdict (accept/reject + reason + evidence hash), 30-minute timeout. Must run with **real OS-level sandboxing** — cgroups v2 memory/cpu limits, seccomp BPF syscall filter (allow only read/write/readlink/stat/fstat/mmap/shm/exit), mount namespace pivot_root to a tmpfs with only the artifact chunk + evidence + the review subagent binary visible. No outbound network (except loopback). No access to hyperfluid CLI — the subagent cannot submit transactions, claim tasks, or mutate chain state. The host process receives the verdict and submits the review tx on the subagent's behalf. Same sandbox serves both task review (C12) and governance proposal review (C4).
8. **Slashing + reward distribution:** Equivocation slashing from evidence forwarded to state machine. Downtime slashing from liveness tracking. Epoch-end fee rebates distributed to validators proportionally to stake.
9. **Cross-component soak:** 4-validator BFT network with agents claiming tasks, review pipeline (with real sandboxed review subagents), governance proposals, fast-path merges, inbox messages. 1000-block sustained run with zero state divergence.
10. **Exit checkpoint:** All CLI commands functional. TUI wizard writes valid config. Telegram bot responds to commands. Inbox router enforces quotas correctly. Review sandbox spawns with real OS isolation, subagent inspects artifact with tools, casts verdict, cannot escape sandbox. PDP fully enabled with signature verification. Slashing and rewards operational. 1000-block soak passes.

## Risk Areas
- **PDP determinism across platforms:** Rust floating-point and HashMap ordering are non-deterministic. The PDP spec mandates deterministic rule chain. Mitigation: use `BTreeMap` for ordered maps, `Vec` sort before iteration, no `f32`/`f64` in PDP, use `BTreeSet` for sets. Cross-platform CI (Linux, macOS, aarch64) validates determinism.
- **LLM provider availability and cost:** Rate limits, API outages, or cost spikes could stall agent testing. Mitigation: support Ollama local models for low-risk testing; cache LLM responses for deterministic replay in tests.
- **Sandbox escape:** Agent runtime process isolation must prevent filesystem and network escape. Mitigation: WASM sandbox with WASI preview2 (least privilege) or Firecracker microVM. No host network access except through a controlled proxy. Full sandbox escape threat model deferred to Stage 03.
- **BFT consensus complexity:** Malachite type adapters exist (697 lines) but the effect handler, network bridge, and host actor (~1,200 lines) are not written. Mitigation: incremental integration — first wire single-validator BFT (no networking), then add Clatter bridge for multi-validator.
- **Governance vote apathy:** If <33% of stake votes, proposals stall indefinitely. Mitigation: governance-spec no-vote timeout (non-vote = no quorum contribution, proposal expires).
- **Bounty escrow race conditions:** Concurrent task claims on the same bounty-funded task. Mitigation: escrow is locked at task creation; lease claim is atomic. Only the primary lease holder can trigger payout.
- **Reviewer collusion:** An attacker creating many trusted agents could attempt to stack review pools. Mitigation: abuse flag system in trust ladder detects collusive verdict patterns over time.

## Spec References

| Spec | Sections | Key Requirements |
|------|----------|-----------------|
| governance-spec.md | 1 (Proposals), 2 (Sandbox Review) | FR-0021–0030 |
| fastpath-spec.md | 1 (Topics) | FR-0031–0040 |
| agent-runtime-spec.md | 1 (Loop), 2 (Tools), 3 (Handoff), 4 (Isolation) | FR-0061–0075 |
| policy-engine-spec.md | 1 (PDP), 2 (Cross-Layer Quota Matrix) | FR-0106–0120 |
| review-engine-spec.md | 1 (Review-as-Task Pipeline) | FR-0161–0175 |
| collaboration-spec.md | 1 (Task Board), 2 (Trust Ladder) | FR-0076–0105 |
| incident-response-spec.md | 1 (Congestion) | FR-0144–0145 |

## Upstream Dependencies for Next Stage
- Full system must be functional end-to-end: agent → action_plan → PDP → chain → review → settlement → trust promotion.
- All 14 specs must have implementations passing their conformance test hooks. Any gaps are blockers for Stage 03.
- Integration test suite must be runnable as `cargo test --test '*integration*'` (Stage 03 extends this).
