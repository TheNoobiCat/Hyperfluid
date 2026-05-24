// === Hyperfluid Node Binary ===
//
// Full node binary: genesis boot, config loading, real block production loop
// via ConsensusDriver, P2P TCP transport, local JSON-RPC server, clean shutdown.
//
// Source: docs/05-planning/stages/stage-01-protocol-core.md

pub mod rpc;

use hyperfluid_consensus::driver::ConsensusDriver;
use hyperfluid_consensus::genesis::GenesisConfig;
use hyperfluid_p2p::transport::PeerCache;
use hyperfluid_p2p::types::{DiscoveryConfig, Hash32};
use hyperfluid_p2p::{identity::Identity, tcp::TcpTransport};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Debug)]
struct NodeConfig {
    gen_genesis: bool,
    genesis_path: Option<PathBuf>,
    genesis: GenesisConfig,
    block_interval_secs: u64,
    p2p_bind: SocketAddr,
    node_key_path: PathBuf,
}

impl NodeConfig {
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();

        let mut gen_genesis = false;
        let mut genesis_path: Option<PathBuf> = None;
        let mut block_interval_secs: u64 = 2;
        let mut p2p_bind_str: Option<String> = None;
        let mut node_key_path = PathBuf::from("node_key");

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
                "--block-interval" => {
                    i += 1;
                    if i < args.len() {
                        block_interval_secs = args[i].parse().unwrap_or(2);
                    }
                }
                "--p2p-bind" => {
                    i += 1;
                    if i < args.len() {
                        p2p_bind_str = Some(args[i].clone());
                    }
                }
                "--node-key" => {
                    i += 1;
                    if i < args.len() {
                        node_key_path = PathBuf::from(&args[i]);
                    }
                }
                _ => {}
            }
            i += 1;
        }

        let p2p_bind: SocketAddr = p2p_bind_str
            .or_else(|| std::env::var("HYPERFLUID_P2P_BIND").ok())
            .unwrap_or_else(|| "0.0.0.0:0".to_string())
            .parse()
            .expect("invalid P2P bind address (expected IP:port format, e.g. 0.0.0.0:9876)");

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

        Self { gen_genesis, genesis_path, genesis, block_interval_secs, p2p_bind, node_key_path }
    }
}

/// Load an existing node identity from disk, or generate a new one and persist it.
fn load_or_create_identity(path: &PathBuf) -> Identity {
    if let Ok(seed_bytes) = std::fs::read(path) {
        if seed_bytes.len() == 32 {
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&seed_bytes);
            let identity = Identity::from_seed(&seed);
            tracing::info!(
                "Loaded node identity from {} (peer_id: {})",
                path.display(),
                hex::encode(identity.peer_id()),
            );
            return identity;
        }
        tracing::warn!(
            "Node key file {} has wrong size ({} bytes, expected 32) — generating new identity",
            path.display(),
            seed_bytes.len(),
        );
    }
    let identity = Identity::generate();
    let seed = identity.to_seed();
    if let Err(e) = std::fs::write(path, seed) {
        tracing::warn!(
            "Failed to persist node identity to {}: {} — identity will not survive restart",
            path.display(),
            e,
        );
    } else {
        tracing::info!(
            "Generated new node identity (peer_id: {}) — saved to {}",
            hex::encode(identity.peer_id()),
            path.display(),
        );
    }
    identity
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
    tracing::info!("Block interval: {}s", config.block_interval_secs);

    // ── Initialise consensus driver with genesis state ──
    let mut driver = ConsensusDriver::new(config.genesis.epoch_length);
    let genesis_block = driver.init_genesis(&config.genesis);

    tracing::info!(
        "Genesis block created: height={}, hash={}, state_root={}",
        genesis_block.header.height,
        hex::encode(genesis_block.header.block_hash()),
        hex::encode(genesis_block.header.state_root),
    );

    // Optionally write genesis config to disk
    if config.gen_genesis {
        let genesis_json = serde_json::to_string_pretty(&config.genesis)
            .expect("failed to serialize genesis config");
        let out_path = config.genesis_path.unwrap_or_else(|| PathBuf::from("genesis.json"));
        std::fs::write(&out_path, genesis_json)
            .unwrap_or_else(|e| panic!("Failed to write genesis file {:?}: {}", out_path, e));
        tracing::info!("Genesis config written to {:?}", out_path);
    }

    // Wrap driver for shared access across async tasks and RPC server
    let driver = Arc::new(Mutex::new(driver));

    // ── Local JSON-RPC server (loopback only) ──
    let rpc_port: u16 =
        std::env::var("HYPERFLUID_RPC_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(8545);
    let (_rpc_handle, rpc_addr) =
        rpc::start_rpc_server(Arc::clone(&driver), ([127, 0, 0, 1], rpc_port).into());
    tracing::info!("JSON-RPC server listening on {} (local-only)", rpc_addr);

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Register signal handler for clean shutdown (ctrl-c, SIGTERM)
    let r_shutdown = r.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        tracing::info!("Shutdown signal received, stopping consensus loop...");
        r_shutdown.store(false, Ordering::SeqCst);
    });

    let block_interval = Duration::from_secs(config.block_interval_secs);

    // ── P2P TCP transport startup ──
    let p2p_bind_addr = config.p2p_bind;
    let p2p_config = DiscoveryConfig::default();
    let peer_cache = Arc::new(tokio::sync::RwLock::new(PeerCache::new()));
    let tcp_transport = Arc::new(TcpTransport::new(p2p_config, Arc::clone(&peer_cache)));

    // Persist the node identity so peer_id is stable across restarts.
    let local_identity = Arc::new(load_or_create_identity(&config.node_key_path));
    let local_peer_id = *local_identity.peer_id();

    {
        let listener = tokio::net::TcpListener::bind(p2p_bind_addr)
            .await
            .expect("failed to bind P2P TCP listener");
        let actual_addr = listener.local_addr().expect("failed to get P2P listener address");
        tracing::info!(
            "P2P TCP listener started on {} (peer_id: {})",
            actual_addr,
            hex::encode(local_peer_id),
        );

        let transport = Arc::clone(&tcp_transport);
        let identity = Arc::clone(&local_identity);
        let r_p2p = running.clone();
        // Build key provider from genesis validator set.
        // Each validator's account pubkey provides the DH and KEM key material
        // needed to authenticate inbound P2P connections.
        let mut key_map: HashMap<Hash32, ([u8; 32], Vec<u8>)> = HashMap::new();
        {
            let account_pubkeys: HashMap<Hash32, &Vec<u8>> = config
                .genesis
                .accounts
                .iter()
                .filter_map(|a| a.pubkey.as_ref().map(|pk| (a.account_id, pk)))
                .collect();
            for v in &config.genesis.validators {
                if let Some(pubkey) = account_pubkeys.get(&v.validator_id) {
                    let mut dh_key = [0u8; 32];
                    let copy_len = pubkey.len().min(32);
                    dh_key[..copy_len].copy_from_slice(&pubkey[..copy_len]);
                    let kem_key =
                        if pubkey.len() > 32 { pubkey[32..].to_vec() } else { Vec::new() };
                    key_map.insert(v.validator_id, (dh_key, kem_key));
                }
            }
        }
        let key_provider = Arc::new(move |peer_id: &Hash32| -> Option<([u8; 32], Vec<u8>)> {
            key_map.get(peer_id).cloned()
        });
        tokio::spawn(async move {
            TcpTransport::accept_loop(listener, identity, key_provider, transport).await;
            r_p2p.store(false, Ordering::Release);
        });
    }

    tracing::info!("Node entering consensus loop (live block production)");

    // Start the async block production loop
    let loop_handle =
        ConsensusDriver::run_block_loop(driver.clone(), running.clone(), block_interval);

    // Wait for the block loop to complete (it exits when running becomes false)
    match loop_handle.await {
        Ok(()) => tracing::info!("Block loop exited cleanly"),
        Err(e) => tracing::error!("Block loop panicked or was cancelled: {}", e),
    }
    running.store(false, Ordering::SeqCst);

    // Give the loop a moment to flush
    tokio::time::sleep(Duration::from_millis(200)).await;

    // Report final state
    {
        let guard = driver.lock();
        match guard {
            Ok(driver) => {
                let last_block = driver.block_store.last();
                let final_height = driver.height;
                let last_hash = last_block
                    .map(|b| hex::encode(b.header.block_hash()))
                    .unwrap_or_else(|| "none".to_string());
                let smt_root = hex::encode(driver.state_machine.compute_state_root());

                tracing::info!(
                    "Node shutdown: final height={}, last_block_hash={}, smt_root={}",
                    final_height,
                    last_hash,
                    smt_root,
                );
            }
            Err(_) => {
                tracing::warn!(
                    "Consensus driver mutex poisoned at shutdown; skipping final state report"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_block_height_zero() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let mut driver = ConsensusDriver::new(genesis.epoch_length);
        let block = driver.init_genesis(&genesis);
        assert_eq!(block.header.height, 0);
        assert_eq!(block.header.parent_hash, [0u8; 32]);
        assert!(block.transactions.is_empty());
    }

    #[test]
    fn genesis_block_epoch_zero() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let mut driver = ConsensusDriver::new(genesis.epoch_length);
        let block = driver.init_genesis(&genesis);
        assert_eq!(block.header.epoch, 0);
    }

    #[test]
    fn genesis_block_timestamp_matches_config() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let mut driver = ConsensusDriver::new(genesis.epoch_length);
        let block = driver.init_genesis(&genesis);
        assert_eq!(block.header.timestamp, genesis.timestamp);
    }

    #[test]
    fn node_produces_real_blocks() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let mut driver = ConsensusDriver::new(genesis.epoch_length);
        driver.init_genesis(&genesis);

        // Produce 5 blocks
        for i in 0..5u64 {
            let block = driver.produce_block(vec![], i + 1);
            assert_eq!(block.header.height, i + 1);
            assert_ne!(block.header.parent_hash, [0u8; 32]);
        }

        assert_eq!(driver.height, 5);
        assert_eq!(driver.block_store.len(), 6); // genesis + 5 produced

        // Verify parent chain integrity
        for window in driver.block_store.windows(2) {
            let parent_hash = window[1].header.parent_hash;
            let expected = window[0].header.block_hash();
            assert_eq!(parent_hash, expected);
        }
    }
}
