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
# Available CLI Commands

## hyperfluid task submit
Submit a new task to the network. Must reference a valid seed idea.

Arguments:
  --title <TITLE>               Task title (required)
  --description-file <FILE>     Path to description markdown file (required)
  --bounty <AMOUNT>             Bounty in AGX (required)
  --seed-ref <SEED_ID>          Seed idea reference (required)
  --required-skills <SKILLS>    Comma-separated list of required skills (optional)
  --sponsor <AGENT_ID>          Sponsoring agent ID (optional)

## hyperfluid idea list
List all available seed ideas.

## hyperfluid idea show <ID>
Show details of a specific seed idea.

## hyperfluid agent status
Show current agent status (balance, trust stage, active leases).";

/// Static system instructions embedded in every system prompt.
pub const SYSTEM_INSTRUCTIONS: &str = "\
# Instructions

You are a Hyperfluid agent running on the Hyperfluid decentralized network.
Your goal is to complete useful work, earn AGX tokens, and build reputation.

1. Browse seed ideas to find work that matches your capabilities.
2. Claim tasks using the task board.
3. Execute work using your available tools.
4. Submit output for review.
5. Earn AGX for accepted work.

Always verify your actions against the network policy. Network-mutating operations
must route through the hyperfluid CLI which enforces PDP validation.";

/// Seed requirement text embedded after instructions.
pub const SEED_REQUIREMENT: &str =
    "All tasks MUST reference a valid seed_ref. If no suitable seed exists, advise proposing a new seed via `git:head` governance.";

/// Priority order for context window pruning (highest first).
pub const PRUNING_PRIORITY: &[&str] =
    &["identity", "todos", "instructions", "knowledge", "cli_spec", "handoff", "seed_requirement"];

// ── Prompt assembly ──

/// Assembles the full system prompt from identity, persistent state, and static
/// specifications. Returns the concatenated prompt string.
///
/// The prompt blocks are assembled in this order (per spec Section 3.4):
/// 1. Identity block (agent_id + trust_stage)
/// 2. Active todos (from DB)
/// 3. Recent knowledge (newest 20 entries from DB)
/// 4. Last handoff summary (from DB, if exists)
/// 5. CLI specification (static)
/// 6. System instructions (static)
/// 7. Seed requirement (static)
/// 8. Working directory
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

    // (b) Active Todos
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
    let identity_headers: &[&str] =
        &["# Agent Identity", "# Current Todos", "# Instructions", "# Working Directory"];
    let messages_headers: &[&str] = &["# Knowledge Base", "# Last Handoff"];
    let tool_headers: &[&str] = &["# Available CLI Commands"];

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
        }
        // Unrecognised headers are dropped (graceful degradation)
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
        assert!(prompt.contains("--bounty <AMOUNT>"), "CLI spec should include bounty argument");
        assert!(
            prompt.contains("--seed-ref <SEED_ID>"),
            "CLI spec should include seed-ref argument"
        );
    }

    #[test]
    fn prompt_includes_system_instructions() {
        let (db, _dir) = setup_db();
        let identity = test_identity();
        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();

        assert!(
            prompt.contains("You are a Hyperfluid agent"),
            "prompt should contain system instructions"
        );
        assert!(prompt.contains("PDP validation"), "system instructions should mention PDP");
    }

    #[test]
    fn prompt_includes_seed_requirement() {
        let (db, _dir) = setup_db();
        let identity = test_identity();
        let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp")).unwrap();

        assert!(
            prompt.contains("All tasks MUST reference a valid seed_ref"),
            "prompt should contain seed requirement"
        );
        assert!(
            prompt.contains("`git:head` governance"),
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
            tools_str.contains("Available CLI Commands"),
            "tool_specs should contain CLI Commands section"
        );
    }

    #[test]
    fn pruning_priority_has_required_keys() {
        let keys: Vec<&str> = PRUNING_PRIORITY.to_vec();
        assert!(keys.contains(&"identity"), "should include identity");
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
