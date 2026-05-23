pub mod agent;
pub mod governance;
pub mod idea;
pub mod query;
pub mod review;
pub mod task;
pub mod tx;

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
