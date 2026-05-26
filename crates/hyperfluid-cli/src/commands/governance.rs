use clap::Subcommand;
use hyperfluid_p2p::identity::Identity;

use crate::commands::{format_output, rpc_post, sha3_256_hash, sign_payload};
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
        /// Target hash for the vote (hex, 32 bytes) — required. Previously hardcoded to zero.
        #[arg(long)]
        target_hash: String,
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
    identity: &Identity,
) -> Result<String, String> {
    let result = match action {
        GovernanceAction::Propose {
            title_hash,
            target_hash,
            description_hash,
            proposer,
            nonce,
        } => {
            let target = parse_hash32(&target_hash)?;
            let proposer_id = parse_hash32(&proposer)?;
            // F-88: Use shared sha3_256_hash
            let proposal_id = sha3_256_hash(&[proposer_id.as_slice(), target.as_slice()].concat());
            let (payload_hex, sig_hex, pubkey_hex) =
                sign_payload(identity, &(proposal_id, target, proposer_id, nonce));
            rpc_post(
                client,
                node_url,
                "/governance/propose",
                serde_json::json!({
                    "proposal_id": hex::encode(proposal_id),
                    "target_hash": target_hash,
                    "title_hash": title_hash,
                    "description_hash": description_hash,
                    "proposer": proposer,
                    "nonce": nonce,
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        // F-69: Use provided target_hash instead of hardcoded zero
        GovernanceAction::Vote { proposal_id, option, voter, nonce, target_hash } => {
            let approve = option.to_lowercase() == "yes" || option.to_lowercase() == "approve";
            let proposal_id_bytes = parse_hash32(&proposal_id)?;
            let voter_id = parse_hash32(&voter)?;
            let (payload_hex, sig_hex, pubkey_hex) =
                sign_payload(identity, &(proposal_id_bytes, approve as u8, voter_id, nonce));
            rpc_post(
                client,
                node_url,
                "/governance/vote",
                serde_json::json!({
                    "proposal_id": proposal_id,
                    "approve": approve,
                    "voter": voter,
                    "nonce": nonce,
                    "target_hash": target_hash,
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        GovernanceAction::List => {
            rpc_post(client, node_url, "/governance/list", serde_json::json!({}))?
        }
        GovernanceAction::Get { proposal_id } => rpc_post(
            client,
            node_url,
            "/governance/get",
            serde_json::json!({ "proposal_id": proposal_id }),
        )?,
    };
    Ok(format_output(&result, format))
}
