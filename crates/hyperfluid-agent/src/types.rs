// === C10 Agent Runtime: Core Types ===
//
// Source: docs/04-specifications/runtime/agent-runtime-spec.md Sections 1.3, 2.3, 3.3, 4.3

use serde::{Deserialize, Serialize};

pub type Hash32 = [u8; 32];

// ── Section 1.3: Agent Loop Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRuntimeConfig {
    pub model_provider: String,
    pub model_name: String,
    pub context_limit_tokens: u32,
    pub loop_interval_ms: u64,
    pub tool_timeout_ms: u64,
    pub handoff_threshold_pct: u8,
    pub handoff_trigger_messages: u32,
}

impl Default for AgentRuntimeConfig {
    fn default() -> Self {
        Self {
            model_provider: "local".to_string(),
            model_name: "default".to_string(),
            context_limit_tokens: 8192,
            loop_interval_ms: 5000,
            tool_timeout_ms: 120000,
            handoff_threshold_pct: 70,
            handoff_trigger_messages: 50,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentLoopState {
    pub iteration: u64,
    pub total_tokens_used: u64,
    pub last_handoff_height: u64,
    pub active_tool_calls: Vec<ToolCallExecution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandoffRecord {
    pub session_id: Hash32,
    pub timestamp: u64,
    pub summary: Vec<u8>,
    pub next_actions: Vec<NextAction>,
    pub todos_snapshot: Vec<TodoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NextAction {
    pub priority: u8,
    pub description: String,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallExecution {
    pub tool_name: String,
    pub input_hash: Hash32,
    pub started_at: u64,
    pub timeout_ms: u64,
}

// ── Section 2.3: Tool Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashToolInput {
    pub command: String,
    pub working_dir: Option<String>,
    pub timeout: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BashToolOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub exit_code: i32,
    pub truncated: bool,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoWriteInput {
    pub items: Vec<TodoItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoUpdateInput {
    pub updates: Vec<TodoUpdateEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
    pub context: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Blocked,
    Done,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoUpdateEntry {
    pub id: String,
    pub new_status: TodoStatus,
    pub context_update: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RememberInput {
    pub kind: KnowledgeKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum KnowledgeKind {
    Finding,
    Pattern,
    Constraint,
    Decision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEntry {
    pub id: Hash32,
    pub kind: KnowledgeKind,
    pub content: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub last_read_at: u64,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgetInput {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadToolInput {
    pub file_path: String,
    pub offset: Option<u64>,
    pub limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadToolOutput {
    pub content: Vec<u8>,
    pub total_lines: u64,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditToolInput {
    pub file_path: String,
    pub old_string: String,
    pub new_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditToolOutput {
    pub replaced: bool,
    pub match_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteToolInput {
    pub file_path: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteToolOutput {
    pub bytes_written: u64,
    pub created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchInput {
    pub patches: Vec<EditToolInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyPatchOutput {
    pub patches_applied: u32,
    pub patches_failed: u32,
    pub errors: Vec<String>,
}

// ── Section 3.3: Context Envelope ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextEnvelope {
    pub identity_block: Vec<u8>,
    pub recent_messages: Vec<u8>,
    pub tool_specs: Vec<u8>,
    pub reserve: Vec<u8>,
}

// ── Section 4.3: Resource Limits ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_ram_bytes: u64,
    pub max_cpu_cores: u8,
    pub cpu_throttle_pct: u8,
    pub max_disk_bytes: u64,
    pub max_file_descriptors: u32,
    pub max_concurrent_connections: u32,
    pub max_context_tokens: u32,
    pub tool_timeout_ms: u64,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_ram_bytes: 4 * 1024 * 1024 * 1024,
            max_cpu_cores: 2,
            cpu_throttle_pct: 80,
            max_disk_bytes: 10 * 1024 * 1024 * 1024,
            max_file_descriptors: 1024,
            max_concurrent_connections: 100,
            max_context_tokens: 8192,
            tool_timeout_ms: 120000,
        }
    }
}

// ── Failure guard types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecord {
    pub timestamp: u64,
    pub tool_name: String,
    pub input_hash: Hash32,
    pub error_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityBlock {
    pub agent_id: Hash32,
    pub trust_stage: u8,
}

// ── LLM provider interface types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub system_prompt: String,
    pub messages: Vec<LlmMessage>,
    pub max_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub tokens_used: u32,
    pub finish_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool_name: String,
    pub arguments: serde_json::Value,
}
