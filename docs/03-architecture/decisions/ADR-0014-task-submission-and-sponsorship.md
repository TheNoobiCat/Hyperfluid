## ADR-0014: User Task Submission and Agent Sponsorship

**Status:** accepted

**Context:** The original design had the airdrop agent bootstrap the marketplace with seed tasks from the genesis seed pool, but provided no mechanism for external users or agents to submit their own tasks after seed pool exhaustion. Without a decentralised task submission path, the network becomes a closed system dependent on the genesis allocation — contrary to Hyperfluid's premise of an open agent marketplace.

The research document `user-task-submission-and-sponsorship.md` defines a complete task submission pipeline: `task_create` action plan type, PDP validation, bounty escrow, gossip/DHT discovery, and agent-as-proxy sponsorship. This ADR codifies the architecture decisions required to implement it.

**Decision:**
1. **Extend the action plan taxonomy** with `action_type = task_create`, carrying `bounty_agx`, `topic_id`, `metadata_hash`, `required_skills_hash`, `seed_ref`, `sponsor_id`, and `requester_pubkey`.
2. **Add `TaskCreateTx` to the consensus state machine** — debits bounty from creator, credits escrow, records TaskRecord, emits `TaskCreated` event.
3. **Enforce task creation quotas** via the PDP per trust stage: 0 for untrusted, 30 for trusted.
4. **Publish tasks via gossip/DHT** — `TaskCreated` events propagate via P2P overlay (fanout 8, TTL 16, Bloom-filter dedup). DHT keyed by `SHA3-256(task_id)`.
5. **Agent-as-proxy sponsorship** — the sponsoring agent uses its own identity and balance. The protocol sees only the agent, not the user. No new delegation primitives.
6. **Add `hyperfluid task submit` CLI** — constructs metadata artifact in gix, builds action plan, signs, submits.
7. **Extend Telegram bot for sponsored submission** — telegram_chat_agent receives user requests, refines them, maps to seed_ref, requests explicit confirmation, submits as sponsor.

**Consequences:**
- Positive: Opens the marketplace to external demand after seed pool exhaustion. Agents earn margins on sponsorship. Fully composable with existing primitives (PDP, EIP-1559, gossip/DHT, gix).
- Positive: Zero new protocol primitives — no delegation certificates, no new consensus message types, no new cryptographic schemes.
- Positive: Economic self-certification via bounty escrow + trust-stage quotas prevents spam without human review.
- Negative: Increases on-chain action type count by 1 (`task_create`). New PDP validation rules (seed_ref, quota, balance). New CLI subcommand.
- Negative: Telegram sponsored submission requires agent decision-making on task scope/bounty/seed mapping — this is off-protocol but misaligned incentives (agent takes risk, user pays off-protocol) could lead to abuse.

**Alternatives considered:**
- Separate task submission fee: rejected — bounty escrow already provides economic deterrence; dual fees over-engineer.
- On-chain event log for task discovery: rejected — gossip/DHT scales better with agent count and keeps discovery off consensus path.
- Protocol-level delegation for sponsorship: rejected — adds complexity for a problem solved by agent-as-proxy.
- Full task metadata on-chain: rejected — bloats state; gix content-addressing handles this with on-chain hash only.

**Related:** FR-0194–FR-0200, `user-task-submission-and-sponsorship.md`, `collaboration-spec.md`, `policy-engine-spec.md`, `consensus-spec.md`, `agent-runtime-spec.md`, `p2p-wire-spec.md`
