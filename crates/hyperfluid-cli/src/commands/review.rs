use clap::Subcommand;

use crate::commands::{format_output, rpc_post};
use crate::OutputFormat;

#[derive(Subcommand)]
pub enum ReviewAction {
    Claim {
        #[arg(long)]
        review_task_id: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        nonce: u64,
    },
    Verdict {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        review_task_id: String,
        #[arg(long)]
        verdict: String,
        #[arg(long)]
        evidence_hash: String,
        #[arg(long)]
        agent: String,
        #[arg(long)]
        nonce: u64,
    },
}

pub fn run(
    action: ReviewAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
) -> Result<String, String> {
    let result = match action {
        ReviewAction::Claim { review_task_id, agent, nonce } => rpc_post(
            client,
            node_url,
            "/tx/submit",
            serde_json::json!({
                "tx_type": "task_claim",
                "payload": hex::encode(format!("review_claim:{}:{}:{}", review_task_id, agent, nonce)),
            }),
        )?,
        ReviewAction::Verdict { task_id, review_task_id, verdict, evidence_hash, agent, nonce } => {
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "task_create",
                    "payload": hex::encode(format!(
                        "review_verdict:{}:{}:{}:{}:{}:{}",
                        task_id, review_task_id, verdict, evidence_hash, agent, nonce
                    )),
                }),
            )?
        }
    };
    Ok(format_output(&result, format))
}
