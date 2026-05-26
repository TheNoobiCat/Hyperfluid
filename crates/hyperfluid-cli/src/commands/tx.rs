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
pub enum TxAction {
    Transfer {
        #[arg(long)]
        sender: String,
        #[arg(long)]
        recipient: String,
        #[arg(long)]
        amount: u128,
        #[arg(long)]
        nonce: u64,
    },
    Bond {
        #[arg(long)]
        validator: String,
        #[arg(long)]
        amount: u128,
        #[arg(long)]
        nonce: u64,
    },
    Unbond {
        #[arg(long)]
        validator: String,
        #[arg(long)]
        nonce: u64,
    },
    Withdraw {
        #[arg(long)]
        validator: String,
        #[arg(long)]
        nonce: u64,
    },
    Renew {
        #[arg(long)]
        validator: String,
        #[arg(long)]
        nonce: u64,
    },
    Delegate {
        #[arg(long)]
        delegator: String,
        #[arg(long)]
        validator: String,
        #[arg(long)]
        amount: u128,
        #[arg(long)]
        nonce: u64,
    },
    Undelegate {
        #[arg(long)]
        delegator: String,
        #[arg(long)]
        validator: String,
        #[arg(long)]
        nonce: u64,
    },
    WithdrawDelegation {
        #[arg(long)]
        delegator: String,
        #[arg(long)]
        validator: String,
        #[arg(long)]
        nonce: u64,
    },
    Commission {
        #[arg(long)]
        validator: String,
        #[arg(long)]
        rate: u16,
        #[arg(long)]
        nonce: u64,
        /// Delegator identity (hex, 32 bytes). Required — no longer hardcoded to zero.
        #[arg(long)]
        delegator: String,
    },
    Evidence {
        #[arg(long)]
        validator: String,
        #[arg(long)]
        evidence_type: u8,
        #[arg(long)]
        evidence_height: u64,
        #[arg(long)]
        missed_blocks: u64,
        #[arg(long)]
        total_window_blocks: u64,
    },
}

pub fn run(
    action: TxAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
    identity: &Identity,
) -> Result<String, String> {
    let result = match action {
        TxAction::Transfer { sender, recipient, amount, nonce } => {
            let sender_id = parse_hash32(&sender)?;
            let recipient_id = parse_hash32(&recipient)?;
            let payload = (sender_id, recipient_id, amount, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "transfer",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        TxAction::Bond { validator, amount, nonce } => {
            let v = parse_hash32(&validator)?;
            let payload = (v, amount, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "staking",
                    "action": "bond",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        TxAction::Unbond { validator, nonce } => {
            let v = parse_hash32(&validator)?;
            let payload = (v, 0u128 /* amount: no transfer */, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "staking",
                    "action": "unbond",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        TxAction::Withdraw { validator, nonce } => {
            let v = parse_hash32(&validator)?;
            let payload = (v, 0u128 /* amount: no transfer */, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "staking",
                    "action": "withdraw",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        TxAction::Renew { validator, nonce } => {
            let v = parse_hash32(&validator)?;
            let payload = (v, 0u128 /* amount: no transfer */, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "staking",
                    "action": "renew",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        TxAction::Delegate { delegator, validator, amount, nonce } => {
            let d = parse_hash32(&delegator)?;
            let v = parse_hash32(&validator)?;
            let payload = (d, v, amount, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "delegation",
                    "action": "delegate",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        TxAction::Undelegate { delegator, validator, nonce } => {
            let d = parse_hash32(&delegator)?;
            let v = parse_hash32(&validator)?;
            let payload = (d, v, 0u128 /* amount: no transfer */, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "delegation",
                    "action": "undelegate",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        TxAction::WithdrawDelegation { delegator, validator, nonce } => {
            let d = parse_hash32(&delegator)?;
            let v = parse_hash32(&validator)?;
            let payload = (d, v, 0u128 /* amount: no transfer */, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "delegation",
                    "action": "withdraw_delegation",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        // F-68: Use provided delegator identity instead of hardcoded zero
        TxAction::Commission { validator, rate, nonce, delegator } => {
            let delegator_id = parse_hash32(&delegator)?;
            let v = parse_hash32(&validator)?;
            let payload = (delegator_id, v, rate as u128, nonce);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "delegation",
                    "action": "set_commission",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
        TxAction::Evidence {
            validator,
            evidence_type,
            evidence_height,
            missed_blocks,
            total_window_blocks,
        } => {
            let v = parse_hash32(&validator)?;
            let payload = (evidence_type, v, evidence_height, missed_blocks, total_window_blocks);
            let (payload_hex, sig_hex, pubkey_hex) = sign_payload(identity, &payload);
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "evidence",
                    "payload": payload_hex,
                    "signature": sig_hex,
                    "pubkey": pubkey_hex,
                }),
            )?
        }
    };
    Ok(format_output(&result, format))
}
