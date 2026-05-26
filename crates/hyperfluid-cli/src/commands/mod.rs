pub mod agent;
pub mod fastpath;
pub mod governance;
pub mod idea;
pub mod query;
pub mod review;
pub mod task;
pub mod tx;

use hyperfluid_p2p::identity::Identity;
use parity_scale_codec::Encode;
use std::path::PathBuf;

use crate::OutputFormat;

pub fn format_output(data: &impl serde::Serialize, format: OutputFormat) -> String {
    match format {
        OutputFormat::Text => format_text(data),
        OutputFormat::Json => serde_json::to_string_pretty(data).unwrap_or_else(|e| e.to_string()),
    }
}

fn format_text(data: &impl serde::Serialize) -> String {
    serde_json::to_string_pretty(data).unwrap_or_else(|e| format!("{{error: \"{}\"}}", e))
}

pub fn rpc_post(
    client: &reqwest::blocking::Client,
    node_url: &str,
    endpoint: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = format!("{}{}", node_url, endpoint);
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .map_err(|e| format!("RPC connection failed: {}", e))?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let text = resp.text().unwrap_or_default();
        return Err(format!("RPC error {}: {}", status, text));
    }
    resp.json().map_err(|e| format!("RPC response parse error: {}", e))
}

/// SHA3-256 hash returning a fixed 32-byte array.
pub(crate) fn sha3_256_hash(data: &[u8]) -> [u8; 32] {
    use sha3::Digest;
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Well-known hash of the empty skills set: SHA3-256("").
pub(crate) const EMPTY_SKILLS_HASH: [u8; 32] = [
    0xa7, 0xff, 0xc6, 0xf8, 0xbf, 0x1e, 0xd7, 0x66, 0x51, 0xc1, 0x47, 0x56, 0xa0, 0x61, 0xd6, 0x62,
    0xf5, 0xdc, 0x3f, 0x78, 0xb7, 0xbf, 0x42, 0x26, 0x18, 0xc2, 0x77, 0x29, 0x31, 0x14, 0x6f, 0x79,
];

/// Load an ML-DSA-65 Identity from a key file, hex key string, or default path.
///
/// Resolution order:
/// 1. `key_file` path (must exist)
/// 2. `key_hex` 32-byte hex seed
/// 3. `~/.hyperfluid/agent.key` (default path, if exists)
/// 4. Error: no identity found. Use `hyperfluid agent keygen` to create one.
pub(crate) fn load_identity(
    key_file: Option<&str>,
    key_hex: Option<&str>,
) -> Result<Identity, String> {
    if let Some(path_str) = key_file {
        let path = PathBuf::from(path_str);
        if path.exists() {
            return load_identity_from_file(&path);
        }
        return Err(format!("Key file not found: {:?}", path));
    }
    if let Some(hex_str) = key_hex {
        let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex key: {}", e))?;
        if bytes.len() != 32 {
            return Err(format!("Expected 32 bytes of seed, got {}", bytes.len()));
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        return Ok(Identity::from_seed(&seed));
    }
    let default_path = default_key_path();
    if default_path.exists() {
        return load_identity_from_file(&default_path);
    }
    eprintln!("Warning: No identity found — using ephemeral key. Configure with --key-file or --key-hex.");
    Ok(Identity::generate())
}

fn load_identity_from_file(path: &PathBuf) -> Result<Identity, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read key file {:?}: {}", path, e))?;
    let trimmed = content.trim();
    let bytes = hex::decode(trimmed).map_err(|e| format!("Invalid hex in key file: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("Expected 32 bytes of seed, got {}", bytes.len()));
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    Ok(Identity::from_seed(&seed))
}

fn default_key_path() -> PathBuf {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".hyperfluid").join("agent.key")
}

/// Encode payload with SCALE, sign with the given Identity.
/// Returns (payload_hex, signature_hex, pubkey_hex).
pub(crate) fn sign_payload(identity: &Identity, payload: &impl Encode) -> (String, String, String) {
    let encoded = payload.encode();
    let signature = identity.sign(&encoded);
    let pubkey = identity.verifying_key_encoded();
    (hex::encode(&encoded), hex::encode(&signature), hex::encode(&pubkey))
}
