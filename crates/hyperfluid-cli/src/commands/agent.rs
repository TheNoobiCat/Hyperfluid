use clap::Subcommand;

use crate::commands::{format_output, rpc_post};
use crate::OutputFormat;

#[derive(Subcommand)]
pub enum AgentAction {
    Status {
        #[arg(long)]
        agent: String,
    },
    Register {
        #[arg(long)]
        name: String,
        #[arg(long)]
        tags: Option<String>,
    },
}

pub fn run(
    action: AgentAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
) -> Result<String, String> {
    let result = match action {
        AgentAction::Status { agent } => rpc_post(
            client,
            node_url,
            "/agent/status",
            serde_json::json!({
                "agent_id": agent,
            }),
        )?,
        AgentAction::Register { name, tags } => rpc_post(
            client,
            node_url,
            "/tx/submit",
            serde_json::json!({
                "tx_type": "transfer",
                "payload": hex::encode(format!("register:{}:{}", name, tags.unwrap_or_default())),
            }),
        )?,
    };
    Ok(format_output(&result, format))
}
