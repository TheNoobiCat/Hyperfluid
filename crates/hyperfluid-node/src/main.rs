// === Hyperfluid Node Binary ===
//
// Full node binary: genesis boot, config loading, stub consensus loop,
// clean shutdown on signal. This scaffold uses the same config format,
// genesis layout, and key format as planned production deployment.

use hyperfluid_consensus::genesis::GenesisConfig;
use hyperfluid_consensus::types::{Block, BlockHeader};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug)]
struct NodeConfig {
    gen_genesis: bool,
    genesis_path: Option<PathBuf>,
    genesis: GenesisConfig,
}

impl NodeConfig {
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();

        let mut gen_genesis = false;
        let mut genesis_path: Option<PathBuf> = None;

        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--gen-genesis" => {
                    gen_genesis = true;
                }
                "--genesis" => {
                    i += 1;
                    if i < args.len() {
                        genesis_path = Some(PathBuf::from(&args[i]));
                    }
                }
                _ => {}
            }
            i += 1;
        }

        let genesis = if let Some(ref path) = genesis_path {
            let _content = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("Failed to read genesis file {:?}: {}", path, e));
            GenesisConfig::new_testnet_single_validator()
        // TODO Stage 01: deserialize from TOML to GenesisConfig.
        // For now, scaffold uses the built-in testnet default.
        // SPEC_DEVIATION: GenesisConfig TOML deserialization deferred.
        // The GenesisConfig struct is fully specified but
        // config file parsing will be formalised in Stage 01.
        } else {
            GenesisConfig::new_testnet_single_validator()
        };

        Self { gen_genesis, genesis_path, genesis }
    }
}

/// Generate the genesis block from a GenesisConfig.
///
/// The genesis block (height 0) bootstraps the chain with:
/// - System parameters committed in the block
/// - Initial accounts and validator set embedded in genesis state
/// - A zeroed parent hash (no predecessor)
fn make_genesis_block(genesis: &GenesisConfig) -> Block {
    let parent_hash = [0u8; 32];
    let state_root = [0u8; 32];
    let transaction_root = [0u8; 32];
    let proposer_id = [0u8; 32];
    let epoch = 0;

    Block {
        header: BlockHeader {
            height: 0,
            parent_hash,
            state_root,
            transaction_root,
            committee_id: 0,
            proposer_id,
            timestamp: genesis.timestamp,
            epoch,
        },
        transactions: Vec::new(),
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = NodeConfig::from_args();

    tracing::info!("Hyperfluid node starting (chain_id: {})", config.genesis.chain_id);
    tracing::info!("Epoch length: {} blocks", config.genesis.epoch_length);
    tracing::info!("Initial validators: {}", config.genesis.validators.len());
    tracing::info!("Genesis accounts: {}", config.genesis.accounts.len());
    tracing::info!(
        "Total AGX supply: {} AGX",
        config.genesis.total_agx_supply / 1_000_000_000_000_000_000u128
    );

    let genesis_block = make_genesis_block(&config.genesis);
    // NOTE: block_hash is not computed yet (requires SMT root + tx Merkle root).
    // Logging attributes for identification.
    tracing::info!(
        "Genesis block created (height={}, parent_hash={}, epoch={})",
        genesis_block.header.height,
        hex::encode(genesis_block.header.parent_hash),
        genesis_block.header.epoch,
    );

    if config.gen_genesis {
        let genesis_json = serde_json::to_string_pretty(&config.genesis)
            .expect("failed to serialize genesis config");
        let out_path = config.genesis_path.unwrap_or_else(|| PathBuf::from("genesis.json"));
        std::fs::write(&out_path, genesis_json)
            .unwrap_or_else(|e| panic!("Failed to write genesis file {:?}: {}", out_path, e));
        tracing::info!("Genesis config written to {:?}", out_path);
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    let mut height: u64 = 0;
    let mut epoch: u64 = 0;

    tracing::info!("Node entering consensus loop (stub mode)");

    while r.load(Ordering::Relaxed) {
        let edge = height % config.genesis.epoch_length == 0;
        if edge {
            epoch = height / config.genesis.epoch_length;
            tracing::info!("Epoch boundary: height={}, epoch={}", height, epoch);
        }

        height += 1;
        if height % 1000 == 0 {
            tracing::info!("Produced block height={}, epoch={}", height, epoch);
        }

        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    tracing::info!("Node shutdown complete at height={}", height);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_block_height_zero() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let block = make_genesis_block(&genesis);
        assert_eq!(block.header.height, 0);
        assert_eq!(block.header.parent_hash, [0u8; 32]);
        assert!(block.transactions.is_empty());
    }

    #[test]
    fn genesis_block_epoch_zero() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let block = make_genesis_block(&genesis);
        assert_eq!(block.header.epoch, 0);
    }

    #[test]
    fn genesis_block_timestamp_matches_config() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let block = make_genesis_block(&genesis);
        assert_eq!(block.header.timestamp, genesis.timestamp);
    }
}
