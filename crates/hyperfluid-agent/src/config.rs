// === C10 Agent Runtime: Configuration ===
//
// Source: docs/04-specifications/runtime/agent-runtime-spec.md Section 5
// TOML-deserializable config that maps to AgentRuntimeConfig + ResourceLimits.

use crate::types::{AgentRuntimeConfig, ResourceLimits};
use serde::{Deserialize, Serialize};
use std::path::Path;

// === Section structs ===

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub agent: AgentSection,
    pub llm: LlmSection,
    #[serde(default)]
    pub telegram: Option<TelegramSection>,
    pub limits: LimitsSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AgentSection {
    pub project_name: String,
    pub agent_name: String,
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub capability_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LlmSection {
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub api_url: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_context_limit")]
    pub context_limit_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelegramSection {
    pub token: String,
    #[serde(default)]
    pub user_id: u64,
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitsSection {
    #[serde(default = "default_loop_interval")]
    pub loop_interval_ms: u64,
    #[serde(default = "default_tool_timeout")]
    pub tool_timeout_ms: u64,
    #[serde(default = "default_handoff_threshold")]
    pub handoff_threshold_pct: u8,
    #[serde(default = "default_handoff_trigger")]
    pub handoff_trigger_messages: u32,
    pub max_ram_bytes: u64,
    pub max_cpu_cores: u8,
    pub cpu_throttle_pct: u8,
    pub max_disk_bytes: u64,
    pub max_file_descriptors: u32,
    pub max_concurrent_connections: u32,
}

// === Default helpers ===

const fn default_context_limit() -> u32 {
    8192
}
const fn default_loop_interval() -> u64 {
    5000
}
const fn default_tool_timeout() -> u64 {
    120_000
}
const fn default_handoff_threshold() -> u8 {
    70
}
const fn default_handoff_trigger() -> u32 {
    50
}

// === ConfigError ===

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Parse(String),

    #[error("missing required field: {0}")]
    MissingField(String),
}

// === impl Config ===

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config =
            toml::from_str(&content).map_err(|e| ConfigError::Parse(e.to_string()))?;
        Ok(config)
    }

    pub fn to_agent_runtime_config(&self) -> AgentRuntimeConfig {
        AgentRuntimeConfig {
            model_provider: self.llm.provider.clone(),
            model_name: self.llm.model.clone(),
            context_limit_tokens: self.llm.context_limit_tokens,
            loop_interval_ms: self.limits.loop_interval_ms,
            tool_timeout_ms: self.limits.tool_timeout_ms,
            handoff_threshold_pct: self.limits.handoff_threshold_pct,
            handoff_trigger_messages: self.limits.handoff_trigger_messages,
        }
    }

    pub fn to_resource_limits(&self) -> ResourceLimits {
        ResourceLimits {
            max_ram_bytes: self.limits.max_ram_bytes,
            max_cpu_cores: self.limits.max_cpu_cores,
            cpu_throttle_pct: self.limits.cpu_throttle_pct,
            max_disk_bytes: self.limits.max_disk_bytes,
            max_file_descriptors: self.limits.max_file_descriptors,
            max_concurrent_connections: self.limits.max_concurrent_connections,
            max_context_tokens: self.llm.context_limit_tokens,
            tool_timeout_ms: self.limits.tool_timeout_ms,
        }
    }
}

// === Default ===

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: AgentSection {
                project_name: "hyperfluid-agent".into(),
                agent_name: "unnamed".into(),
                agent_id: None,
                capability_tags: Vec::new(),
            },
            llm: LlmSection {
                provider: "local".into(),
                model: "default".into(),
                api_url: None,
                api_key: None,
                context_limit_tokens: 8192,
            },
            telegram: None,
            limits: LimitsSection {
                loop_interval_ms: 5000,
                tool_timeout_ms: 120_000,
                handoff_threshold_pct: 70,
                handoff_trigger_messages: 50,
                max_ram_bytes: 4 * 1024 * 1024 * 1024,
                max_cpu_cores: 2,
                cpu_throttle_pct: 80,
                max_disk_bytes: 10 * 1024 * 1024 * 1024,
                max_file_descriptors: 1024,
                max_concurrent_connections: 100,
            },
        }
    }
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conforms_to_agent_runtime_spec_section_5_default_config() {
        let cfg = Config::default();
        assert_eq!(cfg.agent.project_name, "hyperfluid-agent");
        assert_eq!(cfg.agent.agent_name, "unnamed");
        assert!(cfg.agent.agent_id.is_none());
        assert_eq!(cfg.llm.provider, "local");
        assert_eq!(cfg.llm.context_limit_tokens, 8192);
        assert!(cfg.telegram.is_none());
        assert_eq!(cfg.limits.loop_interval_ms, 5000);
        assert_eq!(cfg.limits.tool_timeout_ms, 120_000);
        assert_eq!(cfg.limits.max_ram_bytes, 4 * 1024 * 1024 * 1024);
    }

    #[test]
    fn conforms_to_agent_runtime_spec_section_5_roundtrip_mappings() {
        let cfg = Config {
            agent: AgentSection {
                project_name: "test-project".into(),
                agent_name: "builder-01".into(),
                agent_id: Some("abc123".into()),
                capability_tags: vec!["build".into(), "test".into()],
            },
            llm: LlmSection {
                provider: "openai".into(),
                model: "gpt-4".into(),
                api_url: Some("https://api.example.com".into()),
                api_key: Some("sk-test".into()),
                context_limit_tokens: 32768,
            },
            telegram: Some(TelegramSection {
                token: "tg-token".into(),
                user_id: 42,
                enabled: true,
            }),
            limits: LimitsSection {
                loop_interval_ms: 10_000,
                tool_timeout_ms: 60_000,
                handoff_threshold_pct: 85,
                handoff_trigger_messages: 100,
                max_ram_bytes: 8 * 1024 * 1024 * 1024,
                max_cpu_cores: 4,
                cpu_throttle_pct: 50,
                max_disk_bytes: 50 * 1024 * 1024 * 1024,
                max_file_descriptors: 4096,
                max_concurrent_connections: 500,
            },
        };

        let arc = cfg.to_agent_runtime_config();
        assert_eq!(arc.model_provider, "openai");
        assert_eq!(arc.model_name, "gpt-4");
        assert_eq!(arc.context_limit_tokens, 32768);
        assert_eq!(arc.loop_interval_ms, 10_000);
        assert_eq!(arc.tool_timeout_ms, 60_000);
        assert_eq!(arc.handoff_threshold_pct, 85);
        assert_eq!(arc.handoff_trigger_messages, 100);

        let rl = cfg.to_resource_limits();
        assert_eq!(rl.max_ram_bytes, 8 * 1024 * 1024 * 1024);
        assert_eq!(rl.max_cpu_cores, 4);
        assert_eq!(rl.cpu_throttle_pct, 50);
        assert_eq!(rl.max_disk_bytes, 50 * 1024 * 1024 * 1024);
        assert_eq!(rl.max_file_descriptors, 4096);
        assert_eq!(rl.max_concurrent_connections, 500);
        assert_eq!(rl.max_context_tokens, 32768);
        assert_eq!(rl.tool_timeout_ms, 60_000);
    }

    #[test]
    fn conforms_to_agent_runtime_spec_section_5_parse_valid_toml() {
        let toml_content = r#"
[agent]
project_name = "hyperfluid"
agent_name = "test-agent"
agent_id = "id-hex"
capability_tags = ["build", "review"]

[llm]
provider = "anthropic"
model = "claude-3"
api_url = "https://api.anthropic.com"
context_limit_tokens = 16384

[telegram]
token = "tg-bot-token"
user_id = 99
enabled = true

[limits]
loop_interval_ms = 3000
tool_timeout_ms = 90000
handoff_threshold_pct = 75
handoff_trigger_messages = 40
max_ram_bytes = 2147483648
max_cpu_cores = 2
cpu_throttle_pct = 60
max_disk_bytes = 5368709120
max_file_descriptors = 512
max_concurrent_connections = 50
"#;

        let cfg: Config = toml::from_str(toml_content).expect("valid toml must parse");
        assert_eq!(cfg.agent.project_name, "hyperfluid");
        assert_eq!(cfg.agent.capability_tags.len(), 2);
        assert_eq!(cfg.llm.provider, "anthropic");
        assert_eq!(cfg.llm.context_limit_tokens, 16384);
        assert!(cfg.telegram.is_some());
        let tg = cfg.telegram.unwrap();
        assert_eq!(tg.token, "tg-bot-token");
        assert_eq!(cfg.limits.max_ram_bytes, 2_147_483_648);
    }

    #[test]
    fn conforms_to_agent_runtime_spec_section_5_default_fills_optional_fields() {
        let toml_content = r#"
[agent]
project_name = "minimal"
agent_name = "min-agent"

[llm]
provider = "local"
model = "tiny"

[limits]
max_ram_bytes = 1073741824
max_cpu_cores = 1
cpu_throttle_pct = 30
max_disk_bytes = 1073741824
max_file_descriptors = 256
max_concurrent_connections = 10
"#;

        let cfg: Config = toml::from_str(toml_content).expect("minimal toml must parse");
        assert_eq!(cfg.agent.capability_tags.len(), 0);
        assert!(cfg.agent.agent_id.is_none());
        assert!(cfg.telegram.is_none());
        assert_eq!(cfg.llm.context_limit_tokens, 8192);
        assert_eq!(cfg.limits.loop_interval_ms, 5000);
        assert_eq!(cfg.limits.tool_timeout_ms, 120_000);
        assert_eq!(cfg.limits.handoff_threshold_pct, 70);
        assert_eq!(cfg.limits.handoff_trigger_messages, 50);
    }

    #[test]
    fn conforms_to_agent_runtime_spec_section_5_rejects_unknown_fields() {
        let toml_content = r#"
[agent]
project_name = "bad"
agent_name = "bad-agent"
unknown_field = "intruder"

[llm]
provider = "local"
model = "tiny"

[limits]
max_ram_bytes = 1073741824
max_cpu_cores = 1
cpu_throttle_pct = 30
max_disk_bytes = 1073741824
max_file_descriptors = 256
max_concurrent_connections = 10
"#;

        let result: Result<Config, _> = toml::from_str(toml_content);
        assert!(result.is_err(), "unknown fields must be rejected");
    }

    #[test]
    fn conforms_to_agent_runtime_spec_section_5_rejects_missing_required_fields() {
        let toml_content = r#"
[agent]
agent_name = "no-project"

[llm]
model = "tiny"

[limits]
max_ram_bytes = 1073741824
max_cpu_cores = 1
cpu_throttle_pct = 30
max_disk_bytes = 1073741824
max_file_descriptors = 256
max_concurrent_connections = 10
"#;

        let result: Result<Config, _> = toml::from_str(toml_content);
        assert!(result.is_err(), "missing required fields must fail");
    }
}
