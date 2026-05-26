use clap::Subcommand;
use hyperfluid_p2p::identity::Identity;
use parity_scale_codec::Encode;

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
pub enum FastPathAction {
    List {
        #[arg(long)]
        topic: Option<String>,
    },
    Propose {
        #[arg(long)]
        topic: String,
        #[arg(long)]
        proposed_head: String,
        #[arg(long)]
        manifest: String,
        #[arg(long)]
        proposer: String,
        #[arg(long)]
        nonce: u64,
    },
    Approve {
        #[arg(long)]
        proposal_id: String,
        #[arg(long)]
        reviewer: String,
        #[arg(long)]
        topic_weight: Option<u64>,
    },
    Challenge {
        #[arg(long)]
        proposal_id: String,
        #[arg(long)]
        topic_id: String,
        #[arg(long)]
        challenger: String,
        #[arg(long)]
        evidence_hash: String,
        #[arg(long)]
        bond: u128,
        #[arg(long)]
        nonce: u64,
    },
    Status {
        #[arg(long)]
        proposal_id: String,
    },
}

pub fn run(
    action: FastPathAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
    identity: &Identity,
) -> Result<String, String> {
    let result = match action {
        FastPathAction::List { topic } => {
            rpc_post(client, node_url, "/fastpath/list", serde_json::json!({ "topic": topic }))?
        }
        FastPathAction::Propose { topic, proposed_head, manifest, proposer, nonce } => {
            let topic_id = parse_hash32(&topic)?;
            let proposed = parse_hash32(&proposed_head)?;
            let manifest_hash = parse_hash32(&manifest)?;
            let proposer_id = parse_hash32(&proposer)?;
            // F-88: Use shared sha3_256_hash (removed duplicate dead_code function)
            let proposal_id = sha3_256_hash(
                &[
                    topic_id.as_slice(),
                    proposer_id.as_slice(),
                    &[0u8; 32], // F-022: base_topic_head defaults to genesis; should be fetched from topic state
                    proposed.as_slice(),
                    &nonce.to_le_bytes(),
                ]
                .concat(),
            );
            let payload = (proposal_id, topic_id, proposer_id, proposed, false);
            let (_, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "fast_path",
                    "payload": hex::encode(payload.encode()),
                    "manifest": hex::encode(manifest_hash),
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        FastPathAction::Approve { proposal_id, reviewer, topic_weight } => {
            let pid = parse_hash32(&proposal_id)?;
            let reviewer_id_bytes = parse_hash32(&reviewer)?;
            let tw = topic_weight.unwrap_or(1);
            let payload = (pid, reviewer_id_bytes, tw);
            let (_, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/fastpath/approve",
                serde_json::json!({
                    "proposal_id": proposal_id,
                    "reviewer_id": reviewer,
                    "topic_weight": tw,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        FastPathAction::Challenge {
            proposal_id,
            topic_id,
            challenger,
            evidence_hash,
            bond,
            nonce,
        } => {
            let pid = parse_hash32(&proposal_id)?;
            let tid = parse_hash32(&topic_id)?;
            let ev = parse_hash32(&evidence_hash)?;
            let challenger_id = parse_hash32(&challenger)?;
            let payload = (pid, tid, challenger_id, ev, true);
            let (_, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "fast_path",
                    "payload": hex::encode(payload.encode()),
                    "bond": bond.to_string(),
                    "nonce": nonce,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        FastPathAction::Status { proposal_id } => rpc_post(
            client,
            node_url,
            "/fastpath/status",
            serde_json::json!({ "proposal_id": proposal_id }),
        )?,
    };
    Ok(format_output(&result, format))
}
