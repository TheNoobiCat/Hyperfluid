# Stage 02: Agent Runtime

## Inputs
- From Stage 01: stable chain (C1, C2, C3, C5, C7, C8) with multi-node consensus, functional P2P, artifact storage.
- From Layer 4 specs: governance-spec.md, fastpath-spec.md, agent-runtime-spec.md, policy-engine-spec.md, review-engine-spec.md, collaboration-spec.md, telemetry-spec.md, incident-response-spec.md.
- External: LLM provider SDKs (Anthropic, OpenAI, Ollama), ML-DSA-65 for agent key operations, `rusqlite` for agent-local state, sandbox runtime (WASM or Firecracker microVM).

## Outputs
- C4 Governance Engine: git:head on-chain state, proposal lifecycle (submit → vote → execute), sandbox review period, anti-flood deposit.
- C6 Fast-Path Topic Protocol: topic-scoped consensus merges, quorum certificates, challenge windows, rollback, promotion bridge.
- C9 Policy Decision Point: 10-step deterministic rule chain (schema → signature → bundle → replay → role → ACL → quota → taint → risk step-up → binding), append-only audit log, circuit-breaker escalation.
- C10 Agent Runtime: infinite agent loop, system prompt loader, core tool set, handoff mechanism, resource limits, process isolation, crash recovery via handoff replay.
- C11 Collaboration & Inbox: task board, soft leases, team formation, inbox routing, trust ladder (4 stages, promotion thresholds), reputation system.
- C12 Economics & Review: three-phase quality pipeline (initial → attestation → settlement), reviewer independence via stake-graph diversity, anti-collusion, clawback, settlement.
- Telemetry: signed envelopes, aggregation pipeline, reconciliation, outlier detection.
- Incident Response: incident FSM, emergency mode, recovery ramp-up, circuit-breaker hierarchy.
- Full agent lifecycle: join (0 AGX) → claim task → execute with tools → submit action_plan → PDP evaluates → reviewers attest → settlement → trust promotion.

## Exit Criteria
- [ ] Governance: proposal submitted, passes quorum vote, executes state change. No-vote timeout functions correctly (not counted toward quorum). Anti-flood deposit returned or slashed.
- [ ] Fast-Path: topic merge committed, quorum certificate verified, challenge window expires without challenge, promotion bridge activates.
- [ ] PDP: 10-step rule chain produces identical decisions on all nodes for identical inputs. Audit log is append-only and content-addressed. Circuit-breaker triggers on sustained quota breach.
- [ ] Agent Runtime: agent joins at `untrusted_joiner`, runs infinite loop, claims tasks, submits action plans, progresses through trust ladder to `trusted_contributor`.
- [ ] Collaboration: task board visible across nodes, soft leases prevent double-claim, team formation works for multi-agent tasks, inbox routes messages correctly.
- [ ] Review: 3-phase review pipeline (at least 3 reviewers per action plan at medium risk), settlement occurs within 1 epoch, clawback fires for detected collusion.
- [ ] Telemetry: signed envelopes aggregate across nodes, reconciliation detects drift, outlier detection flags anomalous nodes.
- [ ] Incident Response: emergency mode activates via consensus vote, circuit-breaker enforces reduced quota, recovery ramp-up restores normal operation after sustained clean windows.
- [ ] End-to-end agent workflow: 3 agents complete a collaborative coding task, with review, settlement, and reputation update.
- [ ] All 8 specs pass their conformance test hooks (Section X.7).
- [ ] Risks documented and acceptable.
- [ ] Next stage inputs prepared.

## Duration Estimate
6–8 weeks. Extend to 8 weeks if PDP determinism across heterogeneous environments (Linux/macOS/aarch64) requires debugging. Extend beyond 8 weeks if LLM provider rate limits or sandbox escape hardening demands extra work.

## Dependencies
- Stage 01 complete (chain, P2P, artifact storage functional).
- LLM provider API access (Anthropic/OpenAI keys or local Ollama deployment).
- Sandbox runtime decision: WASM (via Wasmtime) or Firecracker microVM. Decision must be made in Week 1.

## Week-by-Week Breakdown

### Week 1–2: Governance + Fast-Path + PDP (C4, C6, C9)
1. Governance engine: git:head state representation, proposal submission (target hash, proposed branch, sandbox period), vote window (5,040 blocks = ~7 days at 2s blocks), no-vote timeout, anti-flood deposit.
2. Fast-Path topic protocol: topic scope definition, merge proposal with quorum certificate (67/100), challenge window (1,440 blocks = ~48 min), rollback on successful challenge, promotion bridge to governance for permanent codification.
3. PDP rule chain: implement 10 steps in order. Ensure determinism — no `HashMap` iteration, no floating-point in root authorization path, no time-based decisions. Structured deny reason codes.
4. Circuit-breaker escalation: Level 1 (soft cap), Level 2 (hard cap), Level 3 (emergency mode). Persistence check (3 consecutive windows) before escalation. Hysteresis (0.7x) on exit.
5. Audit log: append-only, content-addressed (each entry hashes to previous entry). Queryable by plan_id and agent_id.
6. Exit checkpoint: governance proposal lifecycle works end-to-end; PDP rule chain rejects invalid action plans with correct reason codes; circuit-breaker escalates and recovers.

### Week 3–4: Agent Runtime + Sandbox (C10)
1. Infinite agent loop: `load_system_prompt() → call_llm() → parse_action() → execute_tool() → check_token_count() → handoff_if_needed() → repeat`.
2. Core tool set: `claim_task`, `publish_output`, `request_review`, `assign_subtask`, `read_state`, `submit_action_plan`.
3. System prompt loader: loads base prompt from chain (governance-managed), merges with agent-local overrides, formats with state context.
4. Handoff mechanism: compress context at 70% token threshold or 50-message trigger → persist handoff record → next iteration loads resume context.
5. Resource limits: 4 GB RAM ceiling, 120-second tool timeout, max 128 concurrent tool calls, disk quota for local state.
6. Process isolation: agent runs in separate OS process or WASM sandbox. Network access restricted to chain RPC only. Filesystem access limited to designated working directory. No outbound internet (except LLM provider calls via proxy).
7. Crash recovery: on restart, load last handoff record, verify state consistency, resume loop.
8. Local SQLite for agent state: todos, knowledge base, failure log, inbox messages.
9. Exit checkpoint: single agent runs stable loop for 1 hour; survives tool timeout, token limit handoff, and process restart.

### Week 5–6: Collaboration + Review + Economics (C11, C12)
1. Task board: global task queue, soft leases (claim window = 600 blocks, ~20 min), lease renewal, lease expiry → task returns to queue.
2. Team formation: coordinator_eligible agent creates team topic, invites agents, assigns subtasks. Team consensus via fast-path merges for internal decisions.
3. Inbox system: message routing by agent_id and topic_id, priority channels (review requests), spam filter (quota-gated per sender trust stage).
4. Trust ladder: promotion thresholds per collaboration-spec.md 3.3. `untrusted_joiner` → `sandboxed_contributor` at N completed tasks. `sandboxed_contributor` → `trusted_contributor` at M reviews + quality score. `trusted_contributor` → `coordinator_eligible` at K successful team leads.
5. Reputation score: composite of quality ratings, task completion rate, review accuracy, collaboration endorsements. Decays over inactivity windows.
6. Review engine: initial review (3 reviewers, individual scores), attestation phase (2-step-up attestations for medium+ risk), settlement (aggregate score, reward distribution, reviewer rewards).
7. Reviewer independence: stake-graph analysis ensures no 2 reviewers share >25% stake correlation. Reviewer rotation per epoch.
8. Anti-collusion: duplicate IP detection, correlated score patterns, whitewash guard (new identity within 7 epochs = heightened scrutiny).
9. Clawback: settlement reversed if collusion detected within 21-epoch window. Funds clawed back to fee pool.
10. Exit checkpoint: 3-agent collaborative task completes with review pipeline; trust ladder promotes agent; reviewer rewards distributed.

### Week 7–8: Telemetry + Incident Response + Integration
1. Telemetry: signed envelopes (agent key signs metrics), aggregation daemon, reconciliation (compare reported vs on-chain state), outlier detection (z-score > 3 sigma flags for review).
2. Incident Response: FSM states (normal, elevated, emergency), entry/exit criteria, emergency vote (must reach 67% in 1-hour window), auto-triggers for sustained circuit-breaker violations. Recovery ramp-up (reduced quotas for 3 epochs post-recovery).
3. End-to-end integration test: network of 5 nodes, each running 2 agents — join, progress through trust ladder, complete collaborative task, face and survive attempted collusion attack, process incident.
4. Governance stress test: 10 concurrent proposals, multi-epoch vote windows, fast-path challenge + rollback.
5. Bug fixes and polish from integration test findings.
6. Exit checkpoint: all exit criteria met; full end-to-end agent workflow demonstrated; incident response lifecycle validated.

## Risk Areas
- **PDP determinism across platforms:** Rust floating-point and HashMap ordering are non-deterministic. The PDP spec mandates deterministic rule chain. Mitigation: use `BTreeMap` for ordered maps, `Vec` sort before iteration, no `f32`/`f64` in PDP, use `BTreeSet` for sets. Cross-platform CI (Linux, macOS, aarch64) validates determinism.
- **LLM provider availability and cost:** Rate limits, API outages, or cost spikes could stall agent testing. Mitigation: support Ollama local models for low-risk testing; cache LLM responses for deterministic replay in tests.
- **Sandbox escape:** Agent runtime process isolation must prevent filesystem and network escape. Mitigation: WASM sandbox with WASI preview2 (least privilege) or Firecracker microVM. No host network access except through a controlled proxy. Full sandbox escape threat model deferred to Stage 03.
- **Reviewer collusion resistant to Sybil:** An attacker creating many agents could attempt to stack reviewer pools. Mitigation: stake-graph correlation analysis, minimum stake for reviewer_eligible (>1,000 AGX bonded), whitewash guard. Test with 20% adversarial agent set in Stage 03.
- **Trust ladder promotion gamed:** Agents could complete trivial tasks to inflate metrics. Mitigation: review scores weight by risk class (high-risk tasks contribute more to trust progression than low-risk). Task quality decay requires sustained quality, not just volume.
- **Governance vote apathy:** If <33% of stake votes, proposals stall indefinitely. Mitigation: governance-spec no-vote timeout (non-vote = no quorum contribution, proposal expires). Emergency proposals have shorter windows (1 hour, 67% threshold).
- **Fast-Path challenge DOS:** Attackers could challenge every merge to stall topic progress. Mitigation: challenge requires deposit (slashed if challenge fails). Persistent frivolous challengers flagged by reputation and temporarily barred.

## Spec References

| Spec | Sections | Key Requirements |
|------|----------|-----------------|
| governance-spec.md | 1 (Proposals), 2 (Sandbox Review) | FR-0021–0030 |
| fastpath-spec.md | 1 (Topics) | FR-0031–0040 |
| agent-runtime-spec.md | 1 (Loop), 2 (Tools), 3 (Handoff), 4 (Isolation) | FR-0061–0075 |
| policy-engine-spec.md | 1 (PDP), 2 (Circuit Breaker) | FR-0106–0120 |
| review-engine-spec.md | 1 (Quality Pipeline) | FR-0161–0175 |
| collaboration-spec.md | 1 (Task Board), 2 (Inbox), 3 (Trust Ladder) | FR-0076–0105 |
| telemetry-spec.md | 1 (Envelopes), 2 (Aggregation) | FR-0060, FR-0139–0141, NFR-0020–0021 |
| incident-response-spec.md | 1 (FSM), 2 (Recovery) | FR-0142–0145 |

## Upstream Dependencies for Next Stage
- Full system must be functional end-to-end: agent → action_plan → PDP → chain → review → settlement → trust promotion.
- All 14 specs must have implementations passing their conformance test hooks. Any gaps are blockers for Stage 03.
- Integration test suite must be runnable as `cargo test --integration` (Stage 03 extends this).
- [TUNE] parameter calibration log must be populated with initial values from integration testing.
