use clap::Subcommand;

use crate::commands::{format_output, rpc_post};
use crate::OutputFormat;

#[derive(Subcommand)]
pub enum QueryAction {
    Balance {
        #[arg(long)]
        account: String,
    },
    Nonce {
        #[arg(long)]
        account: String,
    },
    StateRoot,
    Block {
        #[arg(long)]
        height: u64,
    },
    Validator {
        #[arg(long)]
        validator_id: String,
    },
    Committee {
        #[arg(long)]
        epoch: Option<u64>,
    },
    GitHead,
    FeeEstimate,
}

pub fn run(
    action: QueryAction,
    format: OutputFormat,
    client: &reqwest::blocking::Client,
    node_url: &str,
) -> Result<String, String> {
    let result = match action {
        QueryAction::Balance { account } => rpc_post(
            client,
            node_url,
            "/query/balance",
            serde_json::json!({ "account_id": account }),
        )?,
        QueryAction::Nonce { account } => rpc_post(
            client,
            node_url,
            "/query/nonce",
            serde_json::json!({ "account_id": account }),
        )?,
        QueryAction::StateRoot => {
            rpc_post(client, node_url, "/query/state_root", serde_json::json!({}))?
        }
        QueryAction::Block { height } => {
            rpc_post(client, node_url, "/query/block", serde_json::json!({ "height": height }))?
        }
        QueryAction::Validator { validator_id } => rpc_post(
            client,
            node_url,
            "/query/validator",
            serde_json::json!({ "validator_id": validator_id }),
        )?,
        QueryAction::Committee { epoch } => {
            rpc_post(client, node_url, "/query/committee", serde_json::json!({ "epoch": epoch }))?
        }
        QueryAction::GitHead => {
            rpc_post(client, node_url, "/query/git-head", serde_json::json!({}))?
        }
        QueryAction::FeeEstimate => {
            rpc_post(client, node_url, "/query/fee-estimate", serde_json::json!({}))?
        }
    };
    Ok(format_output(&result, format))
}
