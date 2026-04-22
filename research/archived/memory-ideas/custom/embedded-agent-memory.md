




# CONCLUSION: Overkilll and a mess, not good for an infinitely running agent







# Autonomous Agent Memory (Embedded SQLite + sqlite-vec)

## 1. Title

- **Autonomous Agent Memory for Infinite Runtime: SQLite Event Log, Failure Memory, and Summary-Vector Recall**

## 2. Executive Summary

- The memory system is built around three problems only: **working context**, **episodic recall**, and **crash recovery**.
- Use a single embedded database file: **SQLite in WAL mode** as source of truth; no external memory server.
- Store everything as append-only events plus a dedicated failure table and one summary row per cycle.
- The highest-impact component is the **context assembly function**, not the storage engine.
- Enforce context priority order: system state, last 5 cycles verbatim, relevant failures, then semantic episodes.
- Use **all-MiniLM-L6-v2** (384-dim) for summary embeddings only; do not embed raw events.
- Use **sqlite-vec** in the same SQLite file for semantic retrieval to keep ops surface minimal.
- Add a pre-tool-call loop guard using `action_hash` from normalized `(tool, params)` to block repeated failures.
- Use deterministic, rule-based compaction; avoid LLM-based compaction in the core path.
- Keep an LLM-agnostic interface while supporting standard OpenAI client usage out of the box.

## 3. System Overview

- **Problem solved**
  - Keep an autonomous agent coherent over long runtimes without human messages.
  - Preserve complete execution trace for debugging, replay, and recovery.
  - Retrieve useful prior episodes without bloating prompts.

- **Design philosophy**
  - Prefer boring primitives first: SQLite + WAL + SQL queries.
  - Keep memory embedded and local to the process.
  - Add semantic recall only where it improves quality (cycle summaries).
  - Make failures first-class and block repeating bad actions.

- **Hard constraints**
  - Agent runs continuously and must recover after crash/restart.
  - Tool loops must be prevented before execution.
  - Prompt builder must stay under strict token budget.
  - Memory stack must support OpenAI and non-OpenAI LLM providers.

## 4. Architecture (CRITICAL SECTION)

### 4.1 Single-Struct Mental Model

```text
Agent { think() / act() / tool() }
    |
    +-- Memory (always on, internal)
          |
          +-- SQLite (events + failures + summaries) [same file]
          +-- sqlite-vec (summary embeddings)        [same file]
          +-- all-MiniLM-L6-v2 (~22MB in-process)    [local model]
```

- One public `Memory` abstraction.
- No external services, no network dependency in memory path.
- Everything private behind three external calls:
  - `record_event(...)`
  - `build_context(...)`
  - `guard_tool_call(...)`

### 4.2 Component Responsibilities

- **Agent loop**
  - Collect observation.
  - Build context from memory.
  - Call LLM.
  - Execute tool.
  - Record everything.

- **SQLite (source of truth)**
  - Append-only event log.
  - Dedicated failure table for fast loop prevention.
  - Cycle summary table for semantic entry points.

- **sqlite-vec**
  - Stores embeddings for summary text only.
  - Returns relevant `cycle_id`s.
  - Full cycle records fetched from SQLite by `cycle_id`.

- **Embedding runtime**
  - `all-MiniLM-L6-v2` local embedding model.
  - Generates summary embeddings at cycle end.
  - No remote embedding API required in critical path.

### 4.3 Data and Control Flow (Step-by-Step)

```text
1) Observe state
   -> INSERT events(type='observation')

2) Build context
   -> fixed system block
   -> last N cycles (N=5) from events
   -> failure matches from failures table
   -> semantic episodes from sqlite-vec (summaries)

3) LLM call
   -> INSERT events(type='llm_request', payload=exact request)
   -> call provider (OpenAI/Anthropic/local)
   -> INSERT events(type='llm_response', payload=exact response)

4) Parse action + preflight guard
   -> hash(normalized tool + params)
   -> query failures(action_hash, recent window)
   -> block/replan if repeated failures exceed threshold

5) Tool execution
   -> INSERT events(type='tool_call', payload=args)
   -> run tool
   -> INSERT events(type='tool_result'|'error', payload=result)
   -> INSERT/UPDATE failures row if failed

6) End cycle summary
   -> deterministic one-sentence summary
   -> INSERT cycle_summaries
   -> embed summary and INSERT into sqlite-vec index
```

### 4.4 Schema (Minimal and Sufficient)

```sql
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;

CREATE TABLE IF NOT EXISTS events (
  id        INTEGER PRIMARY KEY,
  cycle_id  TEXT NOT NULL,
  ts        INTEGER NOT NULL,
  type      TEXT NOT NULL,
  payload   TEXT NOT NULL -- JSON
);
CREATE INDEX IF NOT EXISTS idx_events_cycle ON events(cycle_id);
CREATE INDEX IF NOT EXISTS idx_events_type_ts ON events(type, ts);

CREATE TABLE IF NOT EXISTS failures (
  id           INTEGER PRIMARY KEY,
  ts           INTEGER NOT NULL,
  tool         TEXT,
  action_hash  TEXT NOT NULL,
  error_type   TEXT,
  error_msg    TEXT,
  resolution   TEXT
);
CREATE INDEX IF NOT EXISTS idx_fail_hash_ts ON failures(action_hash, ts);

CREATE TABLE IF NOT EXISTS cycle_summaries (
  cycle_id      TEXT PRIMARY KEY,
  ts            INTEGER NOT NULL,
  summary_text  TEXT NOT NULL
);
```

```sql
-- sqlite-vec table (extension-specific DDL may vary by build)
CREATE VIRTUAL TABLE IF NOT EXISTS cycle_summary_vec
USING vec0(
  cycle_id TEXT PRIMARY KEY,
  embedding FLOAT[384]
);
```

## 5. Core Mechanisms

### 5.1 What Gets Stored (Complete Agent Trace)

- **Events table** stores raw runtime history:
  - `observation`
  - `llm_request`
  - `llm_response`
  - `tool_call`
  - `tool_result`
  - `error`
  - `summary` (optional mirror of `cycle_summaries`)

- **Payload discipline**
  - `llm_request.payload` includes model, temperature, max_tokens, tools schema, and final prompt text.
  - `llm_response.payload` includes raw assistant output, tool choice, token usage, stop reason.
  - `tool_call.payload` includes tool name and normalized params.
  - `tool_result.payload` includes success/failure, return data, duration, side-effect references.
  - `error.payload` includes stack/context plus recovery decision.

- **Failure table**
  - Fast path for "should I block this action before running it?"
  - Independent from event replay, so checks remain O(log n) with index.

- **Cycle summaries**
  - One short summary per cycle for semantic lookup.
  - Full details still in `events`; vectors only point to relevant cycles.

### 5.2 Context Assembly Function (Primary Product Surface)

- Priority order is strict:
  1. **System identity and current global goal** (always included).
  2. **Last N cycles verbatim** (`N=5` default, `N=10` upper bound).
  3. **Relevant failure memory** for pending action/tool.
  4. **Semantic recall** from `cycle_summaries` embeddings.

- Token packing rules:
  - Fixed budget for each section.
  - Never let semantic episodes evict last-N cycles.
  - Reserve minimum token slice for failure memory.

```python
def build_context(db, vec, current_goal, pending_action, budget):
    blocks = []

    # 1) Always-present system state
    blocks.append(system_identity_block())

    # 2) Recent verbatim cycles (non-negotiable)
    recent = fetch_last_cycles(db, n=5)
    blocks.append(format_cycles_verbatim(recent))

    # 3) Failure memory (prevents repeated bad actions)
    if pending_action is not None:
        h = action_hash(pending_action.tool, pending_action.params)
        fails = fetch_recent_failures(db, h, window_sec=86400, limit=8)
        blocks.append(format_failure_block(fails))

    # 4) Semantic episodes from summaries
    q_emb = embed(current_goal)  # MiniLM
    cycle_ids = vec_search(vec, q_emb, k=8)
    episodes = fetch_cycles_by_id(db, cycle_ids)
    blocks.append(format_semantic_episodes(episodes))

    return pack_with_token_budget(blocks, budget, hard_order=True)
```

### 5.3 Pre-Tool Failure Guard (Loop Prevention)

```python
def should_block_action(db, tool, params, now_ts):
    h = action_hash(tool, normalize(params))
    count = db.scalar(
        "SELECT COUNT(*) FROM failures WHERE action_hash=? AND ts>?",
        (h, now_ts - 3600)
    )
    if count > 2:
        return True, "repeated failure in last hour; replan required"
    return False, None
```

- This check runs **before every tool call**.
- If blocked:
  - write `events(type='error', payload={'reason': 'loop_guard'})`
  - force LLM replan with failure context injected.

### 5.4 LLM Integration: Standard OpenAI Client + Provider-Agnostic Interface

```python
class LLMClient:
    def complete(self, system_prompt, user_prompt, tools, params):
        raise NotImplementedError()

class OpenAIClientAdapter(LLMClient):
    def __init__(self, client, model):
        self.client = client  # standard OpenAI SDK client
        self.model = model

    def complete(self, system_prompt, user_prompt, tools, params):
        return self.client.responses.create(
            model=self.model,
            input=[
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            tools=tools,
            temperature=params.temperature,
            max_output_tokens=params.max_tokens
        )
```

```python
class AnthropicAdapter(LLMClient): ...
class LocalOllamaAdapter(LLMClient): ...
class AnyHTTPAdapter(LLMClient): ...
```

- Agent loop interacts with `LLMClient` only.
- Every adapter must serialize request/response into identical `events` payload shape.

### 5.5 Memory Implementations (Practical Variants, Same Core Schema)

1. **Log-Only Baseline**
   - SQLite `events` only.
   - Use for initial observability and replay.

2. **Log + Failure Guard**
   - Add `failures` and `action_hash` check.
   - First production-safe autonomous version.

3. **Log + Prompt Discipline**
   - Add strict context assembly ordering and token caps.
   - Largest quality jump after loop prevention.

4. **Summary Semantic Recall**
   - Add `cycle_summaries` + sqlite-vec.
   - Episodic recall without embedding explosion.

5. **Deterministic Compaction**
   - Keep summaries/failures/durable outputs.
   - Drop redundant intermediate entries by policy.

6. **Action Outcome Cache**
   - Cache successful tool outputs keyed by normalized action hash.
   - Short-circuit repeated deterministic actions.

7. **State Checkpoint Snapshots**
   - Every M cycles, store compact world-state checkpoint event.
   - Fast restart and reduced replay time.

8. **Replay Validator Mode**
   - Re-run stored cycles in dry-run mode for regression checks.
   - Catches prompt regressions and tool contract drift.

## 6. Design Decisions & Tradeoffs

### Tradeoff A: SQLite + WAL vs Specialized KV

- **Option 1: SQLite + WAL (chosen)**
  - Pros: SQL debugging, easy tooling, one file, mature crash behavior.
  - Cons: Lower write throughput ceiling than specialized KV engines.
- **Option 2: specialized LSM KV engine**
  - Pros: Higher raw KV throughput.
  - Cons: Less transparent debugging, higher complexity for ad hoc analysis.
- **Why chosen**
  - Primary bottleneck is context quality, not storage throughput.
  - SQLite remains sufficient for typical autonomous agent event rates.

### Tradeoff B: Embed All Events vs Embed Cycle Summaries

- **Option 1: Embed all events**
  - Pros: fine-grained semantic search.
  - Cons: high compute/storage cost, noisy retrieval, slower ingestion.
- **Option 2: Embed one summary per cycle (chosen)**
  - Pros: compact index, meaningful recall units, fast updates.
  - Cons: summary may omit useful detail.
- **Why chosen**
  - Retrieval lands on cycle IDs, then pulls exact records from SQLite.
  - Better relevance-density per vector.

### Tradeoff C: LLM-Based vs Rule-Based Compaction

- **Option 1: LLM compaction**
  - Pros: flexible natural-language abstraction.
  - Cons: expensive, nondeterministic, fails when provider is unavailable.
- **Option 2: Rule-based compaction (chosen)**
  - Pros: deterministic, cheap, no extra dependency in maintenance path.
  - Cons: lower semantic richness in compressed history.
- **Why chosen**
  - Infinite-runtime systems need predictable maintenance behavior.

### Tradeoff D: Semantic Recall First vs Recency First

- **Option 1: semantic-first ordering**
  - Pros: more historical breadth.
  - Cons: can crowd out immediate state and cause action drift.
- **Option 2: recency-first ordering (chosen)**
  - Pros: prevents loopiness and stale decisions.
  - Cons: may miss distant but relevant episode when budget is tight.
- **Why chosen**
  - Last N cycles and failures are operationally critical.

### Tradeoff E: Provider-Specific APIs vs LLM Adapter Interface

- **Option 1: direct provider coupling**
  - Pros: minimal initial code.
  - Cons: migration and A/B testing become painful.
- **Option 2: common adapter interface (chosen)**
  - Pros: easy provider swap, consistent logging schema.
  - Cons: slight abstraction overhead.
- **Why chosen**
  - Long-running agents require model agility over time.

## 7. Failure Modes & Edge Cases

### 7.1 Repeated Tool Failure Loops (Highest Risk)

- **What happens**
  - Agent repeatedly calls same tool with equivalent params after failures.
- **Why it happens**
  - Missing pre-call failure memory check.
- **Handling**
  - Hash normalized action.
  - Block after threshold in time window.
  - Force replan prompt containing last failures and alternative actions.

### 7.2 Context Drift

- **What happens**
  - Prompt no longer reflects latest world state.
- **Why it happens**
  - Semantic retrieval crowds out recent history.
- **Handling**
  - Always include last 5 cycles verbatim before semantic episodes.
  - Recency-weighted packing; hard token reservation for recent block.

### 7.3 Crash During Write

- **What happens**
  - Process dies after partial cycle.
- **Why it happens**
  - Power loss/process kill between events.
- **Handling**
  - SQLite WAL recovers committed rows.
  - On restart, detect incomplete cycle by missing terminal event and mark as interrupted.

### 7.4 LLM Provider Outage / Timeout

- **What happens**
  - Think step unavailable.
- **Why it happens**
  - API/network failure.
- **Handling**
  - Switch adapter to fallback provider/local model.
  - Log outage event; reduce action rate; keep durable state.

### 7.5 Embedding Pipeline Backlog

- **What happens**
  - Cycle summaries exist but vectors lag behind.
- **Why it happens**
  - CPU saturation or queue growth.
- **Handling**
  - Retrieval degrades to recency + failure-only mode temporarily.
  - Backfill embeddings asynchronously with bounded queue.

### 7.6 Tool Contract Drift

- **What happens**
  - Tool input/output format changes silently.
- **Why it happens**
  - Dependency updates or API version changes.
- **Handling**
  - Validate tool payload schema before execution.
  - Record parse errors with tool/version in `events`.
  - Auto-replan with explicit schema error context.

## 8. Scalability Analysis

### Small Scale (10-100 nodes)

- **Typical deployment**
  - Single-agent or small fleet, each with local SQLite file.
- **Performance**
  - Write path: append events + occasional failure inserts.
  - Recall path: low-latency local SQL + sqlite-vec lookup.
- **Bottlenecks**
  - Prompt assembly logic quality, not DB throughput.
- **Operational load**
  - Low; one file backup and periodic compaction.

### Medium Scale (1k-10k nodes)

- **Typical deployment**
  - Per-agent local memory remains embedded; no shared memory in core path.
- **Performance concerns**
  - Fleet-wide observability aggregation, not per-agent DB limits.
- **Bottlenecks**
  - Embedding CPU budget and compaction scheduling.
- **Mitigations**
  - Per-agent embedding queue limits, staggered compaction windows, centralized metrics export.

### Large Scale (100k+ nodes)

- **Typical deployment**
  - Same local-first architecture; centralized systems consume exported summaries/events asynchronously.
- **Bottlenecks**
  - Fleet telemetry volume and log retention economics.
- **Risks**
  - Inconsistent policy rollout causing behavior divergence across agents.
- **Mitigations**
  - Versioned prompt-builder policy, schema migrations with compatibility windows, immutable event contracts.

## 9. Recommended Architecture

- **Final choice**
  - Embedded **SQLite + WAL** for `events`, `failures`, and `cycle_summaries`.
  - Embedded **sqlite-vec** for summary embedding search in the same DB file.
  - Local **all-MiniLM-L6-v2** model for embeddings.
  - LLM adapter interface with first-class OpenAI client implementation.

- **Why optimal**
  - Minimal moving parts.
  - No external memory service dependency.
  - Strong crash recovery and easy debugging.
  - Correct emphasis on context discipline and failure prevention.

- **Alternatives rejected now**
  - Multi-engine tiered memory stacks: higher complexity before proven need.
  - Full-event embedding pipelines: expensive and lower retrieval signal.
  - LLM-based compaction in core loop: unreliable maintenance dependency.

## 10. Implementation Plan

### Phase 1: Agent Loop and LLM Adapter

- Build `LLMClient` interface.
- Implement OpenAI adapter using standard SDK client.
- Implement one fallback adapter (Anthropic or local HTTP).
- Run loop with tool execution and no memory retrieval yet.

### Phase 2: Durable Event Log

- Add SQLite file initialization with WAL settings.
- Add `events` table and append-only writes.
- Record every `llm_request`, `llm_response`, `tool_call`, `tool_result`, `error`.
- Add cycle IDs and strict payload schema version field.

### Phase 3: Failure Guard

- Add `failures` table and action-hash normalization.
- Run pre-tool check before every call.
- Block repeated failures and trigger explicit replan branch.

### Phase 4: Context Assembly Function

- Implement strict-priority prompt builder:
  - system block
  - last 5 cycles
  - failure memory
  - optional semantic episodes
- Add hard token budgeting and overflow truncation policy.
- Add unit tests for deterministic packing order.

### Phase 5: Episode Summary Vectors

- Add `cycle_summaries` table.
- Generate one deterministic summary per cycle.
- Embed summaries with all-MiniLM-L6-v2.
- Add sqlite-vec index and semantic cycle lookup.

### Phase 6: Rule-Based Compaction and Recovery Hygiene

- Add deterministic retention rules (keep failures, summaries, durable outputs).
- Compact old redundant event rows by policy.
- On startup, detect interrupted cycles and mark recoverable state.
- Add periodic backup/export job for SQLite file snapshots.

## 11. Future Improvements

- Add confidence scoring to context blocks to improve token allocation.
- Add per-tool adaptive failure thresholds instead of a global threshold.
- Add deterministic "resolution memory" linking failures to eventual successful actions.
- Add replay-based evaluation harness for prompt-builder policy changes.
- Add optional encrypted payload columns for sensitive tool outputs.
- Add schema version migration framework with backward readers.
- Add fleet-level observability export (metrics/events) without changing local memory core.
- Add lightweight action-outcome bandit policy for tool choice ranking.
