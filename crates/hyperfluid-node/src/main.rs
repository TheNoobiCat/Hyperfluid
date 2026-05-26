// === Hyperfluid Node Binary ===
//
// Full node binary: genesis boot, config loading, real block production loop
// via ConsensusDriver, P2P TCP transport, local JSON-RPC server, clean shutdown.
//
// Source: docs/05-planning/stages/stage-01-protocol-core.md

pub mod rpc;

use hyperfluid_consensus::driver::ConsensusDriver;
use hyperfluid_consensus::genesis::GenesisConfig;
use hyperfluid_consensus::malachite::Address32;
use hyperfluid_consensus::malachite_consensus;
use hyperfluid_consensus::network_bridge;
use hyperfluid_p2p::tcp;
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
    multi_validator: bool,
    peer_addrs: Vec<(SocketAddr, Hash32)>,
}

impl NodeConfig {
    fn from_args() -> Self {
        let args: Vec<String> = std::env::args().collect();

        let mut gen_genesis = false;
        let mut genesis_path: Option<PathBuf> = None;
        let mut block_interval_secs: u64 = 2;
        let mut p2p_bind_str: Option<String> = None;
        let mut node_key_path = PathBuf::from("node_key");
        let mut multi_validator = false;
        let mut peer_addrs: Vec<(SocketAddr, Hash32)> = Vec::new();

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
                "--multi-validator" => {
                    multi_validator = true;
                }
                "--peers" => {
                    i += 1;
                    if i < args.len() {
                        peer_addrs = args[i]
                            .split(',')
                            .filter_map(|entry| {
                                let entry = entry.trim();
                                if let Some((addr_str, peer_id_str)) = entry.split_once('=') {
                                    let addr: SocketAddr = addr_str.trim().parse().ok()?;
                                    let peer_id_bytes = hex::decode(peer_id_str.trim()).ok()?;
                                    if peer_id_bytes.len() == 32 {
                                        let mut peer_id = [0u8; 32];
                                        peer_id.copy_from_slice(&peer_id_bytes);
                                        Some((addr, peer_id))
                                    } else {
                                        None
                                    }
                                } else {
                                    // Peer ID required for P2P connections.
                                    // Bare addresses are not supported.
                                    tracing::warn!(
                                        "Skipping peer entry with no peer_id: {}",
                                        entry,
                                    );
                                    None
                                }
                            })
                            .collect();
                    }
                }
                unknown => {
                    eprintln!("Warning: unknown flag '{}' — ignored", unknown);
                }
            }
            i += 1;
        }

        let p2p_bind: SocketAddr = p2p_bind_str
            .or_else(|| std::env::var("HYPERFLUID_P2P_BIND").ok())
            .unwrap_or_else(|| "0.0.0.0:0".to_string())
            .parse()
            .expect("invalid P2P bind address (expected IP:port format, e.g. 0.0.0.0:9876)");

        let genesis = if let Some(ref path) = genesis_path {
            let content = std::fs::read_to_string(path)
                .unwrap_or_else(|e| panic!("Failed to read genesis file {:?}: {}", path, e));
            serde_json::from_str(&content)
                .unwrap_or_else(|e| panic!("Invalid genesis file {:?}: {}", path, e))
        } else {
            GenesisConfig::new_testnet_single_validator()
        };

        Self {
            gen_genesis,
            genesis_path,
            genesis,
            block_interval_secs,
            p2p_bind,
            node_key_path,
            multi_validator,
            peer_addrs,
        }
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
    let mut driver = ConsensusDriver::new(
        config.genesis.epoch_length,
        [0u8; 32], // node_id: set after identity load
        [0u8; 32], // git_head_commit: genesis
    );
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

    let listener = tokio::net::TcpListener::bind(p2p_bind_addr)
        .await
        .expect("failed to bind P2P TCP listener");
    let actual_addr = listener.local_addr().expect("failed to get P2P listener address");
    tracing::info!(
        "P2P TCP listener started on {} (peer_id: {})",
        actual_addr,
        hex::encode(local_peer_id),
    );

    // Build key provider from genesis validator set.
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
                let kem_key = if pubkey.len() > 32 { pubkey[32..].to_vec() } else { Vec::new() };
                key_map.insert(v.validator_id, (dh_key, kem_key));
            }
        }
    }
    let key_provider = Arc::new(move |peer_id: &Hash32| -> Option<([u8; 32], Vec<u8>)> {
        key_map.get(peer_id).cloned()
    });

    if config.multi_validator {
        // ── Multi-Validator BFT Mode ──
        tracing::info!("Node entering multi-validator BFT consensus mode");

        let bft_config = malachite_consensus::ConsensusNetworkConfig::default();
        let channels = malachite_consensus::ConsensusChannels::default();
        let node_addr = Address32::new(*local_identity.peer_id());

        let mut entries = Vec::new();
        for v in &config.genesis.validators {
            let voting_power: u64 =
                if v.bonded_stake > u64::MAX as u128 { u64::MAX } else { v.bonded_stake as u64 };
            match key_provider(&v.validator_id) {
                Some((dh, _kem)) => {
                    entries.push((v.validator_id, dh.to_vec(), voting_power));
                }
                None => {
                    tracing::warn!(
                        "No key found for validator {} — skipping",
                        hex::encode(v.validator_id),
                    );
                }
            }
        }
        let validator_set = malachite_consensus::build_validator_set(entries);

        let proposer_seed = {
            use sha3::Digest;
            let mut h = sha3::Sha3_256::new();
            h.update(local_peer_id);
            h.update(config.genesis.timestamp.to_le_bytes());
            let mut out = [0u8; 32];
            out.copy_from_slice(&h.finalize());
            out
        };

        // Build external network bridge with empty peer list (peers added dynamically).
        let bridge = Arc::new(std::sync::Mutex::new(network_bridge::NetworkBridge {
            outgoing: channels.outgoing_tx.clone(),
            peers: Vec::new(),
        }));

        // Consensus handler: TCP inbound messages → BFT incoming channel.
        let incoming_tx = channels.incoming_tx.clone();
        let consensus_handler: tcp::ConsensusMessageHandler =
            Arc::new(move |_peer_id: Hash32, data: Vec<u8>| {
                if data.is_empty() {
                    return;
                }
                let msg = match data[0] {
                    0x01 => match network_bridge::decode_vote(&data[1..]) {
                        Some(vote) => malachite_consensus::ConsensusNetworkMsg::Vote(vote),
                        None => {
                            tracing::warn!("BFT: failed to decode vote from peer");
                            return;
                        }
                    },
                    0x02 => match network_bridge::decode_proposal(&data[1..]) {
                        Some(proposal) => {
                            malachite_consensus::ConsensusNetworkMsg::Proposal(proposal)
                        }
                        None => {
                            tracing::warn!("BFT: failed to decode proposal from peer");
                            return;
                        }
                    },
                    tag => {
                        tracing::warn!("BFT: unknown consensus msg tag 0x{:02x}", tag);
                        return;
                    }
                };
                let _ = incoming_tx.send(msg);
            });

        // Start TCP accept loop with consensus handler.
        {
            let transport = Arc::clone(&tcp_transport);
            let identity = Arc::clone(&local_identity);
            let r_p2p = running.clone();
            let handler = consensus_handler.clone();
            let kp_accept = key_provider.clone();
            tokio::spawn(async move {
                TcpTransport::accept_loop(
                    listener,
                    identity,
                    kp_accept,
                    transport,
                    Some(handler),
                    None, // peer_registry — inbound senders deferred (outbound via connect_and_maintain)
                )
                .await;
                r_p2p.store(false, Ordering::Release);
            });
        }

        // Start BFT consensus loop.
        let bft_handle = ConsensusDriver::run_bft_loop(
            driver.clone(),
            running.clone(),
            bft_config,
            channels,
            local_identity.clone(),
            node_addr,
            validator_set,
            proposer_seed,
            None,                 // peer_tx_rx_pairs — not used with external bridge
            Some(bridge.clone()), // external bridge
        );

        // Connect to configured peers.
        let bridge_for_peers = Arc::clone(&bridge);
        let handler_for_peers = consensus_handler.clone();
        let identity_for_peers = Arc::clone(&local_identity);
        let my_bind_addr = actual_addr;
        let key_provider_for_peers = key_provider.clone();
        tokio::spawn(async move {
            for (peer_addr, peer_id) in config.peer_addrs.iter().copied() {
                if peer_addr == my_bind_addr {
                    continue;
                }
                let (remote_dh, remote_kem) = match key_provider_for_peers(&peer_id) {
                    Some(keys) => keys,
                    None => {
                        tracing::warn!(
                            "No key material for peer_id {} — skipping peer {}",
                            hex::encode(peer_id),
                            peer_addr,
                        );
                        continue;
                    }
                };
                tracing::info!(
                    "Connecting to peer: {} (peer_id: {})",
                    peer_addr,
                    hex::encode(peer_id),
                );

                match tcp::connect_and_maintain(
                    peer_addr,
                    Arc::clone(&identity_for_peers),
                    peer_id,
                    remote_dh,
                    remote_kem,
                    Arc::clone(&handler_for_peers),
                )
                .await
                {
                    Ok((_peer_id, sender)) => {
                        tracing::info!(
                            "Connected to peer {} (peer_id: {})",
                            peer_addr,
                            hex::encode(_peer_id),
                        );
                        if let Ok(mut b) = bridge_for_peers.lock() {
                            b.peers.push(sender);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Failed to connect to peer {}: {}", peer_addr, e);
                    }
                }
            }
        });

        // Wait for the BFT loop to complete.
        match bft_handle.await {
            Ok(()) => tracing::info!("BFT loop exited cleanly"),
            Err(e) => tracing::error!("BFT loop panicked: {}", e),
        }
    } else {
        // ── Single-Validator Block Production Mode (default) ──
        {
            let transport = Arc::clone(&tcp_transport);
            let identity = Arc::clone(&local_identity);
            let r_p2p = running.clone();
            tokio::spawn(async move {
                TcpTransport::accept_loop(listener, identity, key_provider, transport, None, None)
                    .await;
                r_p2p.store(false, Ordering::Release);
            });
        }

        tracing::info!("Node entering consensus loop (live block production)");

        let loop_handle =
            ConsensusDriver::run_block_loop(driver.clone(), running.clone(), block_interval);

        match loop_handle.await {
            Ok(()) => tracing::info!("Block loop exited cleanly"),
            Err(e) => tracing::error!("Block loop panicked or was cancelled: {}", e),
        }
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
        let mut driver = ConsensusDriver::new(genesis.epoch_length, [0u8; 32], [0u8; 32]);
        let block = driver.init_genesis(&genesis);
        assert_eq!(block.header.height, 0);
        assert_eq!(block.header.parent_hash, [0u8; 32]);
        assert!(block.transactions.is_empty());
    }

    #[test]
    fn genesis_block_epoch_zero() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let mut driver = ConsensusDriver::new(genesis.epoch_length, [0u8; 32], [0u8; 32]);
        let block = driver.init_genesis(&genesis);
        assert_eq!(block.header.epoch, 0);
    }

    #[test]
    fn genesis_block_timestamp_matches_config() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let mut driver = ConsensusDriver::new(genesis.epoch_length, [0u8; 32], [0u8; 32]);
        let block = driver.init_genesis(&genesis);
        assert_eq!(block.header.timestamp, genesis.timestamp);
    }

    #[test]
    fn node_produces_real_blocks() {
        let genesis = GenesisConfig::new_testnet_single_validator();
        let mut driver = ConsensusDriver::new(genesis.epoch_length, [0u8; 32], [0u8; 32]);
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
