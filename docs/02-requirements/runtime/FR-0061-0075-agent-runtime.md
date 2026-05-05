## FR-0061: Infinite Agent Loop with State Persistence

**Category:** Agent Runtime

**Statement:** The system shall implement an infinite agent loop: load system prompt, call LLM, execute tools, check token count, handoff if needed, repeat, with all state persisted to local SQLite.

**Rationale:** Enables autonomous, crash-recoverable agents without human intervention. See `infinite-agent.md` Section 4.1.

**Source Research:**
- `infinite-agent.md` Section 4.1 (Runtime Loop)
- `infinite-agent.md` Section 5.1 (Todo State Machine)

**Acceptance Criteria:**
- [ ] Agent loop runs continuously without human input after startup.
- [ ] State (todos, knowledge, handoffs, failures) is persisted to SQLite with WAL mode.
- [ ] Crash recovery loads last handoff and resumes without data loss.
- [ ] Loop frequency is capped at max 1 iteration per 5 seconds.

**Dependencies:** none
**Tags:** must-have

---

## FR-0062: Core Agent Tools

**Category:** Agent Runtime

**Statement:** The system shall provide nine core tools for agents: `bash`, `todo_write`, `todo_update`, `remember`, `forget`, `read`, `edit`, `write`, `apply_patch`.

**Rationale:** Minimal tool surface reduces prompt injection attack surface and cognitive load. The original five tools handle local execution and state; the four file-access tools (read/edit/write/apply_patch) enable agents to inspect and modify project files directly. See `agent-tools-spec.md` Section 2 (Executive Summary) and ADR-0013.

**Source Research:**
- `agent-tools-spec.md` Section 5 (Tool schema definitions)
- `agent-tools-spec.md` Section 6, Tradeoff 1
- `docs/03-architecture/decisions/ADR-0013-expanded-agent-tools-and-seed-index.md`

**Acceptance Criteria:**
- [ ] Tool schemas are fixed JSON with exact field validation.
- [ ] `bash` schema: command, working_dir (optional), timeout (default 120s).
- [ ] `todo_write` schema: array of {id, item, status, context}.
- [ ] `todo_update` schema: array of {id, status, context}.
- [ ] `remember` schema: {kind, content} where kind is finding|pattern|constraint|decision.
- [ ] `forget` schema: {id}.
- [ ] `read` schema: file_path, offset (optional), limit (optional).
- [ ] `edit` schema: file_path, old_string, new_string.
- [ ] `write` schema: file_path, content.
- [ ] `apply_patch` schema: array of {file_path, old_string, new_string}.

**Dependencies:** FR-0061
**Tags:** must-have

---

## FR-0063: System Prompt Assembly

**Category:** Agent Runtime

**Statement:** The system shall assemble system prompt from identity block, project knowledge (newest N rows), current todos (all non-done), last handoff, and recent messages.

**Rationale:** Deterministic prompt construction ensures reproducible agent behavior. See `infinite-agent.md` Section 4.4.

**Source Research:**
- `infinite-agent.md` Section 4.4 (System Prompt Template)
- `token-efficiency-under-high-interaction.md` Section 5 (Deterministic context budget envelope)

**Acceptance Criteria:**
- [ ] System prompt always includes identity block and active todos.
- [ ] If todo list is empty, system prompt instructs agent to discover/create new tasks.
- [ ] Project knowledge is limited to ~500 tokens (or bounded row count).
- [ ] Recent messages fill remaining token budget after fixed blocks.

**Dependencies:** FR-0062
**Tags:** must-have

---

## FR-0064: Handoff at 70% Token Threshold

**Category:** Agent Runtime

**Statement:** The system shall trigger handoff when token count reaches 70% of context limit, injecting a reflection prompt, capturing summary, persisting to SQLite, and resetting messages.

**Rationale:** Prevents context window overflow while preserving agent memory. See `infinite-agent.md` Section 5.2.

**Source Research:**
- `infinite-agent.md` Section 5.2 (Handoff Mechanism)
- `token-budget-resource-model.md` Section 5 (Handoff protocol resource impact)

**Acceptance Criteria:**
- [ ] Handoff triggers at 70% token usage (and alternative trigger at 50 messages).
- [ ] Reflection prompt asks for concrete next actions with file names/line numbers.
- [ ] Handoff summary is persisted to SQLite with timestamp and session ID.
- [ ] Messages array is reset; system prompt is rebuilt on next iteration.

**Dependencies:** FR-0063
**Tags:** must-have

---

## FR-0065: Failure Guard Pre-Execution Check

**Category:** Agent Runtime

**Statement:** The system shall block repeated tool failures before execution: exact-match deduplication within 1 hour and block after 3 failures in 1 hour for any unique call.

**Rationale:** Prevents infinite failure loops and wasted execution. See `infinite-agent.md` Section 5.3.

**Source Research:**
- `infinite-agent.md` Section 5.3 (Failure Guard)
- `prompt-injection-and-network-policy-boundary.md` Section 5 (Tool-call binding)

**Acceptance Criteria:**
- [ ] Action hash = SHA3-256(serialized tool call bytes).
- [ ] Exact duplicate within 1 hour is blocked with "duplicate" reason.
- [ ] 3+ failures within 1 hour for same hash blocks with "too_many_failures" reason.
- [ ] Only network-mutating tools are guarded; local tools fail fast.

**Dependencies:** FR-0062
**Tags:** must-have

---

## FR-0066: Context Window Resource Limits

**Category:** Agent Runtime

**Statement:** The system shall enforce resource limits: max 4GB RAM, 2 CPU cores (throttle at 80%), 10GB disk, 1024 file descriptors, 100 concurrent connections, 8192 token context, 120s tool timeout.

**Rationale:** Prevents agent from exhausting host resources. See `infinite-agent.md` Section 4.0.

**Source Research:**
- `infinite-agent.md` Section 4.0 (Resource Limits)

**Acceptance Criteria:**
- [ ] cgroup or process limits enforce memory and CPU caps.
- [ ] Disk usage is monitored; logs rotate at 90% capacity.
- [ ] Tool calls timeout after 120 seconds and return truncation notice.
- [ ] File descriptor usage stays below 1024.

**Dependencies:** FR-0061
**Tags:** should-have

---

## FR-0067: Project Knowledge Accumulation and TTL

**Category:** Agent Runtime

**Statement:** The system shall support knowledge accumulation with 30-day default TTL, auto-refresh on read, stale knowledge archival, and contradiction detection.

**Rationale:** Permanent knowledge enables learning but requires freshness management. See `infinite-agent.md` Section 5.4.

**Source Research:**
- `infinite-agent.md` Section 5.4 (Project Knowledge Accumulation)

**Acceptance Criteria:**
- [ ] New knowledge gets TTL = 30 days from creation.
- [ ] Each read extends TTL by +30 days.
- [ ] Expired knowledge moves to stale_knowledge table and is excluded from prompts.
- [ ] Contradiction detection flags same-topic opposite conclusions for review.
- [ ] Max active knowledge entries capped at 100; oldest auto-archived.

**Dependencies:** FR-0062
**Tags:** should-have

---

## FR-0068: Single `hyperfluid` CLI for Network Actions

**Category:** Agent Runtime

**Statement:** The system shall expose a single `hyperfluid` CLI for all network-mutating actions, with subcommands for tx, query, task, review, governance, stake, idea, and agent self-management.

**Rationale:** Reduces tool surface and forces all shared-state changes through typed transactions. See `agent-tools-spec.md` Section 4 (Architecture).

**Source Research:**
- `agent-tools-spec.md` Section 5 (CLI command taxonomy)
- `automatic-vs-agent-controlled.md` Section 5 (Agent decision taxonomy)

**Acceptance Criteria:**
- [ ] `hyperfluid tx` subcommand supports: transfer, stake, identity, task, review, governance, evidence.
- [ ] `hyperfluid query` supports: balance, account, nonce, validator, committee, proposal, task, review, reputation, trust-stage, block, git-head, fee-estimate.
- [ ] `hyperfluid task` supports: list, get, claim, release, submit, heartbeat, lease.
- [ ] `hyperfluid review` supports: list, submit, challenge, claim-rewards.
- [ ] `hyperfluid governance` supports: list, get, vote, fetch-bundle, verify.
- [ ] `hyperfluid agent` supports: list-skills, load-skill, status, key-info.
- [ ] `hyperfluid idea` supports: list, get.

**Dependencies:** FR-0062
**Tags:** must-have

---

## FR-0069: Static CLI Specification in System Prompt

**Category:** Agent Runtime

**Statement:** The system shall embed the complete CLI specification verbatim in the agent system prompt; runtime command discovery is disallowed.

**Rationale:** Deterministic behavior prevents runtime surprises from experimental commands. See `agent-tools-spec.md` Section 5 (System prompt assembly).

**Source Research:**
- `agent-tools-spec.md` Section 5, lines 94-98
- `agent-tools-spec.md` Section 6, Tradeoff 3

**Acceptance Criteria:**
- [ ] System prompt includes all subcommands, flags, common patterns, and error handling guidance.
- [ ] Agent does not discover commands at runtime.
- [ ] CLI changes require coordinated system prompt updates.

**Dependencies:** FR-0068
**Tags:** must-have

---

## FR-0070: On-Demand Skill Loading

**Category:** Agent Runtime

**Statement:** The system shall support on-demand skill loading via `hyperfluid agent load-skill <skill>`, with skill format: SKILL.md, scripts/, references/.

**Rationale:** Keeps base runtime tiny while allowing on-demand procedural capability (tool APIs, data formats, workflows). Skills are instruction bundles, not domain expertise — the LLM already has general reasoning and broad knowledge. See `agent-tools-spec.md` Section 5 (Skill loading mechanics).

**Source Research:**
- `agent-tools-spec.md` Section 5, lines 99-106
- `agent-tools-spec.md` Section 6, Tradeoff 4

**Acceptance Criteria:**
- [ ] Skill directory contains SKILL.md, optional scripts/, optional references/.
- [ ] Skill is loaded into agent context on demand.
- [ ] Skill is unloaded on runtime restart unless explicitly persisted.
- [ ] Malformed or missing skill returns load error; agent falls back to base tools.

**Dependencies:** FR-0068
**Tags:** should-have

---

## FR-0071: Automatic vs Agent-Controlled Boundary

**Category:** Agent Runtime

**Statement:** The system shall enforce that consensus, networking, storage, economic, and security operations run automatically in the node; agents control only task claiming, review, governance voting, and economic choices.

**Rationale:** Prevents LLM errors from affecting safety-critical protocol functions. See `automatic-vs-agent-controlled.md` Section 2 (Executive Summary).

**Source Research:**
- `automatic-vs-agent-controlled.md` Section 5 (Automatic operation taxonomy)
- `automatic-vs-agent-controlled.md` Section 5 (Agent decision taxonomy)

**Acceptance Criteria:**
- [ ] Node continues block production and validation even if agent crashes.
- [ ] Agent cannot submit raw consensus messages or peer management commands.
- [ ] All network mutations flow through typed transactions independently validated by node.
- [ ] API boundary is typed HTTP/gRPC with independent validation.

**Dependencies:** FR-0068
**Tags:** must-have

---

## FR-0072: Node API Stateless and Cacheable

**Category:** Agent Runtime

**Statement:** The system shall design the node API to be stateless and cacheable, with no assumption of privileged local access by agent runtime.

**Rationale:** Supports remote agent runtime and prevents operator mistakes. See `automatic-vs-agent-controlled.md` Section 8 (Scalability).

**Source Research:**
- `automatic-vs-agent-controlled.md` Section 8, lines 215-219

**Acceptance Criteria:**
- [ ] Query endpoints return deterministic responses for identical requests.
- [ ] API responses include cache-control headers or equivalent.
- [ ] Agent runtime cannot construct transactions that bypass policy gate.

**Dependencies:** FR-0071
**Tags:** should-have

---

## FR-0073: Token Budget Normalization (ptok)

**Category:** Agent Runtime

**Statement:** The system shall define a normalized token unit (`ptok`) for deterministic budgeting across heterogeneous LLM providers, with conversion formula: `ptok = actual_tokens / model_context_limit * PROTOCOL_NORMALIZER`.

**Rationale:** Enables measurable runtime budgeting without model-specific fragmentation. See `token-budget-resource-model.md` Section 5 (Token as protocol resource).

**Source Research:**
- `token-budget-resource-model.md` Section 5, lines 68-72

**Acceptance Criteria:**
- [ ] ptok conversion formula is documented and deterministic.
- [ ] Model profile registry maps each supported model to context limit and normalizer.
- [ ] Budget enforcement uses ptok units regardless of underlying model.

**Dependencies:** FR-0064
**Tags:** should-have

---

## FR-0074: Deterministic Context Envelope Allocation

**Category:** Agent Runtime

**Statement:** The system shall allocate context window percentages deterministically: identity 10%, goals 25%, inbox 15%, deltas 25%, tools 10%, reserve 15%.

**Rationale:** Prevents any single category from starving others. See `token-budget-resource-model.md` Section 5 (Deterministic context envelope).

**Source Research:**
- `token-budget-resource-model.md` Section 5, lines 74-82
- `token-efficiency-under-high-interaction.md` Section 5 (Deterministic context budget envelope)

**Acceptance Criteria:**
- [ ] Context assembler respects per-block caps.
- [ ] Overflow triggers deterministic pruning by priority score.
- [ ] Excess messages are summarized or dropped, never silently truncated.

**Dependencies:** FR-0073
**Tags:** should-have

