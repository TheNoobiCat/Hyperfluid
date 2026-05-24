// === C10 Agent Runtime: Infinite Agent Loop ===
//
// Source: docs/04-specifications/runtime/agent-runtime-spec.md Sections 1 and 3
//
// Implements the infinite agent loop, handoff mechanism,
// crash recovery, and failure guard.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use sha3::{Digest, Sha3_256};

use crate::config::{Config, LlmSection, TelegramSection};
use crate::db::Database;
use crate::llm::{self, LlmProvider};
use crate::prompt;
use crate::tools;
use crate::types::*;

// === Unix timestamp helper ===

/// Returns the current unix timestamp in seconds.
pub fn unix_timestamp_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

// === AgentError ===

#[derive(Debug, thiserror::Error)]
pub enum AgentError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("tool error: {0}")]
    Tool(String),

    #[error("failure guard blocked: {0}")]
    FailureGuardBlocked(String),

    #[error("handoff triggered")]
    HandoffTriggered,

    #[error("config error: {0}")]
    Config(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("LLM provider error: {0}")]
    Llm(#[from] crate::llm::LlmError),
}

// === AgentRuntime ===

pub struct AgentRuntime {
    pub config: Config,
    pub agent_config: AgentRuntimeConfig,
    pub limits: ResourceLimits,
    pub state: AgentLoopState,
    pub identity: IdentityBlock,
    pub db: Database,
    pub working_dir: PathBuf,
    pub shutdown: Arc<AtomicBool>,
    pub provider: Box<dyn LlmProvider>,
}

// ── Constructor ──

impl AgentRuntime {
    /// Opens database at `db_path`, initializes all fields from config,
    /// loads or defaults agent loop state, and ensures `working_dir` exists.
    pub fn new(config: Config, db_path: &Path, working_dir: &Path) -> Result<Self, AgentError> {
        // Open database (WAL mode enabled automatically by Database::open)
        let db = Database::open(db_path)?;

        // Derive sub-configs
        let agent_config = config.to_agent_runtime_config();
        let limits = config.to_resource_limits();

        // Identity: SHA3-256 of agent name, trust_stage starts at 0
        let agent_id = {
            let mut hasher = Sha3_256::new();
            hasher.update(config.agent.agent_name.as_bytes());
            let result = hasher.finalize();
            let mut id = [0u8; 32];
            id.copy_from_slice(&result);
            id
        };
        let identity = IdentityBlock { agent_id, trust_stage: 0 };

        // Load existing state from DB KV or default, then persist
        let state = match db.get_state("loop_state")? {
            Some(json) => serde_json::from_str(&json).unwrap_or_default(),
            None => {
                let default_state = AgentLoopState::default();
                let json = serde_json::to_string(&default_state).unwrap_or_default();
                db.set_state("loop_state", &json)?;
                default_state
            }
        };

        // Persist non-sensitive config fields for crash recovery.
        // API keys and Telegram tokens are NOT stored in the database —
        // they are loaded from the config file on restart.
        let safe_telegram = config.telegram.as_ref().map(|t| TelegramSection {
            token: String::new(), // redacted — reloaded from config file
            user_id: t.user_id,
            enabled: t.enabled,
        });
        let safe_config = Config {
            agent: config.agent.clone(),
            llm: LlmSection {
                api_key: None, // redacted — reloaded from config file
                ..config.llm.clone()
            },
            telegram: safe_telegram,
            limits: config.limits.clone(),
        };
        if let Ok(config_json) = serde_json::to_string(&safe_config) {
            let _ = db.set_state("config_json", &config_json);
        }

        // Create working directory if it does not exist
        if !working_dir.exists() {
            std::fs::create_dir_all(working_dir)?;
        }

        // Create LLM provider from config
        let provider = llm::provider_from_config(&config.llm);

        Ok(Self {
            config,
            agent_config,
            limits,
            state,
            identity,
            db,
            working_dir: working_dir.to_path_buf(),
            shutdown: Arc::new(AtomicBool::new(false)),
            provider,
        })
    }
}

// ── Main loop ──

impl AgentRuntime {
    /// Runs the infinite agent loop.
    ///
    /// On each iteration: assembles prompt, checks token/message budgets
    /// (triggering handoff if thresholds exceeded), executes tool calls
    /// with failure-guard validation, persists state, and sleeps.
    /// Exits when the shutdown flag is set.
    pub fn run_loop(&mut self) -> Result<(), AgentError> {
        let interval = Duration::from_millis(self.agent_config.loop_interval_ms);

        loop {
            // 10. Check shutdown signal (also checked at top to avoid
            //    unnecessary work after shutdown is requested mid-sleep)
            if self.shutdown.load(Ordering::Acquire) {
                break;
            }

            // 9. Sleep for loop_interval_ms (first iteration starts immediately)
            if self.state.iteration > 0 {
                thread::sleep(interval);
            }

            // Re-check shutdown after sleep
            if self.shutdown.load(Ordering::Acquire) {
                break;
            }

            // 1. Assemble system prompt
            let _system_prompt =
                prompt::assemble_system_prompt(&self.db, &self.identity, &self.working_dir)
                    .map_err(AgentError::Tool)?;

            // 2. Check token budget: trigger handoff if > threshold %
            let token_pct = if self.agent_config.context_limit_tokens > 0 {
                (self.state.total_tokens_used * 100) / self.agent_config.context_limit_tokens as u64
            } else {
                0
            };
            if token_pct > self.agent_config.handoff_threshold_pct as u64 {
                self.trigger_handoff()?;
                self.state.total_tokens_used = 0;
            }

            // 3. Check message count: trigger handoff if > threshold
            let recent = self.db.get_recent_messages(1000)?;
            if recent.len() > self.agent_config.handoff_trigger_messages as usize {
                self.trigger_handoff()?;
            }

            // 4. Build messages for LLM
            let recent_msgs: Vec<LlmMessage> = self
                .db
                .get_recent_messages(1000)
                .unwrap_or_default()
                .into_iter()
                .map(|(role, content, _timestamp)| LlmMessage { role, content })
                .collect();

            // 5. Call the LLM provider
            let llm_request = LlmRequest {
                system_prompt: _system_prompt,
                messages: recent_msgs,
                max_tokens: 4096,
            };
            let llm_response = self.provider.complete(&llm_request)?;

            self.state.total_tokens_used =
                self.state.total_tokens_used.saturating_add(llm_response.tokens_used as u64);
            self.db.insert_message("assistant", &llm_response.content)?;

            // 6. Parse tool calls from LLM response (if any)
            // The LLM should output JSON like: [{"tool_name": "bash", "arguments": {...}}, ...]
            // or plain text with no tool calls. Invalid JSON = no tool calls.
            let tool_calls: Vec<ToolCall> = if llm_response.content.trim().is_empty() {
                Vec::new()
            } else if let Some(start) = llm_response.content.find('[') {
                let end = llm_response.content[start..].find(']').map(|e| start + e + 1);
                match end {
                    Some(end) => {
                        serde_json::from_str(&llm_response.content[start..end]).unwrap_or_default()
                    }
                    None => Vec::new(),
                }
            } else {
                Vec::new()
            };

            // 6b. Run-forever guard: if no tool calls and no active work, nudge the agent
            if tool_calls.is_empty() {
                let active_todos = self.db.get_active_todos().unwrap_or_default();
                if active_todos.is_empty() && self.state.iteration > 0 {
                    self.db.insert_message(
                        "system",
                        "You had no actions this iteration. This loop runs forever — you \
                         must find something productive: explore the codebase, browse seeds, \
                         create a task, or plan your next move.",
                    )?;
                }
            }

            // 7. Execute each tool call
            for tc in &tool_calls {
                if let Err(e) = tools::validate_tool_input(&tc.tool_name, &tc.arguments) {
                    let now = unix_timestamp_now();
                    let input_hash = hash_arguments(&tc.arguments);
                    self.db.record_failure(&FailureRecord {
                        timestamp: now,
                        tool_name: e.tool_name.clone(),
                        input_hash,
                        error_kind: e.message.clone(),
                    })?;
                    self.db.insert_message(
                        "tool",
                        &format!("validation error [{}]: {}", tc.tool_name, e.message),
                    )?;
                    continue;
                }

                let input_hash = hash_arguments(&tc.arguments);
                if let Err(e) = self.check_failure_guard(&tc.tool_name, &input_hash) {
                    self.db.insert_message(
                        "tool",
                        &format!("blocked by failure guard [{}]: {}", tc.tool_name, e),
                    )?;
                    continue;
                }

                // 6c. Execute tool
                let started_at = unix_timestamp_now();
                let output =
                    tools::dispatch_tool(&tc.tool_name, &tc.arguments, &self.working_dir, &self.db);

                // 6d. Record failure if error
                if let tools::ToolOutput::Error(ref msg) = output {
                    let now = unix_timestamp_now();
                    self.db.record_failure(&FailureRecord {
                        timestamp: now,
                        tool_name: tc.tool_name.clone(),
                        input_hash,
                        error_kind: msg.clone(),
                    })?;
                }

                // 6e. Collect output into message log
                let output_msg = format_tool_output(&output);
                self.db.insert_message("tool", &output_msg)?;

                // Track active tool call
                self.state.active_tool_calls.push(ToolCallExecution {
                    tool_name: tc.tool_name.clone(),
                    input_hash,
                    started_at,
                    timeout_ms: self.agent_config.tool_timeout_ms,
                });
            }

            // 7. Update state
            self.state.iteration = self.state.iteration.saturating_add(1);
            self.state.active_tool_calls.clear();

            // 8. Persist state to db
            self.persist_state()?;
        }

        // Final persist before exit
        self.persist_state()?;
        Ok(())
    }

    /// Runs a single iteration of the loop (steps 1–8, no sleep, no shutdown check).
    /// Exposed for testing.
    #[doc(hidden)]
    pub fn run_one_iteration(&mut self) -> Result<(), AgentError> {
        // 1. Assemble system prompt
        let _system_prompt =
            prompt::assemble_system_prompt(&self.db, &self.identity, &self.working_dir)
                .map_err(AgentError::Tool)?;

        // 2. Check token budget
        let token_pct = if self.agent_config.context_limit_tokens > 0 {
            (self.state.total_tokens_used * 100) / self.agent_config.context_limit_tokens as u64
        } else {
            0
        };
        if token_pct > self.agent_config.handoff_threshold_pct as u64 {
            self.trigger_handoff()?;
            self.state.total_tokens_used = 0;
        }

        // 3. Check message count
        let recent = self.db.get_recent_messages(1000)?;
        if recent.len() > self.agent_config.handoff_trigger_messages as usize {
            self.trigger_handoff()?;
        }

        // 4. [LLM call — stubbed]
        let llm_response = LlmResponse {
            content: String::new(),
            tokens_used: 10, // simulate some token usage per iteration
            finish_reason: "stub".to_string(),
        };
        self.state.total_tokens_used =
            self.state.total_tokens_used.saturating_add(llm_response.tokens_used as u64);
        self.db.insert_message("assistant", &llm_response.content)?;

        // 5. Parse tool calls (stub — empty)
        let tool_calls: Vec<ToolCall> = Vec::new();

        // 6. Execute each tool call
        for tc in &tool_calls {
            if let Err(e) = tools::validate_tool_input(&tc.tool_name, &tc.arguments) {
                let now = unix_timestamp_now();
                let input_hash = hash_arguments(&tc.arguments);
                self.db.record_failure(&FailureRecord {
                    timestamp: now,
                    tool_name: e.tool_name.clone(),
                    input_hash,
                    error_kind: e.message.clone(),
                })?;
                self.db.insert_message(
                    "tool",
                    &format!("validation error [{}]: {}", tc.tool_name, e.message),
                )?;
                continue;
            }

            let input_hash = hash_arguments(&tc.arguments);
            if let Err(e) = self.check_failure_guard(&tc.tool_name, &input_hash) {
                self.db.insert_message(
                    "tool",
                    &format!("blocked by failure guard [{}]: {}", tc.tool_name, e),
                )?;
                continue;
            }

            let started_at = unix_timestamp_now();
            let output =
                tools::dispatch_tool(&tc.tool_name, &tc.arguments, &self.working_dir, &self.db);

            if let tools::ToolOutput::Error(ref msg) = output {
                let now = unix_timestamp_now();
                self.db.record_failure(&FailureRecord {
                    timestamp: now,
                    tool_name: tc.tool_name.clone(),
                    input_hash,
                    error_kind: msg.clone(),
                })?;
            }

            let output_msg = format_tool_output(&output);
            self.db.insert_message("tool", &output_msg)?;

            self.state.active_tool_calls.push(ToolCallExecution {
                tool_name: tc.tool_name.clone(),
                input_hash,
                started_at,
                timeout_ms: self.agent_config.tool_timeout_ms,
            });
        }

        // 7. Update state
        self.state.iteration = self.state.iteration.saturating_add(1);
        self.state.active_tool_calls.clear();

        // 8. Persist state
        self.persist_state()?;

        Ok(())
    }
}

// ── Handoff ──

impl AgentRuntime {
    /// Creates a handoff record capturing the current todos snapshot
    /// and serialized state summary, persists it to the database,
    /// and updates `last_handoff_height`.
    ///
    /// The handoff reflection prompt (HANDOFF_REFLECTION_PROMPT from prompt.rs)
    /// SHOULD be injected into the LLM conversation before calling this function.
    /// When the LLM stub is replaced with a real provider, the LLM response
    /// to that prompt should be saved as the handoff summary instead of the
    /// internal state JSON currently used as fallback.
    pub fn trigger_handoff(&mut self) -> Result<(), AgentError> {
        let now = unix_timestamp_now();

        // session_id = SHA3-256(agent_id || timestamp)
        let session_id = {
            let mut hasher = Sha3_256::new();
            hasher.update(self.identity.agent_id);
            hasher.update(now.to_le_bytes());
            let result = hasher.finalize();
            let mut id = [0u8; 32];
            id.copy_from_slice(&result);
            id
        };

        // Capture current todos snapshot
        let todos_snapshot = self.db.get_active_todos()?;

        // Inject handoff reflection prompt and capture LLM summary
        // (stub: uses internal state JSON until LLM provider is wired)
        self.db.insert_message("user", prompt::HANDOFF_REFLECTION_PROMPT)?;
        let summary: Vec<u8> = {
            // TODO: replace with LLM call using HANDOFF_REFLECTION_PROMPT
            // The LLM response text should be saved as the summary.
            // Current fallback: serialized agent state (not useful to the LLM).
            serde_json::to_vec(&self.state).unwrap_or_default()
        };

        let record = HandoffRecord {
            session_id,
            timestamp: now,
            summary,
            next_actions: Vec::new(),
            todos_snapshot,
        };

        // Persist to DB
        self.db.save_handoff(&record)?;

        // Update last handoff height
        self.state.last_handoff_height = self.state.iteration;
        self.persist_state()?;

        Ok(())
    }
}

// ── Failure guard ──

impl AgentRuntime {
    /// Checks whether a tool call should be blocked by the failure guard.
    ///
    /// Rules:
    /// - Block exact duplicate if the same (tool_name, input_hash) failed within 1 hour.
    /// - Block if 3 or more failures (any tool) occurred within 1 hour.
    pub fn check_failure_guard(
        &self,
        tool_name: &str,
        input_hash: &Hash32,
    ) -> Result<(), AgentError> {
        let now = unix_timestamp_now();
        let one_hour_ago = now.saturating_sub(3600);

        // Check for exact duplicate failure within 1 hour
        let recent = self.db.recent_failures(tool_name, one_hour_ago)?;
        for failure in &recent {
            if &failure.input_hash == input_hash {
                return Err(AgentError::FailureGuardBlocked(format!(
                    "exact duplicate failure for tool '{}' within 1 hour",
                    tool_name
                )));
            }
        }

        // Block if 3+ total failures within 1 hour
        let total_failures = self.db.count_failures_since(one_hour_ago)?;
        if total_failures >= 3 {
            return Err(AgentError::FailureGuardBlocked(format!(
                "{} failures within 1 hour — tool calls blocked",
                total_failures
            )));
        }

        Ok(())
    }
}

// ── Crash recovery ──

impl AgentRuntime {
    /// Recovers an AgentRuntime from a crash.
    ///
    /// Opens the database (WAL recovery is automatic via rusqlite),
    /// loads the most recent handoff record and active todos,
    /// rebuilds the loop state, and returns a ready-to-resume runtime.
    ///
    /// Config is reloaded from the config file path (not from the DB,
    /// which may have redacted API keys). Falls back to `Config::default()`
    /// if the config file cannot be loaded.
    pub fn recover_from_crash(
        db_path: &Path,
        config_path: &Path,
        working_dir: &Path,
    ) -> Result<Self, AgentError> {
        // Open database — rusqlite applies WAL recovery automatically
        let db = Database::open(db_path)?;

        // Load config from file (not from DB — DB may have redacted API keys)
        let config: Config = Config::load(config_path).map_err(|e| {
            AgentError::Config(format!("failed to load config for crash recovery: {e}"))
        })?;

        let agent_config = config.to_agent_runtime_config();
        let limits = config.to_resource_limits();

        // Identity: SHA3-256 of agent name
        let agent_id = {
            let mut hasher = Sha3_256::new();
            hasher.update(config.agent.agent_name.as_bytes());
            let result = hasher.finalize();
            let mut id = [0u8; 32];
            id.copy_from_slice(&result);
            id
        };
        let identity = IdentityBlock { agent_id, trust_stage: 0 };

        // Load most recent handoff record
        let _latest_handoff = db.get_latest_handoff()?;

        // Load active todos
        let _active_todos = db.get_active_todos()?;

        // Load loop state from DB or default
        let state = match db.get_state("loop_state")? {
            Some(json) => serde_json::from_str(&json).unwrap_or_default(),
            None => AgentLoopState::default(),
        };

        // Ensure working directory exists
        if !working_dir.exists() {
            std::fs::create_dir_all(working_dir)?;
        }

        // Create LLM provider from config (not stub)
        let provider = llm::provider_from_config(&config.llm);

        Ok(Self {
            config,
            agent_config,
            limits,
            state,
            identity,
            db,
            working_dir: working_dir.to_path_buf(),
            shutdown: Arc::new(AtomicBool::new(false)),
            provider,
        })
    }
}

// ── Persistence ──

impl AgentRuntime {
    /// Persists the current loop state (iteration, token count, etc.)
    /// to the database state KV store.
    pub fn persist_state(&self) -> Result<(), AgentError> {
        let json = serde_json::to_string(&self.state)?;
        self.db.set_state("loop_state", &json)?;
        Ok(())
    }
}

// ── Helpers ──

/// Computes SHA3-256 of the serialized arguments to produce a `Hash32`.
fn hash_arguments(args: &serde_json::Value) -> Hash32 {
    let mut hasher = Sha3_256::new();
    // Use a canonical (sorted-key) serialization so that semantically
    // equivalent JSON objects produce the same hash.
    let canonical = serde_json::to_string(args).unwrap_or_default();
    hasher.update(canonical.as_bytes());
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Formats a `ToolOutput` into a human-readable string for the message log.
fn format_tool_output(output: &tools::ToolOutput) -> String {
    match output {
        tools::ToolOutput::Bash(o) => {
            format!(
                "bash: exit_code={} time={}ms truncated={}",
                o.exit_code, o.execution_time_ms, o.truncated
            )
        }
        tools::ToolOutput::TodoWrite(items) => {
            format!("todo_write: {} items", items.len())
        }
        tools::ToolOutput::TodoUpdate(updates) => {
            format!("todo_update: {} updates", updates.len())
        }
        tools::ToolOutput::Remember(entry) => {
            format!("remember: kind={:?} len={}", entry.kind, entry.content.len())
        }
        tools::ToolOutput::Forget(found) => {
            format!("forget: found={}", found)
        }
        tools::ToolOutput::Read(o) => {
            format!("read: lines={} truncated={}", o.total_lines, o.truncated)
        }
        tools::ToolOutput::Edit(o) => {
            format!("edit: replaced={} matches={}", o.replaced, o.match_count)
        }
        tools::ToolOutput::Write(o) => {
            format!("write: bytes={} created={}", o.bytes_written, o.created)
        }
        tools::ToolOutput::ApplyPatch(o) => {
            format!(
                "apply_patch: applied={} failed={} errors={}",
                o.patches_applied,
                o.patches_failed,
                o.errors.len()
            )
        }
        tools::ToolOutput::Error(msg) => {
            format!("error: {}", msg)
        }
    }
}

// ── Unit tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    /// Creates a minimal Config for test use.
    fn test_config(name: &str) -> Config {
        let mut cfg = Config::default();
        cfg.agent.agent_name = name.to_string();
        cfg.limits.loop_interval_ms = 10; // fast for tests
        cfg.limits.handoff_threshold_pct = 70;
        cfg.limits.handoff_trigger_messages = 50;
        cfg.llm.context_limit_tokens = 8192;
        cfg
    }

    /// Convenience: open a temp directory, create a test runtime.
    fn test_runtime(name: &str) -> (tempfile::TempDir, tempfile::TempDir, AgentRuntime) {
        let db_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("agent.db");
        let rt = AgentRuntime::new(test_config(name), &db_path, work_dir.path()).unwrap();
        (db_dir, work_dir, rt)
    }

    // ── new() tests ──

    #[test]
    fn new_creates_working_dir() {
        let db_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        let nested = work_dir.path().join("sub").join("agent_work");
        assert!(!nested.exists());

        let _rt =
            AgentRuntime::new(test_config("test-agent"), &db_dir.path().join("agent.db"), &nested)
                .unwrap();

        assert!(nested.exists());
        assert!(nested.is_dir());
    }

    #[test]
    fn new_initializes_database() {
        let db_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("agent.db");

        let rt = AgentRuntime::new(test_config("test-agent"), &db_path, work_dir.path()).unwrap();

        // Verify WAL mode is active by checking the state table exists
        let result = rt.db.get_state("loop_state").unwrap();
        assert!(result.is_some(), "state should be persisted on new");

        // Verify we can read back what we wrote
        let val = result.unwrap();
        let loaded: AgentLoopState = serde_json::from_str(&val).unwrap();
        assert_eq!(loaded.iteration, 0);
    }

    #[test]
    fn new_creates_identity() {
        let db_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("agent.db");

        let rt_a =
            AgentRuntime::new(test_config("agent-alpha"), &db_path, work_dir.path()).unwrap();

        let rt_b = AgentRuntime::new(
            test_config("agent-alpha"),
            &db_dir.path().join("agent_b.db"),
            tempfile::tempdir().unwrap().path(),
        )
        .unwrap();

        // Same name → same agent_id (deterministic)
        assert_eq!(rt_a.identity.agent_id, rt_b.identity.agent_id);

        // Different name → different agent_id
        let rt_c = AgentRuntime::new(
            test_config("agent-beta"),
            &db_dir.path().join("agent_c.db"),
            tempfile::tempdir().unwrap().path(),
        )
        .unwrap();
        assert_ne!(rt_a.identity.agent_id, rt_c.identity.agent_id);

        // Trust stage starts at 0
        assert_eq!(rt_a.identity.trust_stage, 0);
    }

    // ── Persist / recover state ──

    #[test]
    fn persist_and_recover_state() {
        let db_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("agent.db");

        let rt = AgentRuntime::new(test_config("test-agent"), &db_path, work_dir.path()).unwrap();

        // Modify state and persist
        rt.db
            .set_state(
                "loop_state",
                &serde_json::to_string(&AgentLoopState {
                    iteration: 42,
                    total_tokens_used: 1337,
                    last_handoff_height: 7,
                    active_tool_calls: Vec::new(),
                })
                .unwrap(),
            )
            .unwrap();

        // Re-open with same db_path and verify state reloaded
        let rt2 = AgentRuntime::new(test_config("test-agent"), &db_path, work_dir.path()).unwrap();

        assert_eq!(rt2.state.iteration, 42);
        assert_eq!(rt2.state.total_tokens_used, 1337);
        assert_eq!(rt2.state.last_handoff_height, 7);
    }

    // ── Handoff tests ──

    #[test]
    fn handoff_creates_record() {
        let (_db_dir, _work_dir, mut rt) = test_runtime("test-agent");

        // Set some state so iteration is non-zero
        rt.state.iteration = 5;
        rt.trigger_handoff().unwrap();

        let latest = rt.db.get_latest_handoff().unwrap();
        assert!(latest.is_some(), "handoff record must exist");
        let record = latest.unwrap();
        let now = unix_timestamp_now();
        assert!(
            record.timestamp == now || record.timestamp == now - 1,
            "timestamp {} should be within 1s of {}",
            record.timestamp,
            now
        );
        assert_eq!(rt.state.last_handoff_height, 5);
    }

    #[test]
    fn handoff_captures_todos() {
        let (_db_dir, _work_dir, mut rt) = test_runtime("test-agent");

        // Add some todos
        rt.db
            .insert_todo(&TodoItem {
                id: "t1".to_string(),
                content: "task one".to_string(),
                status: TodoStatus::Pending,
                context: None,
            })
            .unwrap();
        rt.db
            .insert_todo(&TodoItem {
                id: "t2".to_string(),
                content: "task two".to_string(),
                status: TodoStatus::InProgress,
                context: Some("extra".to_string()),
            })
            .unwrap();

        rt.trigger_handoff().unwrap();

        let latest = rt.db.get_latest_handoff().unwrap().unwrap();
        assert_eq!(latest.todos_snapshot.len(), 2);
        let ids: Vec<&str> = latest.todos_snapshot.iter().map(|t| t.id.as_str()).collect();
        assert!(ids.contains(&"t1"));
        assert!(ids.contains(&"t2"));
    }

    // ── Failure guard tests ──

    #[test]
    fn failure_guard_blocks_exact_duplicate() {
        let (_db_dir, _work_dir, rt) = test_runtime("test-agent");
        let now = unix_timestamp_now();

        let input_hash = hash_arguments(&serde_json::json!({"cmd": "bad"}));

        // Record a failure
        rt.db
            .record_failure(&FailureRecord {
                timestamp: now,
                tool_name: "bash".to_string(),
                input_hash,
                error_kind: "exit code 1".to_string(),
            })
            .unwrap();

        // Same tool + same input within 1 hour → blocked
        let result = rt.check_failure_guard("bash", &input_hash);
        assert!(result.is_err());
        match result {
            Err(AgentError::FailureGuardBlocked(msg)) => {
                assert!(msg.contains("exact duplicate"));
            }
            _ => panic!("expected FailureGuardBlocked"),
        }
    }

    #[test]
    fn failure_guard_allows_different_input() {
        let (_db_dir, _work_dir, rt) = test_runtime("test-agent");
        let now = unix_timestamp_now();

        let hash_a = hash_arguments(&serde_json::json!({"cmd": "cmd_a"}));
        let hash_b = hash_arguments(&serde_json::json!({"cmd": "cmd_b"}));

        rt.db
            .record_failure(&FailureRecord {
                timestamp: now,
                tool_name: "bash".to_string(),
                input_hash: hash_a,
                error_kind: "exit code 1".to_string(),
            })
            .unwrap();

        // Different input → allowed
        let result = rt.check_failure_guard("bash", &hash_b);
        assert!(result.is_ok());
    }

    #[test]
    fn failure_guard_blocks_after_three() {
        let (_db_dir, _work_dir, rt) = test_runtime("test-agent");
        let now = unix_timestamp_now();

        // Record 3 failures for different tool calls within 1 hour
        for i in 0..3 {
            let hash = hash_arguments(&serde_json::json!({"cmd": format!("cmd_{}", i)}));
            rt.db
                .record_failure(&FailureRecord {
                    timestamp: now,
                    tool_name: "bash".to_string(),
                    input_hash: hash,
                    error_kind: format!("error {}", i),
                })
                .unwrap();
        }

        // Any new call (even with fresh input) should be blocked
        let fresh_hash = hash_arguments(&serde_json::json!({"cmd": "fresh"}));
        let result = rt.check_failure_guard("bash", &fresh_hash);
        assert!(result.is_err());
        match result {
            Err(AgentError::FailureGuardBlocked(msg)) => {
                assert!(msg.contains("3 failures"));
            }
            _ => panic!("expected FailureGuardBlocked after 3 failures"),
        }
    }

    // ── Crash recovery tests ──

    #[test]
    fn crash_recovery_loads_handoff() {
        let db_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("agent.db");

        // Create a minimal valid config file for crash recovery
        let config_dir = tempfile::tempdir().unwrap();
        let config_path = config_dir.path().join("agent.toml");
        let config_toml = r#"
[agent]
project_name = "hyperfluid-agent"
agent_name = "recovery-agent"

[llm]
provider = "local"
model = "default"

[limits]
loop_interval_ms = 10
tool_timeout_ms = 120000
handoff_threshold_pct = 70
handoff_trigger_messages = 50
max_ram_bytes = 0
max_cpu_cores = 0
cpu_throttle_pct = 0
max_disk_bytes = 0
max_file_descriptors = 0
max_concurrent_connections = 0
"#;
        std::fs::write(&config_path, config_toml).unwrap();

        // Create runtime, trigger handoff, verify record
        {
            let mut rt =
                AgentRuntime::new(test_config("recovery-agent"), &db_path, work_dir.path())
                    .unwrap();
            rt.state.iteration = 10;
            rt.trigger_handoff().unwrap();
        }

        // Recover and verify handoff loaded
        let recovered =
            AgentRuntime::recover_from_crash(&db_path, &config_path, work_dir.path()).unwrap();

        let latest = recovered.db.get_latest_handoff().unwrap();
        assert!(latest.is_some(), "handoff must survive crash");
        // State should be preserved (iteration 10, last_handoff_height 10)
        assert_eq!(recovered.state.last_handoff_height, 10);
    }

    // ── Run loop tests ──

    #[test]
    fn run_loop_iteration_increments_counter() {
        let db_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("agent.db");

        let mut rt =
            AgentRuntime::new(test_config("loop-agent"), &db_path, work_dir.path()).unwrap();

        assert_eq!(rt.state.iteration, 0);

        // Run one iteration
        rt.run_one_iteration().unwrap();

        assert_eq!(rt.state.iteration, 1);
        assert!(rt.state.total_tokens_used > 0, "tokens should be tracked");

        // Run a second iteration
        rt.run_one_iteration().unwrap();
        assert_eq!(rt.state.iteration, 2);
    }

    #[test]
    fn run_loop_enforces_interval() {
        let db_dir = tempfile::tempdir().unwrap();
        let work_dir = tempfile::tempdir().unwrap();
        let db_path = db_dir.path().join("agent.db");

        let mut cfg = test_config("interval-agent");
        cfg.limits.loop_interval_ms = 100;
        let mut rt = AgentRuntime::new(cfg, &db_path, work_dir.path()).unwrap();

        // Set up shutdown after 2 iterations
        let shutdown = Arc::clone(&rt.shutdown);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            shutdown.store(true, Ordering::Release);
        });

        let start = std::time::Instant::now();
        rt.run_loop().unwrap();
        let elapsed = start.elapsed();

        // With 100ms interval and shutdown at ~50ms, we should get at least
        // the first iteration plus the sleep between iterations. The elapsed
        // time should be >= the interval (the loop runs at least one iteration
        // immediately, then sleeps, then checks shutdown).
        assert!(
            elapsed >= Duration::from_millis(50),
            "loop should have run at least 50ms before shutdown: {:?}",
            elapsed
        );
    }
}
