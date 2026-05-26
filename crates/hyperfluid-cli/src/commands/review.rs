use clap::Subcommand;
use hyperfluid_p2p::identity::Identity;

use crate::commands::{format_output, rpc_post, sign_payload};
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
    List {
        #[arg(long)]
        status: Option<String>,
    },
}

pub fn run(
    action: ReviewAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
    identity: &Identity,
) -> Result<String, String> {
    let result = match action {
        // F-32: Wire nonce into Review Claim payload and sign with Identity
        ReviewAction::Claim { review_task_id, agent, nonce } => {
            let task_id = parse_hash32(&review_task_id)?;
            let agent_id = parse_hash32(&agent)?;
            let payload = (task_id, agent_id, nonce, 0u128, false);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "claim_task",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        // F-33: Wire nonce into Verdict payload and sign with Identity
        ReviewAction::Verdict {
            task_id: _,
            review_task_id,
            verdict,
            evidence_hash,
            agent,
            nonce,
        } => {
            let r_task_id = parse_hash32(&review_task_id)?;
            let reviewer = parse_hash32(&agent)?;
            let accept = verdict.to_lowercase() == "accept";
            let evidence = parse_hash32(&evidence_hash)?;
            let payload = (r_task_id, reviewer, accept, evidence, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "submit_review",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        ReviewAction::List { status } => {
            rpc_post(client, node_url, "/review/list", serde_json::json!({ "status": status }))?
        }
    };
    Ok(format_output(&result, format))
}
