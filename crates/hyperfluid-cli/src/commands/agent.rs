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
        AgentAction::Status { agent } => {
            rpc_post(client, node_url, "/agent/status", serde_json::json!({ "agent_id": agent }))?
        }
        AgentAction::Register { name: _, tags: _ } => {
            serde_json::json!({
                "action": "agent_register",
                "status": "not_implemented",
                "note": "On-chain agent registration is not yet implemented. ",
                "message": format!(
                    "Registration will be available in a future protocol upgrade. \
                     For now, agents operate with a local identity derived from their config file."
                ),
            })
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
