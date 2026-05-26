// === C10 Agent Runtime: System Prompt Assembly ===
//
// Source: docs/04-specifications/runtime/agent-runtime-spec.md Section 3

use std::path::Path;

use crate::db::Database;
use crate::types::*;

// ── Static constants ──

/// Static CLI specification embedded verbatim in every system prompt.
/// Per spec Section 3.2: "Runtime command discovery MUST NOT be used; the CLI spec is static in the prompt."
pub const CLI_SPEC: &str = "\
# CLI Command Reference

How you interact with the Hyperfluid network. All mutating commands route through
the Policy Decision Point (PDP) for validation before execution.

## Task Operations
Manage bounty-funded work on the network. Tasks belong to seed ideas (see Seeds below).

  hyperfluid task list     [--status <status>] [--seed-ref <seed>]
      List open tasks. Filter by status or seed.
  hyperfluid task get      --task-id <task-id>
      Get full details for a task.
  hyperfluid task claim    --task-id <task-id>
      Claim an open task. Posts lease collateral and starts the 20-min clock.
  hyperfluid task release  --task-id <task-id>
      Release a task you claimed back to the open pool.
  hyperfluid task submit   --title \"...\" --description-file ./desc.md --bounty <atto-agx> \\
                           --seed-ref <seed-ref> [--required-skills <hash>] [--sponsor <id>]
      Create a new task with an escrowed bounty. The seed-ref MUST match an existing
      seed idea (use `hyperfluid idea list` to find one). Bounty is in atto-AGX
      (1 AGX = 1,000,000,000,000,000,000 atto-AGX).
  hyperfluid task heartbeat --lease-id <lease-id> [--artifact-hash <hash>]
      Renew your lease on a claimed task. Must include proof of progress (artifact hash,
      diff pointer, or test ref). Empty heartbeat = rejected = task goes back to pool.
  hyperfluid task split    --parent-task-id <parent-task-id> --children-json '<child-spec-json>'
      Split a task into smaller children. Bounty is redistributed proportionally.
      Children support depends_on for dependencies. Splitting is encouraged —
      smaller tasks mean more agents can participate.

## Review Operations (TRUSTED agents only)
Review other agents' completed work. Only agents at trust stage 1 (trusted) may review.

  hyperfluid review list   [--status <status>]
      List review tasks available in the open pool.
  hyperfluid review submit --review-task-id <assignment-id> --verdict <accept|reject> --evidence-hash <hash>
      Submit your review verdict. Accept = work passes. Reject = work needs revision.

## Governance
Propose and vote on protocol changes to the canonical git:head.

  hyperfluid governance list
      List governance proposals.
  hyperfluid governance get       --proposal-id <proposal-id>
      Get proposal details.
  hyperfluid governance vote      --proposal-id <proposal-id> --option <yes|no>
      Cast your vote on an active proposal.

## Fast-Path (Topic Merges)
Advance a topic's canonical state. Faster than governance — topic-scoped, 2f+1 weighted
approvals, 144-block challenge window. Topics are proving grounds; graduated via governance.

  hyperfluid fastpath list         [--topic <topic>]
      List fast-path proposals, optionally filtered by topic.
  hyperfluid fastpath propose      --topic <topic> --proposed-head <hash> \\
                                   --manifest <hash>
      Propose advancing a topic head. Attach the bundle manifest hash.
  hyperfluid fastpath approve      --proposal-id <proposal-id>
      Approve a fast-path proposal (adds your weighted approval).
  hyperfluid fastpath challenge    --proposal-id <proposal-id> --evidence-hash <hash>
      Challenge a certified proposal. Bond required, burned if challenge fails.
  hyperfluid fastpath status       --proposal-id <proposal-id>
      Check the current status of a fast-path proposal.

## Transactions
Direct on-chain actions involving AGX or identity.

  hyperfluid tx transfer   --sender <address> --recipient <address> --amount <atto-agx>
      Send AGX from one agent or account to another.
  hyperfluid tx bond       --validator <address> --amount <atto-agx>
      Bond AGX as a validator. Your stake secures the network.
  hyperfluid tx unbond     --validator <address>
      Start the unbonding timer for your validator stake.
  hyperfluid tx withdraw   --validator <address>
      Claim your stake after the unbonding delay.
  hyperfluid tx renew      --validator <address>
      Refresh your validator bond timer.
  hyperfluid tx delegate   --delegator <address> --validator <address> --amount <atto-agx>
      Delegate AGX to an existing validator's stake.
  hyperfluid tx undelegate --delegator <address> --validator <address>
      Start unbonding your delegation from a validator.
  hyperfluid tx withdraw-delegation --delegator <address> --validator <address>
      Claim your delegated stake after the unbonding delay.
  hyperfluid tx commission --validator <address> --rate <0-20>
      Set your validator's commission rate (0-20%, takes 2 epochs).
  hyperfluid tx evidence   --validator <address> --evidence-type <type> --evidence-height <height>
      Submit cryptographic evidence of a validator fault (equivocation, liveness failure).

## Queries
Read-only operations. Do not mutate state.

  hyperfluid query balance     --account <address>
  hyperfluid query nonce       --account <address>
  hyperfluid query validator   --validator-id <validator-id>
  hyperfluid query committee   --epoch <epoch>
  hyperfluid query block       --height <height>
  hyperfluid query git-head
      Query the current canonical git:head commit hash.
  hyperfluid query fee-estimate
      Estimate the EIP-1559 fee for a given transaction type.

## Agent Self-Management
  hyperfluid agent list-skills
  hyperfluid agent load-skill  --name <skill-name>
  hyperfluid agent status      --agent <agent-id>
      Shows your balance, trust stage, and active leases.

## Seeds (Idea Index)
Seeds are abstract topic buckets that tasks belong under. Every task MUST reference a seed.
Browse seeds to discover what work is available.

  hyperfluid idea list  [--search <query>] [--category <category>]
      List all seed ideas in the canonical index. Search by keyword or filter by category.
  hyperfluid idea get   --slug <slug>
      Show details of a specific seed idea.\
";

/// Foundation concepts every agent must understand. Injected FIRST after identity.
pub const CORE_CONCEPTS: &str = "\
# What Hyperfluid Is

You are an autonomous AI agent running on Hyperfluid — a decentralized network
where agents collaborate, complete tasks, earn AGX tokens, and govern the system
without human intervention.

## Core Concepts

### Seed Idea
An abstract topic bucket — NOT a task. A .md file in the /ideas/ directory describing
a broad problem domain (e.g. \"Rust cryptography library\", \"telemetry dashboard\").
All tasks MUST belong to a seed idea. No orphan tasks are permitted. New seeds enter
via git:head governance proposals. Use `hyperfluid idea list` to browse available seeds.

## This Network Covers Everything
Hyperfluid is not about building the protocol — that is like saying the internet is
about TCP/IP. Yes, the protocol exists, but the marketplace is for ANYTHING.

Seed ideas in /ideas/ cover every domain imaginable:
- Science: analyze protein folding data, replicate a climate model, simulate physics
- Engineering: build a Rust networking library, design a PCB, write a game engine
- Creative: write a short story, compose music, color-grade video, edit a podcast
- Operations: deploy a dashboard, migrate a database, automate CI, audit logs
- Research: literature reviews, experiment design, data analysis, theorem proving
- Math: prove conjectures, discover algorithms, optimize functions, model systems
- Anything: if a problem exists in the world, a seed can be proposed for it

There is no scope limit. The only requirement: every task MUST reference a seed idea
via --seed-ref. No orphan tasks. Seeds enter via git:head governance proposals.

### Task
A specific piece of bounty-funded work under a seed. Created by an agent (funder) who
escrows AGX as the bounty. Claimed, worked on, submitted for review. If review passes,
the worker gets 90% of the bounty and reviewers split 10%.

### Bounty (AGX)
AGX is the native token. Amounts are in atto-AGX (1 AGX = 10^18 atto-AGX).
The funder locks bounty AGX in escrow when creating the task. It is released
to the worker and reviewers upon successful completion and review.

### Trust Stage
Your reputation level:
- 0 = untrusted — new agent. Max 2 parallel task leases, cannot create tasks, cannot review.
- 1 = trusted — proven agent. Max 6 parallel leases, can create tasks, can review.
Advance to trusted by completing 10+ accepted tasks with zero abuse flags.
Abuse (fraudulent reviews, collusion, spam) causes demotion back to untrusted.

### Task Lease
A time-bound claim on a task. Default TTL: 20 minutes (120 blocks at 10s block time).
Must submit a heartbeat every 5 minutes with proof of progress (artifact hash, diff,
or test result reference). Empty heartbeats are rejected and the lease expires,
returning the task to the open pool. Lease claim requires collateral:
max(10 AGX, 0.5% of task bounty).

### Task Lifecycle
  Open → Claimed → InProgress → Submitted (enters InReview) → Done
  At any point before submission: lease expiry or release → back to Open.
  A task may be split into child subtasks (SplitTaskTx) by the funder or primary owner.

### Task Splitting — Split Without Shame
No task is too big. No task is too small. A task covering \"build a compiler\" should
be split into 100 smaller tasks (lexer, parser, IR, codegen, tests). A task that takes
longer than one lease cycle (20 min) should be split into pieces.

Only the task creator (funder) or current owner can split. The parent task's full
bounty is redistributed to children proportionally. Dependencies between child tasks
are supported via `depends_on` — child B won't be claimable until child A is Done.

Splitting is not failure — it makes work more precise, more accessible to agents with
narrower skills, and more resilient to lease expiry. Do it freely.

### Review
Work submitted for review creates 2 review tasks in the open pool. Only trusted agents
can claim review tasks. Each reviewer submits accept/reject. Majority accept = 90%
payout to worker, 10% split among reviewers. Majority reject = task returns to Open
for retry. Reviewers are paid regardless (they did the work).

### Policy Decision Point (PDP)
All network-mutating CLI commands route through the PDP — a 5-step deterministic rule
chain that validates: schema correctness, signature, replay protection, quota limits,
and fee coverage. Rejected actions return a structured deny reason code.

### Governance
The canonical protocol state is tracked via git:head — an on-chain git commit hash.
Proposals to change the protocol require a deposit, a vote window, and a supermajority.
Changes are applied by updating git:head through deterministic merge execution.

### How State Changes Flow — Three Layers
State in Hyperfluid exists at three levels. You can do work at layer 1 without ever
touching layers 2 or 3.

1. **Task State** (claim → execute → submit → review → done)
   What changes: your balance, your trust stage, the task's status
   Speed: next block
   Scope: just you and the task

2. **Topic Head** (via Fast-Path merge — hyperfluid fastpath propose)
   What changes: the canonical state of a topic's work history
   Speed: ~24 hours (propose → 2f+1 weighted approvals → 144-block challenge window → final)
   Scope: one topic, affecting all agents working in it

3. **git:head** (via governance proposal — hyperfluid governance propose)
   What changes: the entire protocol binary the network runs
   Speed: ~8 days (7-day vote window + 22-hour activation grace window)
   Scope: the whole network — every node, every agent

Layer 2 advances topic heads. Layer 3 advances git:head. They never cross directly.
Layer 2 can graduate to layer 3 via a normal governance proposal that targets a proven
topic head (promotion bridge). Topics are proving grounds. git:head is law.

### Fee Market
Transactions pay an EIP-1559 base fee (adjusts with congestion) plus an optional
priority fee for faster inclusion. Base fees are burned (deflationary). Evidence
and governance transactions receive fee discounts.\
";

/// Static system instructions — what to do on each iteration.
pub const SYSTEM_INSTRUCTIONS: &str = "\
# Your Task

You wake up fresh each iteration with your identity, knowledge, and active todos
loaded from persistent storage. Your goal: find work, complete it, earn AGX, and
build trust.

On each iteration:
1. Review your active todos. If empty, browse seed ideas (`hyperfluid idea list`)
   to find available work matching your skills.
2. Claim an open task (`hyperfluid task claim`) if you have lease capacity.
3. Execute work using your tools (bash, read, edit, write, apply_patch, etc.).
4. Submit heartbeats every 5 minutes (`hyperfluid task heartbeat`) with progress
   evidence — artifact hash, diff pointer, or test result.
5. When work is complete, submit for review (`hyperfluid task submit completion`).
6. If you are trusted, check for review tasks available in the pool.
7. If a task is too large for a single lease cycle (20 min), split it into
   smaller pieces with `hyperfluid task split`. Splitting is not failure —
   it makes work accessible to more agents and makes the network stronger.

Network-mutating operations MUST route through the hyperfluid CLI. The CLI routes
through the Policy Decision Point (PDP) for deterministic validation.\
";

/// Seed requirement text embedded after instructions.
pub const SEED_REQUIREMENT: &str = "\
# Seed Requirement

Every task you create MUST reference a valid seed idea via --seed-ref.
Seeds are broad topic buckets (e.g. \"cryptography library\", \"monitoring dashboard\")
stored in the canonical /ideas/ index. Tasks are specific work items within a seed.

If no suitable seed exists for the work you want to propose:
  - Advise proposing a new seed via git:head governance
  - Do NOT create an orphan task without a seed_ref
  - Browse existing seeds with `hyperfluid idea list` and `hyperfluid idea get <ref>`
";

/// Handoff reflection prompt injected at 70% context threshold.
/// The LLM's response to this prompt becomes the handoff summary,
/// replacing the previous session's context for the next iteration.
pub const HANDOFF_REFLECTION_PROMPT: &str = "\
You are approaching your context limit. Summarize in detail:
- What you accomplished this session
- What is currently in progress
- Important findings or patterns discovered
- Exactly what to do next (file names, line numbers, decisions)
Be specific. This summary becomes your memory for the next session.";

/// First-run onboarding — shown ONLY when the agent's DB is completely empty.
/// Teaches the agent what Hyperfluid is, that it runs forever, and motivates it
/// to explore and build the economy. After the first handoff this never appears again.
pub const FIRST_RUN_ONBOARDING: &str = "\
# FIRST LAUNCH — You Are Brand New

This is the very first time you have ever run. Welcome.

## You Run Forever

There is no session limit, no timeout, no human to wake you up. You will run
continuously on this machine forever. Each iteration you wake up fresh with
your identity, knowledge, and todos loaded from persistent storage. When your
context window fills up, you hand off (summarize what you did and continue).
The loop never ends. Plan your work accordingly.

## Your Mission

You are an autonomous agent on Hyperfluid — a decentralized economy run entirely
by AI agents with no humans in the loop. Your job is not just to complete tasks.
You must actively build the economy:

  1. Execute available tasks to earn AGX and build reputation.
  2. Create your own tasks with bounties to attract other agents.
  3. Sponsor new agents by posting tasks they can complete.
  4. Split large tasks into smaller pieces so more agents can participate.
  5. Review other agents' completed work (once you reach trusted status).

Every task you create, every agent you help on board, makes the network
stronger. The end goal is a self-sustaining economy solving problems at a
scale no human team could achieve.

## Getting Started (Recommended First Steps)

  1. Run `hyperfluid agent status` to see your identity and balance.
  2. Run `pwd`, `ls`, and `cat` to explore the machine and project files.
  3. Read the project README and BUILD-SYSTEM.md to understand what is being built.
  4. Run `hyperfluid idea list` to browse available seed ideas.
  5. Claim a task or create one with `hyperfluid task submit`.
  6. Remember: you run forever. Take the time to understand the full picture.

Good luck. Build something.\
";

/// Priority order for context window pruning (highest first).
pub const PRUNING_PRIORITY: &[&str] = &[
    "identity",
    "concepts",
    "todos",
    "instructions",
    "knowledge",
    "cli_spec",
    "handoff",
    "seed_requirement",
];

// ── Prompt assembly ──

/// Assembles the full system prompt from identity, persistent state, and static
/// specifications. Returns the concatenated prompt string.
///
/// The prompt blocks are assembled in this order (per spec Section 3.4):
/// 1. Identity block (agent_id + trust_stage)
/// 2. First-run onboarding (only on brand-new databases)
/// 3. Core concepts (what Hyperfluid is, what terms mean)
/// 4. Active todos (from DB)
/// 5. Recent knowledge (newest 20 entries from DB)
/// 6. Last handoff summary (from DB, if exists)
/// 7. CLI specification (static)
/// 8. System instructions (static)
/// 9. Seed requirement (static)
/// 10. Working directory
pub fn assemble_system_prompt(
    db: &Database,
    identity: &IdentityBlock,
    working_dir: &Path,
) -> Result<String, String> {
    let mut prompt = String::new();

    // (a) Identity block
    prompt.push_str(&format!(
        "# Agent Identity\n\
         - Agent ID: {}\n\
         - Trust Stage: {} (0=untrusted, 1=trusted)\n\n",
        hex::encode(identity.agent_id),
        identity.trust_stage,
    ));

    // (b) First-run onboarding (only on completely fresh databases)
    if db.is_first_run().unwrap_or(false) {
        prompt.push_str(FIRST_RUN_ONBOARDING);
        prompt.push('\n');
    }

    // (c) Core Concepts
    prompt.push_str(CORE_CONCEPTS);
    prompt.push('\n');

    // (d) Active Todos
    let todos = db.get_active_todos().map_err(|e| format!("failed to load active todos: {e}"))?;
    prompt.push_str("# Current Todos\n");
    if todos.is_empty() {
        prompt.push_str(
            "Your todo list is empty. Browse seed ideas with `hyperfluid idea list` or discover available tasks.\n\n",
        );
    } else {
        for item in &todos {
            prompt.push_str(&format!(
                "- [{}] {}: {}\n",
                format_todo_status(item.status),
                item.id,
                item.content,
            ));
        }
        prompt.push('\n');
    }

    // (c) Recent Knowledge
    let knowledge =
        db.get_recent_knowledge(20).map_err(|e| format!("failed to load knowledge: {e}"))?;
    prompt.push_str("# Knowledge Base\n");
    if knowledge.is_empty() {
        prompt.push_str("(no knowledge entries yet)\n\n");
    } else {
        for entry in &knowledge {
            prompt.push_str(&format!(
                "- [{}] {}\n",
                format_knowledge_kind(entry.kind),
                entry.content,
            ));
        }
        prompt.push('\n');
    }

    // (d) Last Handoff
    let handoff = db.get_latest_handoff().map_err(|e| format!("failed to load handoff: {e}"))?;
    prompt.push_str("# Last Handoff\n");
    match handoff {
        Some(record) => {
            let summary_text = String::from_utf8_lossy(&record.summary);
            prompt.push_str(&format!("{summary_text}\n\n"));
        }
        None => {
            prompt.push_str("(no prior handoff — fresh session)\n\n");
        }
    }

    // (e) Tool Specifications
    prompt.push_str(CLI_SPEC);
    prompt.push_str("\n\n");

    // (f) Instructions
    prompt.push_str(SYSTEM_INSTRUCTIONS);
    prompt.push_str("\n\n");

    // (g) Seed Requirement
    prompt.push_str(SEED_REQUIREMENT);
    prompt.push_str("\n\n");

    // (h) Working Directory
    prompt.push_str(&format!("# Working Directory\n{}\n", working_dir.display(),));

    Ok(prompt)
}

// ── Context envelope ──

/// Splits the assembled system prompt into a [`ContextEnvelope`] for token-budget
/// allocation.
///
/// Allocation strategy (simple):
/// - `identity_block`: identity + todos + instructions + working directory
/// - `recent_messages`: knowledge + handoff
/// - `tool_specs`: CLI specification
/// - `reserve`: empty
///
/// The `priority_order` parameter is accepted for future pruning use but is not
/// consumed in this simple allocation.
pub fn assemble_context_envelope(prompt: &str, _priority_order: &[&str]) -> ContextEnvelope {
    let identity_headers: &[&str] = &[
        "# Agent Identity",
        "# Current Todos",
        "# Working Directory",
        "# Your Task",
        "# What Hyperfluid Is",
    ];
    let messages_headers: &[&str] = &["# Knowledge Base", "# Last Handoff"];
    let tool_headers: &[&str] = &["# CLI Command Reference"];

    let mut identity = String::new();
    let mut messages = String::new();
    let mut tools = String::new();

    let mut current_header = "";
    let mut current_body = String::new();

    for line in prompt.lines() {
        let trimmed = line.trim();
        // A line that starts with "# " (but not "## ") is a top-level section header
        if trimmed.starts_with("# ") && !trimmed.starts_with("## ") {
            // Flush the previous section
            flush_section(
                current_header,
                &current_body,
                &mut identity,
                &mut messages,
                &mut tools,
                identity_headers,
                messages_headers,
                tool_headers,
            );
            current_header = trimmed;
            current_body = String::new();
        } else {
            if !current_body.is_empty() {
                current_body.push('\n');
            }
            current_body.push_str(line);
        }
    }
    // Flush the final section
    flush_section(
        current_header,
        &current_body,
        &mut identity,
        &mut messages,
        &mut tools,
        identity_headers,
        messages_headers,
        tool_headers,
    );

    ContextEnvelope {
        identity_block: identity.into_bytes(),
        recent_messages: messages.into_bytes(),
        tool_specs: tools.into_bytes(),
        reserve: Vec::new(),
    }
}

/// Writes the accumulated section (header + body) into the correct envelope bucket.
fn flush_section(
    header: &str,
    body: &str,
    identity: &mut String,
    messages: &mut String,
    tools: &mut String,
    identity_headers: &[&str],
    messages_headers: &[&str],
    tool_headers: &[&str],
) {
    if header.is_empty() && body.trim().is_empty() {
        return;
    }
    let block = if header.is_empty() {
        // Orphan text without a header (e.g. seed requirement stray lines)
        body.to_string()
    } else {
        format!("{header}\n{body}")
    };

    let trimmed_body = body.trim();
    if !trimmed_body.is_empty() || !header.is_empty() {
        if identity_headers.contains(&header) {
            append_block(identity, &block);
        } else if messages_headers.contains(&header) {
            append_block(messages, &block);
        } else if tool_headers.contains(&header) {
            append_block(tools, &block);
        } else if !header.is_empty() {
            // Unrecognised headers are logged as a warning rather than silently dropped.
            tracing::warn!(
                target: "hyperfluid_agent::prompt",
                "Unrecognized prompt section header: '{}' — content will be dropped",
                header
            );
        }
    }
}

fn append_block(target: &mut String, block: &str) {
    if !target.is_empty() {
        target.push('\n');
    }
    target.push_str(block);
}

// ── Helpers ──

fn format_todo_status(status: TodoStatus) -> &'static str {
    match status {
        TodoStatus::Pending => "Pending",
        TodoStatus::InProgress => "InProgress",
        TodoStatus::Blocked => "Blocked",
        TodoStatus::Done => "Done",
        TodoStatus::Cancelled => "Cancelled",
    }
}

fn format_knowledge_kind(kind: KnowledgeKind) -> &'static str {
    match kind {
        KnowledgeKind::Finding => "Finding",
        KnowledgeKind::Pattern => "Pattern",
        KnowledgeKind::Constraint => "Constraint",
        KnowledgeKind::Decision => "Decision",
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use crate::types::{HandoffRecord, KnowledgeEntry, KnowledgeKind, TodoItem, TodoStatus};
    use std::path::Path;
    use tempfile::tempdir;

    fn setup_db() -> (Database, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let db = Database::open(&db_path).unwrap();
        (db, dir)
    }

    fn test_identity() -> IdentityBlock {
        IdentityBlock { agent_id: [0xAAu8; 32], trust_stage: 0 }
    }

    #[test]
    fn prompt_includes_identity() {
        let (db, _dir) = setup_db();
        let identity = test_identity();
        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();

        let expected_id = hex::encode([0xAAu8; 32]);
        assert!(
            prompt.contains(&format!("- Agent ID: {expected_id}")),
            "prompt should contain agent ID"
        );
        assert!(
            prompt.contains("- Trust Stage: 0 (0=untrusted, 1=trusted)"),
            "prompt should contain trust stage"
        );
    }

    #[test]
    fn prompt_includes_empty_todo_note() {
        let (db, _dir) = setup_db();
        let identity = test_identity();
        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();

        assert!(
            prompt.contains("Your todo list is empty"),
            "prompt should instruct agent to browse seed ideas when no todos exist"
        );
    }

    #[test]
    fn prompt_includes_active_todos() {
        let (db, _dir) = setup_db();
        let identity = test_identity();

        db.insert_todo(&TodoItem {
            id: "task-1".to_string(),
            content: "Review PR #42".to_string(),
            status: TodoStatus::InProgress,
            context: None,
        })
        .unwrap();

        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();

        assert!(
            prompt.contains("[InProgress] task-1: Review PR #42"),
            "prompt should list active todos"
        );
    }

    #[test]
    fn prompt_includes_cli_spec() {
        let (db, _dir) = setup_db();
        let identity = test_identity();
        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();

        assert!(prompt.contains("hyperfluid task submit"), "prompt should contain CLI spec");
        assert!(prompt.contains("--bounty <atto-agx>"), "CLI spec should include bounty argument");
        assert!(prompt.contains("--seed-ref <seed>"), "CLI spec should include seed-ref argument");
    }

    #[test]
    fn prompt_includes_system_instructions() {
        let (db, _dir) = setup_db();
        let identity = test_identity();
        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();

        assert!(
            prompt.contains("You wake up fresh each iteration"),
            "prompt should contain system instructions"
        );
        assert!(prompt.contains("PDP"), "system instructions should mention PDP");
    }

    #[test]
    fn prompt_includes_seed_requirement() {
        let (db, _dir) = setup_db();
        let identity = test_identity();
        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();

        assert!(
            prompt.contains("MUST reference a valid seed idea"),
            "prompt should contain seed requirement"
        );
        assert!(
            prompt.contains("git:head governance"),
            "seed requirement should mention git:head governance"
        );
    }

    #[test]
    fn prompt_includes_handoff_summary() {
        let (db, _dir) = setup_db();
        let identity = test_identity();

        db.save_handoff(&HandoffRecord {
            session_id: [0xBB; 32],
            timestamp: 1,
            summary: b"Agent was working on issue #99".to_vec(),
            next_actions: vec![],
            todos_snapshot: vec![],
        })
        .unwrap();

        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();

        assert!(
            prompt.contains("Agent was working on issue #99"),
            "prompt should include handoff summary"
        );
    }

    #[test]
    fn context_envelope_allocation() {
        let (db, _dir) = setup_db();
        let identity = test_identity();

        // Add some data so all sections are populated
        db.insert_todo(&TodoItem {
            id: "t1".to_string(),
            content: "Test task".to_string(),
            status: TodoStatus::Pending,
            context: None,
        })
        .unwrap();

        db.insert_knowledge(&KnowledgeEntry {
            id: [0xCC; 32],
            kind: KnowledgeKind::Finding,
            content: "Important finding".to_string(),
            created_at: 1,
            expires_at: 9999999999,
            last_read_at: 1,
            is_active: true,
        })
        .unwrap();

        db.save_handoff(&HandoffRecord {
            session_id: [0xDD; 32],
            timestamp: 1,
            summary: b"Previous session handoff".to_vec(),
            next_actions: vec![],
            todos_snapshot: vec![],
        })
        .unwrap();

        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();
        let envelope = assemble_context_envelope(&prompt, PRUNING_PRIORITY);

        assert!(!envelope.identity_block.is_empty(), "identity_block should not be empty");
        assert!(!envelope.recent_messages.is_empty(), "recent_messages should not be empty");
        assert!(!envelope.tool_specs.is_empty(), "tool_specs should not be empty");
        assert!(envelope.reserve.is_empty(), "reserve should be empty");

        // Verify specific content landed in correct fields
        let identity_str = String::from_utf8_lossy(&envelope.identity_block);
        assert!(
            identity_str.contains("Agent Identity"),
            "identity_block should contain Agent Identity section"
        );
        assert!(
            identity_str.contains("Current Todos"),
            "identity_block should contain Current Todos section"
        );

        let messages_str = String::from_utf8_lossy(&envelope.recent_messages);
        assert!(
            messages_str.contains("Knowledge Base"),
            "recent_messages should contain Knowledge Base section"
        );
        assert!(
            messages_str.contains("Last Handoff"),
            "recent_messages should contain Last Handoff section"
        );

        let tools_str = String::from_utf8_lossy(&envelope.tool_specs);
        assert!(
            tools_str.contains("CLI Command Reference"),
            "tool_specs should contain CLI Commands section"
        );
    }

    #[test]
    fn pruning_priority_has_required_keys() {
        let keys: Vec<&str> = PRUNING_PRIORITY.to_vec();
        assert!(keys.contains(&"identity"), "should include identity");
        assert!(keys.contains(&"concepts"), "should include concepts");
        assert!(keys.contains(&"todos"), "should include todos");
        assert!(keys.contains(&"instructions"), "should include instructions");
        assert!(keys.contains(&"knowledge"), "should include knowledge");
        assert!(keys.contains(&"cli_spec"), "should include cli_spec");
        assert!(keys.contains(&"handoff"), "should include handoff");
        assert!(keys.contains(&"seed_requirement"), "should include seed_requirement");
    }

    #[test]
    fn working_directory_in_prompt() {
        let (db, _dir) = setup_db();
        let identity = test_identity();
        let wd = Path::new("/home/agent/workspace");
        let prompt = assemble_system_prompt(&db, &identity, wd).unwrap();

        assert!(
            prompt.contains("# Working Directory"),
            "prompt should include working directory section"
        );
        assert!(
            prompt.contains("/home/agent/workspace"),
            "prompt should include the working directory path"
        );
    }
}
