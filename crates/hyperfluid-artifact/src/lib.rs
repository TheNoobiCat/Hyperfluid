//! C8 Artifact Availability & Storage
//!
//! Content-addressed storage via SHA3-256, proof-of-possession,
//! parallel retrieval, retention tiers, replication leases, repair coordination.
//!
//! Source: docs/04-specifications/storage/artifact-availability-spec.md

mod chunks;
mod manifest;
pub mod store;
mod types;

pub use chunks::chunk_bytes_for_test;
pub use chunks::{
    compute_chunk_merkle_root, hash_leaf, merkle_proof_for_chunk, verify_merkle_proof,
    EMPTY_MERKLE_ROOT,
};
pub use manifest::compute_manifest_root_hash;
pub use store::{
    chunk_exists, chunk_path, delete_chunk, load_chunk, store_chunk, StoreConfig, StoreError,
};
pub use types::{
    ArtifactClass, ArtifactManifest, LeaseStatus, ProofOfPossession, RepairEntry, RepairQueue,
    ReplicationLease, RetentionTier,
};

/// Verify the Merkle inclusion proof inside a `ProofOfPossession`.
///
/// # SPEC_DEVIATION: lease signature verification delegated to caller
///
/// This function only checks the Merkle proof — it does **not** verify
/// `proof.lease_signature`. Callers MUST independently verify the lease
/// signature using `hyperfluid_p2p::identity::Identity::verify_with_pubkey()`
/// before treating the proof as authentic. See `ProofOfPossession::build`.
pub fn verify_proof_of_possession(
    proof: &ProofOfPossession,
    expected_chunk_root: &[u8; 32],
) -> bool {
    debug_assert!(
        !proof.lease_signature.is_empty(),
        "lease signature should have been verified by caller; see SPEC_DEVIATION note"
    );
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(&proof.chunk_bytes);
    let mut leaf_hash = [0u8; 32];
    leaf_hash.copy_from_slice(&hasher.finalize());

    verify_merkle_proof(&leaf_hash, proof.chunk_index, &proof.merkle_proof, expected_chunk_root)
}
