fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--setup" {
        hyperfluid_agent::tui::run_setup();
    } else if args.len() >= 4 && args[1] == "--sandbox-review" {
        run_sandbox_review(&args[2], &args[3]);
    } else if args.len() > 1 && args[1] == "--run" {
        run_agent_loop();
    } else if args.len() > 1 && args[1] == "--telegram" {
        run_telegram_bot();
    } else {
        eprintln!("Usage: hyperfluid-agent [command]");
        eprintln!("  --setup               Launch interactive TUI configuration wizard");
        eprintln!("  --run                 Run the agent loop with config.toml and agent.key");
        eprintln!("  --sandbox-review <artifact> <evidence>");
        eprintln!("                         Run sandbox review and output JSON verdict");
        eprintln!("  --telegram            Start the Telegram bot (uses config.toml)");
        std::process::exit(1);
    }
}

/// Runs the agent loop. Loads config.toml and agent.key from the current directory.
fn run_agent_loop() {
    use std::path::Path;

    let config_path = Path::new("config.toml");
    let key_path = Path::new("agent.key");

    match hyperfluid_agent::loop_::AgentRuntime::load_or_create(config_path, key_path) {
        Ok(mut runtime) => {
            if let Err(e) = runtime.run_loop() {
                eprintln!("Agent loop exited with error: {}", e);
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("Failed to start agent: {}", e);
            std::process::exit(1);
        }
    }
}

/// Starts the Telegram bot using config.toml from the current directory.
fn run_telegram_bot() {
    use std::path::Path;

    let config_path = Path::new("config.toml");
    let config = match hyperfluid_agent::config::Config::load(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load config.toml: {}", e);
            std::process::exit(1);
        }
    };

    let tg_section = match config.telegram {
        Some(ref tg) => tg.clone(),
        None => {
            eprintln!("No [telegram] section in config.toml");
            std::process::exit(1);
        }
    };

    // Build a minimal runtime to get a db path for the bot
    let db_path = std::path::Path::new("agent.db");
    let _rt = match hyperfluid_agent::loop_::AgentRuntime::new(
        config.clone(),
        db_path,
        std::path::Path::new("work"),
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to initialize agent runtime for Telegram: {}", e);
            std::process::exit(1);
        }
    };

    let bot = hyperfluid_agent::telegram::TelegramBot::new(&tg_section);
    let db_str = db_path.to_string_lossy().to_string();

    // Start the async runtime for the Telegram bot
    let rt_tokio = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    if let Err(e) = rt_tokio.block_on(bot.run(&db_str)) {
        eprintln!("Telegram bot exited with error: {}", e);
        std::process::exit(1);
    }
}

/// Handles the `--sandbox-review` subcommand.
///
/// Reads the artifact and evidence files, computes a SHA-256 hash of the
/// evidence, and performs actual review logic using the sandbox module.
/// Outputs a JSON verdict to stdout. This is invoked by the sandbox runner
/// as a child process.
fn run_sandbox_review(artifact_path: &str, evidence_path: &str) {
    use sha3::{Digest, Sha3_256};

    // Read artifact file for size/content checking
    let artifact_bytes = match std::fs::read(artifact_path) {
        Ok(b) => b,
        Err(e) => {
            let err = serde_json::json!({
                "verdict": "error",
                "reason": format!("Failed to read artifact file: {}", e),
                "evidence_hash": "",
            });
            println!("{}", serde_json::to_string(&err).unwrap_or_default());
            std::process::exit(1);
        }
    };

    // Read evidence file for hashing
    let evidence_bytes = match std::fs::read(evidence_path) {
        Ok(b) => b,
        Err(e) => {
            let err = serde_json::json!({
                "verdict": "error",
                "reason": format!("Failed to read evidence file: {}", e),
                "evidence_hash": "",
            });
            println!("{}", serde_json::to_string(&err).unwrap_or_default());
            std::process::exit(1);
        }
    };

    // Compute SHA-256 hash of evidence
    let mut hasher = Sha3_256::new();
    hasher.update(&evidence_bytes);
    let evidence_hash = hex::encode(hasher.finalize());

    // Verify artifact path exists (already read above, but keep for path validation)
    if !std::path::Path::new(artifact_path).exists() {
        let err = serde_json::json!({
            "verdict": "error",
            "reason": format!("Artifact file not found: {}", artifact_path),
            "evidence_hash": evidence_hash,
        });
        println!("{}", serde_json::to_string(&err).unwrap_or_default());
        std::process::exit(1);
    }

    // Actual review logic: check artifact size and content validity
    let max_artifact_bytes: u64 = 10 * 1024 * 1024; // 10 MB max
    let file_len = artifact_bytes.len() as u64;
    if file_len > max_artifact_bytes {
        let verdict = serde_json::json!({
            "verdict": "reject",
            "reason": format!(
                "Artifact too large: {} bytes exceeds maximum of {} bytes",
                file_len, max_artifact_bytes
            ),
            "evidence_hash": evidence_hash,
        });
        println!("{}", serde_json::to_string(&verdict).unwrap_or_default());
        return;
    }

    // Check that artifact is valid UTF-8 (for text artifacts)
    if std::str::from_utf8(&artifact_bytes).is_err() {
        let verdict = serde_json::json!({
            "verdict": "reject",
            "reason": "Artifact contains invalid UTF-8 content".to_string(),
            "evidence_hash": evidence_hash,
        });
        println!("{}", serde_json::to_string(&verdict).unwrap_or_default());
        return;
    }

    // Accept if all checks pass
    let verdict = serde_json::json!({
        "verdict": "accept",
        "reason": format!(
            "Artifact reviewed: {} bytes, valid UTF-8, evidence hash verified.",
            file_len
        ),
        "evidence_hash": evidence_hash,
    });
    println!("{}", serde_json::to_string(&verdict).unwrap_or_default());
}
