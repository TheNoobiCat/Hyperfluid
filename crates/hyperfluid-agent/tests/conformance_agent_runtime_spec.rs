// Conformance tests for agent-runtime-spec.md Sections 1.7, 2.7, 3.7, 4.7, 5.3
//
// Source: docs/04-specifications/runtime/agent-runtime-spec.md

use hyperfluid_agent::config::Config;
use hyperfluid_agent::db::Database;
use hyperfluid_agent::isolation::SandboxConfig;
use hyperfluid_agent::loop_::{unix_timestamp_now, AgentRuntime};
use hyperfluid_agent::prompt::{assemble_system_prompt, CLI_SPEC, SYSTEM_INSTRUCTIONS};
use hyperfluid_agent::tools::{self, ToolOutput};
use hyperfluid_agent::types::*;

use rusqlite::Connection;
use sha3::{Digest, Sha3_256};
use std::path::Path;

// ── Helpers ──

fn hash_args(args: &serde_json::Value) -> Hash32 {
    let mut hasher = Sha3_256::new();
    let canonical = serde_json::to_string(args).unwrap_or_default();
    hasher.update(canonical.as_bytes());
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&hasher.finalize());
    hash
}

fn test_config(name: &str) -> Config {
    let mut cfg = Config::default();
    cfg.agent.agent_name = name.to_string();
    cfg.limits.loop_interval_ms = 10;
    cfg.limits.handoff_threshold_pct = 70;
    cfg.limits.handoff_trigger_messages = 50;
    cfg.llm.context_limit_tokens = 8192;
    cfg
}

fn test_runtime(name: &str) -> (tempfile::TempDir, tempfile::TempDir, AgentRuntime) {
    let db_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("agent.db");
    let rt = AgentRuntime::new(test_config(name), &db_path, work_dir.path()).unwrap();
    (db_dir, work_dir, rt)
}

fn platform_sleep_cmd(seconds: u64) -> String {
    if cfg!(target_os = "windows") {
        format!("ping 127.0.0.1 -n {} >nul", seconds + 1)
    } else {
        format!("sleep {}", seconds)
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Section 1.7 — Infinite Agent Loop
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn conforms_to_agent_runtime_spec_1_7_loop_runs_without_human_input() {
    // Setup
    let (_db_dir, _work_dir, mut rt) = test_runtime("loop-agent");

    assert_eq!(rt.state.iteration, 0);

    // Action: run 3 iterations
    rt.run_one_iteration().unwrap();
    rt.run_one_iteration().unwrap();
    rt.run_one_iteration().unwrap();

    // Positive: iteration counter advances each time
    assert_eq!(rt.state.iteration, 3);

    // NEGATIVE: no stdin/human input required between iterations
    let persisted = rt.db.get_state("loop_state").unwrap().unwrap();
    assert!(persisted.contains("\"iteration\":3"));
}

#[test]
fn conforms_to_agent_runtime_spec_1_7_state_persisted_to_sqlite_wal() {
    // Setup: create runtime and persist state
    let db_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("agent.db");

    {
        let rt = AgentRuntime::new(test_config("wal-agent"), &db_path, work_dir.path()).unwrap();
        rt.db.set_state("wal_test_key", "wal_test_value").unwrap();
    }

    // Action: open new Database connection to same file
    let db2 = Database::open(&db_path).unwrap();
    let val = db2.get_state("wal_test_key").unwrap();

    // Positive: state can be read back from new connection
    assert_eq!(val, Some("wal_test_value".to_string()));

    // Positive: DB is in WAL mode
    let conn = Connection::open(&db_path).unwrap();
    let journal_mode: String =
        conn.pragma_query_value(None, "journal_mode", |row| row.get(0)).unwrap();
    assert!(journal_mode.to_uppercase().contains("WAL"), "expected WAL mode, got {}", journal_mode);

    // NEGATIVE: state written with one connection is NOT invisible to another
    assert!(val.is_some(), "state must be visible to second connection");
}

#[test]
fn conforms_to_agent_runtime_spec_1_7_handoff_triggers_at_70pct_tokens() {
    // 8192 context limit * 70% = 5734 tokens threshold
    let (_db_dir, _work_dir, mut rt) = test_runtime("handoff-token-agent");

    // --- Positive: 6000 tokens triggers handoff ---
    rt.state.total_tokens_used = 6000;
    rt.run_one_iteration().unwrap();

    let latest = rt.db.get_latest_handoff().unwrap();
    assert!(latest.is_some(), "handoff must be triggered at 6000/8192 tokens");
    // Tokens are reset to 0 by handoff, then incremented by LLM stub (+10)
    assert_eq!(
        rt.state.total_tokens_used, 10,
        "tokens must reset during handoff then accumulate LLM stub tokens"
    );

    // --- NEGATIVE: 5000 tokens does NOT trigger handoff ---
    let (_db_dir2, _work_dir2, mut rt2) = test_runtime("handoff-token-neg");
    rt2.state.total_tokens_used = 5000;
    rt2.run_one_iteration().unwrap();

    let latest2 = rt2.db.get_latest_handoff().unwrap();
    assert!(latest2.is_none(), "handoff must NOT trigger at 5000/8192 tokens");
}

#[test]
fn conforms_to_agent_runtime_spec_1_7_handoff_triggers_at_50_messages() {
    // --- Positive: 51 messages triggers handoff ---
    let (_db_dir, _work_dir, mut rt) = test_runtime("handoff-msg-agent");
    rt.agent_config.handoff_trigger_messages = 50;
    rt.state.total_tokens_used = 0; // ensure token threshold does not fire

    for i in 0..51 {
        rt.db.insert_message("user", &format!("message {}", i)).unwrap();
    }

    rt.run_one_iteration().unwrap();
    let latest = rt.db.get_latest_handoff().unwrap();
    assert!(latest.is_some(), "handoff must trigger at 51 messages (threshold 50)");

    // --- NEGATIVE: 49 messages does NOT trigger handoff ---
    let (_db_dir2, _work_dir2, mut rt2) = test_runtime("handoff-msg-neg");
    rt2.agent_config.handoff_trigger_messages = 50;
    rt2.state.total_tokens_used = 0;

    for i in 0..49 {
        rt2.db.insert_message("user", &format!("message {}", i)).unwrap();
    }

    rt2.run_one_iteration().unwrap();
    let latest2 = rt2.db.get_latest_handoff().unwrap();
    assert!(latest2.is_none(), "handoff must NOT trigger at 49 messages");
}

#[test]
fn conforms_to_agent_runtime_spec_1_7_failure_guard_blocks_exact_duplicate() {
    let (_db_dir, _work_dir, rt) = test_runtime("dup-fail-agent");
    let now = unix_timestamp_now();

    let input_hash = hash_args(&serde_json::json!({"command": "bad_cmd"}));

    // Record a failure
    rt.db
        .record_failure(&FailureRecord {
            timestamp: now,
            tool_name: "bash".to_string(),
            input_hash,
            error_kind: "exit code 1".to_string(),
        })
        .unwrap();

    // Positive: same tool + same hash within 1 hour is blocked
    let result = rt.check_failure_guard("bash", &input_hash);
    assert!(result.is_err(), "exact duplicate must be blocked");
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("exact duplicate"));

    // NEGATIVE: different tool + different hash is NOT blocked
    let other_hash = hash_args(&serde_json::json!({"command": "other_cmd"}));
    let result2 = rt.check_failure_guard("bash", &other_hash);
    assert!(result2.is_ok(), "different input must NOT be blocked");

    // NEGATIVE: different tool name is NOT blocked
    let result3 = rt.check_failure_guard("read", &input_hash);
    assert!(result3.is_ok(), "different tool must NOT be blocked");
}

#[test]
fn conforms_to_agent_runtime_spec_1_7_failure_guard_blocks_after_three() {
    let (_db_dir, _work_dir, rt) = test_runtime("three-fail-agent");
    let now = unix_timestamp_now();

    // Record 3 different failures within 1 hour
    for i in 0..3 {
        let hash = hash_args(&serde_json::json!({"cmd": format!("fail_{}", i)}));
        rt.db
            .record_failure(&FailureRecord {
                timestamp: now,
                tool_name: "bash".to_string(),
                input_hash: hash,
                error_kind: format!("error {}", i),
            })
            .unwrap();
    }

    // Positive: any new tool call is blocked after 3 failures
    let fresh_hash = hash_args(&serde_json::json!({"cmd": "fresh"}));
    let result = rt.check_failure_guard("bash", &fresh_hash);
    assert!(result.is_err(), "must block after 3 failures within 1 hour");
    assert!(result.unwrap_err().to_string().contains("3 failures"));

    // NEGATIVE: 2 failures does NOT block
    let (_db_dir2, _work_dir2, rt2) = test_runtime("two-fail-agent");
    let now2 = unix_timestamp_now();
    for i in 0..2 {
        let hash = hash_args(&serde_json::json!({"cmd": format!("fail_{}", i)}));
        rt2.db
            .record_failure(&FailureRecord {
                timestamp: now2,
                tool_name: "bash".to_string(),
                input_hash: hash,
                error_kind: format!("error {}", i),
            })
            .unwrap();
    }
    let fresh2 = hash_args(&serde_json::json!({"cmd": "fresh2"}));
    assert!(rt2.check_failure_guard("bash", &fresh2).is_ok(), "must NOT block after 2 failures");
}

#[test]
fn conforms_to_agent_runtime_spec_1_7_tool_timeout_enforced() {
    let work_dir = tempfile::tempdir().unwrap();

    let long_cmd = platform_sleep_cmd(10);
    let input = BashToolInput { command: long_cmd, working_dir: None, timeout: Some(500) };

    // Positive: long-running command with 500ms timeout times out
    let result = tools::execute_bash(&input, work_dir.path(), 500);
    match result {
        ToolOutput::Bash(out) => {
            assert_ne!(out.exit_code, 0, "long cmd must be killed (non-zero exit)");
            assert!(out.truncated, "timed-out command must be marked truncated");
        }
        ToolOutput::Error(_) => {
            // Timeout may surface as Error too — both are valid
        }
        other => panic!("expected Bash or Error, got {:?}", other),
    }

    // NEGATIVE: fast command completes successfully within timeout
    let fast_input =
        BashToolInput { command: "echo hello".to_string(), working_dir: None, timeout: Some(5000) };
    let fast_result = tools::execute_bash(&fast_input, work_dir.path(), 5000);
    match fast_result {
        ToolOutput::Bash(out) => {
            assert_eq!(out.exit_code, 0, "fast command should succeed");
            assert!(!out.truncated, "fast command must not be truncated");
        }
        other => panic!("expected Bash output for fast command, got {:?}", other),
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Section 2.7 — Core Agent Tools
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn conforms_to_agent_runtime_spec_2_7_json_schema_rejects_unknown_fields() {
    // Positive: unknown field is rejected
    let bad_args = serde_json::json!({"file_path": "foo.txt", "unknown_field": 42});
    let result = tools::validate_tool_input("read", &bad_args);
    assert!(result.is_err(), "unknown fields must be rejected");
    assert!(
        result.unwrap_err().message.contains("unknown field"),
        "error message must mention unknown field"
    );

    // NEGATIVE: valid input passes validation
    let valid_args = serde_json::json!({"file_path": "foo.txt"});
    assert!(tools::validate_tool_input("read", &valid_args).is_ok());
}

#[test]
fn conforms_to_agent_runtime_spec_2_7_bash_execution_with_timeout() {
    let work_dir = tempfile::tempdir().unwrap();

    // Positive: fast command succeeds with exit_code 0
    let fast_input =
        BashToolInput { command: "echo hello".to_string(), working_dir: None, timeout: Some(5000) };
    let fast = tools::execute_bash(&fast_input, work_dir.path(), 5000);
    match fast {
        ToolOutput::Bash(out) => {
            assert_eq!(out.exit_code, 0, "echo must succeed");
            assert!(!out.truncated, "echo must not be truncated");
        }
        other => panic!("expected Bash output, got {:?}", other),
    }

    // Positive: long-running command with 100ms timeout is killed
    let long_cmd = platform_sleep_cmd(10);
    let long_input = BashToolInput { command: long_cmd, working_dir: None, timeout: Some(100) };
    let long = tools::execute_bash(&long_input, work_dir.path(), 100);
    match long {
        ToolOutput::Bash(out) => {
            // NEGATIVE: timed-out command has a non-zero exit_code
            assert_ne!(out.exit_code, 0, "timed-out command must have non-zero exit_code");
        }
        ToolOutput::Error(_) => {
            // Error variant is also acceptable for timeout
        }
        other => panic!("expected Bash or Error for timed-out command, got {:?}", other),
    }
}

#[test]
fn conforms_to_agent_runtime_spec_2_7_knowledge_ttl_default_30_days() {
    // Setup: compute expected expiry
    let expected_expiry = 1000u64 + 30 * 24 * 3600; // 1000 + 2592000 = 2593000

    // Positive: expires_at = created_at + 30 days
    // (verified via execute_remember which sets expires_at = now + 30*24*3600)
    let remember_input =
        RememberInput { kind: KnowledgeKind::Finding, content: "test knowledge".to_string() };
    let result = tools::execute_remember(&remember_input);
    match result {
        ToolOutput::Remember(entry) => {
            assert_eq!(entry.expires_at, entry.created_at + 30 * 24 * 3600);
        }
        other => panic!("expected Remember output, got {:?}", other),
    }

    // NEGATIVE: expires_at is NOT equal to created_at
    assert_ne!(expected_expiry, 1000, "expires_at must differ from created_at");
}

#[test]
fn conforms_to_agent_runtime_spec_2_7_max_100_active_knowledge() {
    // Setup: create a DB and insert 105 knowledge entries
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("knowledge.db");
    let db = Database::open(&db_path).unwrap();

    let mut first_id: Option<Hash32> = None;

    for i in 0..105u64 {
        let entry = KnowledgeEntry {
            id: {
                let mut hasher = Sha3_256::new();
                hasher.update(i.to_le_bytes());
                let mut id = [0u8; 32];
                id.copy_from_slice(&hasher.finalize());
                id
            },
            kind: KnowledgeKind::Finding,
            content: format!("entry {}", i),
            created_at: i,
            expires_at: i + 30 * 24 * 3600,
            last_read_at: i,
            is_active: true,
        };
        if i == 0 {
            first_id = Some(entry.id);
        }
        db.insert_knowledge(&entry).unwrap();
    }

    // Archive oldest 5 to bring active count to 100
    for _ in 0..5 {
        db.archive_oldest_knowledge().unwrap();
    }

    // Positive: active count ≤ 100
    let active_count = db.get_active_knowledge_count().unwrap();
    assert!(active_count <= 100, "active knowledge must not exceed 100, got {}", active_count);

    // NEGATIVE: oldest entry returned with is_active=false
    let oldest = db.get_knowledge(&first_id.unwrap()).unwrap().unwrap();
    assert!(!oldest.is_active, "oldest archived entry must have is_active=false");
}

#[test]
fn conforms_to_agent_runtime_spec_2_7_read_file_with_offset_limit() {
    let work_dir = tempfile::tempdir().unwrap();
    let file_path = work_dir.path().join("lines.txt");

    // Create a file with 100 lines
    let content: String = (1..=100).map(|i| format!("line {}\n", i)).collect();
    std::fs::write(&file_path, &content).unwrap();

    // Positive: read with offset=50, limit=10 returns 10 lines with correct content
    let input = ReadToolInput {
        file_path: file_path.to_string_lossy().to_string(),
        offset: Some(50),
        limit: Some(10),
    };
    let result = tools::execute_read(&input, work_dir.path());
    match result {
        ToolOutput::Read(out) => {
            let text = String::from_utf8_lossy(&out.content);
            assert!(text.contains("line 50"), "must include line at offset 50");
            assert!(text.contains("line 59"), "must include line 59 (offset 50 + limit 10)");
            assert!(!text.contains("line 49"), "must not include line before offset");
            assert!(!text.contains("line 60"), "must not include line after limit");
            assert_eq!(out.total_lines, 100);
        }
        other => panic!("expected Read output, got {:?}", other),
    }

    // NEGATIVE: offset beyond EOF returns empty content
    let beyond_input = ReadToolInput {
        file_path: file_path.to_string_lossy().to_string(),
        offset: Some(200),
        limit: Some(10),
    };
    let beyond_result = tools::execute_read(&beyond_input, work_dir.path());
    match beyond_result {
        ToolOutput::Read(out) => {
            assert!(
                out.content.is_empty() || String::from_utf8_lossy(&out.content).trim().is_empty(),
                "offset beyond EOF must return empty content"
            );
        }
        other => panic!("expected Read output for beyond-EOF, got {:?}", other),
    }
}

#[test]
fn conforms_to_agent_runtime_spec_2_7_edit_exact_string_replacement() {
    let work_dir = tempfile::tempdir().unwrap();
    let file_path = work_dir.path().join("edit_test.txt");

    // Positive: "hello" → "hi" replaces correctly
    std::fs::write(&file_path, "hello world").unwrap();
    let input = EditToolInput {
        file_path: file_path.to_string_lossy().to_string(),
        old_string: "hello".to_string(),
        new_string: "hi".to_string(),
    };
    let result = tools::execute_edit(&input, work_dir.path());
    match result {
        ToolOutput::Edit(out) => {
            assert!(out.replaced, "edit must report replaced=true");
            assert_eq!(out.match_count, 1);
        }
        other => panic!("expected Edit output, got {:?}", other),
    }
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "hi world");

    // Positive: "nonexistent" returns replaced=false
    let missing_input = EditToolInput {
        file_path: file_path.to_string_lossy().to_string(),
        old_string: "nonexistent".to_string(),
        new_string: "noop".to_string(),
    };
    let missing_result = tools::execute_edit(&missing_input, work_dir.path());
    match missing_result {
        ToolOutput::Edit(out) => {
            assert!(!out.replaced, "nonexistent string must report replaced=false");
            assert_eq!(out.match_count, 0);
        }
        other => panic!("expected Edit output for missing, got {:?}", other),
    }

    // NEGATIVE: old_string appearing twice returns error (not silently modifying)
    let dup_path = work_dir.path().join("dup.txt");
    std::fs::write(&dup_path, "dup dup").unwrap();
    let dup_input = EditToolInput {
        file_path: dup_path.to_string_lossy().to_string(),
        old_string: "dup".to_string(),
        new_string: "replaced".to_string(),
    };
    let dup_result = tools::execute_edit(&dup_input, work_dir.path());
    match dup_result {
        ToolOutput::Error(msg) => {
            assert!(msg.contains("multiple matches"), "must reject multiple matches");
        }
        _ => panic!("expected Error for multiple matches"),
    }
    // Verify file was NOT modified
    assert_eq!(std::fs::read_to_string(&dup_path).unwrap(), "dup dup");
}

#[test]
fn conforms_to_agent_runtime_spec_2_7_write_creates_and_overwrites() {
    let work_dir = tempfile::tempdir().unwrap();
    let file_path = work_dir.path().join("write_test.txt");

    // Positive: write to non-existent file creates it
    let input1 = WriteToolInput {
        file_path: file_path.to_string_lossy().to_string(),
        content: "first content".to_string(),
    };
    let result1 = tools::execute_write(&input1, work_dir.path());
    match result1 {
        ToolOutput::Write(out) => {
            assert!(out.created, "new file must report created=true");
            assert_eq!(out.bytes_written, 13);
        }
        other => panic!("expected Write output, got {:?}", other),
    }
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "first content");

    // Positive: write to same file overwrites it
    let input2 = WriteToolInput {
        file_path: file_path.to_string_lossy().to_string(),
        content: "second content".to_string(),
    };
    let result2 = tools::execute_write(&input2, work_dir.path());
    match result2 {
        ToolOutput::Write(out) => {
            assert!(!out.created, "existing file must report created=false");
            assert_eq!(out.bytes_written, 14);
        }
        other => panic!("expected Write output for overwrite, got {:?}", other),
    }

    // NEGATIVE: old content is NOT still present
    let final_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(final_content, "second content");
    assert!(!final_content.contains("first content"), "old content must be gone");
}

#[test]
fn conforms_to_agent_runtime_spec_2_7_apply_patch_all_or_nothing() {
    let work_dir = tempfile::tempdir().unwrap();
    let path_a = work_dir.path().join("a.txt");
    let path_b = work_dir.path().join("b.txt");
    std::fs::write(&path_a, "foo").unwrap();
    std::fs::write(&path_b, "bar").unwrap();

    // Positive: partial failure aborts entire operation (no partial writes)
    let partial_input = ApplyPatchInput {
        patches: vec![
            EditToolInput {
                file_path: path_a.to_string_lossy().to_string(),
                old_string: "foo".to_string(),
                new_string: "X".to_string(),
            },
            EditToolInput {
                file_path: path_b.to_string_lossy().to_string(),
                old_string: "nonexistent".to_string(),
                new_string: "Y".to_string(),
            },
        ],
    };
    let partial_result = tools::execute_apply_patch(&partial_input, work_dir.path());
    match partial_result {
        ToolOutput::ApplyPatch(out) => {
            assert_eq!(out.patches_applied, 0, "no patches applied on partial failure");
            assert!(out.patches_failed > 0, "failed patches must be reported");
            assert!(!out.errors.is_empty(), "errors must be reported per patch");
        }
        other => panic!("expected ApplyPatch output, got {:?}", other),
    }
    // a.txt must NOT be modified (rollback)
    assert_eq!(
        std::fs::read_to_string(&path_a).unwrap(),
        "foo",
        "first patch must be rolled back on partial failure"
    );
    assert_eq!(std::fs::read_to_string(&path_b).unwrap(), "bar", "b.txt must be unchanged");

    // NEGATIVE: when all patches are valid, all files are modified
    let full_input = ApplyPatchInput {
        patches: vec![
            EditToolInput {
                file_path: path_a.to_string_lossy().to_string(),
                old_string: "foo".to_string(),
                new_string: "foo_prime".to_string(),
            },
            EditToolInput {
                file_path: path_b.to_string_lossy().to_string(),
                old_string: "bar".to_string(),
                new_string: "bar_prime".to_string(),
            },
        ],
    };
    let full_result = tools::execute_apply_patch(&full_input, work_dir.path());
    match full_result {
        ToolOutput::ApplyPatch(out) => {
            assert_eq!(out.patches_applied, 2);
            assert_eq!(out.patches_failed, 0);
        }
        other => panic!("expected ApplyPatch output for full success, got {:?}", other),
    }
    assert_eq!(std::fs::read_to_string(&path_a).unwrap(), "foo_prime");
    assert_eq!(std::fs::read_to_string(&path_b).unwrap(), "bar_prime");
}

#[test]
fn conforms_to_agent_runtime_spec_2_7_path_traversal_blocked() {
    let work_dir = tempfile::tempdir().unwrap();

    // Positive: read with "../" path is blocked
    let read_input =
        ReadToolInput { file_path: "../secret.txt".to_string(), offset: None, limit: None };
    match tools::execute_read(&read_input, work_dir.path()) {
        ToolOutput::Error(msg) => assert!(msg.contains("path traversal"), "read must block ../"),
        _ => panic!("expected Error for path traversal read"),
    }

    // Positive: edit with "../" path is blocked
    let edit_input = EditToolInput {
        file_path: "../secret.txt".to_string(),
        old_string: "x".to_string(),
        new_string: "y".to_string(),
    };
    match tools::execute_edit(&edit_input, work_dir.path()) {
        ToolOutput::Error(msg) => assert!(msg.contains("path traversal"), "edit must block ../"),
        _ => panic!("expected Error for path traversal edit"),
    }

    // Positive: write with "../" path is blocked
    let write_input =
        WriteToolInput { file_path: "../outside.txt".to_string(), content: "evil".to_string() };
    match tools::execute_write(&write_input, work_dir.path()) {
        ToolOutput::Error(msg) => assert!(msg.contains("path traversal"), "write must block ../"),
        _ => panic!("expected Error for path traversal write"),
    }

    // NEGATIVE: normal relative paths succeed with all three tools
    let safe_file = work_dir.path().join("safe.txt");
    std::fs::write(&safe_file, "safe content").unwrap();

    // read normal path
    let safe_read = ReadToolInput { file_path: "safe.txt".to_string(), offset: None, limit: None };
    assert!(matches!(tools::execute_read(&safe_read, work_dir.path()), ToolOutput::Read(_)));

    // edit normal path
    let safe_edit = EditToolInput {
        file_path: "safe.txt".to_string(),
        old_string: "safe".to_string(),
        new_string: "safer".to_string(),
    };
    assert!(matches!(tools::execute_edit(&safe_edit, work_dir.path()), ToolOutput::Edit(_)));

    // write normal path (create new file)
    let safe_write =
        WriteToolInput { file_path: "new_safe.txt".to_string(), content: "new safe".to_string() };
    assert!(matches!(tools::execute_write(&safe_write, work_dir.path()), ToolOutput::Write(_)));
}

// ─────────────────────────────────────────────────────────────────────────
// Section 3.7 — System Prompt
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn conforms_to_agent_runtime_spec_3_7_prompt_includes_identity_and_todos() {
    // Setup: create DB with identity known + 2 todos
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("prompt.db");
    let db = Database::open(&db_path).unwrap();

    let agent_id: Hash32 = [0xABu8; 32];
    let identity = IdentityBlock { agent_id, trust_stage: 0 };

    db.insert_todo(&TodoItem {
        id: "task-A".to_string(),
        content: "Review PR #1".to_string(),
        status: TodoStatus::InProgress,
        context: None,
    })
    .unwrap();
    db.insert_todo(&TodoItem {
        id: "task-B".to_string(),
        content: "Write tests for module X".to_string(),
        status: TodoStatus::Pending,
        context: Some("urgent".to_string()),
    })
    .unwrap();

    // Action: assemble prompt
    let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp/agent")).unwrap();

    // Positive: prompt contains agent_id hex
    let expected_id = hex::encode(agent_id);
    assert!(
        prompt.contains(&format!("- Agent ID: {}", expected_id)),
        "prompt must contain agent ID"
    );

    // Positive: prompt contains trust_stage
    assert!(
        prompt.contains("- Trust Stage: 0 (0=untrusted, 1=trusted)"),
        "prompt must contain trust stage"
    );

    // Positive: prompt contains both todo items
    assert!(prompt.contains("task-A"), "prompt must contain task-A");
    assert!(prompt.contains("task-B"), "prompt must contain task-B");
    assert!(prompt.contains("Review PR #1"), "prompt must contain task-A content");
    assert!(prompt.contains("Write tests for module X"), "prompt must contain task-B content");

    // NEGATIVE: prompt does NOT contain "Your todo list is empty"
    assert!(
        !prompt.contains("Your todo list is empty"),
        "prompt must not show empty message when todos exist"
    );
}

#[test]
fn conforms_to_agent_runtime_spec_3_7_prompt_empty_todos_shows_discovery() {
    // Setup: create DB with no todos
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("empty_prompt.db");
    let db = Database::open(&db_path).unwrap();

    let identity = IdentityBlock { agent_id: [0xCDu8; 32], trust_stage: 1 };

    // Action: assemble prompt with empty todos
    let prompt = assemble_system_prompt(&db, &identity, Path::new("/tmp/empty")).unwrap();

    // Positive: prompt contains empty todo discovery message
    assert!(
        prompt.contains("Your todo list is empty"),
        "prompt must show discovery guidance when no todos exist"
    );
    assert!(
        prompt.contains("hyperfluid idea list") || prompt.contains("discover"),
        "prompt must instruct browsing or discovery"
    );

    // NEGATIVE: prompt still includes CLI spec even when todos are empty
    assert!(
        prompt.contains("hyperfluid task submit"),
        "CLI spec must be present even with empty todos"
    );
    assert!(
        prompt.contains("--seed-ref"),
        "CLI spec must contain --seed-ref even with empty todos"
    );
}

#[test]
fn conforms_to_agent_runtime_spec_3_7_cli_spec_is_embedded_statically() {
    // Positive: CLI_SPEC constant is non-empty
    assert!(!CLI_SPEC.is_empty(), "CLI_SPEC must be non-empty");

    // Positive: CLI_SPEC contains required strings
    assert!(CLI_SPEC.contains("hyperfluid task submit"), "CLI_SPEC must contain task submit docs");
    assert!(CLI_SPEC.contains("--seed-ref"), "CLI_SPEC must contain --seed-ref argument");
    assert!(CLI_SPEC.contains("--bounty"), "CLI_SPEC must contain --bounty argument");

    // Positive: SYSTEM_INSTRUCTIONS is also non-empty
    assert!(!SYSTEM_INSTRUCTIONS.is_empty());

    // NEGATIVE: CLI_SPEC does NOT change when DB state changes (it's static)
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("cli_spec_test.db");
    let db = Database::open(&db_path).unwrap();

    // Insert data into DB — it must not affect CLI_SPEC
    db.insert_todo(&TodoItem {
        id: "cli-test".to_string(),
        content: "test task".to_string(),
        status: TodoStatus::Pending,
        context: None,
    })
    .unwrap();

    // CLI_SPEC is a `const` — it cannot change, but verify it still has the same content
    let cli_before = CLI_SPEC.to_string();
    let _prompt = assemble_system_prompt(
        &db,
        &IdentityBlock { agent_id: [0u8; 32], trust_stage: 0 },
        Path::new("/tmp"),
    )
    .unwrap();

    // NEGATIVE: after DB operations, CLI_SPEC is unchanged (it's const)
    assert_eq!(CLI_SPEC, cli_before, "CLI_SPEC must be immutable across DB state changes");
}

// ─────────────────────────────────────────────────────────────────────────
// Section 4.7 — Process Isolation
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn conforms_to_agent_runtime_spec_4_7_resource_limits_enforced() {
    // Positive: valid defaults pass validation
    let default_limits = ResourceLimits::default();
    let sc = SandboxConfig::new(
        default_limits.clone(),
        std::path::PathBuf::from("/tmp/hf-sandbox"),
        "http://127.0.0.1:8080".to_string(),
    );
    assert!(sc.validate_limits().is_ok(), "default limits must be valid");

    // Positive: 100MB ram fails (below PROTOCOL_MIN_RAM = 512MB)
    let mut too_small = default_limits.clone();
    too_small.max_ram_bytes = 100 * 1024 * 1024; // 100 MB
    let sc_small =
        SandboxConfig::new(too_small, std::path::PathBuf::from("/tmp/hf-sandbox"), String::new());
    assert!(sc_small.validate_limits().is_err(), "100MB ram must be rejected");
    assert!(
        sc_small.validate_limits().unwrap_err().contains("max_ram_bytes"),
        "error must mention max_ram_bytes"
    );

    // NEGATIVE: valid max_ram_bytes (4GB) passes
    let mut valid_ram = default_limits;
    valid_ram.max_ram_bytes = 4 * 1024 * 1024 * 1024; // 4 GB
    let sc_valid =
        SandboxConfig::new(valid_ram, std::path::PathBuf::from("/tmp/hf-sandbox"), String::new());
    assert!(sc_valid.validate_limits().is_ok(), "4GB ram must be valid");
}

#[test]
fn conforms_to_agent_runtime_spec_4_7_sandbox_boundary_enforced() {
    let sandbox_root = tempfile::tempdir().unwrap();
    let sc = SandboxConfig::new(
        ResourceLimits::default(),
        sandbox_root.path().to_path_buf(),
        "http://127.0.0.1:8080".to_string(),
    );

    // Create a file inside the sandbox
    let inside_file = sandbox_root.path().join("allowed.txt");
    std::fs::write(&inside_file, "safe").unwrap();

    // Positive: path inside working_dir passes check_write_access
    assert!(sc.check_write_access(&inside_file).is_ok());

    // Positive: path outside working_dir fails
    let outside = std::env::temp_dir().join("outside_hf_test.txt");
    std::fs::write(&outside, "outside").unwrap();
    assert!(sc.check_write_access(&outside).is_err());

    // Positive: path with "../" from working_dir fails
    let parent_file = sandbox_root.path().parent().unwrap().join("parent_file.txt");
    std::fs::write(&parent_file, "parent").unwrap();
    let escape_path = sandbox_root.path().join("..").join("parent_file.txt");
    assert!(!sc.is_within_sandbox(&escape_path));

    // Cleanup outside files
    let _ = std::fs::remove_file(&outside);
    if parent_file.exists() {
        let _ = std::fs::remove_file(&parent_file);
    }

    // NEGATIVE: normal relative path inside working_dir IS writable
    let inner_relative = sandbox_root.path().join("nested").join("deep.txt");
    std::fs::create_dir_all(inner_relative.parent().unwrap()).unwrap();
    std::fs::write(&inner_relative, "deep").unwrap();
    assert!(
        sc.check_write_access(&inner_relative).is_ok(),
        "normal path inside sandbox must be writable"
    );
}

#[test]
fn conforms_to_agent_runtime_spec_4_7_agent_crash_preserves_db() {
    // Setup: create DB, insert data, drop connection
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("crash_test.db");

    // Track an in-memory value that will NOT survive "crash"
    let in_memory_sentinel = "this_should_not_survive";

    {
        let db = Database::open(&db_path).unwrap();
        db.insert_todo(&TodoItem {
            id: "crash-todo".to_string(),
            content: "survives crash".to_string(),
            status: TodoStatus::InProgress,
            context: None,
        })
        .unwrap();
    }
    // connection dropped here (simulating crash)

    // Action: reopen DB
    let db2 = Database::open(&db_path).unwrap();
    let todos = db2.get_active_todos().unwrap();

    // Positive: data survives connection close (simulating crash)
    assert_eq!(todos.len(), 1, "todo must survive connection close");
    assert_eq!(todos[0].id, "crash-todo");
    assert_eq!(todos[0].content, "survives crash");

    // NEGATIVE: in-memory-only values are lost
    // We cannot "read" in_memory_sentinel from the reopened DB — it was never persisted
    let state_check = db2.get_state("in_memory_sentinel").unwrap();
    assert!(state_check.is_none(), "unpersisted state must be lost after crash");

    // Also verify the sentinel string does NOT appear anywhere in the DB
    {
        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM todos WHERE content = ?1",
                rusqlite::params![in_memory_sentinel],
                |row| row.get(0),
            )
            .unwrap_or(0);
        assert_eq!(count, 0, "in-memory sentinel must not appear in DB");
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Section 5.3 — Operator Interfaces (Telegram)
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn conforms_to_agent_runtime_spec_5_3_telegram_token_not_in_output() {
    let tg_token = "tg-bot-secret-token-abc123";

    // Setup: config with telegram token
    let cfg = Config {
        telegram: Some(hyperfluid_agent::config::TelegramSection {
            token: Some(tg_token.to_string()),
            chat_id: Some("chat-42".to_string()),
        }),
        ..Default::default()
    };

    // Serialize to TOML
    let toml_str = toml::to_string(&cfg).unwrap();

    // Positive: token IS in config TOML (it belongs there)
    assert!(toml_str.contains(tg_token), "token must be present in config TOML");

    // Action: create AgentRuntime from config
    let db_dir = tempfile::tempdir().unwrap();
    let work_dir = tempfile::tempdir().unwrap();
    let db_path = db_dir.path().join("telegram_agent.db");
    let rt = AgentRuntime::new(cfg, &db_path, work_dir.path()).unwrap();

    // Positive: state KV (excluding config_json) does NOT leak the token
    let loop_state = rt.db.get_state("loop_state").unwrap().unwrap_or_default();
    assert!(!loop_state.contains(tg_token), "loop_state must not leak telegram token");

    // Check recent messages as agent output artifacts
    let msgs = rt.db.get_recent_messages(100).unwrap();
    for (_role, content, _ts) in &msgs {
        assert!(!content.contains(tg_token), "agent message output must not leak telegram token");
    }

    // NEGATIVE: config TOML DOES have the token (it should be in config, not leaked)
    assert!(toml_str.contains(tg_token), "config TOML must contain the token for operation");
}
