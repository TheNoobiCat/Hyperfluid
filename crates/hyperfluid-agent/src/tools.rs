// === C10 Agent Runtime: Core Agent Tools ===
//
// Source: docs/04-specifications/runtime/agent-runtime-spec.md Section 2

use crate::types::*;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const MAX_OUTPUT_BYTES: usize = 100 * 1024;
const DEFAULT_TIMEOUT_MS: u64 = 120_000;

// ── Tool output & error types ──

#[derive(Debug)]
pub enum ToolOutput {
    Bash(BashToolOutput),
    TodoWrite(Vec<TodoItem>),
    TodoUpdate(Vec<TodoUpdateEntry>),
    Remember(KnowledgeEntry),
    Forget(bool),
    Read(ReadToolOutput),
    Edit(EditToolOutput),
    Write(WriteToolOutput),
    ApplyPatch(ApplyPatchOutput),
    Error(String),
}

#[derive(Debug)]
pub struct ToolError {
    pub tool_name: String,
    pub message: String,
}

// ── Tool dispatch ──

pub fn dispatch_tool(
    tool_name: &str,
    arguments: &serde_json::Value,
    working_dir: &Path,
) -> ToolOutput {
    if let Err(e) = validate_tool_input(tool_name, arguments) {
        return ToolOutput::Error(e.message);
    }

    match tool_name {
        "bash" => {
            let input: BashToolInput = serde_json::from_value(arguments.clone()).unwrap();
            let timeout_ms = input.timeout.unwrap_or(DEFAULT_TIMEOUT_MS);
            execute_bash(&input, working_dir, timeout_ms)
        }
        "todo_write" => {
            let input: TodoWriteInput = serde_json::from_value(arguments.clone()).unwrap();
            execute_todo_write(&input)
        }
        "todo_update" => {
            let input: TodoUpdateInput = serde_json::from_value(arguments.clone()).unwrap();
            execute_todo_update(&input)
        }
        "remember" => {
            let input: RememberInput = serde_json::from_value(arguments.clone()).unwrap();
            execute_remember(&input)
        }
        "forget" => {
            let input: ForgetInput = serde_json::from_value(arguments.clone()).unwrap();
            execute_forget(&input)
        }
        "read" => {
            let input: ReadToolInput = serde_json::from_value(arguments.clone()).unwrap();
            execute_read(&input, working_dir)
        }
        "edit" => {
            let input: EditToolInput = serde_json::from_value(arguments.clone()).unwrap();
            execute_edit(&input, working_dir)
        }
        "write" => {
            let input: WriteToolInput = serde_json::from_value(arguments.clone()).unwrap();
            execute_write(&input, working_dir)
        }
        "apply_patch" => {
            let input: ApplyPatchInput = serde_json::from_value(arguments.clone()).unwrap();
            execute_apply_patch(&input, working_dir)
        }
        _ => ToolOutput::Error(format!("unknown tool: {}", tool_name)),
    }
}

// ── Validation ──

pub fn validate_tool_input(
    tool_name: &str,
    arguments: &serde_json::Value,
) -> Result<(), ToolError> {
    let obj = arguments.as_object().ok_or(ToolError {
        tool_name: tool_name.to_string(),
        message: "arguments must be a JSON object".to_string(),
    })?;

    match tool_name {
        "bash" => validate_bash(obj, tool_name),
        "todo_write" => validate_todo_write(obj, tool_name),
        "todo_update" => validate_todo_update(obj, tool_name),
        "remember" => validate_remember(obj, tool_name),
        "forget" => validate_forget(obj, tool_name),
        "read" => validate_read(obj, tool_name),
        "edit" => validate_edit(obj, tool_name),
        "write" => validate_write(obj, tool_name),
        "apply_patch" => validate_apply_patch(obj, tool_name),
        _ => Err(ToolError {
            tool_name: tool_name.to_string(),
            message: format!("unknown tool: {}", tool_name),
        }),
    }
}

fn allowed_keys(
    obj: &serde_json::Map<String, serde_json::Value>,
    allowed: &[&str],
    tool_name: &str,
) -> Result<(), ToolError> {
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(ToolError {
                tool_name: tool_name.to_string(),
                message: format!("unknown field: {}", key),
            });
        }
    }
    Ok(())
}

fn require_string(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    tool_name: &str,
) -> Result<(), ToolError> {
    match obj.get(key) {
        Some(v) if v.is_string() => Ok(()),
        _ => Err(ToolError {
            tool_name: tool_name.to_string(),
            message: format!("field '{}' must be a string", key),
        }),
    }
}

#[allow(dead_code)]
fn require_number(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    tool_name: &str,
) -> Result<(), ToolError> {
    match obj.get(key) {
        Some(v) if v.is_number() => Ok(()),
        _ => Err(ToolError {
            tool_name: tool_name.to_string(),
            message: format!("field '{}' must be a number", key),
        }),
    }
}

fn optional_string(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    tool_name: &str,
) -> Result<(), ToolError> {
    match obj.get(key) {
        Some(v) if !v.is_string() => Err(ToolError {
            tool_name: tool_name.to_string(),
            message: format!("field '{}' must be a string if provided", key),
        }),
        _ => Ok(()),
    }
}

fn optional_number(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    tool_name: &str,
) -> Result<(), ToolError> {
    match obj.get(key) {
        Some(v) if !v.is_number() => Err(ToolError {
            tool_name: tool_name.to_string(),
            message: format!("field '{}' must be a number if provided", key),
        }),
        _ => Ok(()),
    }
}

fn validate_bash(
    obj: &serde_json::Map<String, serde_json::Value>,
    tool_name: &str,
) -> Result<(), ToolError> {
    allowed_keys(obj, &["command", "working_dir", "timeout"], tool_name)?;
    require_string(obj, "command", tool_name)?;
    optional_string(obj, "working_dir", tool_name)?;
    optional_number(obj, "timeout", tool_name)?;
    Ok(())
}

fn validate_todo_write(
    obj: &serde_json::Map<String, serde_json::Value>,
    tool_name: &str,
) -> Result<(), ToolError> {
    allowed_keys(obj, &["items"], tool_name)?;
    match obj.get("items") {
        Some(serde_json::Value::Array(items)) => {
            for (i, item) in items.iter().enumerate() {
                let item_obj = item.as_object().ok_or(ToolError {
                    tool_name: tool_name.to_string(),
                    message: format!("items[{}] must be an object", i),
                })?;
                allowed_keys(item_obj, &["id", "content", "status", "context"], tool_name)?;
                require_string(item_obj, "id", tool_name)?;
                require_string(item_obj, "content", tool_name)?;
                require_string(item_obj, "status", tool_name)?;
                optional_string(item_obj, "context", tool_name)?;
            }
            Ok(())
        }
        _ => Err(ToolError {
            tool_name: tool_name.to_string(),
            message: "field 'items' must be an array".to_string(),
        }),
    }
}

fn validate_todo_update(
    obj: &serde_json::Map<String, serde_json::Value>,
    tool_name: &str,
) -> Result<(), ToolError> {
    allowed_keys(obj, &["updates"], tool_name)?;
    match obj.get("updates") {
        Some(serde_json::Value::Array(updates)) => {
            for (i, update) in updates.iter().enumerate() {
                let up_obj = update.as_object().ok_or(ToolError {
                    tool_name: tool_name.to_string(),
                    message: format!("updates[{}] must be an object", i),
                })?;
                allowed_keys(up_obj, &["id", "new_status", "context_update"], tool_name)?;
                require_string(up_obj, "id", tool_name)?;
                require_string(up_obj, "new_status", tool_name)?;
                optional_string(up_obj, "context_update", tool_name)?;
            }
            Ok(())
        }
        _ => Err(ToolError {
            tool_name: tool_name.to_string(),
            message: "field 'updates' must be an array".to_string(),
        }),
    }
}

fn validate_remember(
    obj: &serde_json::Map<String, serde_json::Value>,
    tool_name: &str,
) -> Result<(), ToolError> {
    allowed_keys(obj, &["kind", "content"], tool_name)?;
    require_string(obj, "kind", tool_name)?;
    require_string(obj, "content", tool_name)?;
    if let Some(v) = obj.get("kind") {
        let kind_str = v.as_str().unwrap_or("");
        match kind_str {
            "Finding" | "Pattern" | "Constraint" | "Decision" => {}
            _ => {
                return Err(ToolError {
                    tool_name: tool_name.to_string(),
                    message: format!(
                        "kind '{}' must be one of: Finding, Pattern, Constraint, Decision",
                        kind_str
                    ),
                });
            }
        }
    }
    Ok(())
}

fn validate_forget(
    obj: &serde_json::Map<String, serde_json::Value>,
    tool_name: &str,
) -> Result<(), ToolError> {
    allowed_keys(obj, &["id"], tool_name)?;
    require_string(obj, "id", tool_name)?;
    Ok(())
}

fn validate_read(
    obj: &serde_json::Map<String, serde_json::Value>,
    tool_name: &str,
) -> Result<(), ToolError> {
    allowed_keys(obj, &["file_path", "offset", "limit"], tool_name)?;
    require_string(obj, "file_path", tool_name)?;
    optional_number(obj, "offset", tool_name)?;
    optional_number(obj, "limit", tool_name)?;
    Ok(())
}

fn validate_edit(
    obj: &serde_json::Map<String, serde_json::Value>,
    tool_name: &str,
) -> Result<(), ToolError> {
    allowed_keys(obj, &["file_path", "old_string", "new_string"], tool_name)?;
    require_string(obj, "file_path", tool_name)?;
    require_string(obj, "old_string", tool_name)?;
    require_string(obj, "new_string", tool_name)?;
    Ok(())
}

fn validate_write(
    obj: &serde_json::Map<String, serde_json::Value>,
    tool_name: &str,
) -> Result<(), ToolError> {
    allowed_keys(obj, &["file_path", "content"], tool_name)?;
    require_string(obj, "file_path", tool_name)?;
    require_string(obj, "content", tool_name)?;
    Ok(())
}

fn validate_apply_patch(
    obj: &serde_json::Map<String, serde_json::Value>,
    tool_name: &str,
) -> Result<(), ToolError> {
    allowed_keys(obj, &["patches"], tool_name)?;
    match obj.get("patches") {
        Some(serde_json::Value::Array(patches)) => {
            for (i, patch) in patches.iter().enumerate() {
                let patch_obj = patch.as_object().ok_or(ToolError {
                    tool_name: tool_name.to_string(),
                    message: format!("patches[{}] must be an object", i),
                })?;
                allowed_keys(patch_obj, &["file_path", "old_string", "new_string"], tool_name)?;
                require_string(patch_obj, "file_path", tool_name)?;
                require_string(patch_obj, "old_string", tool_name)?;
                require_string(patch_obj, "new_string", tool_name)?;
            }
            Ok(())
        }
        _ => Err(ToolError {
            tool_name: tool_name.to_string(),
            message: "field 'patches' must be an array".to_string(),
        }),
    }
}

// ── Path traversal guard ──

pub fn check_path_traversal(path: &str, working_dir: &Path) -> bool {
    if path.contains("..") {
        return false;
    }
    let working_dir = match working_dir.canonicalize() {
        Ok(wd) => wd,
        Err(_) => return false,
    };
    let path_buf = PathBuf::from(path);
    let resolved = if path_buf.is_absolute() { path_buf } else { working_dir.join(&path_buf) };
    match resolved.canonicalize() {
        Ok(canonical) => canonical.starts_with(&working_dir),
        Err(_) => match resolved.parent() {
            Some(parent) => match parent.canonicalize() {
                Ok(parent_canonical) => parent_canonical.starts_with(&working_dir),
                Err(_) => false,
            },
            None => false,
        },
    }
}

fn guarded_path(path: &str, working_dir: &Path, _tool_name: &str) -> Result<PathBuf, ToolOutput> {
    if !check_path_traversal(path, working_dir) {
        return Err(ToolOutput::Error(format!("path traversal blocked: {}", path)));
    }
    let path_buf = PathBuf::from(path);
    Ok(if path_buf.is_absolute() { path_buf } else { working_dir.join(&path_buf) })
}

// ── Bash execution ──

pub fn execute_bash(input: &BashToolInput, working_dir: &Path, timeout_ms: u64) -> ToolOutput {
    let effective_dir = match &input.working_dir {
        Some(d) => PathBuf::from(d),
        None => working_dir.to_path_buf(),
    };

    let mut cmd = build_shell_command(&input.command);
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped()).current_dir(&effective_dir);

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            return ToolOutput::Error(format!("bash spawn failed: {}", e));
        }
    };

    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let deadline = start + timeout;

    let exit_code = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code().unwrap_or(-1),
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    break -1;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(_) => {
                let _ = child.kill();
                break -1;
            }
        }
    };

    let elapsed = start.elapsed();
    let timed_out = elapsed >= timeout;

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    if let Some(mut out) = child.stdout.take() {
        let _ = out.read_to_end(&mut stdout);
    }
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_end(&mut stderr);
    }

    let stdout_trunc = truncate_bytes(&mut stdout);
    let stderr_trunc = truncate_bytes(&mut stderr);

    ToolOutput::Bash(BashToolOutput {
        stdout,
        stderr,
        exit_code,
        truncated: stdout_trunc || stderr_trunc || timed_out,
        execution_time_ms: elapsed.as_millis() as u64,
    })
}

fn build_shell_command(command: &str) -> Command {
    #[cfg(target_os = "windows")]
    {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", command]);
        cmd
    }
    #[cfg(not(target_os = "windows"))]
    {
        let mut cmd = Command::new("sh");
        cmd.args(&["-c", command]);
        cmd
    }
}

fn truncate_bytes(data: &mut Vec<u8>) -> bool {
    if data.len() > MAX_OUTPUT_BYTES {
        data.truncate(MAX_OUTPUT_BYTES);
        true
    } else {
        false
    }
}

// ── Todo tools ──

pub fn execute_todo_write(input: &TodoWriteInput) -> ToolOutput {
    ToolOutput::TodoWrite(input.items.clone())
}

pub fn execute_todo_update(input: &TodoUpdateInput) -> ToolOutput {
    ToolOutput::TodoUpdate(input.updates.clone())
}

// ── Knowledge tools ──

pub fn execute_remember(input: &RememberInput) -> ToolOutput {
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(input.content.as_bytes());
    let id: Hash32 = hasher.finalize().into();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    ToolOutput::Remember(KnowledgeEntry {
        id,
        kind: input.kind,
        content: input.content.clone(),
        created_at: now,
        expires_at: now + 30 * 24 * 3600,
        last_read_at: now,
        is_active: true,
    })
}

pub fn execute_forget(_input: &ForgetInput) -> ToolOutput {
    ToolOutput::Forget(true)
}

// ── File tools ──

pub fn execute_read(input: &ReadToolInput, working_dir: &Path) -> ToolOutput {
    let file_path = match guarded_path(&input.file_path, working_dir, "read") {
        Ok(p) => p,
        Err(e) => return e,
    };

    let content = match std::fs::read(&file_path) {
        Ok(c) => c,
        Err(e) => {
            return ToolOutput::Error(format!("file not found: {} ({})", input.file_path, e));
        }
    };

    let text = String::from_utf8_lossy(&content);
    let lines: Vec<&str> = text.lines().collect();
    let total_lines = lines.len() as u64;

    let offset = input.offset.unwrap_or(1).max(1);
    let limit = input.limit.unwrap_or(u64::MAX);
    let start = (offset as usize).saturating_sub(1).min(lines.len());
    let end = (start + limit as usize).min(lines.len());
    let selected = &lines[start..end];

    let mut result = selected.join("\n").into_bytes();
    if !result.is_empty() {
        result.push(b'\n');
    }
    let truncated = (end - start) < lines.len().saturating_sub(start);

    ToolOutput::Read(ReadToolOutput { content: result, total_lines, truncated })
}

pub fn execute_edit(input: &EditToolInput, working_dir: &Path) -> ToolOutput {
    let file_path = match guarded_path(&input.file_path, working_dir, "edit") {
        Ok(p) => p,
        Err(e) => return e,
    };

    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => {
            return ToolOutput::Error(format!("file not found: {} ({})", input.file_path, e));
        }
    };

    let match_count = content.matches(&input.old_string).count() as u32;

    if match_count == 0 {
        return ToolOutput::Edit(EditToolOutput { replaced: false, match_count: 0 });
    }

    if match_count > 1 {
        return ToolOutput::Error(format!(
            "multiple matches: old_string found {} times; provide more surrounding context",
            match_count
        ));
    }

    let new_content = content.replacen(&input.old_string, &input.new_string, 1);
    if let Err(e) = std::fs::write(&file_path, &new_content) {
        return ToolOutput::Error(format!("write failed: {}", e));
    }

    ToolOutput::Edit(EditToolOutput { replaced: true, match_count: 1 })
}

pub fn execute_write(input: &WriteToolInput, working_dir: &Path) -> ToolOutput {
    let file_path = match guarded_path(&input.file_path, working_dir, "write") {
        Ok(p) => p,
        Err(e) => return e,
    };

    let created = !file_path.exists();
    let content_bytes = input.content.as_bytes();

    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                return ToolOutput::Error(format!("mkdir failed: {}", e));
            }
        }
    }

    if let Err(e) = std::fs::write(&file_path, content_bytes) {
        return ToolOutput::Error(format!("write failed: {}", e));
    }

    ToolOutput::Write(WriteToolOutput { bytes_written: content_bytes.len() as u64, created })
}

// ── Apply patch (all-or-nothing) ──

pub fn execute_apply_patch(input: &ApplyPatchInput, working_dir: &Path) -> ToolOutput {
    let mut errors: Vec<String> = vec![String::new(); input.patches.len()];
    let mut has_error = false;

    // Phase 1: validate all patches
    struct ValidatedPatch<'a> {
        file_path: PathBuf,
        old_string: &'a str,
        new_string: &'a str,
        original_content: String,
    }

    let mut validated: Vec<ValidatedPatch> = Vec::with_capacity(input.patches.len());

    for (i, patch) in input.patches.iter().enumerate() {
        let file_path = match guarded_path(&patch.file_path, working_dir, "apply_patch") {
            Ok(p) => p,
            Err(e) => {
                if let ToolOutput::Error(msg) = e {
                    errors[i] = msg;
                } else {
                    errors[i] = format!("path check failed for patch {}", i);
                }
                has_error = true;
                continue;
            }
        };

        let content = match std::fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                errors[i] = format!("file not found: {} ({})", patch.file_path, e);
                has_error = true;
                continue;
            }
        };

        let match_count = content.matches(&patch.old_string).count();
        if match_count == 0 {
            errors[i] = format!("patch {}: old_string not found in {}", i, patch.file_path);
            has_error = true;
            continue;
        }
        if match_count > 1 {
            errors[i] = format!(
                "patch {}: old_string found {} times in {}; provide more context",
                i, match_count, patch.file_path
            );
            has_error = true;
            continue;
        }

        validated.push(ValidatedPatch {
            file_path,
            old_string: &patch.old_string,
            new_string: &patch.new_string,
            original_content: content,
        });
    }

    if has_error {
        let patches_applied = 0u32;
        let patches_failed = input.patches.len() as u32;
        let errors: Vec<String> = errors.into_iter().filter(|e| !e.is_empty()).collect();
        return ToolOutput::ApplyPatch(ApplyPatchOutput {
            patches_applied,
            patches_failed,
            errors,
        });
    }

    // Phase 2: apply all patches
    let mut applied = 0u32;
    for v in &validated {
        let new_content = v.original_content.replacen(v.old_string, v.new_string, 1);
        if let Err(e) = std::fs::write(&v.file_path, &new_content) {
            return ToolOutput::Error(format!(
                "apply_patch write failed for {}: {}",
                v.file_path.display(),
                e
            ));
        }
        applied += 1;
    }

    ToolOutput::ApplyPatch(ApplyPatchOutput {
        patches_applied: applied,
        patches_failed: 0,
        errors: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Validation tests ──

    #[test]
    fn validates_bash_correctly() {
        let args = serde_json::json!({"command": "echo hello"});
        assert!(validate_tool_input("bash", &args).is_ok());

        let args = serde_json::json!({"command": "ls", "working_dir": "/tmp", "timeout": 5000});
        assert!(validate_tool_input("bash", &args).is_ok());

        let args = serde_json::json!({"command": 123});
        assert!(validate_tool_input("bash", &args).is_err());

        let args = serde_json::json!({"command": "ls", "extra_field": "bad"});
        assert!(validate_tool_input("bash", &args).is_err());
    }

    #[test]
    fn rejects_unknown_tool() {
        let args = serde_json::json!({});
        assert!(validate_tool_input("nonexistent", &args).is_err());
    }

    #[test]
    fn validates_todo_write_correctly() {
        let args = serde_json::json!({
            "items": [
                {"id": "1", "content": "task one", "status": "Pending"},
                {"id": "2", "content": "task two", "status": "InProgress", "context": "extra info"}
            ]
        });
        assert!(validate_tool_input("todo_write", &args).is_ok());

        let args = serde_json::json!({"items": [{"id": "1", "content": "task"}]});
        assert!(validate_tool_input("todo_write", &args).is_err());
    }

    #[test]
    fn validates_remember_kind_enum() {
        let args = serde_json::json!({"kind": "Finding", "content": "test"});
        assert!(validate_tool_input("remember", &args).is_ok());

        let args = serde_json::json!({"kind": "Pattern", "content": "test"});
        assert!(validate_tool_input("remember", &args).is_ok());

        let args = serde_json::json!({"kind": "Constraint", "content": "test"});
        assert!(validate_tool_input("remember", &args).is_ok());

        let args = serde_json::json!({"kind": "Decision", "content": "test"});
        assert!(validate_tool_input("remember", &args).is_ok());

        let args = serde_json::json!({"kind": "Invalid", "content": "test"});
        assert!(validate_tool_input("remember", &args).is_err());
    }

    #[test]
    fn validates_forget_correctly() {
        let args = serde_json::json!({"id": "abc123"});
        assert!(validate_tool_input("forget", &args).is_ok());

        let args = serde_json::json!({"id": 42});
        assert!(validate_tool_input("forget", &args).is_err());

        let args = serde_json::json!({});
        assert!(validate_tool_input("forget", &args).is_err());
    }

    #[test]
    fn validates_read_correctly() {
        let args = serde_json::json!({"file_path": "/some/file.txt"});
        assert!(validate_tool_input("read", &args).is_ok());

        let args = serde_json::json!({"file_path": "/some/file.txt", "offset": 10, "limit": 50});
        assert!(validate_tool_input("read", &args).is_ok());

        let args = serde_json::json!({"offset": 10});
        assert!(validate_tool_input("read", &args).is_err());
    }

    #[test]
    fn validates_edit_correctly() {
        let args =
            serde_json::json!({"file_path": "foo.txt", "old_string": "a", "new_string": "b"});
        assert!(validate_tool_input("edit", &args).is_ok());

        let args = serde_json::json!({"file_path": "foo.txt", "old_string": "a"});
        assert!(validate_tool_input("edit", &args).is_err());
    }

    #[test]
    fn validates_write_correctly() {
        let args = serde_json::json!({"file_path": "new.txt", "content": "hello"});
        assert!(validate_tool_input("write", &args).is_ok());

        let args = serde_json::json!({"content": "hello"});
        assert!(validate_tool_input("write", &args).is_err());
    }

    #[test]
    fn validates_apply_patch_correctly() {
        let args = serde_json::json!({
            "patches": [
                {"file_path": "a.txt", "old_string": "x", "new_string": "y"},
                {"file_path": "b.txt", "old_string": "z", "new_string": "w"}
            ]
        });
        assert!(validate_tool_input("apply_patch", &args).is_ok());

        let args = serde_json::json!({"patches": [{"file_path": "a.txt"}]});
        assert!(validate_tool_input("apply_patch", &args).is_err());
    }

    // ── Path traversal tests ──

    #[test]
    fn blocks_dotdot_path() {
        let temp = std::env::temp_dir();
        assert!(!check_path_traversal("../etc/passwd", &temp));
        assert!(!check_path_traversal("foo/../../bar", &temp));
    }

    #[test]
    fn allows_safe_relative_path() {
        let temp = std::env::temp_dir();
        let subdir = temp.join("test_safe_path");
        std::fs::create_dir_all(&subdir).unwrap();
        assert!(check_path_traversal("test_safe_path/bar.txt", &temp));
        let _ = std::fs::remove_dir_all(&subdir);
    }

    #[test]
    fn rejects_absolute_path_outside_working_dir() {
        let temp = std::env::temp_dir();
        assert!(!check_path_traversal("/etc/passwd", &temp));
    }

    // ── Read tool tests ──

    #[test]
    fn read_file_with_offset_limit() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "line1\nline2\nline3\nline4\nline5\n").unwrap();

        let input = ReadToolInput {
            file_path: file_path.to_string_lossy().to_string(),
            offset: Some(2),
            limit: Some(2),
        };
        let result = execute_read(&input, dir.path());
        match result {
            ToolOutput::Read(out) => {
                let text = String::from_utf8_lossy(&out.content);
                assert!(text.contains("line2"));
                assert!(text.contains("line3"));
                assert!(!text.contains("line1"));
                assert!(!text.contains("line4"));
                assert_eq!(out.total_lines, 5);
            }
            other => panic!("expected Read output, got {:?}", other),
        }
    }

    #[test]
    fn read_file_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let input =
            ReadToolInput { file_path: "nonexistent.txt".to_string(), offset: None, limit: None };
        let result = execute_read(&input, dir.path());
        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("not found")),
            _ => panic!("expected Error output"),
        }
    }

    #[test]
    fn read_blocks_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let input =
            ReadToolInput { file_path: "../secret.txt".to_string(), offset: None, limit: None };
        let result = execute_read(&input, dir.path());
        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("path traversal")),
            _ => panic!("expected Error output"),
        }
    }

    // ── Edit tool tests ──

    #[test]
    fn edit_single_match() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("edit.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let input = EditToolInput {
            file_path: file_path.to_string_lossy().to_string(),
            old_string: "hello".to_string(),
            new_string: "goodbye".to_string(),
        };
        let result = execute_edit(&input, dir.path());
        match result {
            ToolOutput::Edit(out) => {
                assert!(out.replaced);
                assert_eq!(out.match_count, 1);
            }
            _ => panic!("expected Edit output"),
        }
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "goodbye world");
    }

    #[test]
    fn edit_no_match() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("edit.txt");
        std::fs::write(&file_path, "hello world").unwrap();

        let input = EditToolInput {
            file_path: file_path.to_string_lossy().to_string(),
            old_string: "nonexistent".to_string(),
            new_string: "replacement".to_string(),
        };
        let result = execute_edit(&input, dir.path());
        match result {
            ToolOutput::Edit(out) => {
                assert!(!out.replaced);
                assert_eq!(out.match_count, 0);
            }
            _ => panic!("expected Edit output"),
        }
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello world");
    }

    #[test]
    fn edit_multiple_matches_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("edit.txt");
        std::fs::write(&file_path, "hello hello hello").unwrap();

        let input = EditToolInput {
            file_path: file_path.to_string_lossy().to_string(),
            old_string: "hello".to_string(),
            new_string: "hi".to_string(),
        };
        let result = execute_edit(&input, dir.path());
        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("multiple matches")),
            _ => panic!("expected Error output"),
        }
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "hello hello hello");
    }

    // ── Write tool tests ──

    #[test]
    fn write_creates_new_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("new.txt");
        let input = WriteToolInput {
            file_path: file_path.to_string_lossy().to_string(),
            content: "fresh content".to_string(),
        };
        let result = execute_write(&input, dir.path());
        match result {
            ToolOutput::Write(out) => {
                assert!(out.created);
                assert_eq!(out.bytes_written, 13);
            }
            _ => panic!("expected Write output"),
        }
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "fresh content");
    }

    #[test]
    fn write_overwrites_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("existing.txt");
        std::fs::write(&file_path, "old").unwrap();

        let input = WriteToolInput {
            file_path: file_path.to_string_lossy().to_string(),
            content: "new content".to_string(),
        };
        let result = execute_write(&input, dir.path());
        match result {
            ToolOutput::Write(out) => {
                assert!(!out.created);
                assert_eq!(out.bytes_written, 11);
            }
            _ => panic!("expected Write output"),
        }
        let content = std::fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "new content");
    }

    #[test]
    fn write_blocks_path_traversal() {
        let dir = tempfile::tempdir().unwrap();
        let input =
            WriteToolInput { file_path: "../outside.txt".to_string(), content: "evil".to_string() };
        let result = execute_write(&input, dir.path());
        match result {
            ToolOutput::Error(msg) => assert!(msg.contains("path traversal")),
            _ => panic!("expected Error output"),
        }
    }

    // ── Apply patch tests ──

    #[test]
    fn apply_patch_atomic_all_succeed() {
        let dir = tempfile::tempdir().unwrap();
        let fp_a = dir.path().join("a.txt");
        let fp_b = dir.path().join("b.txt");
        std::fs::write(&fp_a, "alpha").unwrap();
        std::fs::write(&fp_b, "beta").unwrap();

        let input = ApplyPatchInput {
            patches: vec![
                EditToolInput {
                    file_path: fp_a.to_string_lossy().to_string(),
                    old_string: "alpha".to_string(),
                    new_string: "alpha_prime".to_string(),
                },
                EditToolInput {
                    file_path: fp_b.to_string_lossy().to_string(),
                    old_string: "beta".to_string(),
                    new_string: "beta_prime".to_string(),
                },
            ],
        };
        let result = execute_apply_patch(&input, dir.path());
        match result {
            ToolOutput::ApplyPatch(out) => {
                assert_eq!(out.patches_applied, 2);
                assert_eq!(out.patches_failed, 0);
            }
            _ => panic!("expected ApplyPatch output"),
        }
        assert_eq!(std::fs::read_to_string(&fp_a).unwrap(), "alpha_prime");
        assert_eq!(std::fs::read_to_string(&fp_b).unwrap(), "beta_prime");
    }

    #[test]
    fn apply_patch_atomic_partial_failure_no_writes() {
        let dir = tempfile::tempdir().unwrap();
        let fp_a = dir.path().join("a.txt");
        let fp_b = dir.path().join("b.txt");
        std::fs::write(&fp_a, "alpha").unwrap();
        std::fs::write(&fp_b, "beta").unwrap();

        let input = ApplyPatchInput {
            patches: vec![
                EditToolInput {
                    file_path: fp_a.to_string_lossy().to_string(),
                    old_string: "alpha".to_string(),
                    new_string: "alpha_prime".to_string(),
                },
                EditToolInput {
                    file_path: fp_b.to_string_lossy().to_string(),
                    old_string: "nonexistent".to_string(),
                    new_string: "irrelevant".to_string(),
                },
            ],
        };
        let result = execute_apply_patch(&input, dir.path());
        match result {
            ToolOutput::ApplyPatch(out) => {
                assert_eq!(out.patches_applied, 0);
                assert_eq!(out.patches_failed, 2);
                assert_eq!(out.errors.len(), 1);
            }
            _ => panic!("expected ApplyPatch output"),
        }
        assert_eq!(std::fs::read_to_string(&fp_a).unwrap(), "alpha");
        assert_eq!(std::fs::read_to_string(&fp_b).unwrap(), "beta");
    }

    #[test]
    fn apply_patch_path_traversal_aborts_all() {
        let dir = tempfile::tempdir().unwrap();
        let fp_a = dir.path().join("a.txt");
        std::fs::write(&fp_a, "alpha").unwrap();

        let input = ApplyPatchInput {
            patches: vec![
                EditToolInput {
                    file_path: fp_a.to_string_lossy().to_string(),
                    old_string: "alpha".to_string(),
                    new_string: "alpha_prime".to_string(),
                },
                EditToolInput {
                    file_path: "../evil.txt".to_string(),
                    old_string: "x".to_string(),
                    new_string: "y".to_string(),
                },
            ],
        };
        let result = execute_apply_patch(&input, dir.path());
        match result {
            ToolOutput::ApplyPatch(out) => {
                assert_eq!(out.patches_applied, 0);
                assert!(out.patches_failed > 0);
            }
            _ => panic!("expected ApplyPatch output"),
        }
        assert_eq!(std::fs::read_to_string(&fp_a).unwrap(), "alpha");
    }
}
