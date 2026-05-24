use clap::Subcommand;
use parity_scale_codec::Encode;

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
    ListSkills,
    LoadSkill {
        #[arg(long)]
        name: String,
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
            client, node_url, "/agent/status",
            serde_json::json!({ "agent_id": agent }),
        )?,
        AgentAction::Register { name, tags } => {
            let payload = (name.as_bytes().to_vec(), tags.unwrap_or_default().as_bytes().to_vec());
            rpc_post(
                client, node_url, "/tx/submit",
                serde_json::json!({
                    "tx_type": "task_create",
                    "payload": hex::encode(payload.encode()),
                }),
            )?
        }
        AgentAction::ListSkills => {
            serde_json::json!({
                "action": "list_skills",
                "note": "skills managed locally by agent process — see skills.rs",
            })
        }
        AgentAction::LoadSkill { name } => {
            serde_json::json!({
                "action": "load_skill",
                "skill": name,
                "note": "skill loaded locally — not an on-chain operation",
            })
        }
    };
    Ok(format_output(&result, format))
}
