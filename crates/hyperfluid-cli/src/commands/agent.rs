use std::fs;
use std::path::PathBuf;

use clap::Subcommand;
use hyperfluid_p2p::identity::Identity;
use serde::Serialize;

use crate::commands::{format_output, rpc_post, sign_payload};
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

#[derive(Serialize)]
struct LocalSkillEntry {
    name: String,
    path: String,
    description: String,
}

/// Resolve the agent skills directory.
fn skills_dir() -> Result<PathBuf, String> {
    if let Ok(dir) = std::env::var("HYPERFLUID_SKILLS_DIR") {
        let p = PathBuf::from(&dir);
        if p.is_dir() {
            return Ok(p);
        }
        return Err(format!("HYPERFLUID_SKILLS_DIR set but '{}' is not a directory", dir));
    }
    // Default: ~/.hyperfluid/skills/
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let p = PathBuf::from(home).join(".hyperfluid").join("skills");
    if p.is_dir() {
        return Ok(p);
    }
    // Also try ./skills relative to cwd
    let cwd = PathBuf::from("skills");
    if cwd.is_dir() {
        return Ok(cwd);
    }
    Err("skills directory not found — set HYPERFLUID_SKILLS_DIR or create ~/.hyperfluid/skills/"
        .into())
}

fn list_local_skills() -> Result<Vec<LocalSkillEntry>, String> {
    let dir = skills_dir()?;
    let mut entries = Vec::new();
    for entry in fs::read_dir(&dir).map_err(|e| format!("cannot read skills dir: {}", e))? {
        let entry = entry.map_err(|e| format!("dir entry error: {}", e))?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        if !skill_md.exists() {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        let description = fs::read_to_string(&skill_md)
            .ok()
            .map(|content| {
                content
                    .lines()
                    .find(|l| l.starts_with("## Description") || l.starts_with("# "))
                    .map(|l| l.trim_start_matches(['#', ' ']).to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_default();
        entries.push(LocalSkillEntry {
            name,
            path: path.to_string_lossy().to_string(),
            description,
        });
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

pub fn run(
    action: AgentAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
    identity: &Identity,
) -> Result<String, String> {
    let result = match action {
        AgentAction::Status { agent } => {
            rpc_post(client, node_url, "/agent/status", serde_json::json!({ "agent_id": agent }))?
        }
        // F-66: Implement proper agent registration with signed transaction
        AgentAction::Register { name, tags } => {
            let pubkey = identity.verifying_key_encoded();
            let name_bytes = name.as_bytes().to_vec();
            let tags_bytes = tags.as_deref().unwrap_or("").as_bytes().to_vec();
            let payload = (pubkey.to_vec(), name_bytes, tags_bytes);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/agent/register",
                serde_json::json!({
                    "tx_type": "agent_register",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        // F-91: Actually list local skills from filesystem
        AgentAction::ListSkills => {
            let skills = list_local_skills().unwrap_or_default();
            serde_json::json!({
                "action": "list_skills",
                "count": skills.len(),
                "skills": skills.iter().map(|s| {
                    serde_json::json!({
                        "name": s.name,
                        "path": s.path,
                        "description": s.description,
                    })
                }).collect::<Vec<_>>(),
                "note": "skills loaded from local filesystem — not an on-chain operation",
            })
        }
        // F-92: Actually load and validate a specific skill
        AgentAction::LoadSkill { name } => {
            let dir = skills_dir()?;
            let skill_dir = dir.join(&name);
            if !skill_dir.is_dir() {
                return Err(format!("skill '{}' not found in skills directory {:?}", name, dir));
            }
            let skill_md = skill_dir.join("SKILL.md");
            if !skill_md.exists() {
                return Err(format!("skill '{}' is missing SKILL.md", name));
            }
            let content = fs::read_to_string(&skill_md)
                .map_err(|e| format!("cannot read SKILL.md: {}", e))?;
            let title = content
                .lines()
                .find(|l| l.starts_with("# "))
                .map(|l| l.trim_start_matches("# ").to_string())
                .unwrap_or_default();
            serde_json::json!({
                "action": "load_skill",
                "skill": name,
                "title": title,
                "path": skill_md.to_string_lossy().to_string(),
                "loaded": true,
                "note": "skill loaded locally — not an on-chain operation",
            })
        }
    };
    Ok(format_output(&result, format))
}
