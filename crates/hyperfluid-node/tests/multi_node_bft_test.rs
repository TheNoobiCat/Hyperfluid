// === Multi-Node BFT Integration Tests ===
//
// Spin up 3 validators on localhost, connect via TCP with Clatter
// handshake, verify BFT consensus produces blocks across nodes.
//
// Source: stage-01-protocol-core.md exit criteria:
//   "Multi-node network: 3+ validators reach consensus, gossip
//    transactions, finalise blocks."

use hyperfluid_consensus::driver::ConsensusDriver;
use hyperfluid_consensus::genesis::{GenesisAccount, GenesisConfig, GenesisValidator};
use hyperfluid_consensus::malachite::Address32;
use hyperfluid_consensus::malachite_consensus::{self, ConsensusChannels, ConsensusNetworkConfig};
use hyperfluid_consensus::network_bridge::{self, NetworkBridge};
use hyperfluid_p2p::identity::Identity;
use hyperfluid_p2p::tcp::{self, ConsensusMessageHandler, TcpTransport};
use hyperfluid_p2p::types::{DiscoveryConfig, Hash32};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clatter::bytearray::ByteArray;
use clatter::crypto::dh::X25519;
use clatter::crypto::kem::rust_crypto_ml_kem::MlKem768;
use clatter::traits::{Dh, Kem};

struct NodeKeys {
    identity: Identity,
    peer_id: Hash32,
    dh_pubkey: [u8; 32],
    kem_pubkey: Vec<u8>,
}

/// Generate a test genesis config with N validators.
fn make_multi_validator_genesis(validator_count: usize) -> (GenesisConfig, Vec<NodeKeys>) {
    let mut keys = Vec::new();
    let mut accounts = Vec::new();
    let mut validators = Vec::new();

    for _ in 0..validator_count {
        let identity = Identity::generate();
        let peer_id = *identity.peer_id();

        let dh = X25519::genkey().expect("DH keygen");
        let kem = MlKem768::genkey().expect("KEM keygen");
        let dh_pubkey: [u8; 32] = dh.public;
        let kem_pubkey = kem.public.as_slice().to_vec();

        let mut pk_bytes = Vec::with_capacity(32 + kem_pubkey.len());
        pk_bytes.extend_from_slice(&dh_pubkey);
        pk_bytes.extend_from_slice(&kem_pubkey);

        keys.push(NodeKeys { identity, peer_id, dh_pubkey, kem_pubkey });

        accounts.push(GenesisAccount {
            account_id: peer_id,
            balance: 10_000_000_000_000_000_000u128,
            pubkey: Some(pk_bytes),
        });
        validators.push(GenesisValidator { validator_id: peer_id, bonded_stake: 1000 });
    }

    let genesis = GenesisConfig {
        chain_id: "test-multi-node".into(),
        timestamp: 0,
        epoch_length: 100,
        committee_size: 100,
        min_stake: 100,
        bond_delay: 100,
        unbond_delay: 1000,
        max_governance_proposals: 32,
        proposal_deposit: 500_000_000_000_000_000_000u128,
        liveness_window_blocks: 100,
        liveness_miss_threshold_pct: 20,
        total_agx_supply: 10_000_000_000_000_000_000_000_000u128,
        airdrop_amount_per_agent: 100_000_000_000_000_000_000u128,
        accounts,
        validators,
    };

    (genesis, keys)
}

async fn bind_ephemeral() -> (tokio::net::TcpListener, SocketAddr) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind ephemeral");
    let addr = listener.local_addr().expect("local addr");
    (listener, addr)
}

/// Spawn a single validator node. Returns final height after shutdown.
async fn spawn_validator_node(
    identity_seed: [u8; 32],
    peer_id: Hash32,
    genesis: GenesisConfig,
    bind_addr: SocketAddr,
    peer_infos: Vec<(SocketAddr, Hash32)>,
) -> u64 {
    let identity = Arc::new(Identity::from_seed(&identity_seed));
    let running = Arc::new(AtomicBool::new(true));
    let p2p_config = DiscoveryConfig::default();
    let peer_cache = Arc::new(tokio::sync::RwLock::new(hyperfluid_p2p::transport::PeerCache::new()));
    let transport = Arc::new(TcpTransport::new(p2p_config, Arc::clone(&peer_cache)));

    let listener = tokio::net::TcpListener::bind(bind_addr).await.expect("bind");

    let mut driver = ConsensusDriver::new(genesis.epoch_length);
    let _ = driver.init_genesis(&genesis);
    let driver = Arc::new(Mutex::new(driver));

    // Build key_provider from genesis accounts.
    let mut key_map: std::collections::HashMap<Hash32, ([u8; 32], Vec<u8>)> = std::collections::HashMap::new();
    for v in &genesis.validators {
        if let Some(acct) = genesis.accounts.iter().find(|a| a.account_id == v.validator_id) {
            if let Some(ref pk) = acct.pubkey {
                let mut dh_key = [0u8; 32];
                let copy_len = pk.len().min(32);
                dh_key[..copy_len].copy_from_slice(&pk[..copy_len]);
                let kem_key = if pk.len() > 32 { pk[32..].to_vec() } else { Vec::new() };
                key_map.insert(v.validator_id, (dh_key, kem_key));
            }
        }
    }
    let key_provider = Arc::new(move |pid: &Hash32| -> Option<([u8; 32], Vec<u8>)> { key_map.get(pid).cloned() });

    // BFT setup
    let bft_config = ConsensusNetworkConfig::default();
    let channels = ConsensusChannels::default();
    let node_addr = Address32::new(peer_id);

    let mut entries = Vec::new();
    for v in &genesis.validators {
        let voting_power = if v.bonded_stake > u64::MAX as u128 { u64::MAX } else { v.bonded_stake as u64 };
        let pk = key_provider(&v.validator_id).map(|(dh, _)| dh.to_vec()).unwrap_or_default();
        entries.push((v.validator_id, pk, voting_power));
    }
    let validator_set = malachite_consensus::build_validator_set(entries);

    let proposer_seed = {
        use sha3::Digest;
        let mut h = sha3::Sha3_256::new();
        h.update(&peer_id);
        h.update(&genesis.timestamp.to_le_bytes());
        let mut out = [0u8; 32];
        out.copy_from_slice(&h.finalize());
        out
    };

    let bridge = Arc::new(Mutex::new(NetworkBridge { outgoing: channels.outgoing_tx.clone(), peers: Vec::new() }));

    let incoming_tx = channels.incoming_tx.clone();
    let consensus_handler: ConsensusMessageHandler = Arc::new(move |_: Hash32, data: Vec<u8>| {
        if data.is_empty() { return; }
        let msg = match data[0] {
            0x01 => match network_bridge::decode_vote(&data[1..]) {
                Some(vote) => malachite_consensus::ConsensusNetworkMsg::Vote(vote),
                None => return,
            },
            0x02 => match network_bridge::decode_proposal(&data[1..]) {
                Some(p) => malachite_consensus::ConsensusNetworkMsg::Proposal(p),
                None => return,
            },
            _ => return,
        };
        let _ = incoming_tx.send(msg);
    });

    // Accept loop
    let r_p2p = running.clone();
    let kp_a = key_provider.clone();
    let h_a = consensus_handler.clone();
    let t_a = Arc::clone(&transport);
    let id_a = Arc::clone(&identity);
    tokio::spawn(async move {
        TcpTransport::accept_loop(listener, id_a, kp_a, t_a, Some(h_a)).await;
        r_p2p.store(false, Ordering::Release);
    });

    // BFT loop
    let bft_handle = ConsensusDriver::run_bft_loop(
        driver.clone(), running.clone(), bft_config, channels,
        identity.clone(), node_addr, validator_set, proposer_seed, None, Some(bridge.clone()),
    );

    // Give peers a moment to start their accept loops before connecting.
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Connect to peers
    let b_conn = Arc::clone(&bridge);
    let h_conn = consensus_handler.clone();
    let id_conn = Arc::clone(&identity);
    let kp_conn = key_provider.clone();
    tokio::spawn(async move {
        for (peer_addr, remote_peer_id) in &peer_infos {
            let (remote_dh, remote_kem) = match kp_conn(remote_peer_id) {
                Some(k) => k,
                None => continue,
            };
            match tcp::connect_and_maintain(
                *peer_addr, Arc::clone(&id_conn), *remote_peer_id, remote_dh, remote_kem, Arc::clone(&h_conn),
            ).await {
                Ok((_, sender)) => {
                    if let Ok(mut b) = b_conn.lock() {
                        b.peers.push(sender);
                    }
                }
                Err(e) => {
                    eprintln!("[test] connect to {} failed: {}", peer_addr, e);
                }
            }
        }
    });

    tokio::time::sleep(Duration::from_secs(12)).await;
    running.store(false, Ordering::SeqCst);
    let _ = bft_handle.await;
    let height = driver.lock().unwrap().height;
    height
}

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
async fn conforms_to_stage01_multi_node_three_validators_reach_consensus() {
    let _ = tracing_subscriber::fmt::try_init();
    let (genesis, node_keys) = make_multi_validator_genesis(3);

    let (l0, a0) = bind_ephemeral().await;
    let (l1, a1) = bind_ephemeral().await;
    let (l2, a2) = bind_ephemeral().await;

    let peers0 = vec![(a1, node_keys[1].peer_id), (a2, node_keys[2].peer_id)];
    let peers1 = vec![(a0, node_keys[0].peer_id), (a2, node_keys[2].peer_id)];
    let peers2 = vec![(a0, node_keys[0].peer_id), (a1, node_keys[1].peer_id)];

    let h0 = spawn_validator_node(node_keys[0].identity.to_seed(), node_keys[0].peer_id, genesis.clone(), a0, peers0);
    let h1 = spawn_validator_node(node_keys[1].identity.to_seed(), node_keys[1].peer_id, genesis.clone(), a1, peers1);
    let h2 = spawn_validator_node(node_keys[2].identity.to_seed(), node_keys[2].peer_id, genesis.clone(), a2, peers2);

    drop(l0); drop(l1); drop(l2);

    let (h0, h1, h2) = tokio::join!(h0, h1, h2);

    assert!(h0 >= 1, "node 0 height {} must be >= 1", h0);
    assert!(h1 >= 1, "node 1 height {} must be >= 1", h1);
    assert!(h2 >= 1, "node 2 height {} must be >= 1", h2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 6)]
async fn conforms_to_stage01_multi_node_two_validators_reach_consensus() {
    let (genesis, node_keys) = make_multi_validator_genesis(2);

    let (l0, a0) = bind_ephemeral().await;
    let (l1, a1) = bind_ephemeral().await;

    let peers0 = vec![(a1, node_keys[1].peer_id)];
    let peers1 = vec![(a0, node_keys[0].peer_id)];

    let h0 = spawn_validator_node(node_keys[0].identity.to_seed(), node_keys[0].peer_id, genesis.clone(), a0, peers0);
    let h1 = spawn_validator_node(node_keys[1].identity.to_seed(), node_keys[1].peer_id, genesis.clone(), a1, peers1);

    drop(l0); drop(l1);

    let (h0, h1) = tokio::join!(h0, h1);

    assert!(h0 >= 1, "node 0 height {} must be >= 1", h0);
    assert!(h1 >= 1, "node 1 height {} must be >= 1", h1);
}
