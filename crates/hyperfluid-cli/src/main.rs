// === Hyperfluid CLI ===
//
// Source: docs/05-planning/stages/stage-02-agent-runtime.md Week 9-10
//
// 8 top-level subcommands: tx, query, task, review, governance, fastpath, agent, idea
// All mutating commands route through PDP validation.
// Machine-parseable JSON output via `--output json`.

mod commands;

use clap::{Parser, Subcommand};
use std::process;

#[derive(Parser)]
#[command(name = "hyperfluid", version, about = "Hyperfluid network CLI")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Output format: text (default) or json
    #[arg(long, global = true, default_value = "text")]
    output: OutputFormat,

    /// Path to ML-DSA-65 seed file (32 bytes as hex). Default: ~/.hyperfluid/agent.key
    #[arg(long, global = true)]
    key_file: Option<String>,

    /// Raw 32-byte hex seed for ML-DSA-65 identity (alternative to --key-file)
    #[arg(long, global = true)]
    key_hex: Option<String>,
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
enum Command {
    /// Transaction commands: transfer, bond, unbond, withdraw, renew, delegate, undelegate, withdraw-delegation, commission, evidence
    Tx {
        #[command(subcommand)]
        action: commands::tx::TxAction,
    },
    /// Query chain state: balance, nonce, state-root, block, validator, committee, git-head, fee-estimate
    Query {
        #[command(subcommand)]
        action: commands::query::QueryAction,
    },
    /// Task management: submit, status, claim, heartbeat, release, split, list, get
    Task {
        #[command(subcommand)]
        action: commands::task::TaskAction,
    },
    /// Review management: claim, verdict, list
    Review {
        #[command(subcommand)]
        action: commands::review::ReviewAction,
    },
    /// Governance: propose, vote, list, get
    Governance {
        #[command(subcommand)]
        action: commands::governance::GovernanceAction,
    },
    /// Fast-Path topic merges: list, propose, approve, challenge, status
    FastPath {
        #[command(subcommand)]
        action: commands::fastpath::FastPathAction,
    },
    /// Agent management: status, register, list-skills, load-skill
    Agent {
        #[command(subcommand)]
        action: commands::agent::AgentAction,
    },
    /// Idea seed index: list, show
    Idea {
        #[command(subcommand)]
        action: commands::idea::IdeaAction,
    },
}

fn main() {
    let cli = Cli::parse();
    let node_url =
        std::env::var("HYPERFLUID_NODE_URL").unwrap_or_else(|_| "http://127.0.0.1:8545".into());

    let identity = match commands::load_identity(cli.key_file.as_deref(), cli.key_hex.as_deref()) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Error loading identity: {}", e);
            process::exit(1);
        }
    };

    let client = reqwest::blocking::Client::new();

    let result = match cli.command {
        Command::Tx { action } => {
            commands::tx::run(action, cli.output, &client, &node_url, &identity)
        }
        Command::Query { action } => commands::query::run(action, cli.output, &client, &node_url),
        Command::Task { action } => {
            commands::task::run(action, cli.output, &client, &node_url, &identity)
        }
        Command::Review { action } => {
            commands::review::run(action, cli.output, &client, &node_url, &identity)
        }
        Command::Governance { action } => {
            commands::governance::run(action, cli.output, &client, &node_url, &identity)
        }
        Command::FastPath { action } => {
            commands::fastpath::run(action, cli.output, &client, &node_url, &identity)
        }
        Command::Agent { action } => {
            commands::agent::run(action, cli.output, &client, &node_url, &identity)
        }
        Command::Idea { action } => commands::idea::run(action, cli.output, &client, &node_url),
    };

    match result {
        Ok(output) => {
            println!("{}", output);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
