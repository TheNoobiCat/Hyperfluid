use clap::Subcommand;

use crate::commands::{format_output, rpc_post};
use crate::OutputFormat;

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
    Commission {
        #[arg(long)]
        validator: String,
        #[arg(long)]
        rate: u16,
        #[arg(long)]
        nonce: u64,
    },
}

pub fn run(
    action: TxAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
) -> Result<String, String> {
    let result = match action {
        TxAction::Transfer { sender, recipient, amount, nonce } => {
            let payload = encode_transfer_payload(&sender, &recipient, amount, nonce)?;
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "transfer",
                    "payload": payload,
                }),
            )?
        }
        TxAction::Bond { validator, amount, nonce } => {
            let payload = encode_staking_payload("bond", &validator, amount, nonce)?;
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "staking",
                    "payload": payload,
                }),
            )?
        }
        TxAction::Unbond { validator, nonce } => {
            let payload = encode_staking_payload("unbond", &validator, 0, nonce)?;
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "staking",
                    "payload": payload,
                }),
            )?
        }
        TxAction::Withdraw { validator, nonce } => {
            let payload = encode_staking_payload("withdraw", &validator, 0, nonce)?;
            rpc_post(
                client,
                node_url,
                "/tx/submit",
                serde_json::json!({
                    "tx_type": "staking",
                    "payload": payload,
                }),
            )?
        }
        TxAction::Delegate { delegator, validator, amount, nonce } => rpc_post(
            client,
            node_url,
            "/tx/submit",
            serde_json::json!({
                "tx_type": "delegation",
                "payload": hex::encode(format!("delegate:{}:{}:{}:{}", delegator, validator, amount, nonce)),
            }),
        )?,
        TxAction::Undelegate { delegator, validator, nonce } => rpc_post(
            client,
            node_url,
            "/tx/submit",
            serde_json::json!({
                "tx_type": "delegation",
                "payload": hex::encode(format!("undelegate:{}:{}:0:{}", delegator, validator, nonce)),
            }),
        )?,
        TxAction::Commission { validator, rate, nonce } => rpc_post(
            client,
            node_url,
            "/tx/submit",
            serde_json::json!({
                "tx_type": "delegation",
                "payload": hex::encode(format!("commission:{}:{}:{}", validator, rate, nonce)),
            }),
        )?,
    };
    Ok(format_output(&result, format))
}

fn encode_transfer_payload(
    sender: &str,
    recipient: &str,
    amount: u128,
    nonce: u64,
) -> Result<String, String> {
    use parity_scale_codec::Encode;
    let sender_id = parse_hash32(sender)?;
    let recipient_id = parse_hash32(recipient)?;
    let payload = (sender_id, recipient_id, amount, nonce);
    Ok(hex::encode(payload.encode()))
}

fn encode_staking_payload(
    _action: &str,
    validator: &str,
    amount: u128,
    nonce: u64,
) -> Result<String, String> {
    use parity_scale_codec::Encode;
    let validator_id = parse_hash32(validator)?;
    let payload = (validator_id, amount, nonce);
    Ok(hex::encode(payload.encode()))
}

fn parse_hash32(hex_str: &str) -> Result<[u8; 32], String> {
    let bytes = hex::decode(hex_str).map_err(|e| format!("invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("expected 32 bytes (64 hex chars), got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}
