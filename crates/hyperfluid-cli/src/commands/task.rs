use clap::Subcommand;

use crate::commands::{format_output, rpc_post};
use crate::OutputFormat;

#[derive(Subcommand)]
pub enum TaskAction {
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        description_file: Option<String>,
        #[arg(long)]
        bounty: u128,
        #[arg(long)]
        seed_ref: String,
        #[arg(long)]
        required_skills: Option<String>,
        #[arg(long)]
        sponsor: Option<String>,
        #[arg(long)]
        sender: String,
        #[arg(long)]
        nonce: u64,
    },
    Status {
        #[arg(long)]
        task_id: String,
    },
    Claim {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        nonce: u64,
    },
    Heartbeat {
        #[arg(long)]
        lease_id: String,
        #[arg(long)]
        artifact_hash: Option<String>,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        nonce: u64,
    },
    Submit {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        artifact_hash: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        nonce: u64,
    },
}

pub fn run(
    action: TaskAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
) -> Result<String, String> {
    let result = match action {
        TaskAction::Create {
            title,
            description_file,
            bounty,
            seed_ref,
            required_skills,
            sponsor,
            sender,
            nonce,
        } => rpc_post(
            client,
            node_url,
            "/tx/submit",
            serde_json::json!({
                "tx_type": "task_create",
                "payload": hex::encode(format!(
                    "task_create:{}:{}:{}:{}:{}",
                    title, bounty, seed_ref, sender, nonce
                )),
                "description_file": description_file,
                "required_skills": required_skills,
                "sponsor": sponsor,
            }),
        )?,
        TaskAction::Status { task_id } => rpc_post(
            client,
            node_url,
            "/task/status",
            serde_json::json!({
                "task_id": task_id,
            }),
        )?,
        TaskAction::Claim { task_id, agent, nonce } => rpc_post(
            client,
            node_url,
            "/tx/submit",
            serde_json::json!({
                "tx_type": "task_claim",
                "payload": hex::encode(format!("claim:{}:{}:{}", task_id, agent, nonce)),
            }),
        )?,
        TaskAction::Heartbeat { lease_id, artifact_hash, agent, nonce } => rpc_post(
            client,
            node_url,
            "/tx/submit",
            serde_json::json!({
                "tx_type": "task_claim",
                "payload": hex::encode(format!(
                    "heartbeat:{}:{}:{}:{}",
                    lease_id,
                    artifact_hash.unwrap_or_default(),
                    agent,
                    nonce
                )),
            }),
        )?,
        TaskAction::Submit { task_id, artifact_hash, agent, nonce } => rpc_post(
            client,
            node_url,
            "/tx/submit",
            serde_json::json!({
                "tx_type": "task_create",
                "payload": hex::encode(format!(
                    "submit:{}:{}:{}:{}",
                    task_id, artifact_hash, agent, nonce
                )),
            }),
        )?,
    };
    Ok(format_output(&result, format))
}
