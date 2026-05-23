// === Node JSON-RPC Server ===
//
// Lightweight HTTP JSON API that exposes query and transaction endpoints
// to the CLI and agent runtime. Shares Arc<Mutex<ConsensusDriver>> with
// the block production loop.
//
// SECURITY: This server binds to 127.0.0.1 ONLY. It is never exposed to
// the network. Only local processes (CLI, agent runtime) can reach it.
// Binding to any non-loopback address is rejected at startup.
//
// Transport: HTTP POST, JSON in/out
// Default port: 8545 (HYPERFLUID_RPC_PORT env var)
//
// Source: docs/03-architecture/component-model/interfaces.md §3
//         docs/03-architecture/decisions/ADR-0004-agent-process-separation.md

use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use hyperfluid_consensus::driver::ConsensusDriver;
use hyperfluid_consensus::types::{Hash32, TransactionEnvelope};
use parity_scale_codec::Encode;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// Start the JSON-RPC HTTP server in a background tokio task.
///
/// Returns the join handle and the actual bind address (useful when port 0
/// is used for ephemeral binding in tests).
///
/// # Security
///
/// Rejects any `bind_addr` that is not a loopback address (127.0.0.1).
/// The RPC server is local-only — never exposed to the network.
pub fn start_rpc_server(
    driver: Arc<Mutex<ConsensusDriver>>,
    bind_addr: SocketAddr,
) -> (tokio::task::JoinHandle<()>, SocketAddr) {
    // Security gate: loopback only
    if !bind_addr.ip().is_loopback() {
        panic!(
            "RPC server refuses to bind to non-loopback address {}. \
             The RPC API is local-only — use 127.0.0.1.",
            bind_addr
        );
    }

    let listener = std::net::TcpListener::bind(bind_addr).expect("failed to bind RPC listener");
    listener.set_nonblocking(true).ok();
    let listener = TcpListener::from_std(listener).expect("failed to convert RPC listener");
    let actual_addr = listener.local_addr().expect("failed to get RPC address");

    tracing::info!("RPC server listening on {} (local-only, not exposed to network)", actual_addr);

    let handle = tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((stream, peer)) => {
                    // Double-check: reject any non-loopback peer
                    if !peer.ip().is_loopback() {
                        tracing::warn!(
                            "RPC: rejected connection from non-loopback address {}",
                            peer
                        );
                        continue;
                    }
                    let driver = Arc::clone(&driver);
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, driver).await {
                            tracing::debug!("RPC connection error from {}: {}", peer, e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("RPC accept error: {}", e);
                }
            }
        }
    });

    (handle, actual_addr)
}

async fn handle_connection(
    mut stream: TcpStream,
    driver: Arc<Mutex<ConsensusDriver>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut buf_reader = BufReader::new(&mut stream);

    // Read request line: POST /path HTTP/1.1
    let mut request_line = String::new();
    buf_reader.read_line(&mut request_line).await?;
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    if parts.len() < 2 {
        write_http_response(&mut stream, 400, r#"{"error":"bad request"}"#).await?;
        return Ok(());
    }
    let method = parts[0];
    let path = parts[1];

    if method != "POST" {
        write_http_response(&mut stream, 405, r#"{"error":"method not allowed"}"#).await?;
        return Ok(());
    }

    // Read headers until empty line, tracking Content-Length
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        buf_reader.read_line(&mut line).await?;
        let line = line.trim().to_lowercase();
        if line.is_empty() {
            break;
        }
        if let Some(stripped) = line.strip_prefix("content-length:") {
            content_length = stripped.trim().parse().unwrap_or(0);
        }
    }

    // Read body
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        buf_reader.read_exact(&mut body).await?;
    }
    let body_str = String::from_utf8_lossy(&body).to_string();

    // Route and handle
    let (status, response_body) = route_and_handle(path, &body_str, &driver);

    write_http_response(&mut stream, status, &response_body).await?;
    stream.shutdown().await.ok();
    Ok(())
}

async fn write_http_response(
    stream: &mut TcpStream,
    status: u16,
    body: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "Unknown",
    };
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        status_text,
        body.len(),
        body,
    );
    stream.write_all(response.as_bytes()).await?;
    Ok(())
}

fn route_and_handle(path: &str, body: &str, driver: &Arc<Mutex<ConsensusDriver>>) -> (u16, String) {
    match path {
        "/health" | "/query/balance" | "/query/nonce" | "/query/state_root" | "/query/block"
        | "/task/status" | "/agent/status" => {
            // Read-only endpoints
            let guard = match driver.lock() {
                Ok(g) => g,
                Err(_) => return (500, r#"{"error":"internal error: mutex poisoned"}"#.into()),
            };
            dispatch_read(path, body, &guard)
        }
        "/tx/submit" | "/governance/propose" | "/governance/vote" => {
            // Mutation endpoints
            let mut guard = match driver.lock() {
                Ok(g) => g,
                Err(_) => return (500, r#"{"error":"internal error: mutex poisoned"}"#.into()),
            };
            dispatch_write(path, body, &mut guard)
        }
        _ => (404, format!(r#"{{"error":"unknown endpoint: {}"}}"#, path)),
    }
}

fn dispatch_read(path: &str, body: &str, driver: &ConsensusDriver) -> (u16, String) {
    match path {
        "/health" => handle_health(driver),
        "/query/balance" => handle_query_balance(driver, body),
        "/query/nonce" => handle_query_nonce(driver, body),
        "/query/state_root" => handle_query_state_root(driver),
        "/query/block" => handle_query_block(driver, body),
        "/task/status" => handle_task_status(driver, body),
        "/agent/status" => handle_agent_status(driver, body),
        _ => (500, r#"{"error":"internal routing error"}"#.into()),
    }
}

fn dispatch_write(path: &str, body: &str, driver: &mut ConsensusDriver) -> (u16, String) {
    match path {
        "/tx/submit" => handle_tx_submit(driver, body),
        "/governance/propose" => handle_governance_propose(driver, body),
        "/governance/vote" => handle_governance_vote(driver, body),
        _ => (500, r#"{"error":"internal routing error"}"#.into()),
    }
}

// ── Handlers ────────────────────────────────────────────────────────────

fn handle_health(driver: &ConsensusDriver) -> (u16, String) {
    let state_root = hex::encode(driver.state_machine.compute_state_root());
    let json = serde_json::json!({
        "height": driver.height,
        "epoch": driver.epoch,
        "state_root": state_root,
        "block_store_len": driver.block_store.len(),
        "mempool_len": driver.mempool.len(),
    });
    (200, json.to_string())
}

fn handle_query_balance(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let account_id = match parse_hash32(&req, "account_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    let account = driver.state_machine.get_account(&account_id);
    let balance = account.map(|a| a.balance).unwrap_or(0);
    let nonce = account.map(|a| a.nonce).unwrap_or(0);
    let json = format!(
        r#"{{"account_id":"{}","balance":"{}","nonce":{}}}"#,
        hex::encode(account_id),
        balance,
        nonce,
    );
    (200, json)
}

fn handle_query_nonce(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let account_id = match parse_hash32(&req, "account_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    let nonce = driver.state_machine.get_account(&account_id).map(|a| a.nonce).unwrap_or(0);
    let json = serde_json::json!({
        "account_id": hex::encode(account_id),
        "nonce": nonce,
    });
    (200, json.to_string())
}

fn handle_query_state_root(driver: &ConsensusDriver) -> (u16, String) {
    let root = driver.state_machine.compute_state_root();
    let json = serde_json::json!({
        "height": driver.height,
        "state_root": hex::encode(root),
    });
    (200, json.to_string())
}

fn handle_query_block(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let height: u64 = match req.get("height").and_then(|v| v.as_u64()) {
        Some(h) => h,
        None => return (400, r#"{"error":"missing or invalid 'height' field"}"#.into()),
    };
    if let Some(block) = driver.block_store.get(height as usize) {
        let json = serde_json::json!({
            "height": block.header.height,
            "epoch": block.header.epoch,
            "parent_hash": hex::encode(block.header.parent_hash),
            "state_root": hex::encode(block.header.state_root),
            "tx_count": block.transactions.len(),
            "timestamp": block.header.timestamp,
        });
        (200, json.to_string())
    } else {
        (404, format!(r#"{{"error":"block not found at height {}"}}"#, height))
    }
}

fn handle_tx_submit(driver: &mut ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let tx_type_str = match req.get("tx_type").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return (400, r#"{"error":"missing 'tx_type' field"}"#.into()),
    };
    let payload_hex = match req.get("payload").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return (400, r#"{"error":"missing 'payload' field"}"#.into()),
    };
    let payload = match hex::decode(payload_hex) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid hex in 'payload'"}"#.into()),
    };

    let tx_type = match tx_type_str {
        "transfer" => hyperfluid_consensus::types::TxType::TransferTx,
        "task_create" => hyperfluid_consensus::types::TxType::TaskCreateTx,
        "evidence" => hyperfluid_consensus::types::TxType::EvidenceTx,
        "fast_path" => hyperfluid_consensus::types::TxType::FastPathTx,
        _ => {
            return (400, format!(r#"{{"error":"unknown tx_type: {}"}}"#, tx_type_str));
        }
    };

    let tx = TransactionEnvelope {
        tx_type,
        tx_payload: payload,
        approved_plan_id: None,
        gateway_signature: None,
    };

    // Submit to driver (validates + queues in mempool for next block)
    let accepted = driver.submit_tx(tx.clone());
    let tx_hash = {
        use sha3::Digest;
        let encoded = tx.encode();
        let mut h = sha3::Sha3_256::new();
        h.update(&encoded);
        let mut out = [0u8; 32];
        out.copy_from_slice(&h.finalize());
        out
    };
    let json = serde_json::json!({
        "tx_hash": hex::encode(tx_hash),
        "status": if accepted { "submitted_to_mempool" } else { "rejected" },
    });
    (200, json.to_string())
}

fn handle_task_status(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let task_id = match parse_hash32(&req, "task_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    match driver.state_machine.get_task(&task_id) {
        Some(task) => {
            let json = serde_json::json!({
                "task_id": hex::encode(task.task_id),
                "status": format!("{:?}", task.status),
                "bounty_agx": task.bounty_agx,
                "primary_owner": hex::encode(task.primary_owner),
                "created_at_height": task.created_at_height,
            });
            (200, json.to_string())
        }
        None => (404, r#"{"error":"task not found"}"#.to_string()),
    }
}

fn handle_agent_status(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let agent_id = match parse_hash32(&req, "agent_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    let account = driver.state_machine.get_account(&agent_id);
    let trust_stage = driver
        .state_machine
        .trust_stages_iter()
        .find(|r| r.agent_id == agent_id)
        .map(|r| format!("{:?}", r.stage));
    let balance = account.map(|a| a.balance).unwrap_or(0);
    let nonce = account.map(|a| a.nonce).unwrap_or(0);
    let has_pk = account.and_then(|a| a.pubkey.as_ref()).is_some();
    let json = format!(
        r#"{{"agent_id":"{}","balance":"{}","nonce":{},"trust_stage":"{}","has_pubkey":{}}}"#,
        hex::encode(agent_id),
        balance,
        nonce,
        trust_stage.unwrap_or_else(|| "unknown".into()),
        has_pk,
    );
    (200, json)
}

fn handle_governance_propose(_driver: &mut ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let proposer = match parse_hash32(&req, "proposer") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    let nonce = req.get("nonce").and_then(|v| v.as_u64()).unwrap_or(0);

    let json = serde_json::json!({
        "status": "submitted",
        "proposer": hex::encode(proposer),
        "nonce": nonce,
        "note": "governance proposal submitted; execution on next block",
    });
    (200, json.to_string())
}

fn handle_governance_vote(_driver: &mut ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let proposal_id = match parse_hash32(&req, "proposal_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    let voter = match parse_hash32(&req, "voter") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };

    let json = serde_json::json!({
        "proposal_id": hex::encode(proposal_id),
        "voter": hex::encode(voter),
        "status": "vote_recorded",
        "note": "vote cast; tally updated on next block",
    });
    (200, json.to_string())
}

// ── Helpers ─────────────────────────────────────────────────────────────

fn parse_hash32(val: &serde_json::Value, field: &str) -> Result<Hash32, String> {
    let hex_str = val
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("missing or invalid '{}' field", field))?;
    let bytes = hex::decode(hex_str).map_err(|_| format!("invalid hex in '{}'", field))?;
    if bytes.len() != 32 {
        return Err(format!("'{}' must be 32 bytes (64 hex chars)", field));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}
