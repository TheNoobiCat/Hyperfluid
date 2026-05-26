use clap::Subcommand;
use hyperfluid_p2p::identity::Identity;
use parity_scale_codec::Encode;

use crate::commands::{format_output, rpc_post, sha3_256_hash, sign_payload, EMPTY_SKILLS_HASH};
use crate::OutputFormat;

type Hash32 = [u8; 32];

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
pub enum TaskAction {
    Submit {
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
        /// Topic ID for task categorization (hex, 32 bytes). Required for submission.
        #[arg(long)]
        topic_id: String,
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
    Release {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        agent: String,
    },
    Split {
        #[arg(long)]
        parent_task_id: String,
        #[arg(long)]
        children_json: String,
        #[arg(long)]
        caller: String,
    },
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        seed_ref: Option<String>,
    },
    Get {
        #[arg(long)]
        task_id: String,
    },
}

pub fn run(
    action: TaskAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
    identity: &Identity,
) -> Result<String, String> {
    let result = match action {
        TaskAction::Submit {
            title,
            description_file,
            bounty,
            seed_ref,
            required_skills,
            sponsor,
            sender,
            nonce,
            topic_id,
        } => {
            // F-67: Topic ID is now required — parse_hash32 will reject invalid hex
            let topic_id_bytes = parse_hash32(&topic_id)?;
            let sender_id = parse_hash32(&sender)?;
            let meta_hash = sha3_256_hash(title.as_bytes());
            // F-89: Use EMPTY_SKILLS_HASH instead of zero for missing skills
            let skills_hash = required_skills
                .as_deref()
                .map(|s| sha3_256_hash(s.as_bytes()))
                .unwrap_or(EMPTY_SKILLS_HASH);
            let payload = (
                sender_id,      // proposer_id
                bounty,         // bounty_agx
                meta_hash,      // metadata_hash
                skills_hash,    // required_skills_hash
                topic_id_bytes, // topic_id (F-67: no longer hardcoded zero)
                nonce,
            );
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "task_create",
                    "payload": hex::encode(payload.encode()),
                    "description_file": description_file,
                    "required_skills": required_skills,
                    "sponsor": sponsor,
                    "seed_ref": seed_ref,
                }),
            )?
        }
        TaskAction::Status { task_id } => {
            rpc_post(client, node_url, "/task/status", serde_json::json!({ "task_id": task_id }))?
        }
        // F-31: Wire nonce into Claim payload and sign with Identity
        TaskAction::Claim { task_id, agent, nonce } => {
            let task_id_bytes = parse_hash32(&task_id)?;
            let agent_id = parse_hash32(&agent)?;
            let payload = (
                task_id_bytes,
                agent_id,
                nonce,
                0u128, /* bid_amount not used */
                false, /* shadow_claim flag */
            );
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
        TaskAction::Heartbeat { lease_id, artifact_hash, agent: _, nonce: _ } => {
            let lease_id_bytes = parse_hash32(&lease_id)?;
            let artifact = artifact_hash.as_deref().map(parse_hash32).transpose()?;
            let payload =
                (lease_id_bytes, artifact, None::<[u8; 32]>, None::<[u8; 32]>, vec![0u8; 0]);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "heartbeat",
                    "payload": hex::encode(payload.encode()),
                }),
            )?
        }
        TaskAction::Release { task_id, agent } => {
            let task_id_bytes = parse_hash32(&task_id)?;
            let agent_id = parse_hash32(&agent)?;
            let payload = (task_id_bytes, agent_id);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "release_task",
                    "payload": hex::encode(payload.encode()),
                }),
            )?
        }
        TaskAction::Split { parent_task_id, children_json, caller } => {
            let parent = parse_hash32(&parent_task_id)?;
            let caller_id = parse_hash32(&caller)?;
            let children: Vec<serde_json::Value> = serde_json::from_str(&children_json)
                .map_err(|e| format!("invalid children JSON: {}", e))?;
            let child_payloads: Vec<(Hash32, u8, Vec<Hash32>, Hash32)> = children
                .iter()
                .map(|c| {
                    let task_id =
                        parse_hash32(c.get("task_id").and_then(|v| v.as_str()).unwrap_or(""))?;
                    let share =
                        c.get("bounty_share_pct").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                    let depends_on: Vec<Hash32> = c
                        .get("depends_on")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .map(|v| parse_hash32(v.as_str().unwrap_or("")))
                                .collect::<Result<Vec<_>, _>>()
                        })
                        .transpose()?
                        .unwrap_or_default();
                    // F-90: Use EMPTY_SKILLS_HASH instead of hardcoded zero
                    let skills_hash = c
                        .get("required_skills_hash")
                        .and_then(|v| v.as_str())
                        .map(parse_hash32)
                        .transpose()?
                        .unwrap_or(EMPTY_SKILLS_HASH);
                    Ok((task_id, share, depends_on, skills_hash))
                })
                .collect::<Result<Vec<_>, String>>()?;
            let payload = (parent, caller_id, child_payloads);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "split_task",
                    "payload": hex::encode(payload.encode()),
                }),
            )?
        }
        TaskAction::List { status, seed_ref } => rpc_post(
            client,
            node_url,
            "/task/list",
            serde_json::json!({ "status": status, "seed_ref": seed_ref }),
        )?,
        TaskAction::Get { task_id } => {
            rpc_post(client, node_url, "/task/status", serde_json::json!({ "task_id": task_id }))?
        }
    };
    Ok(format_output(&result, format))
}
