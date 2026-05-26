// === C10 Agent Runtime: TUI Setup Wizard ===
//
// Interactive terminal configuration wizard for hyperfluid-agent.
// Collects setup fields and writes config.toml.

use crate::config::{AgentSection, Config, LimitsSection, LlmSection, TelegramSection};
use crossterm::{
    cursor, execute,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
};
use std::io::{stdin, stdout, Read, Write};
use std::path::Path;

/// Default resource limits used when no config exists.
fn default_limits() -> LimitsSection {
    LimitsSection {
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
    }
}

/// Read a single line of input in raw mode with character echo.
fn read_line_raw() -> String {
    let mut input = String::new();
    let mut handle = stdin().lock();
    let mut buf = [0u8; 1];

    loop {
        if handle.read_exact(&mut buf).is_err() {
            break;
        }
        match buf[0] {
            b'\r' | b'\n' => {
                let _ = stdout().write_all(b"\r\n");
                let _ = stdout().flush();
                break;
            }
            b'\x7f' | b'\x08' => {
                // backspace — remove last character if any
                if input.pop().is_some() {
                    let _ = stdout().write_all(b"\x08 \x08");
                    let _ = stdout().flush();
                }
            }
            c if (0x20..=0x7e).contains(&c) => {
                // printable ASCII
                input.push(c as char);
                let _ = stdout().write_all(&[c]);
                let _ = stdout().flush();
            }
            _ => {
                // ignore other control characters
            }
        }
    }
    input
}

/// Prompt the user for a field with an optional default value.
/// Returns the user input trimmed, or the default if empty.
fn prompt_field(prompt: &str, default: Option<&str>) -> String {
    let mut out = stdout();
    if let Some(d) = default {
        let _ = write!(out, "{} [{}]: ", prompt, d);
    } else {
        let _ = write!(out, "{}: ", prompt);
    }
    let _ = out.flush();

    let input = read_line_raw();
    let trimmed = input.trim().to_string();

    if trimmed.is_empty() {
        default.unwrap_or("").to_string()
    } else {
        trimmed
    }
}

/// Run the interactive TUI setup wizard.
///
/// 1. Reads existing `config.toml` if present to pre-fill defaults.
/// 2. Prompts each field in raw mode with echo.
/// 3. Serializes the result to TOML and writes `config.toml`.
pub fn run_setup() {
    if let Err(e) = enable_raw_mode() {
        eprintln!("Failed to enable raw mode: {}. Falling back to line input.", e);
    }

    let mut out = stdout();
    let _ = execute!(out, Clear(ClearType::All), cursor::MoveTo(0, 0));
    let _ = writeln!(out, "╔══════════════════════════════════════╗");
    let _ = writeln!(out, "║     Hyperfluid Agent Setup          ║");
    let _ = writeln!(out, "╚══════════════════════════════════════╝");
    let _ = writeln!(out);
    let _ = out.flush();

    // ── Load existing config for pre-fill ──
    let existing = if Path::new("config.toml").exists() {
        std::fs::read_to_string("config.toml").ok().and_then(|s| toml::from_str::<Config>(&s).ok())
    } else {
        None
    };

    // Extract default values as owned strings
    let def_project = existing.as_ref().map(|c| c.agent.project_name.clone());
    let def_agent = existing.as_ref().map(|c| c.agent.agent_name.clone());
    let def_provider = existing.as_ref().map(|c| c.llm.provider.clone());
    let def_url = existing.as_ref().and_then(|c| c.llm.api_url.clone());
    let def_key = existing.as_ref().and_then(|c| c.llm.api_key.clone());
    let def_tags = existing.as_ref().and_then(|c| {
        if c.agent.capability_tags.is_empty() {
            None
        } else {
            Some(c.agent.capability_tags.join(","))
        }
    });
    let def_tg_token = existing.as_ref().and_then(|c| c.telegram.as_ref().map(|t| t.token.clone()));

    // ── Prompt fields ──
    let project_name = prompt_field("Project Name", def_project.as_deref());
    let agent_name = prompt_field("Agent Name", def_agent.as_deref());
    let llm_provider = prompt_field("LLM Provider (openai/ollama)", def_provider.as_deref());
    let api_url = prompt_field("API URL", def_url.as_deref());
    let api_key = prompt_field("API Key (optional)", def_key.as_deref());
    let capability_tags_str =
        prompt_field("Capability Tags (comma-separated)", def_tags.as_deref());
    let telegram_token_str = prompt_field("Telegram Token (optional)", def_tg_token.as_deref());

    // ── Parse capability tags ──
    let capability_tags: Vec<String> = if capability_tags_str.trim().is_empty() {
        Vec::new()
    } else {
        capability_tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    };

    // ── Build Telegram section ──
    let telegram = if telegram_token_str.is_empty() {
        None
    } else {
        let user_id_str = prompt_field("Telegram User ID (numeric)", None);
        let user_id: u64 = user_id_str.parse().unwrap_or(0);
        Some(TelegramSection { token: telegram_token_str, user_id, enabled: true })
    };

    // ── Build config ──
    let config = Config {
        agent: AgentSection {
            project_name,
            agent_name,
            agent_id: existing.as_ref().and_then(|c| c.agent.agent_id.clone()),
            capability_tags,
        },
        llm: LlmSection {
            provider: llm_provider,
            model: "default".into(),
            api_url: if api_url.is_empty() { None } else { Some(api_url) },
            api_key: if api_key.is_empty() { None } else { Some(api_key) },
            context_limit_tokens: existing
                .as_ref()
                .map(|c| c.llm.context_limit_tokens)
                .unwrap_or(8192),
        },
        telegram,
        limits: existing.as_ref().map(|c| c.limits.clone()).unwrap_or_else(default_limits),
    };

    // ── Serialize and write ──
    let toml_str = match toml::to_string_pretty(&config) {
        Ok(s) => s,
        Err(e) => {
            let _ = writeln!(stdout(), "\nFailed to serialize config: {}", e);
            let _ = stdout().flush();
            return;
        }
    };
    if let Err(e) = std::fs::write("config.toml", &toml_str) {
        let _ = writeln!(stdout(), "\n❌ Failed to write config.toml: {}", e);
    } else {
        let _ = writeln!(stdout(), "\n✅ config.toml written successfully.");
    }

    // Restore terminal
    let _ = disable_raw_mode();
}
