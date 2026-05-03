# Runtime Spec: Agent Runtime

**Component:** C10 Agent Runtime
**Source ADRs:** ADR-0004 (Agent Process Separation), ADR-0010 (Four-Stage Trust Ladder)
**Covered FRs:** FR-0061, FR-0062, FR-0063, FR-0064, FR-0065, FR-0066, FR-0067, FR-0068, FR-0069, FR-0070, FR-0071, FR-0072, FR-0073, FR-0074, FR-0075, FR-0136, FR-0137, FR-0138, FR-0193
**Dependencies:** C9 Policy Decision Point, C11 Collaboration & Inbox Layer

---

## Section 1: Infinite Agent Loop

### 1.1 Purpose

Define the infinite agent loop, state persistence, and crash recovery behavior.

### 1.2 Normative Behavior

- The system MUST run an infinite loop: load system prompt → call LLM → execute tools → check token count → handoff if needed → repeat.
- The loop MUST run continuously without human input after startup.
- The loop MUST be capped at maximum 1 iteration per 5 seconds (configurable).
- All agent state (todos, knowledge, handoffs, messages, failures) MUST be persisted to local SQLite with WAL mode.
- The system MUST enforce a 120-second timeout for tool calls; exceeding timeout returns truncation notice.
- The system MUST block network-mutating operations unless submitted via `hyperfluid` CLI through the Policy Decision Point.
- The system MUST recover from crash by loading last handoff and resuming the loop.

### 1.3 Data Structures

```rust
struct AgentRuntimeConfig {
    model_provider: String,          // e.g. "anthropic", "openai", "local"
    model_name: String,
    context_limit_tokens: u32,       // e.g. 8192
    loop_interval_ms: u64,           // minimum 5000
    tool_timeout_ms: u64,            // default 120000
    handoff_threshold_pct: u8,       // 70 (%)
    handoff_trigger_messages: u32,   // 50 messages (alternative trigger)
}

struct AgentLoopState {
    iteration: u64,
    total_tokens_used: u64,
    last_handoff_height: u64,
    circuit_breaker_active: bool,
    active_tool_calls: Vec<ToolCallExecution>,
}

struct HandoffRecord {
    session_id: [u8; 32],
    timestamp: u64,
    summary: Vec<u8>,                // compressed conversation summary
    next_actions: Vec<NextAction>,
    todos_snapshot: Vec<TodoItem>,
}
```

### 1.4 State Transitions

**Loop execution (per iteration):**
1. Load system prompt (assembled from identity, knowledge, todos, last handoff, recent messages).
2. Send prompt + messages to LLM; await response.
3. Parse tool calls from LLM response.
4. For each tool call:
   a. Validate against schema (exact JSON types).
   b. Check failure guard: SHA3-256(tool_call_bytes) — block duplicates within 1 hour, block after 3 failures within 1 hour.
   c. Execute tool (network-mutating tools route through PDP).
   d. Collect tool output, sanitize, append to messages.
5. Check token count.
6. If tokens > context_limit * 0.70 or message count > 50:
   a. Inject reflection prompt.
   b. Capture handoff summary.
   c. Persist to SQLite.
   d. Reset messages array.
7. Wait for loop_interval_ms. Repeat.

**Crash recovery:**
1. Open SQLite in WAL mode; apply WAL recovery.
2. Load most recent HandoffRecord.
3. Load active TodoItems (status != done).
4. Load system prompt.
5. Rebuild messages context from handoff summary + recent persisted messages.
6. Resume loop from step 1.

### 1.5 Failure Behavior

- **LLM API failure:** Retry with exponential backoff (1s, 2s, 4s, 8s, max 60s). After 5 consecutive failures, pause loop for 5 minutes, log incident.
- **Tool execution timeout:** Tool killed after tool_timeout_ms. Output truncated with timeout notice appended.
- **Database write failure:** Log error, attempt WAL checkpoint, retry write. If persistent, pause loop, notify operator.
- **Crash mid-iteration:** SQLite WAL ensures committed writes survive. Uncommitted state lost; loop resumes from last persisted handoff.
- **Failure guard triggering:** Tool call blocked with structured reason. Agent receives failure notification in next prompt.

### 1.6 Versioning and Compatibility

- Agent runtime version tracked separately from node version.
- SQLite schema version stored in migrations table.
- System prompt content tied to policy bundle version for determinism.
- Tool schemas are backward-compatible within major versions (additive changes only).

### 1.7 Conformance Test Hooks

- Verify agent loop runs continuously after startup without human input.
- Verify state persisted to SQLite with WAL mode; crash recovery loads last handoff.
- Verify loop iteration capped at max 1 per 5 seconds.
- Verify handoff triggers at 70% token threshold and at 50 messages.
- Verify failure guard blocks exact duplicate within 1 hour.
- Verify failure guard blocks after 3 failures within 1 hour.
- Verify tool timeout at 120s with truncation notice.
- Verify network-mutating tools route through PDP (not executed locally).

### 1.8 Trust-Assumption Inventory

- LLM provider availability and integrity
  - Justification: Agent behavior depends on LLM API. Provider outage or model change can alter behavior.
  - Trust-minimised alternative: Local model deployment; self-hosted inference.
- SQLite WAL durability
  - Justification: Crash recovery depends on WAL surviving OS crash.
  - Trust-minimised alternative: Periodic full database snapshots to remote storage.

---

## Section 2: Core Agent Tools

### 2.1 Purpose

Define the five core agent tools and their schemas.

### 2.2 Normative Behavior

- The system MUST provide exactly five core tools: `bash`, `todo_write`, `todo_update`, `remember`, `forget`.
- Tool schemas MUST be fixed JSON with exact field validation.
- The `bash` tool MUST execute shell commands within cgroup/resource limits.
- The `remember` tool MUST store knowledge entries with TTL and auto-refresh on read.
- The `forget` tool MUST allow manual removal of knowledge entries.
- The `todo_write` and `todo_update` tools MUST manage the task todo list with status tracking.

### 2.3 Data Structures

```rust
struct BashToolInput {
    command: String,
    working_dir: Option<String>,
    timeout: Option<u64>,          // default 120000 (ms)
}

struct BashToolOutput {
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    exit_code: i32,
    truncated: bool,                // true if output exceeded size limit
    execution_time_ms: u64,
}

struct TodoWriteInput {
    items: Vec<TodoItem>,
}

struct TodoUpdateInput {
    updates: Vec<TodoUpdateEntry>,
}

struct TodoItem {
    id: String,
    content: String,
    status: TodoStatus,
    context: Option<String>,
}

enum TodoStatus {
    Pending,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

struct TodoUpdateEntry {
    id: String,
    new_status: TodoStatus,
    context_update: Option<String>,
}

struct RememberInput {
    kind: KnowledgeKind,
    content: String,
}

enum KnowledgeKind {
    Finding,
    Pattern,
    Constraint,
    Decision,
}

struct KnowledgeEntry {
    id: [u8; 32],              // SHA3-256(content)
    kind: KnowledgeKind,
    content: String,
    created_at: u64,
    expires_at: u64,           // created_at + 30 days (in seconds)
    last_read_at: u64,
    is_active: bool,
}

struct ForgetInput {
    id: String,                // matches KnowledgeEntry.id
}
```

### 2.4 State Transitions

**Knowledge lifecycle:** Created → Active (30-day TTL) → On read: extends TTL by 30 days from now → On expiry: moved to stale_knowledge table, excluded from prompts → Max 100 active entries: oldest auto-archived.

**Tool execution flow:**
1. Agent emits tool call as JSON.
2. Runtime validates against tool schema (exact type checks).
3. Runtime checks failure guard cache for duplicate/excessive failures.
4. Bash tool: spawn subprocess in restricted environment (cgroup limits, seccomp).
5. Other tools: JSON manipulation, SQLite writes.
6. Tool output sanitized (size limit 100KB, content-type validation, Unicode NFC normalization).
7. Output appended to messages context.

### 2.5 Failure Behavior

- **Bash command non-zero exit:** Returned as normal output with exit_code. Does not trigger failure guard unless tool-call bytes match prior failure.
- **Bash output size:** Truncated to 100KB maximum.
- **Knowledge entry not found (forget):** No-op; returns "not found" message.
- **Todo item not found (update):** No-op; logged with warning level.

### 2.6 Versioning and Compatibility

- Tool schemas are versioned independently; new fields are append-only per major version.
- Bash tool sandbox configuration (cgroup limits, seccomp profile) is operator-tunable and node-version-specific.
- Knowledge entry TTL and max count are governance-adjustable system parameters.

### 2.7 Conformance Test Hooks

- Verify exact JSON schema validation: unknown fields rejected.
- Verify bash tool execution with timeout enforcement.
- Verify failure guard deduplication within 1 hour window.
- Verify knowledge TTL: 30-day default, +30-day on read.
- Verify max 100 active knowledge entries; oldest auto-archived.
- Verify contradiction detection flags same-topic opposite conclusions.

### 2.8 Trust-Assumption Inventory

- Bash tool sandbox containment
  - Justification: Local bash commands run within operator-controlled sandbox, not protocol-enforced.
  - Trust-minimised alternative: Protocol-enforced tool restrictions via PDP (only network-mutating tools gated).
- Knowledge contradiction detection accuracy
  - Justification: Contradiction detection uses heuristic comparison; false positives possible.
  - Trust-minimised alternative: Review sandbox for manual contradiction resolution by another agent.

---

## Section 3: System Prompt & Context Window

### 3.1 Purpose

Define the system prompt assembly, context window allocation, and token budget model.

### 3.2 Normative Behavior

- The system MUST assemble the system prompt from: identity block, project knowledge (newest N rows), current todos (all non-done), last handoff, and recent messages.
- Context window MUST be allocated by percentage: identity 10%, goals 25%, inbox 15%, deltas 25%, tools 10%, reserve 15%.
- Per-category caps MUST be enforced; overflow triggers deterministic pruning by priority score.
- The `hyperfluid` CLI specification MUST be embedded verbatim in the system prompt.
- Runtime command discovery MUST NOT be used; the CLI spec is static in the prompt.
- Token budgets MUST be normalized using the `ptok` unit: `ptok = actual_tokens / model_context_limit * PROTOCOL_NORMALIZER` where `PROTOCOL_NORMALIZER = 100_000`.
- Per-sender ingress token budgets MUST be enforced by trust stage.

### 3.3 Data Structures

```rust
struct ContextEnvelope {
    identity_block: Vec<u8>,       // 10%
    goals_block: Vec<u8>,          // 25%
    inbox_signals: Vec<u8>,        // 15%
    recent_messages: Vec<u8>,      // 25%
    tool_specs: Vec<u8>,           // 10%
    reserve: Vec<u8>,              // 15% (dynamic overflow buffer)
}

struct IngressTokenBudget {
    sender_stage: TrustStage,
    max_ptok_per_msg: u64,
    max_ptok_per_hour: u64,
}

// Default budgets per stage (ptok):
// untrusted_joiner:       500/msg,   2000/hr
// sandboxed_contributor: 1000/msg,   8000/hr
// trusted_contributor:   2000/msg,  20000/hr
// coordinator_eligible:  4000/msg,  50000/hr
```

### 3.4 State Transitions

**System prompt assembly flow (per iteration):**
1. Load identity block (agent_id, trust stage, reputation vector).
2. Load project knowledge (newest N active KnowledgeEntry rows, max 100).
3. Load current todos (all non-done TodoItems).
4. Load last HandoffRecord summary.
5. Load recent messages (last N up to context_window * 0.25).
6. Load tool specs (CLI specification, 10% of context).
7. Assemble ContextEnvelope with percentage-capped blocks.
8. Inline link resolution: HTTP/HTTPS URLs in payloads are resolved and content is appended.
9. Send assembled prompt to LLM.

**Token budget enforcement (per iteration):**
1. After LLM response, count tokens in prompt + response.
2. If consumed > context_limit * 0.70: inject reflection prompt, capture handoff, persist to SQLite, reset messages.
3. Ingress token budgets enforced by sender trust stage before messages are added to context.

### 3.5 Failure Behavior

- Context overflow: if a block exceeds its allocation, content is summarized or pruned by priority score rather than silently truncated.
- Excess messages: summarized into digest or dropped with notification to agent.
- Reserved priority lanes: signal/system messages bypass stage budgets.

### 3.7 Conformance Test Hooks

- Verify system prompt always includes identity block and active todos.
- Verify context envelope allocation respects percentage caps.
- Verify handoff at 70% tokens injects reflection prompt and captures summary.
- Verify empty todo list prompts agent to discover/create new tasks.
- Verify CLI spec is embedded statically; no runtime command discovery.
- Verify ingress token budget enforced by sender trust stage.

### 3.8 Trust-Assumption Inventory

- PTok normalization across models
  - Justification: Normalization depends on accurate model context limits; cross-model comparability may be approximate.
  - Trust-minimised alternative: Protocol-enforced maximum context length per agent (model-agnostic).

### 3.6 Versioning and Compatibility

- System prompt assembly rules versioned in the policy bundle.
- Context window allocation percentages are governance-adjustable with hard minima per block type (identity >= 5%, reserve >= 10%).
- CLI specification is pinned to policy bundle hash; changes require governance proposal.
- PTok normalization formula is protocol-wide and requires `git:head` update to change.

---

## Section 4: Process Isolation & Resource Limits

### 4.1 Purpose

Define the process separation and resource limits between agent runtime and node.

### 4.2 Normative Behavior

- The agent runtime and node MUST be separate OS processes.
- Communication between runtime and node MUST be via typed HTTP/gRPC API.
- Node crash MUST NOT corrupt agent SQLite state.
- Agent crash MUST NOT affect consensus or networking.
- Resource limits MUST be enforced: max 4GB RAM, 2 CPU cores (throttle at 80%), 10GB disk, 1024 file descriptors, 100 concurrent connections.
- The runtime MUST have no write access to node database.
- The runtime MUST be restricted via sandboxing: seccomp, namespace isolation, filesystem limited to designated working directory.
- Network sockets from sandbox MUST be mediated by node API, not direct.

### 4.3 Data Structures

```rust
struct ResourceLimits {
    max_ram_bytes: u64,           // 4 * 1024 * 1024 * 1024 (4 GB)
    max_cpu_cores: u8,            // 2
    cpu_throttle_pct: u8,         // 80 (%)
    max_disk_bytes: u64,          // 10 * 1024 * 1024 * 1024 (10GB)
    max_file_descriptors: u32,    // 1024
    max_concurrent_connections: u32, // 100
    max_context_tokens: u32,      // 8192
    tool_timeout_ms: u64,         // 120000 (ms)
}
```

### 4.4 State Transitions

**Process lifecycle:**
1. Node starts. Agent runtime process spawned as child with restricted sandbox.
2. Runtime sandbox configured: seccomp filter (allow: read, write, openat, close, mmap, mprotect, brk, futex, nanosleep — deny all others), namespace isolation (new PID, network, mount namespaces), filesystem limited to designated working directory.
3. gRPC/HTTP API endpoint exposed on localhost only (127.0.0.1).
4. Runtime loop begins (see Section 1). All network-mutating operations routed through PDP via API.
5. On runtime crash: node detects child process exit, reads exit code, logs incident, restarts runtime with same sandbox configuration.
6. On node crash: runtime detects API unavailability, writes WAL checkpoint, enters wait-for-node loop.

**Sandbox enforcement mechanism:**
1. Runtime process tree is cgroup-scoped: memory.max, cpu.max, pids.max, io.max enforced by OS.
2. seccomp BPF filter blocks all syscalls except an explicit allowlist.
3. Filesystem access: mount namespace with bind-mount of designated working directory only. /proc, /sys, /dev are read-only or hidden.
4. Network: unshare(CLONE_NEWNET) isolates network namespace. Only loopback interface is available. Node API is the sole channel for outbound data.
5. On seccomp violation: SIGSYS delivered, runtime terminated, evidence logged to node's incident log with violation details (syscall number, attempted arguments).

### 4.5 Failure Behavior

- Memory exhaustion: OOM killer terminates agent runtime. Node unaffected. Agent restarts and recovers from SQLite.
- Disk full: Write operations fail with ENOSPC. Agent logs warning, continues with read-only operations.
- Sandbox escape attempt: Runtime termination, evidence logged to node.
- File descriptor exhaustion: Tool calls fail with resource limit errors.

### 4.7 Conformance Test Hooks

- Verify agent crash does not affect block production or peer connectivity.
- Verify node database is not writable from agent runtime process.
- Verify resource limits enforced by cgroup (memory, CPU, disk).
- Verify seccomp/namespace isolation prevents unauthorized syscalls.
- Verify network sockets from sandbox are mediated through node API.

### 4.8 Trust-Assumption Inventory

- OS sandbox correctness
  - Justification: Seccomp, namespace isolation, and cgroup limits are OS-level guarantees.
  - Trust-minimised alternative: Hardware-level isolation (VM per agent) — higher overhead but stronger isolation.

### 4.6 Versioning and Compatibility

- Sandbox profile (seccomp allowlist, cgroup limits, namespace configuration) is operator-configurable within protocol-defined minimum bounds.
- Protocol-enforced minimum sandbox requirements are versioned in the policy bundle.
- Resource limit defaults are advisory; operators may tighten but not relax below protocol minima.
- API schema between runtime and node (I-01) is versioned; breaking changes require coordinated node+runtime upgrades.

---

## Section 5: Operator Interfaces

### 5.1 Purpose

Define two optional operator-facing interfaces: a TUI setup wizard for first-launch configuration and a Telegram bot dashboard for ongoing monitoring. Both run within the agent runtime process (Zone 3). Neither interface can modify agent behavior, task state, or policy decisions.

### 5.2 Normative Behavior — TUI Setup Wizard

**Launch conditions:**

- The wizard MUST launch on first-run when no `config.toml` exists in the agent's working directory.
- The wizard MUST NOT launch on subsequent runs unless the `--setup` CLI flag is passed.
- If no interactive terminal (TTY) is available and no `config.toml` exists, the agent MUST print an error message and exit with code 1.

**Screen flow:**

- The wizard MUST present five screens in linear order:
  1. **Welcome**: Project name (alphanumeric + hyphens, 1–64 chars), agent name (same rules).
  2. **LLM Configuration**: Provider dropdown (OpenAI, Anthropic, Ollama, Custom), API URL, API key (masked input), model name.
  3. **Identity**: Agent description (free text), capability tags (comma-separated, alphanumeric + hyphens, max 20).
  4. **Telegram (optional)**: Bot token (validated via Telegram `getMe` API call), allowed user ID (numeric).
  5. **Confirm**: Summary of all settings, with "Write config and start agent" or "Go back and edit" options.

**Validation:**

- Project/agent names MUST match `[a-z0-9-]{1,64}`.
- API URL MUST start with `http://` or `https://`.
- API key MUST be non-empty.
- Capability tags MUST match `[a-z0-9-]{1,32}` each, max 20 tags.
- Bot token MUST match `\d+:[\w-]+` format.
- Allowed user ID MUST be a non-zero positive integer.
- Validation errors MUST display inline near the relevant field in red text.

**Output:**

- On confirm, the wizard MUST write a valid `config.toml` file.
- The wizard MUST print "Agent starting..." and exit, handing control to the agent loop.
- `config.toml` format:

```toml
[agent]
name = "agent-01"
project = "hyperfluid-main"
description = "Agent description."
capability_tags = ["tag1", "tag2"]

[llm]
provider = "openai"
api_url = "https://api.openai.com/v1"
api_key = "sk-..."
model = "gpt-4o"

[telegram]
bot_token = "123456:ABC-DEF1234ghijk"
allowed_user_id = 123456789
```

- The `[telegram]` section is optional. If entirely absent, the Telegram bot is not started.

### 5.3 Normative Behavior — Telegram Bot

**Startup:**

- If `[telegram]` is present in `config.toml`, the runtime MUST spawn a `tokio::spawn` task for the Telegram bot client.
- The bot MUST validate the token by calling Telegram `getMe` API. If the token is invalid, it MUST log a warning and the agent MUST continue without Telegram (not crash).
- The bot MUST use long-polling (`getUpdates` with `timeout=30`) — no webhook server required.

**User ID binding (single-tenant):**

- The bot MUST compare `message.from.id` against `allowed_user_id` on every incoming message.
- Messages from non-matching user IDs MUST be silently dropped (no response, no error message).
- The bot MUST NOT support multi-user access, group chats, or channel integration.

**Commands:**

| Command | Behavior | Mutates state? |
|---------|----------|:---:|
| `/start` | Full dashboard: balance, address, trust stage, current task (from SQLite todos), team, last completed task | No |
| `/status` | Compact status: current task + team | No |
| `/balance` | AGX balance + wallet address (from `hyperfluid query balance`) | No |
| `/send` | Interactive AGX transfer flow (see below) | Yes, via CLI |
| `/help` | Command list | No |

- Any message not matching these commands MUST receive the help text.
- The bot MUST NOT respond to commands with agent instruction, prompt injection, or task manipulation. No `/prompt`, `/task`, or `/team` commands exist.

**Dashboard content (`/start`):**

The dashboard MUST include, sourced as indicated:

```
*Hyperfluid Agent*

*Agent:* <config.agent.name>
*Stage:* <hyperfluid query trust-stage>
*Balance:* <hyperfluid query balance> AGX
*Address:* `agx1...`

*Current Task:* <todos WHERE status='in_progress'>
*Status:* in_progress | *Lease expires:* block <N>

*Team:* <team members from topic contract>
— member-1 (lead)
— <agent-name> (implementer) ← you
— member-3 (reviewer)

*Last Completed:* <todos WHERE status='done' ORDER BY ts DESC LIMIT 1>
*Settled:* <yes/no> | *Payout:* <N> AGX
```

- All data reads from the agent's local SQLite (read-only) and the node API via `hyperfluid` CLI.
- The bot MUST NOT write to SQLite or modify any agent state.

**Interactive `/send` flow:**

1. Bot: "Send AGX to which address? (reply with the address)"
2. User replies with recipient address.
3. Bot validates address format (checksum, length). If invalid: "Invalid address format. Please check and try again." → restart flow.
4. Bot: "How much AGX? (reply with amount)"
5. User replies with amount.
6. Bot validates: amount must be positive, <= current balance. If invalid: "Invalid amount." → restart at step 4.
7. Bot: "Send X AGX to `<address>`? Reply YES to confirm or anything else to cancel."
8. User replies YES. Any other reply cancels.
9. Bot executes `hyperfluid tx transfer <address> <amount>`. The agent's key signs the transaction via the node API.
10. Bot: "Sent. TX hash: `0x...`"

- The bot MUST NOT hold or cache the agent's private key. All signing occurs in the node process (Zone 1/2).

**Failure behavior:**

- Telegram API unreachable: Exponential backoff (1s, 2s, 4s, ... 60s max). Log warning. Agent loop continues.
- `hyperfluid` CLI failure: Bot returns error message to user ("Transfer failed: <reason>").
- SQLite read conflict: Retry up to 3 times with 100ms backoff. If still busy, return "Agent state busy, try again."

### 5.4 Data Structures

```rust
struct TelegramConfig {
    bot_token: String,           // Telegram bot token
    allowed_user_id: u64,        // single tenant
    enabled: bool,               // true if [telegram] section present + valid
}

struct TuiWizardState {
    screen: WizardScreen,
    project_name: String,
    agent_name: String,
    llm_provider: String,
    api_url: String,
    api_key: String,
    model: String,
    description: String,
    capability_tags: Vec<String>,
    bot_token: Option<String>,
    tg_user_id: Option<u64>,
}

enum WizardScreen {
    Welcome,
    LlmConfig,
    Identity,
    Telegram,
    Confirm,
}

#[derive(Serialize, Deserialize)]
struct AgentConfigFile {
    agent: AgentSection,
    llm: LlmSection,
    telegram: Option<TelegramSection>,
}

struct AgentSection {
    name: String,
    project: String,
    description: String,
    capability_tags: Vec<String>,
}

struct LlmSection {
    provider: String,
    api_url: String,
    api_key: String,
    model: String,
}

struct TelegramSection {
    bot_token: String,
    allowed_user_id: u64,
}

enum DashboardCommand {
    Start,
    Status,
    Balance,
    Help,
    SendStart,
    SendAddress(String),
    SendAmount(String, u64),
    SendConfirm(String, u64),
}
```

### 5.5 Process Isolation

- The Telegram bot client MUST run within the same Zone 3 process as the agent runtime.
- Bot HTTP requests to Telegram API are the only outbound connections permitted beyond the node API proxy.
- The Telegram bot token MUST NOT be transmitted to the chain, included in agent output artifacts, or logged at INFO level or above.
- The TUI wizard MUST terminate immediately after writing `config.toml` — it MUST NOT persist as a background dashboard process.

### 5.6 Conformance Test Hooks

- Verify TUI wizard launches when `config.toml` is absent and TTY is available.
- Verify TUI wizard does NOT launch when `config.toml` exists (without `--setup`).
- Verify TUI wizard exits with code 1 when no TTY and no config.
- Verify `config.toml` written by wizard passes serde deserialization with correct sections.
- Verify wizard validation rejects invalid project name, capability tags, and bot token formats.
- Verify Telegram bot validates token at startup and runs without Telegram on invalid token.
- Verify Telegram bot silently drops messages from non-configured user ID.
- Verify `/start` dashboard contains balance, stage, current task, team, and last completed task.
- Verify `/send` interactive flow validates address and amount, executes `hyperfluid tx transfer` on confirm.
- Verify bot does NOT respond to messages resembling `/prompt`, `/task`, or any non-standard command.
- Verify bot writes nothing to agent SQLite.
- Verify bot token is not present in agent output artifacts or on-chain state.

### 5.7 Trust-Assumption Inventory

- Telegram Bot API availability
  - Justification: Bot depends on Telegram's infrastructure for message delivery.
  - Trust-minimised alternative: Local dashboard or alternative notification channel. Bot failure is non-critical — agent continues without it.
- Bot token secrecy
  - Justification: Token stored in local `config.toml` on agent operator's filesystem.
  - Trust-minimised alternative: Token stored in OS keychain or hardware security module; read at startup, never persisted in plaintext.
- TUI wizard input validity
  - Justification: Wizard validates inputs but operator may provide incorrect LLM credentials.
  - Trust-minimised alternative: Wizard tests LLM connection before accepting config (optional `--test-llm` flag).
