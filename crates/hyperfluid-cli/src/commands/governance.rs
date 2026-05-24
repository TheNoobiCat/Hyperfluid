use clap::Subcommand;

use crate::commands::{format_output, rpc_post};
use crate::OutputFormat;

fn parse_hash32(hex_str: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(hex_str).map_err(|e| format!("invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes (64 hex chars), got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

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
    List,
    Get {
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
        GovernanceAction::Propose { title_hash, target_hash, description_hash, proposer, nonce } => {
            let target = parse_hash32(&target_hash)?;
            let proposer_id = parse_hash32(&proposer)?;
            let proposal_id = {
                use sha3::Digest;
                let mut h = sha3::Sha3_256::new();
                h.update(proposer_id);
                h.update(target);
                let mut out = [0u8; 32];
                out.copy_from_slice(&h.finalize());
                out
            };
            rpc_post(
                client, node_url, "/governance/propose",
                serde_json::json!({
                    "proposal_id": hex::encode(proposal_id),
                    "target_hash": target_hash,
                    "title_hash": title_hash,
                    "description_hash": description_hash,
                    "proposer": proposer,
                    "nonce": nonce,
                }),
            )?
        }
        GovernanceAction::Vote { proposal_id, option, voter, nonce } => {
            let approve = option.to_lowercase() == "yes" || option.to_lowercase() == "approve";
            rpc_post(
                client, node_url, "/governance/vote",
                serde_json::json!({
                    "proposal_id": proposal_id,
                    "approve": approve,
                    "voter": voter,
                    "nonce": nonce,
                    "target_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                }),
            )?
        }
        GovernanceAction::List => {
            rpc_post(client, node_url, "/governance/list", serde_json::json!({}))?
        }
        GovernanceAction::Get { proposal_id } => rpc_post(
            client, node_url, "/governance/get",
            serde_json::json!({ "proposal_id": proposal_id }),
        )?,
    };
    Ok(format_output(&result, format))
}
