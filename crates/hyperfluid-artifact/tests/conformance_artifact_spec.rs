// Conformance tests for artifact-availability-spec.md Section 1.7
//
// Source: docs/04-specifications/storage/artifact-availability-spec.md

use hyperfluid_artifact::{
    chunk_bytes_for_test, compute_chunk_merkle_root, compute_manifest_root_hash,
    verify_proof_of_possession, ArtifactClass, ArtifactManifest, ProofOfPossession, RepairEntry,
    RepairQueue, RetentionTier,
};

fn test_manifest(
    class: ArtifactClass,
    tier: RetentionTier,
    chunk_count: u32,
    size_bytes: u64,
) -> ArtifactManifest {
    let mut manifest = ArtifactManifest {
        artifact_root_hash: [0u8; 32],
        chunk_root_hash: [0u8; 32],
        size_bytes,
        chunk_count,
        class,
        retention_tier: tier,
        min_replica_count: class.default_min_replicas(),
        created_at_height: 100,
        expires_at_height: 1000,
        producer_signature: vec![7u8; 64],
    };
    manifest.artifact_root_hash = compute_manifest_root_hash(&manifest);
    manifest
}

// ── Hook 1: Verify artifact root hash is deterministic ──

#[test]
fn conforms_to_artifact_spec_1_7_root_hash_deterministic() {
    let m1 = test_manifest(ArtifactClass::ResearchOutput, RetentionTier::MediumTerm, 2, 100);
    let m2 = test_manifest(ArtifactClass::ResearchOutput, RetentionTier::MediumTerm, 2, 100);

    let root1 = compute_manifest_root_hash(&m1);
    let root2 = compute_manifest_root_hash(&m2);
    assert_eq!(root1, root2, "identical manifests must produce identical root hashes");
}

#[test]
fn conforms_to_artifact_spec_1_7_root_hash_different_content_different_hash() {
    let m1 = test_manifest(ArtifactClass::ResearchOutput, RetentionTier::MediumTerm, 2, 100);
    let m2 = test_manifest(ArtifactClass::GovernanceBundle, RetentionTier::Pinned, 5, 500);

    let root1 = compute_manifest_root_hash(&m1);
    let root2 = compute_manifest_root_hash(&m2);
    assert_ne!(root1, root2, "different manifests must produce different root hashes");
}

// ── Hook 2: Verify chunk root hash is correct Merkle root ──

#[test]
fn conforms_to_artifact_spec_1_7_chunk_merkle_root_correct() {
    let chunks: Vec<&[u8]> =
        vec![b"chunk_01_data_here", b"chunk_02_data_here", b"chunk_03_data_here"];
    let root = compute_chunk_merkle_root(&chunks);

    // Recompute manually with balanced tree (odd leaf duplicated).
    use sha3::{Digest, Sha3_256};
    let h = |d: &[u8]| {
        let mut hasher = Sha3_256::new();
        hasher.update(d);
        let mut out = [0u8; 32];
        out.copy_from_slice(&hasher.finalize());
        out
    };
    let pair = |a: &[u8; 32], b: &[u8; 32]| {
        let mut data = Vec::new();
        data.extend_from_slice(a);
        data.extend_from_slice(b);
        h(&data)
    };
    let leaf0 = h(chunks[0]);
    let leaf1 = h(chunks[1]);
    let leaf2 = h(chunks[2]);
    let node01 = pair(&leaf0, &leaf1);
    let node22 = pair(&leaf2, &leaf2);
    let expected = pair(&node01, &node22);
    assert_eq!(root, expected, "chunk Merkle root must match manual recomputation");
}

#[test]
fn conforms_to_artifact_spec_1_7_chunk_merkle_root_empty_rejected() {
    let chunks: Vec<&[u8]> = vec![];
    let root = compute_chunk_merkle_root(&chunks);
    // Empty set should return zero hash
    assert_eq!(root, [0u8; 32], "empty chunks must return zero hash");
}

#[test]
fn conforms_to_artifact_spec_1_7_chunk_merkle_root_single_chunk() {
    let chunks: Vec<&[u8]> = vec![b"only_chunk"];
    let root = compute_chunk_merkle_root(&chunks);

    // Single leaf: Merkle root = hash(leaf)
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(chunks[0]);
    let mut expected = [0u8; 32];
    expected.copy_from_slice(&hasher.finalize());
    assert_eq!(root, expected, "single chunk merkle root = hash of leaf");
}

// ── Hook 3: Verify proof-of-possession ──

#[test]
fn conforms_to_artifact_spec_1_7_proof_of_possession_valid() {
    let chunks: Vec<Vec<u8>> = vec![
        b"chunk_01".to_vec(),
        b"chunk_02".to_vec(),
        b"chunk_03".to_vec(),
        b"chunk_04".to_vec(),
    ];
    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let chunk_root = compute_chunk_merkle_root(&chunk_refs);

    // Create a proof for chunk_index 1
    let proof = ProofOfPossession::build(&chunks, 1, chunk_root, [9u8; 32], 200);

    assert!(verify_proof_of_possession(&proof, &chunk_root), "valid proof must verify");
}

#[test]
fn conforms_to_artifact_spec_1_7_proof_of_possession_wrong_chunk_rejected() {
    let chunks: Vec<Vec<u8>> = vec![b"chunk_01".to_vec(), b"chunk_02".to_vec()];
    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let chunk_root = compute_chunk_merkle_root(&chunk_refs);

    let mut proof = ProofOfPossession::build(&chunks, 0, chunk_root, [1u8; 32], 100);
    proof.chunk_bytes = b"wrong_chunk_data".to_vec();

    assert!(
        !verify_proof_of_possession(&proof, &chunk_root),
        "tampered chunk must fail verification"
    );
}

#[test]
fn conforms_to_artifact_spec_1_7_proof_of_possession_wrong_root_rejected() {
    let chunks: Vec<Vec<u8>> = vec![b"chunk_01".to_vec(), b"chunk_02".to_vec()];
    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let chunk_root = compute_chunk_merkle_root(&chunk_refs);

    let proof = ProofOfPossession::build(&chunks, 0, chunk_root, [1u8; 32], 100);
    let wrong_root = [0xFFu8; 32];

    assert!(!verify_proof_of_possession(&proof, &wrong_root), "wrong root must fail verification");
}

// ── Hook 6: Governance bundles require 5 replicas ──

#[test]
fn conforms_to_artifact_spec_1_7_governance_bundle_min_replicas() {
    assert_eq!(ArtifactClass::GovernanceBundle.default_min_replicas(), 5);
}

#[test]
fn conforms_to_artifact_spec_1_7_research_output_min_replicas() {
    assert_eq!(ArtifactClass::ResearchOutput.default_min_replicas(), 2);
}

#[test]
fn conforms_to_artifact_spec_1_7_review_evidence_min_replicas() {
    assert_eq!(ArtifactClass::ReviewEvidence.default_min_replicas(), 3);
}

#[test]
fn conforms_to_artifact_spec_1_7_telemetry_archive_min_replicas() {
    assert_eq!(ArtifactClass::TelemetryArchive.default_min_replicas(), 2);
}

// ── Hook 8: Expired artifact transitions to pruned state ──

#[test]
fn conforms_to_artifact_spec_1_7_expired_artifact_pruned() {
    let manifest = ArtifactManifest {
        artifact_root_hash: [0u8; 32],
        chunk_root_hash: [0u8; 32],
        size_bytes: 100,
        chunk_count: 2,
        class: ArtifactClass::ResearchOutput,
        retention_tier: RetentionTier::ShortTerm,
        min_replica_count: 2,
        created_at_height: 100,
        expires_at_height: 150,
        producer_signature: vec![],
    };

    // Before expiry
    assert!(!manifest.is_expired(140));
    // At expiry
    assert!(manifest.is_expired(150));
    // After expiry
    assert!(manifest.is_expired(200));
}

#[test]
fn conforms_to_artifact_spec_1_7_pinned_never_expires() {
    let manifest = ArtifactManifest {
        artifact_root_hash: [0u8; 32],
        chunk_root_hash: [0u8; 32],
        size_bytes: 100,
        chunk_count: 2,
        class: ArtifactClass::GovernanceBundle,
        retention_tier: RetentionTier::Pinned,
        min_replica_count: 5,
        created_at_height: 100,
        expires_at_height: 0,
        producer_signature: vec![],
    };

    assert!(!manifest.is_expired(100));
    assert!(!manifest.is_expired(10_000_000));
}

// ── Hook 9: Repair queue prioritizes governance bundles ──

#[test]
fn conforms_to_artifact_spec_1_7_repair_queue_governance_priority() {
    let mut queue = RepairQueue::new(10);

    queue.push(RepairEntry {
        artifact_root_hash: [1u8; 32],
        artifact_class: ArtifactClass::ResearchOutput,
        current_replica_count: 1,
        target_replica_count: 2,
        priority: 5,
        entered_at_height: 100,
    });
    queue.push(RepairEntry {
        artifact_root_hash: [2u8; 32],
        artifact_class: ArtifactClass::GovernanceBundle,
        current_replica_count: 1,
        target_replica_count: 5,
        priority: 0,
        entered_at_height: 200,
    });
    queue.push(RepairEntry {
        artifact_root_hash: [3u8; 32],
        artifact_class: ArtifactClass::TelemetryArchive,
        current_replica_count: 1,
        target_replica_count: 2,
        priority: 5,
        entered_at_height: 50,
    });

    // Governance (priority 0 = highest) should come first
    let next = queue.pop_highest();
    assert_eq!(next.unwrap().artifact_root_hash, [2u8; 32]);
}

#[test]
fn conforms_to_artifact_spec_1_7_repair_queue_empty_returns_none() {
    let mut queue = RepairQueue::new(10);
    assert!(queue.pop_highest().is_none());
}

// ── Hook 5: Chunk bytes helper ──

#[test]
fn conforms_to_artifact_spec_1_7_chunks_for_test_produces_correct_data() {
    let data = b"test_data_here";
    let chunks = chunk_bytes_for_test(data, 4);
    assert_eq!(chunks.len(), 4);
    let expected_chunk_size = data.len().div_ceil(4);
    assert_eq!(chunks[0].len(), expected_chunk_size);
    let combined: Vec<u8> = chunks.iter().flat_map(|c| c.iter()).copied().collect();
    assert_eq!(combined, data.to_vec());
}
