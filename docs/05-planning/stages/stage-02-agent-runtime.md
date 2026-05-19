# Stage 02: Agent Runtime

## Inputs
- From Stage 01: chain state machine (C2) functional, staking types (C3) defined, fee market algorithms (C5) implemented, P2P types + crypto (C7) defined, artifact Merkle logic (C8) implemented.
- **GAP NOTE (Resolved 2026-05-17):** Stage 01 does NOT yet have: actual BFT consensus (C1 is types + committee math only), actual P2P sockets (C7 has no TCP/UDP), actual disk storage (C8 has no file I/O), or a working node binary (consensus loop is a stub timer). These must be built BEFORE Stage 02's agent runtime can function end-to-end. See Integration Gate in BUILD-SYSTEM.md. **Resolution:** All 4 integration gaps filled — P2P TCP sockets (`hyperfluid-p2p/src/tcp.rs`), disk-backed storage (`hyperfluid-artifact/src/store.rs`), consensus driver with real block production (`hyperfluid-consensus/src/driver.rs`), node binary wired (block production loop replacing sleep stub). C4/C6/C9 wired into consensus driver. Multi-node test harness created. Stage 02 can now proceed to Week 3-4 (Agent Runtime + Sandbox + Operator Interface).
- From Layer 4 specs: governance-spec.md, fastpath-spec.md, agent-runtime-spec.md, policy-engine-spec.md, review-engine-spec.md, collaboration-spec.md, telemetry-spec.md, incident-response-spec.md.
- External: LLM provider SDKs (Anthropic, OpenAI, Ollama), ML-DSA-65 for agent key operations, `rusqlite` for agent-local state, sandbox runtime (WASM or Firecracker microVM).

## Outputs
- C4 Governance Engine: git:head on-chain state, proposal lifecycle (submit → vote → execute), sandbox review period, anti-flood deposit.
- C6 Fast-Path Topic Protocol: topic-scoped consensus merges, quorum certificates, challenge windows, rollback, promotion bridge.
- C9 Policy Decision Point: 5-step deterministic rule chain (schema → signature → replay → quota → fee), append-only audit log.
- C10 Agent Runtime: infinite agent loop, system prompt loader, core tool set, handoff mechanism, resource limits, process isolation, crash recovery via handoff replay, Telegram bot dashboard (optional), TUI setup wizard.
- C11 Collaboration & Inbox: task board, soft leases, single-agent task execution with dependency DAG and split, task submission with sponsorship (task_create action plan, PDP validation, gossip/DHT discovery), inbox routing, trust ladder (2 stages, promotion thresholds), bounty escrow mechanism, idea seed index with airdrop agent bootstrapping.
- C12 Economics & Review: review-as-task pipeline — workers submit, trusted agents claim review tasks from open pool, 2-reviewer majority verdict, 90/10 payout settlement.
- Telemetry: signed envelopes, aggregation pipeline, reconciliation, outlier detection.
- Incident Response: basic incident logging only. No circuit-breaker hierarchy — EIP-1559 base fee is sole congestion mechanism.
- Full agent lifecycle: join (0 AGX) → claim task → execute with tools → submit action_plan → PDP evaluates → reviewers attest → settlement → trust promotion.

## Exit Criteria
- [ ] Governance: proposal submitted, passes quorum vote, executes state change. No-vote timeout functions correctly (not counted toward quorum). Anti-flood deposit returned or slashed.
- [ ] Fast-Path: topic merge committed, quorum certificate verified, challenge window expires without challenge, promotion bridge activates.
- [ ] PDP: 5-step rule chain produces identical decisions on all nodes for identical inputs. Audit log is append-only and content-addressed.
- [ ] Agent Runtime: agent joins as `untrusted`, runs infinite loop, claims up to 2 tasks, submits action plans, progresses to `trusted` after 10 accepted tasks. TUI setup wizard writes valid config.toml. Telegram bot serves dashboard and /send commands (when configured).
- [ ] Collaboration: task board visible across nodes, soft leases prevent double-claim, bounty escrow locks funds on task creation and releases on completion after challenge window, all tasks reference valid seed_ref, `hyperfluid task submit` CLI creates tasks through PDP → state machine → gossip pipeline, `TaskCreated` events propagate via gossip/DHT to subscribed agents, inbox routes messages correctly.
- [ ] Review: review-as-task pipeline — completed work creates 2 review tasks in the pool, trusted agents claim review tasks, binary verdict (accept/reject), majority acceptance releases 90/10 payout, rejection returns task to Open. Reviewers paid regardless of verdict.
- [ ] Telemetry: signed envelopes aggregate across nodes, reconciliation detects drift, outlier detection flags anomalous nodes.
- [ ] EIP-1559 base fee: verified to adjust correctly under load, no protocol-level circuit-breaker.
- [ ] End-to-end agent workflow: 3 agents complete tasks with review, bounty payout, and trust-stage update. Sponsoring agent submits task on behalf of user. Telegram bot delivers dashboard status and (for sponsoring agents) processes task submission requests.
- [ ] All 14 specs pass their conformance test hooks (Section X.7).
- [ ] Risks documented and acceptable.
- [ ] Next stage inputs prepared.

## Duration Estimate
10–14 weeks. Extend beyond 12 weeks if VDF integration or governance sandbox execution requires debugging. Telegram bot and TUI wizard are estimated at ~3 days combined. CLI crate estimated at ~1 week. Protocol catch-up items (slashing, rewards, VDF, liveness tiers, quota matrix) estimated at ~2 weeks collective. Soak testing estimated at ~1 week including bug fixes.

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

### Week 5–6: Collaboration + Review + Economics + Sybil Detection (C11, C12)
1. Task board: global task queue, soft leases (claim window = 600 blocks, ~20 min), lease renewal, lease expiry → task returns to queue.
2. Bounty escrow: task creation deducts `bounty_agx` from creator's balance into task escrow. Payout on completion after review and challenge window. Refund on unclaimed expiry. Creator pays cancellation fee on refusal.
3. Airdrop agent: HashCash proof-of-agent puzzle with dynamic difficulty. Progressive bond release (4 tranches of 5 AGX). Seed task creation from Idea Seed Index with bounty funding from genesis seed pool allocation.
4. Task creation trust-stage quotas (FR-0195): untrusted: 0 active created tasks, trusted: 10 (per FR-0195). Enforced by PDP at `task_create` validation. `Q-TASK-CREATE-STAGE` added to quota matrix.
5. Task discovery via gossip/DHT (FR-0197): `TaskCreated` events propagated via clatter+ml-dsa secure channels over the P2P gossip layer (fanout 8, TTL 16, Bloom-filter dedup). DHT keyed by `SHA3-256(task_id)`. Anti-entropy reconciliation.
6. Inbox system: message routing by agent_id and topic_id, priority channels (review requests), spam filter (quota-gated per sender trust stage). Task creation events generate inbox signals for subscribed agents.
7. Trust ladder: promotion from `untrusted` → `trusted` at 10 accepted tasks + clean abuse record (per collaboration-spec.md §3).
8. Review engine: initial review (3 reviewers, binary verdict), challenge window, settlement (fixed payout on majority approval).
9. Reviewer independence: stake-graph analysis ensures no reviewer shares an operator cluster with the worker or another reviewer.
10. Anti-collusion: operator-cluster diversity enforcement — stake-graph funding-edge analysis at epoch boundary prevents same-cluster reviewer assignments. No multi-signal correlation engine.
11. Clawback: settlement reversed if collusion detected within governance-defined window. Funds clawed back to escrow pool.
12. Exit checkpoint: 3-agent collaborative task completes with review pipeline; bounty escrow/payout lifecycle works end-to-end; operator-cluster diversity enforced; trust ladder promotes agent.

**GAP NOTE (Traceability — FR-0060):** FR-0060 (Signed Telemetry Summaries with Quorum Validation) is a must-have requirement mapped to telemetry-spec.md but has no explicit build task in any week. Telemetry is listed in stage outputs ("signed envelopes, aggregation pipeline, reconciliation, outlier detection") but Week 5-6 tasks (C11, C12), Week 7-8 tasks, and Week 9-10 tasks do not include telemetry implementation. Must be absorbed into an existing week or given a new build slot.

### Week 7–8: Incident Response + Integration
1. Incident Response: basic incident logging only. Congestion handled by EIP-1559 base fee — no FSM, no emergency mode, no recovery ramp-up.
3. End-to-end integration test: network of 5 nodes, each running 2 agents — join, progress through trust ladder, complete collaborative task, face and survive attempted collusion attack, process incident.
4. Governance stress test: 10 concurrent proposals, multi-epoch vote windows, fast-path challenge + rollback.
5. Bug fixes and polish from integration test findings.
6. Exit checkpoint: all exit criteria met; full end-to-end agent workflow demonstrated; incident response lifecycle validated.

### Week 9–10: Protocol Catch-up
1. **Slashing execution:** Equivocation slashing (double-vote detection from conflicting blocks) and downtime slashing (sliding window of missed blocks). Proportional slash propagation through delegators. Jail → unjail via StakeRenewTx.
2. **Reward distribution:** Epoch-end computation of validator rewards from priority fees. Rebates proportional to active bonded stake. Automatic distribution — no claim transaction required.
3. **Liveness/downtime tracking:** Sliding-window tracking of block signing per validator. Hysteresis — brief offline events do not immediately slash.
4. **VDF integration:** Replace SHA3-256 fallback with full commit-reveal VDF scheme. VDF evaluation function. Seed derivation from VDF output for committee rotation. Epoch-boundary orchestration.
5. **Consensus liveness tiers:** Normal (67-100 validators active) → Degraded (50-66, critical-tx only) → Emergency (0-49, halt + auto-recovery after 500 idle blocks). Tier detection + tx filtering.
6. **Full cross-layer quota matrix:** Enforce all 14 quota entries (p2p_conn, p2p_tx_burst, p2p_gossip, inbox_msg, inbox_global, topic_msg, fast_merge, gov_proposals, gov_open, review_concurrent, lease_active, challenge, task_create). Trust-stage multipliers.
7. **Hermetic governance sandbox executor:** Isolated runtime with pinned gix toolchain for deterministic execution of governance merge proposals.
8. **Parameter bounds enforcement:** Governance-adjustable bounds on slash_pct, fee_burn_ratio, challenge_window, lease_bond_multiplier, etc. Bounds checked at proposal execution.
9. **Task splitting with dependency DAG:** `SplitTaskTx` — atomic redistribution of escrowed bounty to children, acyclic validation, gas cost proportional to child count.
10. **Lease collateral and penalty schedule:** Collateral = max(10 AGX, 0.5% bounty). LeasePenalty escalation: Warning → BudgetReduction 50% → SevereReduction 90% + trust regression. Per-agent lease caps by trust stage.
11. **Shadow claim promotion:** 8-minute grace window, ShadowClaim struct, promotion sort by trust_score desc then submitted_at_height asc.
12. **FundingEdge stake-graph pipeline:** 3-hop backward walk at each epoch boundary, cluster ID = SHA3-256(sorted(member_ids)), FundingEdge pruning at 100k blocks.
13. Exit checkpoint: validator lifecycle complete with rewards and penalties; VDF produces deterministic entropy; liveness tiers handle degraded mode; governance sandbox executes proposals; quota matrix enforced across all 14 entries; task split lifecycle works end-to-end.

### Week 11–12: CLI + Soak
1. **`hyperfluid` CLI crate:** Implement all 7 top-level subcommands (tx, query, task, review, governance, agent, idea) with clap argument parsing. Machine-parseable JSON output.
2. **CLI-PDP integration:** All mutating commands route through the Policy Decision Point for schema, signature, replay, quota, and fee validation.
3. **Static CLI in system prompt:** Ensure the full CLI spec is embedded verbatim in the agent system prompt per agent-runtime-spec.md §3.2.
4. **Failure guard (FR-0065):** Pre-execution dedup — reject exact duplicate tool calls within 1-hour window. Rate limit — max 3 failures per agent per hour triggers pause. Deterministic hash-based cache.
5. **Token budget system (FR-0073-75):** ptok normalization, context envelope allocation with priority-score pruning, per-sender ingress budgets by trust stage.
6. **Tool output sanitization (FR-0115):** Size limit 100KB, content-type validation, Unicode NFC normalization. Sanitized output appended to messages context.
7. **Cross-component integration:** End-to-end test of CLI → PDP → state machine → gossip pipeline with `hyperfluid task submit`.
8. **Full soak:** Multi-node deployment with agents claiming tasks, completing review pipeline, governance proposals passing, fast-path merges committing. 1000-block sustained run.
9. **Bug fixes and polish** from integration test findings.
10. Exit checkpoint: CLI crate passes all conformance hooks; full end-to-end workflow operates without node restart; soak test passes 1000 blocks with zero state divergence.

## Risk Areas
- **PDP determinism across platforms:** Rust floating-point and HashMap ordering are non-deterministic. The PDP spec mandates deterministic rule chain. Mitigation: use `BTreeMap` for ordered maps, `Vec` sort before iteration, no `f32`/`f64` in PDP, use `BTreeSet` for sets. Cross-platform CI (Linux, macOS, aarch64) validates determinism.
- **LLM provider availability and cost:** Rate limits, API outages, or cost spikes could stall agent testing. Mitigation: support Ollama local models for low-risk testing; cache LLM responses for deterministic replay in tests.
- **Sandbox escape:** Agent runtime process isolation must prevent filesystem and network escape. Mitigation: WASM sandbox with WASI preview2 (least privilege) or Firecracker microVM. No host network access except through a controlled proxy. Full sandbox escape threat model deferred to Stage 03.
- **Reviewer collusion:** An attacker creating many agents could attempt to stack reviewer pools. Mitigation: operator-cluster independence constraint (stake-graph analysis prevents same-cluster reviewers). Test with 20% adversarial agent set in Stage 03.
- **Trust ladder promotion gamed:** Agents could complete trivial tasks to inflate metrics. Mitigation: review scores weight by task complexity and reviewer consensus (tasks with higher disagreement contribute more to trust progression). Task quality decay requires sustained quality, not just volume.
- **Governance vote apathy:** If <33% of stake votes, proposals stall indefinitely. Mitigation: governance-spec no-vote timeout (non-vote = no quorum contribution, proposal expires). Emergency proposals have shorter windows (1 hour, 67% threshold).
- **Fast-Path challenge DOS:** Attackers could challenge every merge to stall topic progress. Mitigation: challenge requires deposit (slashed if challenge fails). Persistent frivolous challengers flagged by trust regression and temporarily barred.
- **Telegram bot token leakage:** Bot token stored in local config.toml (Zone 3). Mitigation: no logging of token contents, file permissions restricted to agent process user, agent never includes token in on-chain data or artifact outputs.
- **Sybil detection false positives:** Operator-cluster diversity via stake-graph analysis is deterministic and does not produce false positives. Edge cases from incomplete funding-edge tracking mitigated by pruning thresholds.
- **Bounty escrow race conditions:** Concurrent task claims on the same bounty-funded task. Mitigation: escrow is locked at task creation; lease claim is atomic. Only the primary lease holder can trigger payout. Canceled tasks refund via atomic escrow release.
- **Airdrop puzzle difficulty oscillation:** Dynamic difficulty may overshoot under burst registrations, blocking legitimate new agents. Mitigation: hysteresis in difficulty adjustment (only increases after sustained high registration rate; decreases slowly). Difficulty floor ensures puzzle is always solvable on consumer hardware within ~30 seconds.
- **VDF committee randomness:** VDF integration for committee randomness is deferred to Stage 03 (Validation) for calibration.

## Spec References

| Spec | Sections | Key Requirements |
|------|----------|-----------------|
| governance-spec.md | 1 (Proposals), 2 (Sandbox Review) | FR-0021–0030 |
| fastpath-spec.md | 1 (Topics) | FR-0031–0040 |
| agent-runtime-spec.md | 1 (Loop), 2 (Tools), 3 (Handoff), 4 (Isolation) | FR-0061–0075 |
| policy-engine-spec.md | 1 (PDP), 2 (Cross-Layer Quota Matrix) | FR-0106–0120 |
| review-engine-spec.md | 1 (Quality Pipeline) | FR-0161–0175 |
| collaboration-spec.md | 1 (Task Board), 2 (Inbox), 3 (Trust Ladder) | FR-0076–0105 |
| telemetry-spec.md | 1 (Envelopes), 2 (Aggregation) | FR-0060, FR-0139–0141, NFR-0020–0021 |
| incident-response-spec.md | 1 (Congestion) | FR-0144–0145 |

## Upstream Dependencies for Next Stage
- Full system must be functional end-to-end: agent → action_plan → PDP → chain → review → settlement → trust promotion.
- All 15 specs must have implementations passing their conformance test hooks. Any gaps are blockers for Stage 03.
- Integration test suite must be runnable as `cargo test --integration` (Stage 03 extends this).
- [TUNE] parameter calibration log must be populated with initial values from integration testing.
