use clap::Subcommand;

use crate::commands::{format_output, rpc_post};
use crate::OutputFormat;

#[derive(Subcommand)]
pub enum GovernanceAction {
    Propose {
        #[arg(long)]
        title_hash: String,
        #[arg(long)]
        target_hash: String,
        #[arg(long)]
        description_hash: String,
        #[arg(long)]
        proposer: String,
        #[arg(long)]
        nonce: u64,
    },
    Vote {
        #[arg(long)]
        proposal_id: String,
        #[arg(long)]
        option: String,
        #[arg(long)]
        voter: String,
        #[arg(long)]
        nonce: u64,
    },
    Status {
        #[arg(long)]
        proposal_id: String,
    },
}

pub fn run(
    action: GovernanceAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
) -> Result<String, String> {
    let result = match action {
        GovernanceAction::Propose {
            title_hash,
            target_hash,
            description_hash,
            proposer,
            nonce,
        } => rpc_post(
            client,
            node_url,
            "/governance/propose",
            serde_json::json!({
                "title_hash": title_hash,
                "target_hash": target_hash,
                "description_hash": description_hash,
                "proposer": proposer,
                "nonce": nonce,
            }),
        )?,
        GovernanceAction::Vote { proposal_id, option, voter, nonce } => rpc_post(
            client,
            node_url,
            "/governance/vote",
            serde_json::json!({
                "proposal_id": proposal_id,
                "option": option,
                "voter": voter,
                "nonce": nonce,
            }),
        )?,
        GovernanceAction::Status { proposal_id: _ } => {
            rpc_post(client, node_url, "/query/state_root", serde_json::json!({}))?
        }
    };
    Ok(format_output(&result, format))
}
