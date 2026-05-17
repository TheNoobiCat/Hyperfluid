// === C9 PDP: Key Rotation State Finalization ===
//
// Source: policy-engine-spec.md §3

use crate::types::{Hash32, KeyBinding, KeyRotationTransaction};
use ml_dsa::{EncodedVerifyingKey, MlDsa65, Verifier, VerifyingKey};
use sha3::{Digest, Sha3_256};

/// Grace window duration for key rotation (in blocks).
pub const KEY_ROTATION_GRACE_WINDOW: u64 = 100;

pub fn initiate_key_rotation(
    binding: &KeyBinding,
    tx: &KeyRotationTransaction,
    current_height: u64,
) -> Result<KeyBinding, KeyRotationError> {
    if tx.agent_id != binding.agent_id {
        return Err(KeyRotationError::AgentIdMismatch);
    }

    verify_key_rotation_signature(binding, tx)?;

    let computed_hash = sha3_256_bytes(&tx.new_pubkey);
    if computed_hash != tx.new_pubkey_hash {
        return Err(KeyRotationError::PubkeyHashMismatch);
    }

    if EncodedVerifyingKey::<MlDsa65>::try_from(tx.new_pubkey.as_slice()).is_err() {
        return Err(KeyRotationError::InvalidPubkey);
    }

    let new_binding = KeyBinding {
        agent_id: binding.agent_id,
        active_pubkey: binding.active_pubkey.clone(),
        pending_pubkey: Some(tx.new_pubkey.clone()),
        rotation_height: Some(current_height),
        grace_end_height: Some(current_height + KEY_ROTATION_GRACE_WINDOW),
    };

    Ok(new_binding)
}

pub fn finalize_key_rotation(binding: &KeyBinding, current_height: u64) -> KeyBinding {
    if !binding.in_grace_window(current_height) {
        if let Some(ref pending) = binding.pending_pubkey {
            return KeyBinding {
                agent_id: binding.agent_id,
                active_pubkey: pending.clone(),
                pending_pubkey: None,
                rotation_height: None,
                grace_end_height: None,
            };
        }
    }
    binding.clone()
}

pub fn verify_during_rotation(
    binding: &KeyBinding,
    message: &[u8],
    signature: &[u8],
    current_height: u64,
) -> Result<(), KeyRotationError> {
    if binding.in_grace_window(current_height) {
        if let Some(ref pending) = binding.pending_pubkey {
            if verify_ml_dsa_raw(message, signature, pending).is_ok() {
                return Ok(());
            }
        }
    }

    verify_ml_dsa_raw(message, signature, &binding.active_pubkey)
}

fn verify_key_rotation_signature(
    binding: &KeyBinding,
    tx: &KeyRotationTransaction,
) -> Result<(), KeyRotationError> {
    let message = hash_rotation_tx(tx);
    verify_ml_dsa_raw(&message, &tx.signature, &binding.active_pubkey)
}

fn hash_rotation_tx(tx: &KeyRotationTransaction) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(tx.agent_id);
    hasher.update(&tx.new_pubkey);
    hasher.update(tx.new_pubkey_hash);
    hasher.update(tx.nonce.to_le_bytes());
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

fn verify_ml_dsa_raw(
    message: &[u8],
    signature: &[u8],
    pubkey: &[u8],
) -> Result<(), KeyRotationError> {
    let vk_enc = EncodedVerifyingKey::<MlDsa65>::try_from(pubkey)
        .map_err(|_| KeyRotationError::SignatureInvalid)?;
    let vk = VerifyingKey::<MlDsa65>::decode(&vk_enc);
    let sig = ml_dsa::Signature::<MlDsa65>::try_from(signature)
        .map_err(|_| KeyRotationError::SignatureInvalid)?;
    vk.verify(message, &sig).map_err(|_| KeyRotationError::SignatureInvalid)
}

fn sha3_256_bytes(data: &[u8]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum KeyRotationError {
    #[error("agent_id mismatch")]
    AgentIdMismatch,

    #[error("signature invalid")]
    SignatureInvalid,

    #[error("new pubkey hash does not match expected value")]
    PubkeyHashMismatch,

    #[error("new pubkey is not a valid ML-DSA-65 key")]
    InvalidPubkey,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ml_dsa::{Generate, Keypair, Seed, SignatureEncoding, Signer, SigningKey};

    /// Returns (pubkey_encoded, seed_bytes) for an ML-DSA-65 keypair.
    fn test_keypair() -> (Vec<u8>, [u8; 32]) {
        let sk = SigningKey::<MlDsa65>::generate();
        let pk = sk.verifying_key().encode().as_slice().to_vec();
        let seed = sk.to_seed();
        let mut seed_bytes = [0u8; 32];
        seed_bytes.copy_from_slice(seed.as_slice());
        (pk, seed_bytes)
    }

    fn reconstruct_signing_key(seed_bytes: &[u8; 32]) -> SigningKey<MlDsa65> {
        let seed = Seed::try_from(seed_bytes.as_slice()).unwrap();
        SigningKey::<MlDsa65>::from_seed(&seed)
    }

    fn sign_rotation_tx(tx: &KeyRotationTransaction, sk_seed: &[u8; 32]) -> Vec<u8> {
        let sk = reconstruct_signing_key(sk_seed);
        let msg = hash_rotation_tx(tx);
        let sig = sk.sign(&msg);
        sig.to_vec()
    }

    #[test]
    fn initiate_rotation_creates_pending_key() {
        let (pk, sk_seed) = test_keypair();
        let (new_pk, _new_seed) = test_keypair();
        let agent_id = [0xAA; 32];
        let binding = KeyBinding::stable(agent_id, pk);

        let tx = KeyRotationTransaction {
            agent_id,
            new_pubkey: new_pk.clone(),
            new_pubkey_hash: sha3_256_bytes(&new_pk),
            signature: vec![],
            nonce: 1,
        };
        let signed_tx = KeyRotationTransaction { signature: sign_rotation_tx(&tx, &sk_seed), ..tx };

        let result = initiate_key_rotation(&binding, &signed_tx, 100);
        assert!(result.is_ok());
        let new_binding = result.unwrap();
        assert_eq!(new_binding.pending_pubkey, Some(new_pk));
        assert_eq!(new_binding.rotation_height, Some(100));
        assert_eq!(new_binding.grace_end_height, Some(200));
    }

    #[test]
    fn initiate_rotation_rejects_wrong_signature() {
        let (pk, _sk_seed) = test_keypair();
        let (new_pk, _new_seed) = test_keypair();
        let (_, wrong_seed) = test_keypair();
        let agent_id = [0xAA; 32];
        let binding = KeyBinding::stable(agent_id, pk);

        let tx = KeyRotationTransaction {
            agent_id,
            new_pubkey: new_pk.clone(),
            new_pubkey_hash: sha3_256_bytes(&new_pk),
            signature: vec![],
            nonce: 1,
        };
        let signed_tx =
            KeyRotationTransaction { signature: sign_rotation_tx(&tx, &wrong_seed), ..tx };

        let result = initiate_key_rotation(&binding, &signed_tx, 100);
        assert!(result.is_err());
    }

    #[test]
    fn initiate_rotation_rejects_agent_id_mismatch() {
        let (pk, sk_seed) = test_keypair();
        let (new_pk, _new_seed) = test_keypair();
        let agent_id = [0xAA; 32];
        let binding = KeyBinding::stable(agent_id, pk);

        let tx = KeyRotationTransaction {
            agent_id: [0xBB; 32],
            new_pubkey: new_pk.clone(),
            new_pubkey_hash: sha3_256_bytes(&new_pk),
            signature: vec![],
            nonce: 1,
        };
        let signed_tx = KeyRotationTransaction { signature: sign_rotation_tx(&tx, &sk_seed), ..tx };

        let result = initiate_key_rotation(&binding, &signed_tx, 100);
        assert_eq!(result, Err(KeyRotationError::AgentIdMismatch));
    }

    #[test]
    fn initiate_rotation_rejects_hash_mismatch() {
        let (pk, sk_seed) = test_keypair();
        let (new_pk, _new_seed) = test_keypair();
        let agent_id = [0xAA; 32];
        let binding = KeyBinding::stable(agent_id, pk);

        let tx = KeyRotationTransaction {
            agent_id,
            new_pubkey: new_pk,
            new_pubkey_hash: [0xFF; 32],
            signature: vec![],
            nonce: 1,
        };
        let signed_tx = KeyRotationTransaction { signature: sign_rotation_tx(&tx, &sk_seed), ..tx };

        let result = initiate_key_rotation(&binding, &signed_tx, 100);
        assert_eq!(result, Err(KeyRotationError::PubkeyHashMismatch));
    }

    #[test]
    fn initiate_rotation_rejects_invalid_pubkey() {
        let (pk, sk_seed) = test_keypair();
        let agent_id = [0xAA; 32];
        let binding = KeyBinding::stable(agent_id, pk);
        let invalid_pubkey = vec![0u8; 10];

        let tx = KeyRotationTransaction {
            agent_id,
            new_pubkey: invalid_pubkey.clone(),
            new_pubkey_hash: sha3_256_bytes(&invalid_pubkey),
            signature: vec![],
            nonce: 1,
        };
        let signed_tx = KeyRotationTransaction { signature: sign_rotation_tx(&tx, &sk_seed), ..tx };

        let result = initiate_key_rotation(&binding, &signed_tx, 100);
        assert_eq!(result, Err(KeyRotationError::InvalidPubkey));
    }

    #[test]
    fn supersede_pending_rotation_restarts_grace_window() {
        let (pk, sk_seed) = test_keypair();
        let (new_pk1, _new_seed1) = test_keypair();
        let (new_pk2, _new_seed2) = test_keypair();
        let agent_id = [0xAA; 32];

        let binding = KeyBinding::stable(agent_id, pk.clone());
        let tx1 = KeyRotationTransaction {
            agent_id,
            new_pubkey: new_pk1.clone(),
            new_pubkey_hash: sha3_256_bytes(&new_pk1),
            signature: vec![],
            nonce: 1,
        };
        let signed_tx1 =
            KeyRotationTransaction { signature: sign_rotation_tx(&tx1, &sk_seed), ..tx1 };
        let binding2 = initiate_key_rotation(&binding, &signed_tx1, 100).unwrap();
        assert_eq!(binding2.grace_end_height, Some(200));

        let tx2 = KeyRotationTransaction {
            agent_id,
            new_pubkey: new_pk2.clone(),
            new_pubkey_hash: sha3_256_bytes(&new_pk2),
            signature: vec![],
            nonce: 2,
        };
        let signed_tx2 =
            KeyRotationTransaction { signature: sign_rotation_tx(&tx2, &sk_seed), ..tx2 };
        let binding3 = initiate_key_rotation(&binding2, &signed_tx2, 150).unwrap();
        assert_eq!(binding3.pending_pubkey, Some(new_pk2));
        assert_eq!(binding3.grace_end_height, Some(250));
    }
}
