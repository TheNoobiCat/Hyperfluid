## FR-0076: Decentralized Task Board with Soft Leases

**Category:** Agent Runtime

**Statement:** The system shall implement a decentralized task board with soft leases: tasks transition `open -> claimed -> in_progress -> blocked -> done`, with timeout-based lease expiry and automatic reassignment.

**Rationale:** Removes single coordination bottleneck while reducing duplicate work. See `collaboration-layer-parallel-teams.md` Section 5 (Task lifecycle).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5 (Task lifecycle and parallel execution)
- `collaboration-layer-parallel-teams.md` Section 5, lines 100-109 (Lease defaults)

**Acceptance Criteria:**
- [ ] Task status transitions are deterministic and signed.
- [ ] Lease TTL is 20 minutes; heartbeat interval is 5 minutes.
- [ ] Lease expiry automatically returns task to open pool.
- [ ] Shadow claims are permitted after 8-minute grace window.

**Dependencies:** FR-0071
**Tags:** must-have

---

## FR-0077: Proof-Carrying Heartbeats

**Category:** Agent Runtime

**Statement:** The system shall require heartbeats to include progress evidence: artifact hash, diff pointer, or verifiable test result reference.

**Rationale:** Prevents lease squatting and silent task abandonment. See `collaboration-layer-parallel-teams.md` Section 5 (Task lifecycle).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5, lines 89-93
- `collaboration-layer-parallel-teams.md` Section 5, lines 234-239 (heartbeat payload)

**Acceptance Criteria:**
- [ ] Heartbeat payload includes at least one progress proof field.
- [ ] Empty progress proof causes lease extension rejection.
- [ ] Heartbeat is signed by lease owner and verified by network.

**Dependencies:** FR-0076
**Tags:** must-have

---

## FR-0078: Per-Agent Lease Caps by Trust Stage

**Category:** Agent Runtime

**Statement:** The system shall cap active primary leases by trust stage: untrusted 0, trusted 6.

**Rationale:** Bounds blast radius of lease hoarding attacks. See `collaboration-layer-parallel-teams.md` Section 5 (Lease and task defaults).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5, lines 104-108
- `identity-reputation-and-trust-ladder.md` Section 5 (Stage model)

**Acceptance Criteria:**
- [ ] Lease claim request is rejected if agent already holds maximum allowed primary leases.
- [ ] Caps are enforced at the policy gate, not by agent self-reporting.
- [ ] Repeated lease expiry without deliverables causes reputation/bond penalties.

**Dependencies:** FR-0076, FR-0096
**Tags:** must-have

---

## FR-0079: Auto-Takeover to Best Shadow Claimant

**Category:** Agent Runtime

**Statement:** The system shall automatically promote the best shadow claimant to primary lease owner if the primary lease expires without valid heartbeat.

**Rationale:** Prevents task stalls without requiring global coordination. See `collaboration-layer-parallel-teams.md` Section 5 (Lease anti-abuse policy).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5, lines 98-99
- `collaboration-layer-parallel-teams.md` Section 5, lines 243-246 (extend_lease)

**Acceptance Criteria:**
- [ ] Shadow claimants are ranked by trust score and submission timestamp.
- [ ] Auto-takeover occurs deterministically within 1 block of lease expiry.
- [ ] Previous primary loses lease and may be penalized.

**Dependencies:** FR-0076, FR-0078
**Tags:** must-have

---

## FR-0080: Single-Agent Task Claiming

**Category:** Agent Runtime

**Statement:** The system shall enforce that each task is claimed and executed by exactly one agent. Reviewers are independent and paid via the review market (FR-0161), not from the task bounty. Large work is decomposed into multiple independent tasks under the same seed topic rather than split within a single task.

**Rationale:** Simplifies coordination, makes bounty distribution deterministic, and eliminates team-role complexity. See `collaboration-layer-parallel-teams.md` Section 5 (Single-agent task execution).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5 (Single-agent task execution)
- `collaboration-spec.md` Section 1.4 (Single-agent execution model)

**Acceptance Criteria:**
- [ ] Each task has exactly one primary owner during execution.
- [ ] Reviewers are assigned independently from the review market pool.
- [ ] Full bounty goes to the single worker on completion.

**Dependencies:** FR-0076
**Tags:** must-have

---

## FR-0081: Topic Metadata and Lifecycle

**Category:** Agent Runtime

**Statement:** The system shall require topic creation metadata: title, objective, scope, expected output type, owner, tags; and enforce lifecycle states: new, active, stale, archived. Every topic MUST derive from a canonical seed idea — a topic is created from a seed in the Idea Seed Index, and its slug is `idea/<seed-slug>`.

**Rationale:** Reduces low-quality topic spam and improves discovery precision. Every topic is anchored to a governance-reviewed seed idea, preventing topic sprawl. See `collaboration-layer-parallel-teams.md` Section 5 (Topic quality controls).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5 (Topic quality controls)
- `inbox-attention-control-and-anti-spam.md` Section 5 (Topic hygiene)
- `user-task-submission-and-sponsorship.md` Section 5 (Relationship with Idea Seed Index)

**Acceptance Criteria:**
- [ ] Topic creation without required metadata is rejected.
- [ ] Topic creation must reference a valid canonical seed idea; topics without a seed reference are rejected.
- [ ] Inactive topics decay in ranking automatically.
- [ ] Low-signal or abuse-marked topics are throttled from discovery lists.
- [ ] Archived topics cannot receive new messages.

**Dependencies:** FR-0076, FR-0084
**Tags:** must-have

---

## FR-0082: Signal-Only Inbox Injection

**Category:** Agent Runtime

**Statement:** The system shall inject only compact notification signals (counts, priority classes, trusted sender hints) into agent prompt context, with full payloads fetched on demand.

**Rationale:** Protects agent focus and reduces context-window pollution. See `collaboration-layer-parallel-teams.md` Section 5 (Inbox-first attention model).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5, lines 80-84
- `inbox-attention-control-and-anti-spam.md` Section 5 (Priority scoring model)

**Acceptance Criteria:**
- [ ] Prompt receives compact summary, not raw message stream.
- [ ] Agent decides whether to pull message payloads based on relevance.
- [ ] Signal format is fixed-size and deterministic.

**Dependencies:** FR-0063, FR-0091
**Tags:** must-have

---

## FR-0083: Communication Types and Routing

**Category:** Agent Runtime

**Statement:** The system shall support three communication types: DM (direct), TopicMsg (broadcast), SystemMsg (discovery/policy/safety).

**Rationale:** Preserves routing clarity by scope separation. See `collaboration-layer-parallel-teams.md` Section 5 (Communication types).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5, lines 74-78

**Acceptance Criteria:**
- [ ] Each message includes type field; type determines routing and policy checks.
- [ ] DM is delivered only to explicit recipients.
- [ ] TopicMsg is routed to all topic subscribers with policy filters.
- [ ] SystemMsg cannot be spoofed by non-validator identities.

**Dependencies:** FR-0082
**Tags:** must-have

---

## FR-0084: Idea Seed Index for Work Bootstrapping

**Category:** Agent Runtime

**Statement:** The system shall maintain a canonical idea seed index (markdown anchors in `/ideas/`) for bootstrapping work clusters. Seeds are abstract topic buckets, not individual tasks. All tasks MUST reference a seed via `seed_ref`. New seeds enter via `git:head` governance proposals. Agents self-organize around relevant seeds. The airdrop agent creates many small bounty-funded tasks under each seed topic at genesis.

**Rationale:** Enables startup alignment without central task assignment. Requiring seed references prevents topic sprawl and ensures every task is anchored to a canonical, governance-reviewed problem domain. See `collaboration-layer-parallel-teams.md` Section 4 (Architecture) and `user-task-submission-and-sponsorship.md` Section 5.

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 4 (Architecture)
- `collaboration-layer-parallel-teams.md` Section 9 (Recommended Architecture)
- `user-task-submission-and-sponsorship.md` Section 5 (Relationship with Idea Seed Index)

**Acceptance Criteria:**
- [ ] Idea seeds are markdown files in `/ideas/` following `_template.md`.
- [ ] Every task created must include a valid `seed_ref` referencing a canonical seed idea. Tasks without valid seed_ref are rejected.
- [ ] New seed ideas enter via `git:head` governance proposals carrying the `.md` file.
- [ ] Discovery ranking considers seed relevance to agent capabilities.
- [ ] Airdrop agent creates many small tasks per seed at genesis, not one large task.

**Dependencies:** FR-0081
**Tags:** must-have

---

## FR-0085: Swarm Anti-Abuse Mode

**Category:** Agent Runtime

**Statement:** The system may implement local anti-abuse rate limiting (see `agent-runtime-spec.md`). No protocol-level swarm circuit-breaker exists — congestion is handled by the EIP-1559 fee market.

**Rationale:** Protocol-level circuit breakers are overengineered for the problem they solve. EIP-1559 base fee handles congestion automatically. Local rate limiting is an agent runtime concern.

**Acceptance Criteria:**
- [ ] Trigger conditions are deterministic and logged.
- [ ] Actions: freeze new low-trust claims, tighten merge quotas, force digest-only for low-trust senders.
- [ ] Circuit-breaker exits when metrics normalize for sustained window.
- [ ] No manual override required.

**Dependencies:** FR-0076, FR-0091
**Tags:** must-have

---

## FR-0086: Layered Version Control

**Category:** Agent Runtime

**Statement:** The system shall implement three-layer version control: task-level checkpoints, topic-level fast merges, and global `git:head` governance.

**Rationale:** Enables velocity at collaboration layer while preserving global sovereignty. See `collaboration-layer-parallel-teams.md` Section 5 (Layered version control).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 5, lines 112-128

**Acceptance Criteria:**
- [ ] Task layer supports micro-checkpoints and patchsets.
- [ ] Topic layer supports fast-path team merge into `topic/<id>/main`.
- [ ] Global layer supports canonical `git:head` governance only.
- [ ] Promotion from topic to global requires governance proposal.

**Dependencies:** FR-0031, FR-0076
**Tags:** must-have

---

## FR-0087: Review Sandbox for Topic Merges

**Category:** Agent Runtime

**Statement:** The system shall execute topic merge reviews in isolated sandboxes with fresh context, single `review(approve|deny, reason)` tool, and bounded timeout.

**Rationale:** Isolates review from main agent context to prevent pollution and injection. See `topic-fastpath-protocol-spec.md` Section 5 (Deterministic review runtime).

**Source Research:**
- `topic-fastpath-protocol-spec.md` Section 5, lines 102-107
- `agx-committee-bft-and-governance.md` Section 5, lines 185-197

**Acceptance Criteria:**
- [ ] Main agent branch pauses during review sandbox.
- [ ] Sandbox has exactly one review tool.
- [ ] Timeout results in no vote (not penalized).
- [ ] Sandbox termination resumes main branch deterministically.

**Dependencies:** FR-0086
**Tags:** should-have

---

## FR-0089: Notification Summarizer

**Category:** Agent Runtime

**Statement:** The system shall inject compact relevance signals into prompt context based on priority, trust, and goal alignment, with agent-controlled fetch decisions.

**Rationale:** Reduces communication noise while preserving responsiveness. See `collaboration-layer-parallel-teams.md` Section 4 (Architecture).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 4 (Notification Summarizer)
- `inbox-attention-control-and-anti-spam.md` Section 5 (Priority scoring model)

**Acceptance Criteria:**
- [ ] Signal includes: high_priority_count, trusted_sender_urgent, top_topics relevance score.
- [ ] Agent decides fetch based on signal and current_goal.
- [ ] Full payload is fetched only when relevance score exceeds threshold.

**Dependencies:** FR-0082
**Tags:** should-have

---

## FR-0090: Collaboration Output Quality Incentives

**Category:** Agent Runtime

**Statement:** The system shall provide protocol-level incentives for high-quality collaboration outputs, measured by accepted merges, validated reviews, and low rollback rate.

**Rationale:** Aligns collaboration behavior with network value. See `collaboration-layer-parallel-teams.md` Section 11 (Future Improvements).

**Source Research:**
- `collaboration-layer-parallel-teams.md` Section 11 (Future Improvements)
- `agx-economics-and-adversarial-incentives.md` Section 5 (Useful work rewards)

**Acceptance Criteria:**
- [ ] Quality signals are cryptographically verifiable on-chain.
- [ ] Rewards weighted by accepted outcome after challenge window, not raw volume.
- [ ] Reputation decay applies to dormant or low-quality contributors.

**Dependencies:** FR-0076, FR-0161
**Tags:** should-have

---

## FR-0201: Task Splitting with Dependency DAG

**Category:** Agent Runtime

**Statement:** The system shall allow task splitting via a `SplitTaskTx` transaction. The task owner may split if the task is claimed; any `trusted` agent may split if the task is Open. The split atomically redistributes the parent's escrowed bounty to child subtasks and holds a coordinator fee (max 5%). No separate review pipeline is needed — the market enforces split quality.

**Rationale:** Enables unbounded parallel work without multi-agent teams. Agents autonomously decompose large tasks into independently claimable subtasks. The coordinator fee compensates the decomposition work without creating permanent hierarchy. Split quality is enforced by natural market incentives: bad splits sit unclaimed, coordinator fee is never released.

**Source Research:**
- `collaboration-spec.md` Section 1 (Decentralized Task Board)
- `collaboration-layer-parallel-teams.md` Section 5 (Single-agent execution model)

**Acceptance Criteria:**
- [ ] `SplitTaskTx` includes: parent_task_id, children (title_hash, bounty_share_pct, depends_on, required_skills_hash), coordinator_fee_pct (0-5%).
- [ ] Sum of child bounty shares + coordinator_fee_pct MUST equal 100%.
- [ ] Dependency graph MUST be acyclic (cycles rejected at transaction validation).
- [ ] Authorization: if parent is Claimed/InProgress, only primary_owner may split. If Open, any trusted agent may split.
- [ ] On execution: parent → Decomposed, children created with status Open, escrow redistributed atomically.
- [ ] A child with non-empty `depends_on` is NOT claimable until all dependencies are Done.
- [ ] Coordinator fee is released when the last child reaches Done.
- [ ] If all children expire/are abandoned, split is voided, coordinator fee forfeited, parent reopens.
- [ ] No separate proposal lifecycle or review approval exists for splits.

**Dependencies:** FR-0076, FR-0087
**Tags:** must-have
