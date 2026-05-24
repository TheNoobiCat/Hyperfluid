//! ML-DSA-65 Identity Provider
//!
//! Generates, stores, and loads ML-DSA-65 keypairs for peer identity.
//! PeerId = SHA3-256(ML-DSA-65 public key bytes).
//!
//! Source: ADR-0016 clatter+ml-dsa secure channel stack
//! Spec: p2p-wire-spec.md Section 1.8 Trust-Assumption Inventory

use crate::types::Hash32;
use ml_dsa::{
    Generate, Keypair, MlDsa65, Seed, Signature, SignatureEncoding, Signer, SigningKey, Verifier,
    VerifyingKey,
};
use sha3::digest::Update;
use sha3::Digest;
use sha3::Sha3_256;

pub const ML_DSA65_PUBKEY_LEN: usize = 1952;
pub const ML_DSA65_SIG_LEN: usize = 3309;
pub const ML_DSA65_SECKEY_LEN: usize = 4032;

/// An ML-DSA-65 identity with signing key and derived PeerId.
pub struct Identity {
    signing_key: SigningKey<MlDsa65>,
    verifying_key: VerifyingKey<MlDsa65>,
    peer_id: Hash32,
}

impl Identity {
    /// Generate a new random ML-DSA-65 identity.
    pub fn generate() -> Self {
        let signing_key = SigningKey::<MlDsa65>::generate();
        let verifying_key = signing_key.verifying_key();
        let peer_id = compute_peer_id(&verifying_key);
        Self { signing_key, verifying_key, peer_id }
    }

    /// Reconstruct an identity from a 32-byte ML-DSA-65 seed.
    ///
    /// Used to load a persisted identity from a keystore. The seed is the
    /// raw private key material — it must be kept secret and never derived
    /// from public data like PeerIds.
    pub fn from_seed(seed_bytes: &[u8; 32]) -> Self {
        let seed = Seed::try_from(seed_bytes.as_slice()).expect("seed must be 32 bytes");
        let signing_key = SigningKey::<MlDsa65>::from_seed(&seed);
        let verifying_key = signing_key.verifying_key();
        let peer_id = compute_peer_id(&verifying_key);
        Self { signing_key, verifying_key, peer_id }
    }

    /// SHA3-256 of the ML-DSA-65 public key bytes.
    pub fn peer_id(&self) -> &Hash32 {
        &self.peer_id
    }

    /// Access the raw signing key for protocol-level use (BFT, PDP).
    pub fn signing_key(&self) -> &SigningKey<MlDsa65> {
        &self.signing_key
    }

    /// Sign a message with ML-DSA-65 (randomized).
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_vec()
    }

    /// Verify a signature against this identity's public key.
    #[must_use]
    pub fn verify(&self, message: &[u8], signature_bytes: &[u8]) -> bool {
        let sig = match Signature::<MlDsa65>::try_from(signature_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        self.verifying_key.verify(message, &sig).is_ok()
    }

    /// Verify a signature given a raw verifying key encoding.
    #[must_use]
    pub fn verify_with_pubkey(
        pubkey_encoded: &[u8],
        message: &[u8],
        signature_bytes: &[u8],
    ) -> bool {
        use ml_dsa::EncodedVerifyingKey;
        let vk_enc = match EncodedVerifyingKey::<MlDsa65>::try_from(pubkey_encoded) {
            Ok(e) => e,
            Err(_) => return false,
        };
        let vk = VerifyingKey::<MlDsa65>::decode(&vk_enc);
        let sig = match Signature::<MlDsa65>::try_from(signature_bytes) {
            Ok(s) => s,
            Err(_) => return false,
        };
        vk.verify(message, &sig).is_ok()
    }

    /// Encoded verifying key bytes (ML-DSA-65, 1952 bytes).
    pub fn verifying_key_encoded(&self) -> Vec<u8> {
        self.verifying_key.encode().as_slice().to_vec()
    }

    /// The signing key seed (32 bytes).
    pub fn to_seed(&self) -> [u8; 32] {
        let seed = self.signing_key.to_seed();
        let mut out = [0u8; 32];
        out.copy_from_slice(seed.as_slice());
        out
    }
}

/// Compute PeerId = SHA3-256(ML-DSA-65 public key encoding).
fn compute_peer_id(verifying_key: &VerifyingKey<MlDsa65>) -> Hash32 {
    let mut hasher = Sha3_256::new();
    Update::update(&mut hasher, verifying_key.encode().as_slice());
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Compute PeerId from raw verifying key bytes.
/// Used by the responder in identity-bound handshakes to verify
/// that the claimed peer_id matches the received verifying key.
pub fn compute_peer_id_from_bytes(vk_bytes: &[u8]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    Update::update(&mut hasher, vk_bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_generate_produces_valid_keypair() {
        let id = Identity::generate();
        let msg = b"handshake transcript";
        let sig = id.sign(msg);
        assert!(!sig.is_empty());
        assert!(id.verify(msg, &sig));
    }

    #[test]
    fn identity_deterministic_from_same_seed() {
        let seed = [42u8; 32];
        let id1 = Identity::from_seed(&seed);
        let id2 = Identity::from_seed(&seed);
        assert_eq!(id1.peer_id(), id2.peer_id());
        assert_eq!(id1.verifying_key_encoded(), id2.verifying_key_encoded());
        assert_eq!(id1.to_seed(), id2.to_seed());
    }

    #[test]
    fn identity_different_seeds_produce_different_keys() {
        let id1 = Identity::from_seed(&[1u8; 32]);
        let id2 = Identity::from_seed(&[2u8; 32]);
        assert_ne!(id1.peer_id(), id2.peer_id());
        assert_ne!(id1.verifying_key_encoded(), id2.verifying_key_encoded());
    }

    #[test]
    fn identity_sign_and_verify_roundtrip() {
        let id = Identity::generate();
        let msg = b"test message for signing";
        let sig = id.sign(msg);
        assert!(id.verify(msg, &sig));
    }

    #[test]
    fn identity_wrong_message_fails_verification() {
        let id = Identity::generate();
        let sig = id.sign(b"original message");
        assert!(!id.verify(b"tampered message", &sig));
    }

    #[test]
    fn identity_wrong_pubkey_fails_verification() {
        let id1 = Identity::generate();
        let id2 = Identity::generate();
        let msg = b"cross-identity test";
        let sig = id1.sign(msg);
        assert!(!id2.verify(msg, &sig));
    }

    #[test]
    fn identity_static_verify_with_pubkey_works() {
        let id = Identity::generate();
        let msg = b"verify with extracted pubkey";
        let sig = id.sign(msg);
        assert!(Identity::verify_with_pubkey(&id.verifying_key_encoded(), msg, &sig));
    }

    #[test]
    fn identity_verify_with_wrong_pubkey_rejected() {
        let id1 = Identity::generate();
        let id2 = Identity::generate();
        let msg = b"cross-identity verify";
        let sig = id1.sign(msg);
        assert!(!Identity::verify_with_pubkey(&id2.verifying_key_encoded(), msg, &sig));
    }

    #[test]
    fn identity_invalid_signature_bytes_rejected() {
        let id = Identity::generate();
        assert!(!id.verify(b"message", &[0u8; 10]));
    }

    #[test]
    fn identity_invalid_pubkey_bytes_rejected() {
        let id = Identity::generate();
        let msg = b"test";
        let sig = id.sign(msg);
        assert!(!Identity::verify_with_pubkey(&[0u8; 10], msg, &sig));
    }

    #[test]
    fn identity_peer_id_constant() {
        let id = Identity::generate();
        assert_eq!(id.peer_id(), id.peer_id());
    }

    #[test]
    fn identity_key_encode_decode_roundtrip() {
        let id = Identity::from_seed(&[99u8; 32]);
        let vk_enc = id.verifying_key_encoded();
        assert_eq!(vk_enc.len(), ML_DSA65_PUBKEY_LEN);

        use ml_dsa::EncodedVerifyingKey;
        let enc = EncodedVerifyingKey::<MlDsa65>::try_from(vk_enc.as_slice()).unwrap();
        let vk = VerifyingKey::<MlDsa65>::decode(&enc);
        let msg = b"roundtrip test";
        let sig = id.sign(msg);

        let sig_obj = ml_dsa::Signature::<MlDsa65>::try_from(sig.as_slice()).unwrap();
        vk.verify(msg, &sig_obj).expect("verification after roundtrip");
    }
}
