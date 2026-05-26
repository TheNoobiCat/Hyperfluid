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
use hyperfluid_state::TaskStatus;
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
        tracing::error!(
            "RPC server refuses to bind to non-loopback address {}. \
             The RPC API is local-only — use 127.0.0.1.",
            bind_addr
        );
        let handle = tokio::spawn(async {});
        return (handle, bind_addr);
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

    if method != "POST" && method != "GET" {
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
        "/health"
        | "/query/balance"
        | "/query/nonce"
        | "/query/state_root"
        | "/query/block"
        | "/query/validator"
        | "/query/committee"
        | "/query/git-head"
        | "/query/fee-estimate"
        | "/task/status"
        | "/task/list"
        | "/task/get"
        | "/review/list"
        | "/governance/list"
        | "/governance/get"
        | "/fastpath/list"
        | "/fastpath/status"
        | "/agent/status" => {
            // Read-only endpoints
            let guard = match driver.lock() {
                Ok(g) => g,
                Err(_) => return (500, r#"{"error":"internal error: mutex poisoned"}"#.into()),
            };
            dispatch_read(path, body, &guard)
        }
        "/tx/submit" | "/governance/propose" | "/governance/vote" | "/fastpath/approve" => {
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
        "/query/validator" => handle_query_validator(driver, body),
        "/query/committee" => handle_query_committee(driver, body),
        "/query/git-head" => handle_query_git_head(driver),
        "/query/fee-estimate" => handle_query_fee_estimate(driver),
        "/task/status" => handle_task_status(driver, body),
        "/task/list" => handle_task_list(driver, body),
        "/task/get" => handle_task_get(driver, body),
        "/review/list" => handle_review_list(driver),
        "/governance/list" => handle_governance_list(driver),
        "/governance/get" => handle_governance_get(driver, body),
        "/fastpath/list" => handle_fastpath_list(driver),
        "/fastpath/status" => handle_fastpath_status(driver, body),
        "/agent/status" => handle_agent_status(driver, body),
        _ => (500, r#"{"error":"internal routing error"}"#.into()),
    }
}

fn dispatch_write(path: &str, body: &str, driver: &mut ConsensusDriver) -> (u16, String) {
    match path {
        "/tx/submit" => handle_tx_submit(driver, body),
        "/governance/propose" => handle_governance_propose(driver, body),
        "/governance/vote" => handle_governance_vote(driver, body),
        "/fastpath/approve" => handle_fastpath_approve(driver, body),
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

    let signature = req
        .get("signature")
        .and_then(|v| v.as_str())
        .map(|s| hex::decode(s).unwrap_or_default())
        .unwrap_or_default();

    let tx_type = match tx_type_str {
        "transfer" => hyperfluid_consensus::types::TxType::TransferTx,
        "task_create" => hyperfluid_consensus::types::TxType::TaskCreateTx,
        "evidence" => hyperfluid_consensus::types::TxType::EvidenceTx,
        "fast_path" => hyperfluid_consensus::types::TxType::FastPathTx,
        "staking" => {
            use hyperfluid_consensus::types::StakingAction;
            let action_str = req.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let action = match action_str {
                "bond" => StakingAction::Bond,
                "unbond" => StakingAction::Unbond,
                "withdraw" => StakingAction::Withdraw,
                "renew" => StakingAction::Renew,
                _ => {
                    return (
                        400,
                        format!(r#"{{"error":"unknown staking action: {}"}}"#, action_str),
                    )
                }
            };
            hyperfluid_consensus::types::TxType::StakingTx(action)
        }
        "delegation" => {
            use hyperfluid_consensus::types::DelegationAction;
            let action_str = req.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let action = match action_str {
                "delegate" => DelegationAction::Delegate,
                "undelegate" => DelegationAction::Undelegate,
                "withdraw_delegation" => DelegationAction::WithdrawDelegation,
                "set_commission" => DelegationAction::SetCommission,
                _ => {
                    return (
                        400,
                        format!(r#"{{"error":"unknown delegation action: {}"}}"#, action_str),
                    )
                }
            };
            hyperfluid_consensus::types::TxType::DelegationTx(action)
        }
        "claim_task" => hyperfluid_consensus::types::TxType::ClaimTaskTx,
        "heartbeat" => hyperfluid_consensus::types::TxType::HeartbeatTx,
        "submit_task" => hyperfluid_consensus::types::TxType::SubmitTaskTx,
        "submit_review" => hyperfluid_consensus::types::TxType::SubmitReviewTx,
        "release_task" => hyperfluid_consensus::types::TxType::ReleaseTaskTx,
        "split_task" => hyperfluid_consensus::types::TxType::SplitTaskTx,
        "governance" => {
            use hyperfluid_consensus::types::GovernanceAction;
            let action_str = req.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let action = match action_str {
                "propose" => GovernanceAction::Propose,
                "vote" => GovernanceAction::Vote,
                _ => {
                    return (
                        400,
                        format!(r#"{{"error":"unknown governance action: {}"}}"#, action_str),
                    )
                }
            };
            hyperfluid_consensus::types::TxType::GovernanceTx(action)
        }
        _ => {
            return (400, format!(r#"{{"error":"unknown tx_type: {}"}}"#, tx_type_str));
        }
    };

    let tx = TransactionEnvelope {
        tx_type,
        tx_payload: payload,
        approved_plan_id: None,
        gateway_signature: None,
        signature,
    };

    // Submit to driver (validates + queues in mempool for next block)
    let accepted = driver.submit_tx(tx.clone()).is_ok();
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

fn handle_governance_propose(driver: &mut ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let proposer = match parse_hash32(&req, "proposer") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    let target_hash = match req.get("target_hash").and_then(|v| v.as_str()) {
        Some(s) => {
            let bytes = match hex::decode(s) {
                Ok(b) => b,
                Err(_) => {
                    return (400, r#"{"error":"Invalid target_hash: hex decode failed"}"#.into())
                }
            };
            if bytes.len() != 32 {
                return (400, r#"{"error":"target_hash must be 32 bytes (64 hex chars)"}"#.into());
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        None => {
            return (400, r#"{"error":"target_hash is required for governance proposals"}"#.into())
        }
    };
    let title_hash = match req.get("title_hash").and_then(|v| v.as_str()) {
        Some(s) => {
            let bytes = match hex::decode(s) {
                Ok(b) => b,
                Err(_) => {
                    return (400, r#"{"error":"Invalid title_hash: hex decode failed"}"#.into())
                }
            };
            if bytes.len() != 32 {
                return (400, r#"{"error":"title_hash must be 32 bytes (64 hex chars)"}"#.into());
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        None => {
            return (400, r#"{"error":"title_hash is required for governance proposals"}"#.into())
        }
    };
    let description_hash = match req.get("description_hash").and_then(|v| v.as_str()) {
        Some(s) => {
            let bytes = match hex::decode(s) {
                Ok(b) => b,
                Err(_) => {
                    return (
                        400,
                        r#"{"error":"Invalid description_hash: hex decode failed"}"#.into(),
                    )
                }
            };
            if bytes.len() != 32 {
                return (
                    400,
                    r#"{"error":"description_hash must be 32 bytes (64 hex chars)"}"#.into(),
                );
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        None => {
            return (
                400,
                r#"{"error":"description_hash is required for governance proposals"}"#.into(),
            )
        }
    };

    // Compute proposal_id = SHA3-256(proposer || target_hash)
    let proposal_id = {
        use sha3::Digest;
        let mut hasher = sha3::Sha3_256::new();
        hasher.update(proposer);
        hasher.update(target_hash);
        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out
    };

    // Build SCALE-encoded GovernancePayload (see driver.rs line 62):
    //   proposal_id (32) + proposer_id (32) + is_vote=false (1) + vote_approve=false (1)
    //   + target_hash (32) + title_hash (32) + description_hash (32)
    let signature = req
        .get("signature")
        .and_then(|v| v.as_str())
        .map(|s| hex::decode(s).unwrap_or_default())
        .unwrap_or_default();
    let mut payload = Vec::with_capacity(162);
    payload.extend_from_slice(&proposal_id);
    payload.extend_from_slice(&proposer);
    payload.push(0u8); // is_vote = false
    payload.push(0u8); // vote_approve = false
    payload.extend_from_slice(&target_hash);
    payload.extend_from_slice(&title_hash);
    payload.extend_from_slice(&description_hash);

    let tx = TransactionEnvelope {
        tx_type: hyperfluid_consensus::types::TxType::GovernanceTx(
            hyperfluid_consensus::types::GovernanceAction::Propose,
        ),
        tx_payload: payload,
        approved_plan_id: None,
        gateway_signature: None,
        signature,
    };

    let accepted = driver.submit_tx(tx).is_ok();
    let json = serde_json::json!({
        "status": if accepted { "submitted_to_mempool" } else { "rejected" },
        "proposer": hex::encode(proposer),
        "target_hash": hex::encode(target_hash),
        "proposal_id": hex::encode(proposal_id),
    });
    (200, json.to_string())
}

fn handle_governance_vote(driver: &mut ConsensusDriver, body: &str) -> (u16, String) {
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
    let vote_yes = req.get("approve").and_then(|v| v.as_bool()).unwrap_or(false);
    let target_hash = match req.get("target_hash").and_then(|v| v.as_str()) {
        Some(s) => {
            let bytes = match hex::decode(s) {
                Ok(b) => b,
                Err(_) => {
                    return (400, r#"{"error":"Invalid target_hash: hex decode failed"}"#.into())
                }
            };
            if bytes.len() != 32 {
                return (400, r#"{"error":"target_hash must be 32 bytes (64 hex chars)"}"#.into());
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            arr
        }
        None => return (400, r#"{"error":"target_hash is required for governance votes"}"#.into()),
    };

    // Compute title_hash from optional "title" string, or use zero if absent.
    let title_hash = match req.get("title").and_then(|v| v.as_str()) {
        Some(title) => {
            use sha3::Digest;
            let mut hasher = sha3::Sha3_256::new();
            hasher.update(title.as_bytes());
            let mut out = [0u8; 32];
            out.copy_from_slice(&hasher.finalize());
            out
        }
        None => return (400, r#"{"error":"title_hash is required for governance votes"}"#.into()),
    };

    // Compute description_hash from optional "description" string, or use zero if absent.
    let description_hash = match req.get("description").and_then(|v| v.as_str()) {
        Some(desc) => {
            use sha3::Digest;
            let mut hasher = sha3::Sha3_256::new();
            hasher.update(desc.as_bytes());
            let mut out = [0u8; 32];
            out.copy_from_slice(&hasher.finalize());
            out
        }
        None => {
            return (400, r#"{"error":"description_hash is required for governance votes"}"#.into())
        }
    };

    // Build SCALE-encoded GovernancePayload for vote:
    //   proposal_id (32) + proposer_id/voter (32) + is_vote=true (1) + vote_approve (1)
    //   + target_hash (32) + title_hash (32) + description_hash (32)
    let signature = req
        .get("signature")
        .and_then(|v| v.as_str())
        .map(|s| hex::decode(s).unwrap_or_default())
        .unwrap_or_default();
    let mut payload = Vec::with_capacity(162);
    payload.extend_from_slice(&proposal_id);
    payload.extend_from_slice(&voter);
    payload.push(1u8); // is_vote = true
    payload.push(if vote_yes { 1u8 } else { 0u8 }); // vote_approve
    payload.extend_from_slice(&target_hash);
    payload.extend_from_slice(&title_hash); // F-83: computed from "title" if provided
    payload.extend_from_slice(&description_hash); // F-83: computed from "description" if provided

    let tx = TransactionEnvelope {
        tx_type: hyperfluid_consensus::types::TxType::GovernanceTx(
            hyperfluid_consensus::types::GovernanceAction::Vote,
        ),
        tx_payload: payload,
        approved_plan_id: None,
        gateway_signature: None,
        signature,
    };

    let accepted = driver.submit_tx(tx).is_ok();
    let json = serde_json::json!({
        "proposal_id": hex::encode(proposal_id),
        "voter": hex::encode(voter),
        "status": if accepted { "submitted_to_mempool" } else { "rejected" },
    });
    (200, json.to_string())
}

fn handle_query_validator(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let validator_id = match parse_hash32(&req, "validator_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    match driver.state_machine.get_validator(&validator_id) {
        Some(v) => {
            let json = serde_json::json!({
                "validator_id": hex::encode(v.validator_id),
                "self_bond": v.self_bond,
                "total_delegated": v.total_delegated,
                "state": format!("{:?}", v.state),
            });
            (200, json.to_string())
        }
        None => (404, r#"{"error":"validator not found"}"#.to_string()),
    }
}

fn handle_query_committee(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let epoch = req.get("epoch").and_then(|v| v.as_u64()).unwrap_or(driver.epoch);
    let committee_hash = driver.committee_history.get(&epoch).copied();
    let validators =
        driver.epoch_validators.get(&epoch).map(|v| v.iter().map(hex::encode).collect::<Vec<_>>());
    let json = serde_json::json!({
        "epoch": epoch,
        "current_epoch": driver.epoch,
        "committee_hash": committee_hash.map(hex::encode),
        "validator_count": validators.as_ref().map(|v| v.len()).unwrap_or(0),
        "validators": validators,
    });
    (200, json.to_string())
}

fn handle_query_git_head(driver: &ConsensusDriver) -> (u16, String) {
    let json = serde_json::json!({
        "git_head": hex::encode(driver.git_head()),
    });
    (200, json.to_string())
}

fn handle_query_fee_estimate(driver: &ConsensusDriver) -> (u16, String) {
    let json = serde_json::json!({
        "base_fee": driver.fee_state.base_fee,
        "target_utilization_pct": driver.fee_config.target_utilization_pct,
    });
    (200, json.to_string())
}

fn handle_task_list(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => serde_json::Value::Null,
    };
    let status_filter = req.get("status").and_then(|v| v.as_str());
    let seed_filter = req.get("seed_ref").and_then(|v| v.as_str());
    let tasks: Vec<serde_json::Value> = driver
        .state_machine
        .tasks_iter()
        .filter(|t| {
            status_filter
                .is_none_or(|s| format!("{:?}", t.status).to_lowercase() == s.to_lowercase())
        })
        .filter(|t| seed_filter.is_none_or(|s| hex::encode(t.seed_ref).starts_with(s)))
        .map(|t| {
            serde_json::json!({
                "task_id": hex::encode(t.task_id),
                "status": format!("{:?}", t.status),
                "bounty_agx": t.bounty_agx,
                "primary_owner": hex::encode(t.primary_owner),
                "created_at_height": t.created_at_height,
            })
        })
        .collect();
    let json = serde_json::json!({
        "tasks": tasks,
        "count": tasks.len(),
    });
    (200, json.to_string())
}

fn handle_task_get(driver: &ConsensusDriver, body: &str) -> (u16, String) {
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
                "topic_id": hex::encode(task.topic_id),
                "seed_ref": hex::encode(task.seed_ref),
                "parent_task_id": hex::encode(task.parent_task_id),
                "depends_on": task.depends_on.iter().map(hex::encode).collect::<Vec<_>>(),
                "funder": hex::encode(task.funder),
                "primary_owner": hex::encode(task.primary_owner),
                "status": format!("{:?}", task.status),
                "bounty_agx": task.bounty_agx,
                "created_at_height": task.created_at_height,
                "lease_expires_height": task.lease_expires_height,
                "required_skills_hash": hex::encode(task.required_skills_hash),
                "metadata_hash": hex::encode(task.metadata_hash),
                "sponsor_id": hex::encode(task.sponsor_id),
                "requester_pubkey": hex::encode(task.requester_pubkey),
                "escrow_status": format!("{:?}", task.escrow_status),
            });
            (200, json.to_string())
        }
        None => (404, r#"{"error":"task not found"}"#.to_string()),
    }
}

fn handle_review_list(driver: &ConsensusDriver) -> (u16, String) {
    let tasks: Vec<serde_json::Value> = driver
        .state_machine
        .tasks_iter()
        .filter(|t| matches!(t.status, TaskStatus::InReview))
        .map(|t| {
            serde_json::json!({
                "task_id": hex::encode(t.task_id),
                "status": "InReview",
                "bounty_agx": t.bounty_agx,
                "created_at_height": t.created_at_height,
            })
        })
        .collect();
    let json = serde_json::json!({
        "review_tasks": tasks,
        "count": tasks.len(),
    });
    (200, json.to_string())
}

fn handle_governance_list(driver: &ConsensusDriver) -> (u16, String) {
    let proposals: Vec<serde_json::Value> = driver
        .governance
        .proposal_ids()
        .iter()
        .filter_map(|id| {
            driver.governance.get_proposal(id).map(|p| {
                serde_json::json!({
                    "proposal_id": hex::encode(p.proposal_id),
                    "proposer_id": hex::encode(p.proposer_id),
                    "status": format!("{:?}", p.status),
                    "yes_weight": p.yes_weight,
                    "no_weight": p.no_weight,
                    "vote_end_height": p.vote_end_height,
                })
            })
        })
        .collect();
    let json = serde_json::json!({
        "proposals": proposals,
        "count": proposals.len(),
    });
    (200, json.to_string())
}

fn handle_governance_get(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let proposal_id = match parse_hash32(&req, "proposal_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    match driver.governance.get_proposal(&proposal_id) {
        Some(p) => {
            let json = serde_json::json!({
                "proposal_id": hex::encode(p.proposal_id),
                "proposer_id": hex::encode(p.proposer_id),
                "proposed_commit": hex::encode(p.proposed_commit),
                "current_commit": hex::encode(p.current_commit),
                "status": format!("{:?}", p.status),
                "yes_weight": p.yes_weight,
                "no_weight": p.no_weight,
                "vote_start_height": p.vote_start_height,
                "vote_end_height": p.vote_end_height,
            });
            (200, json.to_string())
        }
        None => (404, r#"{"error":"proposal not found"}"#.to_string()),
    }
}

fn handle_fastpath_list(driver: &ConsensusDriver) -> (u16, String) {
    let proposals: Vec<serde_json::Value> = driver
        .fastpath
        .proposals_iter()
        .map(|p| {
            serde_json::json!({
                "proposal_id": hex::encode(p.proposal_id),
                "topic_id": hex::encode(p.topic_id),
                "proposer_id": hex::encode(p.proposer_id),
                "expires_at_height": p.expires_at_height,
            })
        })
        .collect();
    let json = serde_json::json!({
        "proposals": proposals,
        "count": proposals.len(),
    });
    (200, json.to_string())
}

fn handle_fastpath_status(driver: &ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let proposal_id = match parse_hash32(&req, "proposal_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    let proposal = driver.fastpath.get_proposal(&proposal_id);
    let certificate = driver.fastpath.get_certificate(&proposal_id);
    let json = serde_json::json!({
        "proposal_id": hex::encode(proposal_id),
        "proposal": proposal.map(|p| serde_json::json!({
            "topic_id": hex::encode(p.topic_id),
            "expires_at_height": p.expires_at_height,
        })),
        "certificate": certificate.map(|_c| serde_json::json!({"issued": true})),
    });
    (200, json.to_string())
}

fn handle_fastpath_approve(driver: &mut ConsensusDriver, body: &str) -> (u16, String) {
    let req: serde_json::Value = match serde_json::from_str(body) {
        Ok(v) => v,
        Err(_) => return (400, r#"{"error":"invalid json"}"#.into()),
    };
    let proposal_id = match parse_hash32(&req, "proposal_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };
    let reviewer_id = match parse_hash32(&req, "reviewer_id") {
        Ok(id) => id,
        Err(e) => return (400, format!(r#"{{"error":"{}"}}"#, e)),
    };

    // F-8: Parse the reviewer's signature (hex-encoded ML-DSA-65 signature)
    let signature_hex = match req.get("signature").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return (400, r#"{"error":"missing 'signature' field"}"#.into()),
    };
    let signature_bytes = match hex::decode(signature_hex) {
        Ok(b) => b,
        Err(_) => return (400, r#"{"error":"invalid hex in 'signature'"}"#.into()),
    };

    // F-25: Compute reason_hash from the reviewer's reason string
    let reason = req.get("reason").and_then(|v| v.as_str()).unwrap_or("");
    let reason_hash = {
        use sha3::Digest;
        let mut hasher = sha3::Sha3_256::new();
        hasher.update(reason.as_bytes());
        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out
    };

    // F-8: Look up the reviewer's public key from the state machine account
    let account = match driver.state_machine.get_account(&reviewer_id) {
        Some(acct) => acct,
        None => return (400, r#"{"error":"Reviewer identity not found"}"#.into()),
    };
    let pubkey = match account.pubkey.as_ref() {
        Some(pk) => pk.clone(),
        None => return (400, r#"{"error":"Reviewer identity not found"}"#.into()),
    };

    // F-8: Build the signed message: proposal_id (32) || reviewer_id (32) || vote_byte (1) || reason_hash (32)
    use hyperfluid_p2p::identity::Identity;
    let vote_byte: u8 = 1u8; // Approve
    let mut signing_message = Vec::with_capacity(97);
    signing_message.extend_from_slice(&proposal_id);
    signing_message.extend_from_slice(&reviewer_id);
    signing_message.push(vote_byte);
    signing_message.extend_from_slice(&reason_hash);

    // F-8: Verify the signature against the reviewer's pubkey
    if !Identity::verify_with_pubkey(&pubkey, &signing_message, &signature_bytes) {
        return (400, r#"{"error":"Invalid reviewer signature"}"#.into());
    }

    use hyperfluid_fastpath::types::ReviewerVote;
    let approval = hyperfluid_fastpath::types::ReviewerSignature {
        reviewer_id,
        vote: ReviewerVote::Approve,
        signature: signature_bytes,
        reason_hash,
    };
    let topic_weight: u128 = req.get("topic_weight").and_then(|v| v.as_u64()).unwrap_or(1) as u128;
    match driver.fastpath.submit_approval(proposal_id, approval, driver.height, topic_weight) {
        Ok(cert) => {
            let json = serde_json::json!({
                "status": "approved",
                "proposal_id": hex::encode(proposal_id),
                "certificate_issued": cert.is_some(),
            });
            (200, json.to_string())
        }
        Err(e) => (400, format!(r#"{{"error":"{:?}"}}"#, e)),
    }
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
