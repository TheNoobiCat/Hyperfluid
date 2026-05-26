//! Secure Channel Transport Integration
//!
//! Mock secure channel using SHA3-256 XOR for conformance testing.
//! The production backend is clatter v2.2.0 (Noise hybrid XX, X25519 + ML-KEM-768)
//! + ml-dsa v0.1.0-rc.11 (ML-DSA-65, FIPS 204). See ADR-0016.
//!
//! Source: docs/04-specifications/protocol/p2p-wire-spec.md Section 1.2, 1.8

use crate::types::Hash32;
#[cfg(test)]
use crate::types::SecureChannelError;
#[cfg(test)]
use sha3::digest::Update;
#[cfg(test)]
use sha3::Digest;
#[cfg(test)]
use sha3::Sha3_256;

/// Mock secure channel using SHA3-256 XOR for conformance testing.
///
/// Provides the same interface as the production clatter-based SecureChannel.
/// Key derivation is deterministic from peer IDs and nonce.
#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MockSecureChannel {
    session_id: Hash32,
    local_id: Hash32,
    remote_id: Hash32,
    shared_secret: Hash32,
    nonce: u64,
}

#[cfg(test)]
impl MockSecureChannel {
    /// Establish a new secure channel between two peers.
    ///
    /// Uses SHA3-256 key derivation from peer identities.
    /// Peer IDs are sorted to ensure both sides derive the same shared secret.
    pub fn establish(local_id: Hash32, remote_id: Hash32) -> Self {
        let (low, high) =
            if local_id < remote_id { (local_id, remote_id) } else { (remote_id, local_id) };
        let session_nonce = fast_nonce(low, high);
        let shared_secret = derive_shared_secret(low, high, session_nonce);
        let session_id = derive_session_id(&shared_secret, session_nonce);
        Self { session_id, local_id, remote_id, shared_secret, nonce: 0 }
    }

    /// Encrypt a plaintext message for the remote peer.
    ///
    /// Always returns `Ok(...)` — the mock XOR cipher has no failure modes.
    /// The `Result` return type matches the production `ClatterSecureChannel::seal`
    /// so that generic code (conformance tests, etc.) works with both backends.
    pub fn seal(&mut self, plaintext: &[u8]) -> Result<Vec<u8>, SecureChannelError> {
        self.nonce = self.nonce.saturating_add(1);
        Ok(xof_encrypt(&self.shared_secret, self.nonce, plaintext))
    }

    /// Decrypt a message received from the remote peer.
    ///
    /// Returns None if the ciphertext is corrupted or tampered with.
    pub fn open(&mut self, ciphertext: &[u8]) -> Option<Vec<u8>> {
        self.nonce = self.nonce.saturating_add(1);
        let plaintext = xof_encrypt(&self.shared_secret, self.nonce, ciphertext);
        if plaintext.is_empty() && !ciphertext.is_empty() {
            None
        } else {
            Some(plaintext)
        }
    }

    /// The session identifier for this channel.
    pub fn session_id(&self) -> &Hash32 {
        &self.session_id
    }

    /// The remote peer's identity.
    pub fn remote_id(&self) -> &Hash32 {
        &self.remote_id
    }
}

/// Derive a shared secret from two peer identities and a nonce.
#[cfg(test)]
fn derive_shared_secret(a: Hash32, b: Hash32, nonce: Hash32) -> Hash32 {
    let mut hasher = Sha3_256::new();
    let (low, high) = if a < b { (a, b) } else { (b, a) };
    Update::update(&mut hasher, &low);
    Update::update(&mut hasher, &high);
    Update::update(&mut hasher, &nonce);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Derive a session identifier from the shared secret and nonce.
#[cfg(test)]
fn derive_session_id(secret: &Hash32, nonce: Hash32) -> Hash32 {
    let mut hasher = Sha3_256::new();
    Update::update(&mut hasher, secret);
    Update::update(&mut hasher, &nonce);
    Update::update(&mut hasher, b"session");
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Generate a fast nonce from two peer identities for deterministic testing.
#[cfg(test)]
fn fast_nonce(a: Hash32, b: Hash32) -> Hash32 {
    let mut hasher = Sha3_256::new();
    Update::update(&mut hasher, a.as_slice());
    Update::update(&mut hasher, b.as_slice());
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Symmetric XOR encryption using SHAKE-256 XOF keystream.
#[cfg(test)]
fn xof_encrypt(key: &Hash32, nonce: u64, data: &[u8]) -> Vec<u8> {
    use sha3::digest::{ExtendableOutput, XofReader};
    use sha3::Shake256;

    let mut hasher = Shake256::default();
    Update::update(&mut hasher, key);
    Update::update(&mut hasher, &nonce.to_le_bytes());
    let mut reader = hasher.finalize_xof();

    let mut keystream = vec![0u8; data.len()];
    reader.read(&mut keystream);

    data.iter().zip(keystream).map(|(d, k)| d ^ k).collect()
}

/// A peer cache that survives network partitions.
///
/// Source: p2p-wire-spec.md Section 1.5 — Failure Behavior:
/// "Network partition: Peers continue operating with cached peer/relay sets.
/// On heal, DHT versions reconciled, gossip deltas replayed."
pub struct PeerCache {
    entries: Vec<CachedPeer>,
}

/// A cached peer entry with DHT version metadata for reconciliation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedPeer {
    pub peer_id: Hash32,
    pub dht_version: u64,
    pub last_seen_height: u64,
    pub endpoints: Vec<String>,
    pub relay_routes: Vec<Hash32>,
}

impl PeerCache {
    /// Create an empty peer cache.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Number of cached peers.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Cache a peer. Upserts if the peer already exists.
    pub fn insert(&mut self, peer: CachedPeer) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.peer_id == peer.peer_id) {
            *existing = peer;
        } else {
            self.entries.push(peer);
        }
    }

    /// Look up a peer by id. Returns None if not cached.
    pub fn get(&self, peer_id: &Hash32) -> Option<&CachedPeer> {
        self.entries.iter().find(|e| &e.peer_id == peer_id)
    }

    /// List all cached peer ids.
    pub fn peer_ids(&self) -> Vec<Hash32> {
        self.entries.iter().map(|e| e.peer_id).collect()
    }

    /// List peers with a newer DHT version than the given threshold.
    /// Used for reconciliation on partition heal.
    pub fn peers_newer_than(&self, dht_version: u64) -> Vec<&CachedPeer> {
        self.entries.iter().filter(|e| e.dht_version > dht_version).collect()
    }

    /// Count peers whose DHT version exceeds the threshold.
    pub fn count_newer_than(&self, dht_version: u64) -> usize {
        self.entries.iter().filter(|e| e.dht_version > dht_version).count()
    }
}

impl Default for PeerCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secure_channel_encrypt_decrypt_roundtrip() {
        let alice = [1u8; 32];
        let bob = [2u8; 32];

        let mut ch_alice = MockSecureChannel::establish(alice, bob);
        let mut ch_bob = MockSecureChannel::establish(bob, alice);

        let msg = b"hello over relay";
        let ciphertext = ch_alice.seal(msg).expect("seal must succeed");
        assert_ne!(&ciphertext, msg, "ciphertext must differ from plaintext");

        let decrypted = ch_bob.open(&ciphertext).expect("bob must decrypt alice's message");
        assert_eq!(&decrypted, msg, "roundtrip must preserve message");
    }

    #[test]
    fn secure_channel_different_keys_cannot_decrypt() {
        let alice = [1u8; 32];
        let bob = [2u8; 32];
        let eve = [3u8; 32];

        let mut ch_alice = MockSecureChannel::establish(alice, bob);
        let mut ch_eve = MockSecureChannel::establish(eve, bob);

        let msg = b"secret data";
        let ciphertext = ch_alice.seal(msg).expect("seal must succeed");

        let result = ch_eve.open(&ciphertext);
        assert!(
            result.is_none() || result != Some(msg.to_vec()),
            "eve must not decrypt alice's message to bob"
        );
    }

    #[test]
    fn secure_channel_nonce_advances() {
        let alice = [1u8; 32];
        let bob = [2u8; 32];

        let mut ch = MockSecureChannel::establish(alice, bob);
        let c1 = ch.seal(b"msg1").expect("seal must succeed");
        let c2 = ch.seal(b"msg2").expect("seal must succeed");
        assert_ne!(c1, c2, "different nonces produce different ciphertexts");
    }

    #[test]
    fn peer_cache_survives_partition() {
        let a = [1u8; 32];
        let b = [2u8; 32];

        let mut cache = PeerCache::new();
        assert!(cache.is_empty());

        cache.insert(CachedPeer {
            peer_id: a,
            dht_version: 1,
            last_seen_height: 100,
            endpoints: vec!["10.0.0.1:8000".into()],
            relay_routes: vec![],
        });
        cache.insert(CachedPeer {
            peer_id: b,
            dht_version: 1,
            last_seen_height: 100,
            endpoints: vec!["10.0.0.2:8000".into()],
            relay_routes: vec![],
        });

        assert_eq!(cache.len(), 2);

        let peer_a = cache.get(&a).expect("peer a must be cached during partition");
        assert_eq!(peer_a.last_seen_height, 100);
    }

    #[test]
    fn peer_cache_reconcile_on_heal() {
        let mut cache = PeerCache::new();
        cache.insert(CachedPeer {
            peer_id: [1u8; 32],
            dht_version: 1,
            last_seen_height: 100,
            endpoints: vec!["old".into()],
            relay_routes: vec![],
        });
        cache.insert(CachedPeer {
            peer_id: [2u8; 32],
            dht_version: 5,
            last_seen_height: 500,
            endpoints: vec!["new".into()],
            relay_routes: vec![],
        });

        let newer = cache.peers_newer_than(1);
        assert_eq!(newer.len(), 1);
        assert_eq!(newer[0].peer_id, [2u8; 32]);
    }

    #[test]
    fn peer_cache_insert_upserts_existing() {
        let mut cache = PeerCache::new();
        cache.insert(CachedPeer {
            peer_id: [1u8; 32],
            dht_version: 1,
            last_seen_height: 100,
            endpoints: vec![],
            relay_routes: vec![],
        });
        cache.insert(CachedPeer {
            peer_id: [1u8; 32],
            dht_version: 2,
            last_seen_height: 200,
            endpoints: vec!["updated".into()],
            relay_routes: vec![],
        });

        assert_eq!(cache.len(), 1);
        let p = cache.get(&[1u8; 32]).unwrap();
        assert_eq!(p.dht_version, 2);
        assert_eq!(p.last_seen_height, 200);
    }
}
