use sha3::{Digest, Sha3_256};

use crate::types::ArtifactManifest;

/// Compute the artifact root hash = SHA3-256 of canonical serialized manifest.
///
/// Per spec Section 1.2: canonical serialization excludes the artifact_root_hash itself
/// and the producer_signature (non-deterministic variable-length field).
pub fn compute_manifest_root_hash(manifest: &ArtifactManifest) -> [u8; 32] {
    let mut hasher = Sha3_256::new();

    // Canonical fields in fixed order per spec Section 1.3:
    // chunk_root_hash, size_bytes, chunk_count, class, retention_tier,
    // min_replica_count, created_at_height, expires_at_height
    Digest::update(&mut hasher, manifest.chunk_root_hash);
    Digest::update(&mut hasher, manifest.size_bytes.to_le_bytes());
    Digest::update(&mut hasher, manifest.chunk_count.to_le_bytes());
    Digest::update(&mut hasher, [manifest.class.discriminant()]);
    Digest::update(&mut hasher, [manifest.retention_tier.discriminant()]);
    Digest::update(&mut hasher, [manifest.min_replica_count]);
    Digest::update(&mut hasher, manifest.created_at_height.to_le_bytes());
    Digest::update(&mut hasher, manifest.expires_at_height.to_le_bytes());

    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}
