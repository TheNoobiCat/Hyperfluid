//! TCP Transport Layer - Real socket I/O for peer connections.
//! Wires connection state machine to TCP socket events.
//! Source: p2p-wire-spec.md Section 1.4

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, RwLock};

use crate::discovery::{transition_connection, ConnectionEvent};
use crate::transport::PeerCache;
use crate::types::{ConnState, ConnectionState, DiscoveryConfig, Hash32};

use crate::identity::Identity;
use crate::identity::{ML_DSA65_PUBKEY_LEN, ML_DSA65_SIG_LEN};
use crate::secure_channel::{ClatterHandshake, ClatterSecureChannel};
use clatter::bytearray::ByteArray;

const HANDSHAKE_BUF_SIZE: usize = 8192;
const FRAME_LEN_BYTES: usize = 4;
const MESSAGE_MAX_BYTES: usize = 1_048_576;
const ML_KEM768_PUBKEY_LEN: usize = 1184;

#[derive(Debug)]
pub enum TcpError {
    Io(std::io::Error),
    Handshake(String),
    Timeout,
    ConnectionRefused,
    ConnectionReset,
    InvalidFrame,
}

impl std::fmt::Display for TcpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TcpError::Io(e) => write!(f, "I/O error: {}", e),
            TcpError::Handshake(s) => write!(f, "Handshake error: {}", s),
            TcpError::Timeout => write!(f, "Connection timed out"),
            TcpError::ConnectionRefused => write!(f, "Connection refused"),
            TcpError::ConnectionReset => write!(f, "Connection reset by peer"),
            TcpError::InvalidFrame => write!(f, "Invalid frame received"),
        }
    }
}

impl std::error::Error for TcpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TcpError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TcpError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::TimedOut => TcpError::Timeout,
            std::io::ErrorKind::ConnectionRefused => TcpError::ConnectionRefused,
            std::io::ErrorKind::ConnectionReset => TcpError::ConnectionReset,
            _ => TcpError::Io(e),
        }
    }
}

/// TCP transport for peer connections.
pub struct TcpTransport {
    config: DiscoveryConfig,
    peer_cache: Arc<RwLock<PeerCache>>,
    connection_states: RwLock<BTreeMap<Hash32, ConnectionState>>,
    active_channels: RwLock<BTreeMap<Hash32, ClatterSecureChannel>>,
}

impl TcpTransport {
    pub fn new(config: DiscoveryConfig, peer_cache: Arc<RwLock<PeerCache>>) -> Self {
        Self {
            config,
            peer_cache,
            connection_states: RwLock::new(BTreeMap::new()),
            active_channels: RwLock::new(BTreeMap::new()),
        }
    }

    pub async fn connection_state(&self, peer_id: &Hash32) -> Option<ConnectionState> {
        self.connection_states.read().await.get(peer_id).cloned()
    }

    pub async fn update_state(&self, peer_id: Hash32, event: ConnectionEvent) -> ConnState {
        let mut states = self.connection_states.write().await;
        let current = states.entry(peer_id).or_insert_with(|| ConnectionState {
            peer_id,
            state: ConnState::Unknown,
            direct_endpoint: None,
            relay_path: None,
            last_probe_height: 0,
            consecutive_failures: 0,
        });

        let new_state = transition_connection(current, event, &self.config);
        let failures = match event {
            ConnectionEvent::DirectConnectTimeout
            | ConnectionEvent::DirectConnectRefused
            | ConnectionEvent::DirectConnectError => current.consecutive_failures + 1,
            ConnectionEvent::DirectConnectSuccess => 0,
            _ => current.consecutive_failures,
        };
        *current = ConnectionState {
            peer_id,
            state: new_state,
            consecutive_failures: failures,
            ..current.clone()
        };
        new_state
    }

    pub async fn start_listener(bind_addr: SocketAddr) -> Result<TcpListener, std::io::Error> {
        TcpListener::bind(bind_addr).await
    }

    pub async fn accept_loop<F>(
        listener: TcpListener,
        local_identity: Arc<Identity>,
        remote_key_provider: Arc<F>,
        transport: Arc<TcpTransport>,
        consensus_handler: Option<ConsensusMessageHandler>,
        peer_registry: Option<Arc<Mutex<Vec<mpsc::UnboundedSender<Vec<u8>>>>>>,
    ) where
        F: Fn(&Hash32) -> Option<([u8; 32], Vec<u8>)> + Send + Sync + 'static,
    {
        loop {
            match listener.accept().await {
                Ok((mut stream, peer_addr)) => {
                    let identity = Arc::clone(&local_identity);
                    let transport = Arc::clone(&transport);
                    let key_provider = Arc::clone(&remote_key_provider);
                    let handler = consensus_handler.clone();
                    let registry = peer_registry.clone();
                    tokio::spawn(async move {
                        let (peer_id, channel) = match handle_inbound(
                            &mut stream,
                            peer_addr,
                            identity,
                            &key_provider,
                            Arc::clone(&transport),
                        )
                        .await
                        {
                            Ok(r) => r,
                            Err(e) => {
                                tracing::warn!("P2P: inbound from {} failed: {}", peer_addr, e,);
                                return;
                            }
                        };

                        if let Some(h) = handler {
                            let (tx, rx) = mpsc::unbounded_channel();
                            let sender = tx.clone();
                            tokio::spawn(async move {
                                peer_message_loop(&mut stream, channel, rx, h, peer_id).await;
                                transport.disconnect(&peer_id).await;
                            });
                            if let Some(reg) = &registry {
                                if let Ok(mut senders) = reg.lock() {
                                    senders.push(sender);
                                }
                            }
                            tracing::info!(
                                "P2P: consensus messaging active for peer {:?}",
                                peer_id,
                            );
                            let _ = sender; // keep clone alive in memory while message loop runs
                        } else {
                            transport.active_channels.write().await.insert(peer_id, channel);
                            tracing::info!(
                                "P2P: inbound connection established from {} (peer_id: {:?})",
                                peer_addr,
                                peer_id,
                            );
                        }
                    });
                }
                Err(e) => {
                    tracing::warn!("P2P: listener accept error: {}", e);
                }
            }
        }
    }

    pub async fn connect_to_peer(
        transport: Arc<TcpTransport>,
        peer_addr: SocketAddr,
        local_identity: &Identity,
        remote_peer_id: Hash32,
        remote_dh_pubkey: [u8; 32],
        remote_kem_pubkey: Vec<u8>,
    ) -> Result<ClatterSecureChannel, TcpError> {
        transport.update_state(remote_peer_id, ConnectionEvent::ProbeInitiated).await;
        let mut stream = TcpStream::connect(peer_addr).await?;
        let handshake_result = perform_initiator_handshake(
            &mut stream,
            local_identity,
            remote_peer_id,
            remote_dh_pubkey,
            remote_kem_pubkey,
        )
        .await;

        match handshake_result {
            Ok(ch) => {
                transport.update_state(remote_peer_id, ConnectionEvent::DirectConnectSuccess).await;
                let mut channels = transport.active_channels.write().await;
                let _ = channels.remove(&remote_peer_id);
                Ok(ch)
            }
            Err(e) => {
                let event = match &e {
                    TcpError::Timeout => ConnectionEvent::DirectConnectTimeout,
                    TcpError::ConnectionRefused => ConnectionEvent::DirectConnectRefused,
                    TcpError::Io(io_error) => {
                        tracing::debug!("P2P connect I/O error: {}", io_error);
                        ConnectionEvent::DirectConnectError
                    }
                    TcpError::ConnectionReset => ConnectionEvent::DirectConnectError,
                    TcpError::InvalidFrame => ConnectionEvent::DirectConnectError,
                    TcpError::Handshake(msg) => {
                        tracing::debug!("P2P connect handshake error: {}", msg);
                        ConnectionEvent::DirectConnectError
                    }
                };
                let new_state = transport.update_state(remote_peer_id, event).await;
                eprintln!(
                    "[hyperfluid-p2p] Connect to {} failed (state {:?}): {}",
                    peer_addr, new_state, e
                );
                Err(e)
            }
        }
    }

    pub async fn run_connection_loop(
        transport: Arc<TcpTransport>,
        peer_addr: SocketAddr,
        local_identity: Arc<Identity>,
        remote_peer_id: Hash32,
        remote_dh_pubkey: [u8; 32],
        remote_kem_pubkey: Vec<u8>,
    ) -> ConnState {
        let max_retries = transport.config.direct_retry_attempts;
        let mut last_error: Option<TcpError> = None;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                eprintln!(
                    "[hyperfluid-p2p] Retry {}/{} connecting to {}",
                    attempt, max_retries, peer_addr
                );
                tokio::time::sleep(std::time::Duration::from_secs(
                    transport.config.direct_retry_timeout_secs,
                ))
                .await;
            }

            match TcpTransport::connect_to_peer(
                Arc::clone(&transport),
                peer_addr,
                &local_identity,
                remote_peer_id,
                remote_dh_pubkey,
                remote_kem_pubkey.clone(),
            )
            .await
            {
                Ok(channel) => {
                    transport.active_channels.write().await.insert(remote_peer_id, channel);
                    let state = transport.connection_state(&remote_peer_id).await;
                    let current = state.map(|s| s.state).unwrap_or(ConnState::Unknown);
                    eprintln!("[hyperfluid-p2p] Connected to {} (state: {:?})", peer_addr, current);
                    return current;
                }
                Err(e) => {
                    last_error = Some(e);
                    let state = transport.connection_state(&remote_peer_id).await;
                    if let Some(s) = state {
                        if s.state == ConnState::RelayActive {
                            return ConnState::RelayActive;
                        }
                    }
                }
            }
        }

        eprintln!(
            "[hyperfluid-p2p] All {} connection attempts to {} failed: {:?}",
            max_retries + 1,
            peer_addr,
            last_error
        );
        transport
            .connection_state(&remote_peer_id)
            .await
            .map(|s| s.state)
            .unwrap_or(ConnState::Unknown)
    }

    pub async fn disconnect(&self, peer_id: &Hash32) {
        self.active_channels.write().await.remove(peer_id);
        self.update_state(*peer_id, ConnectionEvent::ConnectionLost).await;
    }

    pub fn peer_cache(&self) -> &Arc<RwLock<PeerCache>> {
        &self.peer_cache
    }

    pub async fn has_active_channel(&self, peer_id: &Hash32) -> bool {
        self.active_channels.read().await.contains_key(peer_id)
    }
}

// --- Frame I/O ---

async fn write_frame(stream: &mut TcpStream, data: &[u8]) -> Result<(), TcpError> {
    if data.len() > u32::MAX as usize {
        return Err(TcpError::InvalidFrame);
    }
    let len_bytes = (data.len() as u32).to_be_bytes();
    stream.write_all(&len_bytes).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_frame(stream: &mut TcpStream) -> Result<Vec<u8>, TcpError> {
    let mut len_buf = [0u8; FRAME_LEN_BYTES];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len > HANDSHAKE_BUF_SIZE {
        return Err(TcpError::InvalidFrame);
    }
    let mut data = vec![0u8; len];
    stream.read_exact(&mut data).await?;
    Ok(data)
}

// --- Clatter handshake over TCP ---

type X25519PubKey = <clatter::crypto::dh::X25519 as clatter::traits::Dh>::PubKey;
type MlKem768PubKey =
    <clatter::crypto::kem::rust_crypto_ml_kem::MlKem768 as clatter::traits::Kem>::PubKey;

async fn perform_initiator_handshake(
    stream: &mut TcpStream,
    local_identity: &Identity,
    remote_id: Hash32,
    remote_dh_pubkey_bytes: [u8; 32],
    remote_kem_pubkey_bytes: Vec<u8>,
) -> Result<ClatterSecureChannel, TcpError> {
    if remote_kem_pubkey_bytes.len() != ML_KEM768_PUBKEY_LEN {
        return Err(TcpError::Handshake(format!(
            "invalid ML-KEM-768 pubkey length: expected {}, got {}",
            ML_KEM768_PUBKEY_LEN,
            remote_kem_pubkey_bytes.len()
        )));
    }
    let remote_dh: X25519PubKey = ByteArray::from_slice(&remote_dh_pubkey_bytes);
    let remote_kem: MlKem768PubKey = ByteArray::from_slice(&remote_kem_pubkey_bytes);

    let mut handshake =
        ClatterHandshake::initiator(local_identity, remote_id, remote_dh, remote_kem)
            .map_err(|e| TcpError::Handshake(format!("initiator setup: {:?}", e)))?;
    let mut buf = [0u8; HANDSHAKE_BUF_SIZE];
    let mut read_buf = [0u8; HANDSHAKE_BUF_SIZE];

    // Msg1: initiator -> responder (e + e_kem)
    let n = handshake
        .write_message(&mut buf)
        .map_err(|e| TcpError::Handshake(format!("msg1 write: {:?}", e)))?;
    write_frame(stream, &buf[..n]).await?;

    // Msg2: responder -> initiator (e + e_kem)
    let msg2 = read_frame(stream).await?;
    handshake
        .read_message(&msg2, &mut read_buf)
        .map_err(|e| TcpError::Handshake(format!("msg2 read: {:?}", e)))?;

    // Msg3: initiator -> responder (s + Skem)
    let n = handshake
        .write_message(&mut buf)
        .map_err(|e| TcpError::Handshake(format!("msg3 write: {:?}", e)))?;
    write_frame(stream, &buf[..n]).await?;

    // Msg4: responder -> initiator (s + Skem)
    let msg4 = read_frame(stream).await?;
    handshake
        .read_message(&msg4, &mut read_buf)
        .map_err(|e| TcpError::Handshake(format!("msg4 read: {:?}", e)))?;

    if !handshake.is_finished() {
        return Err(TcpError::Handshake("handshake not finished after 4 messages".into()));
    }

    let channel =
        handshake.finalize().map_err(|e| TcpError::Handshake(format!("finalize: {:?}", e)))?;

    // Identity binding: prove we own the key that derives our peer_id.
    // Sends verifying_key || signature(session_id) as a self-contained proof
    // of possession. The responder verifies peer_id = SHA3-256(verifying_key)
    // and checks the signature without needing external key lookup.
    let verifying_key = local_identity.verifying_key_encoded();
    let sig = local_identity.sign(channel.session_id());
    let mut id_frame = Vec::with_capacity(ML_DSA65_PUBKEY_LEN + ML_DSA65_SIG_LEN);
    id_frame.extend_from_slice(&verifying_key);
    id_frame.extend_from_slice(&sig);
    write_frame(stream, &id_frame).await?;

    Ok(channel)
}

/// Type alias for a consensus message handler callback.
/// Receives (peer_id, decrypted_bytes) — the raw wire-format consensus message.
pub type ConsensusMessageHandler = Arc<dyn Fn(Hash32, Vec<u8>) + Send + Sync + 'static>;

async fn handle_inbound<F>(
    stream: &mut TcpStream,
    peer_addr: SocketAddr,
    local_identity: Arc<Identity>,
    remote_key_provider: &Arc<F>,
    transport: Arc<TcpTransport>,
) -> Result<(Hash32, ClatterSecureChannel), TcpError>
where
    F: Fn(&Hash32) -> Option<([u8; 32], Vec<u8>)>,
{
    let preamble = read_frame(stream).await?;
    if preamble.len() != 32 {
        return Err(TcpError::Handshake("invalid preamble: expected 32-byte peer_id".into()));
    }
    let remote_peer_id: Hash32 = match preamble[..32].try_into() {
        Ok(id) => id,
        Err(_) => {
            return Err(TcpError::Handshake(
                "invalid preamble: could not convert to 32-byte peer_id".into(),
            ));
        }
    };

    let (remote_dh_pubkey, remote_kem_pubkey) = (remote_key_provider)(&remote_peer_id)
        .ok_or_else(|| TcpError::Handshake(format!("unknown peer: {:?}", remote_peer_id)))?;

    let channel = perform_responder_handshake_on_split(
        stream,
        &local_identity,
        remote_peer_id,
        remote_dh_pubkey,
        remote_kem_pubkey,
    )
    .await?;

    {
        let mut cache = transport.peer_cache.write().await;
        let entry = crate::transport::CachedPeer {
            peer_id: remote_peer_id,
            dht_version: 0,
            last_seen_height: 0,
            endpoints: vec![peer_addr.to_string()],
            relay_routes: vec![],
        };
        cache.insert(entry);
    }

    Ok((remote_peer_id, channel))
}

async fn perform_responder_handshake_on_split(
    stream: &mut TcpStream,
    local_identity: &Identity,
    remote_id: Hash32,
    remote_dh_pubkey_bytes: [u8; 32],
    remote_kem_pubkey_bytes: Vec<u8>,
) -> Result<ClatterSecureChannel, TcpError> {
    let (mut reader, mut writer) = stream.split();

    async fn read_frame_split<R: AsyncReadExt + Unpin>(
        reader: &mut R,
    ) -> Result<Vec<u8>, TcpError> {
        let mut len_buf = [0u8; FRAME_LEN_BYTES];
        reader.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len > HANDSHAKE_BUF_SIZE {
            return Err(TcpError::InvalidFrame);
        }
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data).await?;
        Ok(data)
    }

    async fn write_frame_split<W: AsyncWriteExt + Unpin>(
        writer: &mut W,
        data: &[u8],
    ) -> Result<(), TcpError> {
        if data.len() > u32::MAX as usize {
            return Err(TcpError::InvalidFrame);
        }
        let len_bytes = (data.len() as u32).to_be_bytes();
        writer.write_all(&len_bytes).await?;
        writer.write_all(data).await?;
        writer.flush().await?;
        Ok(())
    }

    if remote_kem_pubkey_bytes.len() != ML_KEM768_PUBKEY_LEN {
        return Err(TcpError::Handshake(format!(
            "invalid ML-KEM-768 pubkey length: expected {}, got {}",
            ML_KEM768_PUBKEY_LEN,
            remote_kem_pubkey_bytes.len()
        )));
    }
    let remote_dh: X25519PubKey = ByteArray::from_slice(&remote_dh_pubkey_bytes);
    let remote_kem: MlKem768PubKey = ByteArray::from_slice(&remote_kem_pubkey_bytes);

    let mut handshake =
        ClatterHandshake::responder(local_identity, remote_id, remote_dh, remote_kem)
            .map_err(|e| TcpError::Handshake(format!("responder setup: {:?}", e)))?;
    let mut buf = [0u8; HANDSHAKE_BUF_SIZE];
    let mut read_buf = [0u8; HANDSHAKE_BUF_SIZE];

    let msg1 = read_frame_split(&mut reader).await?;
    handshake
        .read_message(&msg1, &mut read_buf)
        .map_err(|e| TcpError::Handshake(format!("msg1 read: {:?}", e)))?;

    let n = handshake
        .write_message(&mut buf)
        .map_err(|e| TcpError::Handshake(format!("msg2 write: {:?}", e)))?;
    write_frame_split(&mut writer, &buf[..n]).await?;

    let msg3 = read_frame_split(&mut reader).await?;
    handshake
        .read_message(&msg3, &mut read_buf)
        .map_err(|e| TcpError::Handshake(format!("msg3 read: {:?}", e)))?;

    let n = handshake
        .write_message(&mut buf)
        .map_err(|e| TcpError::Handshake(format!("msg4 write: {:?}", e)))?;
    write_frame_split(&mut writer, &buf[..n]).await?;

    if !handshake.is_finished() {
        return Err(TcpError::Handshake("handshake not finished after 4 messages".into()));
    }

    let channel =
        handshake.finalize().map_err(|e| TcpError::Handshake(format!("finalize: {:?}", e)))?;

    // Identity binding: verify the initiator owns the claimed peer_id.
    let id_frame = read_frame_split(&mut reader).await?;
    if id_frame.len() < ML_DSA65_PUBKEY_LEN + ML_DSA65_SIG_LEN {
        return Err(TcpError::Handshake(format!(
            "invalid identity binding frame: expected >= {} bytes, got {}",
            ML_DSA65_PUBKEY_LEN + ML_DSA65_SIG_LEN,
            id_frame.len()
        )));
    }
    let vk_bytes = &id_frame[..ML_DSA65_PUBKEY_LEN];
    let sig_bytes = &id_frame[ML_DSA65_PUBKEY_LEN..];
    let computed_peer_id = crate::identity::compute_peer_id_from_bytes(vk_bytes);
    if computed_peer_id != remote_id {
        return Err(TcpError::Handshake(
            "identity binding: peer_id does not match claimed identity".into(),
        ));
    }
    if !Identity::verify_with_pubkey(vk_bytes, channel.session_id(), sig_bytes) {
        return Err(TcpError::Handshake("identity binding: signature verification failed".into()));
    }

    Ok(channel)
}

/// Persistent message loop for a single peer connection.
/// After the Clatter handshake completes, this loop handles:
/// - Reading encrypted frames from TCP, decrypting, forwarding to consensus handler
/// - Receiving plaintext from mpsc channel, encrypting, writing framed to TCP
///
/// Uses `tokio::select!` so read and write share the same channel (avoiding Mutex
/// on nonce state). Exits when either side closes.
async fn peer_message_loop(
    stream: &mut TcpStream,
    mut channel: ClatterSecureChannel,
    mut outgoing_rx: mpsc::UnboundedReceiver<Vec<u8>>,
    consensus_handler: ConsensusMessageHandler,
    peer_id: Hash32,
) {
    use tokio::io::AsyncReadExt;
    use tokio::io::AsyncWriteExt;

    let (mut reader, mut writer) = stream.split();

    loop {
        tokio::select! {
            // --- Inbound: read framed ciphertext, decrypt, forward ---
            frame_result = async {
                let mut len_buf = [0u8; FRAME_LEN_BYTES];
                reader.read_exact(&mut len_buf).await?;
                let len = u32::from_be_bytes(len_buf) as usize;
                if len > MESSAGE_MAX_BYTES {
                    return Err::<Vec<u8>, std::io::Error>(
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "frame too large"),
                    );
                }
                let mut data = vec![0u8; len];
                reader.read_exact(&mut data).await?;
                Ok(data)
            } => {
                match frame_result {
                    Ok(frame) => {
                        if let Some(plaintext) = channel.open(&frame) {
                            consensus_handler(peer_id, plaintext);
                        }
                    }
                    Err(_) => break,
                }
            }

            // --- Outbound: receive plaintext, encrypt, write framed ---
            plaintext = outgoing_rx.recv() => {
                let Some(plaintext) = plaintext else { break; };
                if let Ok(ciphertext) = channel.seal(&plaintext) {
                    let len_bytes = (ciphertext.len() as u32).to_be_bytes();
                    if writer.write_all(&len_bytes).await.is_err() { break; }
                    if writer.write_all(&ciphertext).await.is_err() { break; }
                }
            }
        }
    }
}

/// Outbound connect with persistent messaging.
/// Connects to `peer_addr`, performs the Clatter initiator handshake,
/// then enters a persistent message loop. Returns an mpsc sender for
/// sending consensus messages to this peer.
pub async fn connect_and_maintain(
    peer_addr: SocketAddr,
    local_identity: Arc<Identity>,
    remote_peer_id: Hash32,
    remote_dh_pubkey: [u8; 32],
    remote_kem_pubkey: Vec<u8>,
    consensus_handler: ConsensusMessageHandler,
) -> Result<(Hash32, mpsc::UnboundedSender<Vec<u8>>), TcpError> {
    let mut stream = TcpStream::connect(peer_addr).await?;

    // Send 32-byte preamble: our peer_id so the responder can look up
    // our DH/KEM keys in its key_provider for the Clatter handshake.
    let local_peer_id = *local_identity.peer_id();
    write_frame(&mut stream, &local_peer_id).await?;

    let channel = perform_initiator_handshake(
        &mut stream,
        &local_identity,
        remote_peer_id,
        remote_dh_pubkey,
        remote_kem_pubkey,
    )
    .await?;

    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        peer_message_loop(&mut stream, channel, rx, consensus_handler, remote_peer_id).await;
    });

    Ok((remote_peer_id, tx))
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::net::TcpListener;

    fn test_config() -> DiscoveryConfig {
        DiscoveryConfig {
            direct_retry_attempts: 3,
            direct_retry_timeout_secs: 1,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn write_and_read_frame_roundtrip() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let data = read_frame(&mut stream).await.unwrap();
            assert_eq!(data, b"hello tcp framing");
        });
        let mut stream = TcpStream::connect(addr).await.unwrap();
        write_frame(&mut stream, b"hello tcp framing").await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn write_and_read_empty_frame() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let data = read_frame(&mut stream).await.unwrap();
            assert!(data.is_empty());
        });
        let mut stream = TcpStream::connect(addr).await.unwrap();
        write_frame(&mut stream, b"").await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn multiple_frames_in_sequence() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let msg1 = read_frame(&mut stream).await.unwrap();
            let msg2 = read_frame(&mut stream).await.unwrap();
            let msg3 = read_frame(&mut stream).await.unwrap();
            assert_eq!(msg1, b"first");
            assert_eq!(msg2, b"second");
            assert_eq!(msg3, b"third");
        });
        let mut stream = TcpStream::connect(addr).await.unwrap();
        write_frame(&mut stream, b"first").await.unwrap();
        write_frame(&mut stream, b"second").await.unwrap();
        write_frame(&mut stream, b"third").await.unwrap();
        server.await.unwrap();
    }

    #[tokio::test]
    async fn state_machine_unknown_to_direct_active() {
        let config = test_config();
        let cache = Arc::new(RwLock::new(PeerCache::new()));
        let transport = TcpTransport::new(config, cache);
        let peer_id = [0xABu8; 32];

        let state = transport.connection_state(&peer_id).await;
        assert!(state.is_none());

        let s = transport.update_state(peer_id, ConnectionEvent::ProbeInitiated).await;
        assert_eq!(s, ConnState::DirectProbing);
        let state = transport.connection_state(&peer_id).await.unwrap();
        assert_eq!(state.state, ConnState::DirectProbing);

        let s = transport.update_state(peer_id, ConnectionEvent::DirectConnectSuccess).await;
        assert_eq!(s, ConnState::DirectActive);
        let state = transport.connection_state(&peer_id).await.unwrap();
        assert_eq!(state.state, ConnState::DirectActive);
    }

    #[tokio::test]
    async fn state_machine_timeout_retry_then_relay() {
        let config = test_config();
        let cache = Arc::new(RwLock::new(PeerCache::new()));
        let transport = TcpTransport::new(config, cache);
        let peer_id = [0xCDu8; 32];

        transport.update_state(peer_id, ConnectionEvent::ProbeInitiated).await;
        let s1 = transport.update_state(peer_id, ConnectionEvent::DirectConnectTimeout).await;
        assert_eq!(s1, ConnState::DirectProbing);
        let s2 = transport.update_state(peer_id, ConnectionEvent::DirectConnectTimeout).await;
        assert_eq!(s2, ConnState::DirectProbing);
        let s3 = transport.update_state(peer_id, ConnectionEvent::DirectConnectTimeout).await;
        assert_eq!(s3, ConnState::RelayActive);
    }

    #[tokio::test]
    async fn state_machine_connection_lost_returns_to_unknown() {
        let config = test_config();
        let cache = Arc::new(RwLock::new(PeerCache::new()));
        let transport = TcpTransport::new(config, cache);
        let peer_id = [0xEFu8; 32];

        transport.update_state(peer_id, ConnectionEvent::ProbeInitiated).await;
        transport.update_state(peer_id, ConnectionEvent::DirectConnectSuccess).await;
        let state = transport.connection_state(&peer_id).await.unwrap();
        assert_eq!(state.state, ConnState::DirectActive);

        let s = transport.update_state(peer_id, ConnectionEvent::ConnectionLost).await;
        assert_eq!(s, ConnState::Unknown);
    }

    #[tokio::test]
    async fn disconnect_cleans_up_channel_and_state() {
        let config = test_config();
        let cache = Arc::new(RwLock::new(PeerCache::new()));
        let transport = TcpTransport::new(config, cache);
        let peer_id = [0x11u8; 32];

        transport.update_state(peer_id, ConnectionEvent::ProbeInitiated).await;
        transport.update_state(peer_id, ConnectionEvent::DirectConnectSuccess).await;
        transport.disconnect(&peer_id).await;
        let state = transport.connection_state(&peer_id).await.unwrap();
        assert_eq!(state.state, ConnState::Unknown);
        assert!(!transport.has_active_channel(&peer_id).await);
    }
}

#[cfg(test)]
mod socket_integration {
    use super::*;
    use crate::identity::Identity;
    use clatter::bytearray::ByteArray;
    use clatter::crypto::dh::X25519;
    use clatter::crypto::kem::rust_crypto_ml_kem::MlKem768;
    use clatter::traits::{Dh, Kem};
    use std::sync::Arc;
    use tokio::net::TcpListener;
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn conforms_to_p2p_spec_1_7_actual_socket_roundtrip() {
        let alice_id = Arc::new(Identity::generate());
        let bob_id = Arc::new(Identity::generate());

        let alice_peer_id = *alice_id.peer_id();

        let bob_dh = X25519::genkey().expect("bob DH keygen");
        let bob_kem = MlKem768::genkey().expect("bob KEM keygen");
        let bob_dh_pub_bytes: [u8; 32] = bob_dh.public;
        let bob_kem_pub_bytes = bob_kem.public.as_slice().to_vec();

        let alice_dh = X25519::genkey().expect("alice DH keygen");
        let alice_kem = MlKem768::genkey().expect("alice KEM keygen");
        let alice_dh_pub_bytes: [u8; 32] = alice_dh.public;
        let alice_kem_pub_bytes = alice_kem.public.as_slice().to_vec();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bob_addr = listener.local_addr().unwrap();

        let bob_id_srv = Arc::clone(&bob_id);
        let alice_peer_id_srv = alice_peer_id;
        let alice_dh_srv = alice_dh_pub_bytes;
        let alice_kem_srv = alice_kem_pub_bytes.clone();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let preamble = read_frame(&mut stream).await.unwrap();
            assert_eq!(preamble.len(), 32);
            // Use perform_responder_handshake_on_split which takes &mut TcpStream
            // (canonical responder handshake implementation; the owned-TcpStream
            // variant was removed as dead code).
            perform_responder_handshake_on_split(
                &mut stream,
                &bob_id_srv,
                alice_peer_id_srv,
                alice_dh_srv,
                alice_kem_srv,
            )
            .await
            .unwrap()
        });

        let mut stream = TcpStream::connect(bob_addr).await.unwrap();
        write_frame(&mut stream, &alice_peer_id).await.unwrap();
        let bob_peer_id = *bob_id.peer_id();

        let channel = perform_initiator_handshake(
            &mut stream,
            &alice_id,
            bob_peer_id,
            bob_dh_pub_bytes,
            bob_kem_pub_bytes,
        )
        .await
        .unwrap();

        let server_channel = server.await.unwrap();

        assert!(!channel.session_id().is_empty());
        assert!(!server_channel.session_id().is_empty());

        let mut alice_ch = channel;
        let mut bob_ch = server_channel;

        let msg = b"actual TCP socket roundtrip successful";
        let ct = alice_ch.seal(msg).expect("seal must succeed");
        assert_ne!(&ct, msg);
        let pt = bob_ch.open(&ct).expect("bob must decrypt");
        assert_eq!(pt, msg);

        let reply = b"acknowledged, channel established";
        let reply_ct = bob_ch.seal(reply).expect("seal must succeed");
        let reply_pt = alice_ch.open(&reply_ct).expect("alice must decrypt");
        assert_eq!(reply_pt, reply);
    }

    #[tokio::test]
    async fn conforms_to_p2p_spec_1_7_actual_socket_lifecycle() {
        let alice_id = Arc::new(Identity::generate());
        let bob_id = Arc::new(Identity::generate());

        let alice_peer_id = *alice_id.peer_id();
        let bob_peer_id = *bob_id.peer_id();

        let bob_dh = X25519::genkey().expect("bob DH keygen");
        let bob_kem = MlKem768::genkey().expect("bob KEM keygen");
        let bob_dh_pub_bytes: [u8; 32] = bob_dh.public;
        let bob_kem_pub_bytes = bob_kem.public.as_slice().to_vec();

        let alice_dh = X25519::genkey().expect("alice DH keygen");
        let alice_kem = MlKem768::genkey().expect("alice KEM keygen");
        let alice_dh_pub_bytes: [u8; 32] = alice_dh.public;
        let alice_kem_pub_bytes = alice_kem.public.as_slice().to_vec();

        let config = DiscoveryConfig {
            direct_retry_attempts: 3,
            direct_retry_timeout_secs: 1,
            ..Default::default()
        };
        let cache = Arc::new(RwLock::new(PeerCache::new()));
        let transport = Arc::new(TcpTransport::new(config, cache));

        // Phase 1: Unknown
        let state = transport.connection_state(&bob_peer_id).await;
        assert!(state.is_none() || state.unwrap().state == ConnState::Unknown);

        // Phase 2: ProbeInitiated -> DirectProbing
        let s = transport.update_state(bob_peer_id, ConnectionEvent::ProbeInitiated).await;
        assert_eq!(s, ConnState::DirectProbing);
        let state = transport.connection_state(&bob_peer_id).await.unwrap();
        assert_eq!(state.state, ConnState::DirectProbing);

        // Phase 3: Establish TCP + handshake
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bob_addr = listener.local_addr().unwrap();

        let bob_id_srv = Arc::clone(&bob_id);
        let alice_peer_id_srv = alice_peer_id;
        let alice_dh_srv = alice_dh_pub_bytes;
        let alice_kem_srv = alice_kem_pub_bytes.clone();

        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let preamble = read_frame(&mut stream).await.unwrap();
            assert_eq!(preamble.len(), 32);
            perform_responder_handshake_on_split(
                &mut stream,
                &bob_id_srv,
                alice_peer_id_srv,
                alice_dh_srv,
                alice_kem_srv,
            )
            .await
            .unwrap()
        });

        let mut stream = TcpStream::connect(bob_addr).await.unwrap();
        write_frame(&mut stream, &alice_peer_id).await.unwrap();
        let channel = perform_initiator_handshake(
            &mut stream,
            &alice_id,
            bob_peer_id,
            bob_dh_pub_bytes,
            bob_kem_pub_bytes,
        )
        .await
        .unwrap();

        let _bob_ch = server.await.unwrap();

        // Phase 4: DirectConnectSuccess -> DirectActive
        let s = transport.update_state(bob_peer_id, ConnectionEvent::DirectConnectSuccess).await;
        assert_eq!(s, ConnState::DirectActive);
        let state = transport.connection_state(&bob_peer_id).await.unwrap();
        assert_eq!(state.state, ConnState::DirectActive);

        // Phase 5: Exchange encrypted message
        let mut alice_ch = channel;
        let msg = b"lifecycle test message over TCP";
        let ct = alice_ch.seal(msg).expect("seal must succeed");
        assert_ne!(&ct, msg);

        // Phase 6: ConnectionLost -> Unknown
        drop(alice_ch);
        let final_state =
            transport.update_state(bob_peer_id, ConnectionEvent::ConnectionLost).await;
        assert_eq!(final_state, ConnState::Unknown);
        let state = transport.connection_state(&bob_peer_id).await.unwrap();
        assert_eq!(state.state, ConnState::Unknown);
    }
}
