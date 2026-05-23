// === Hyperfluid CLI ===
//
// Source: docs/05-planning/stages/stage-02-agent-runtime.md Week 9-10
//
// 7 top-level subcommands: tx, query, task, review, governance, agent, idea
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
}

#[derive(Clone, Copy, clap::ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Subcommand)]
enum Command {
    /// Transaction commands: transfer, bond, unbond, withdraw, delegate, undelegate, commission
    Tx {
        #[command(subcommand)]
        action: commands::tx::TxAction,
    },
    /// Query chain state: balance, nonce, state-root, block
    Query {
        #[command(subcommand)]
        action: commands::query::QueryAction,
    },
    /// Task management: create, status, claim, heartbeat, submit, sponsor
    Task {
        #[command(subcommand)]
        action: commands::task::TaskAction,
    },
    /// Review management: claim, verdict
    Review {
        #[command(subcommand)]
        action: commands::review::ReviewAction,
    },
    /// Governance: propose, vote, status
    Governance {
        #[command(subcommand)]
        action: commands::governance::GovernanceAction,
    },
    /// Agent management: status, register
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
    let client = reqwest::blocking::Client::new();

    let result = match cli.command {
        Command::Tx { action } => commands::tx::run(action, cli.output, &client, &node_url),
        Command::Query { action } => commands::query::run(action, cli.output, &client, &node_url),
        Command::Task { action } => commands::task::run(action, cli.output, &client, &node_url),
        Command::Review { action } => commands::review::run(action, cli.output, &client, &node_url),
        Command::Governance { action } => {
            commands::governance::run(action, cli.output, &client, &node_url)
        }
        Command::Agent { action } => commands::agent::run(action, cli.output, &client, &node_url),
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
