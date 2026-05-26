// Conformance tests for artifact-availability-spec.md Section 1.7
//
// Source: docs/04-specifications/storage/artifact-availability-spec.md

use hyperfluid_artifact::{
    chunk_bytes_for_test, compute_chunk_merkle_root, compute_manifest_root_hash,
    verify_proof_of_possession, ArtifactClass, ArtifactManifest, ProofOfPossession, RepairEntry,
    RepairQueue, RetentionTier, EMPTY_MERKLE_ROOT,
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
    // Empty set should return the well-known sentinel
    assert_eq!(root, *EMPTY_MERKLE_ROOT, "empty chunks must return EMPTY_MERKLE_ROOT sentinel");
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
    let proof = ProofOfPossession::build(&chunks, 1, chunk_root, [9u8; 32], 200, vec![1u8; 64])
        .expect("valid proof must build");

    assert!(verify_proof_of_possession(&proof, &chunk_root), "valid proof must verify");
}

#[test]
fn conforms_to_artifact_spec_1_7_proof_of_possession_wrong_chunk_rejected() {
    let chunks: Vec<Vec<u8>> = vec![b"chunk_01".to_vec(), b"chunk_02".to_vec()];
    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let chunk_root = compute_chunk_merkle_root(&chunk_refs);

    let mut proof = ProofOfPossession::build(&chunks, 0, chunk_root, [1u8; 32], 100, vec![2u8; 64])
        .expect("valid proof must build");
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

    let proof = ProofOfPossession::build(&chunks, 0, chunk_root, [1u8; 32], 100, vec![3u8; 64])
        .expect("valid proof must build");
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

// ── Hook 4: Parallel retrieval from multiple providers ──

/// Hook 4: Verify parallel retrieval from min_replica_count + 2 providers succeeds.
///
/// Models the scenario where artifacts are distributed across multiple providers.
/// Retrieving from N+2 providers ensures redundancy — even if one provider returns
/// corrupt data, the remaining providers' correct chunks can be assembled.
#[test]
fn conforms_to_artifact_spec_1_7_parallel_retrieval_from_multiple_providers() {
    let data = b"artifact_payload_for_parallel_retrieval_test";
    let n_chunks: usize = 6;
    let chunks = chunk_bytes_for_test(data, n_chunks);
    assert_eq!(chunks.len(), n_chunks);

    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let expected_root = compute_chunk_merkle_root(&chunk_refs);

    // Simulate 4 providers (min_replica_count=2, +2 = 4 providers)
    let provider_count: usize = 4;
    let mut providers: Vec<Vec<Vec<u8>>> = Vec::with_capacity(provider_count);
    for _ in 0..provider_count {
        providers.push(chunks.clone());
    }

    // Provider 2 returns corrupt data for chunk index 3
    if providers.len() >= 3 {
        providers[2][3] = b"corrupted_chunk_data_xxx".to_vec();
    }

    // Retrieve chunk 3 from all providers, verify at least one is correct
    let mut correct_retrievals = 0u32;
    for (p_idx, provider_chunks) in providers.iter().enumerate() {
        let proof = merkle_proof_for_test(provider_chunks, 3);
        let leaf = leaf_hash_for_test(&provider_chunks[3]);
        let valid = verify_merkle_proof_for_test(&leaf, 3, &proof, &expected_root);
        if valid {
            correct_retrievals += 1;
        } else {
            // Provider p_idx returned corrupt data — expected for provider 2
            if p_idx != 2 {
                panic!("provider {} should have valid data", p_idx);
            }
        }
    }

    assert!(correct_retrievals >= 3, "at least 3 of 4 providers must return valid chunks");
}

/// Hook 4: Negative — single corrupt provider does not break retrieval.
#[test]
fn conforms_to_artifact_spec_1_7_parallel_retrieval_corrupt_isolation() {
    let data = b"critical_governance_payload";
    let n_chunks: usize = 3;
    let chunks = chunk_bytes_for_test(data, n_chunks);
    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let root = compute_chunk_merkle_root(&chunk_refs);

    // Only 1 of 3 providers has corrupt data for chunk 0
    let good_providers: Vec<Vec<Vec<u8>>> = vec![chunks.clone(), chunks.clone()];
    let mut corrupt_chunks = chunks.clone();
    corrupt_chunks[0] = b"totally_wrong_data_blob".to_vec();
    let corrupt_provider = corrupt_chunks;

    // Verify the good providers pass
    for provider in &good_providers {
        let proof = merkle_proof_for_test(provider, 0);
        let leaf = leaf_hash_for_test(&provider[0]);
        assert!(verify_merkle_proof_for_test(&leaf, 0, &proof, &root), "good provider must verify");
    }

    // Verify the corrupt provider fails
    let bad_proof = merkle_proof_for_test(&corrupt_provider, 0);
    let bad_leaf = leaf_hash_for_test(&corrupt_provider[0]);
    assert!(
        !verify_merkle_proof_for_test(&bad_leaf, 0, &bad_proof, &root),
        "corrupt provider must fail verification"
    );
}

/// Hook 4: Edge case — all providers are correct.
#[test]
fn conforms_to_artifact_spec_1_7_parallel_retrieval_all_correct() {
    let data = b"simple_payload";
    let chunks = chunk_bytes_for_test(data, 2);
    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let root = compute_chunk_merkle_root(&chunk_refs);

    // 4 providers all with correct data
    for _ in 0..4 {
        let proof = merkle_proof_for_test(&chunks, 0);
        let leaf = leaf_hash_for_test(&chunks[0]);
        assert!(verify_merkle_proof_for_test(&leaf, 0, &proof, &root));
    }
}

// ── Hook 7: AtRisk detection triggers repair ──

/// Hook 7: Verify AtRisk detection triggers repair coordinator.
///
/// When a lease goes from Active to AtRisk (e.g., proof-of-possession failure),
/// a RepairEntry is added to the repair queue. The coordinator pops entries by
/// priority (governance first, telemetry last). This test models the AtRisk → repair flow.
#[test]
fn conforms_to_artifact_spec_1_7_atrisk_triggers_repair() {
    let mut queue = RepairQueue::new(10);

    // Simulate AtRisk detection: 3 leases detected as AtRisk
    // Lease 1: Governance bundle — should be highest priority
    queue.push(RepairEntry {
        artifact_root_hash: [10u8; 32],
        artifact_class: ArtifactClass::GovernanceBundle,
        current_replica_count: 1,
        target_replica_count: 5,
        priority: ArtifactClass::GovernanceBundle.repair_priority(),
        entered_at_height: 500,
    });

    // Lease 2: Review evidence — medium priority
    queue.push(RepairEntry {
        artifact_root_hash: [20u8; 32],
        artifact_class: ArtifactClass::ReviewEvidence,
        current_replica_count: 1,
        target_replica_count: 3,
        priority: ArtifactClass::ReviewEvidence.repair_priority(),
        entered_at_height: 510,
    });

    // Lease 3: Research output — lower priority, but entered earlier
    queue.push(RepairEntry {
        artifact_root_hash: [30u8; 32],
        artifact_class: ArtifactClass::ResearchOutput,
        current_replica_count: 0,
        target_replica_count: 2,
        priority: ArtifactClass::ResearchOutput.repair_priority(),
        entered_at_height: 400,
    });

    // Lease 4: Telemetry archive — lowest priority
    queue.push(RepairEntry {
        artifact_root_hash: [40u8; 32],
        artifact_class: ArtifactClass::TelemetryArchive,
        current_replica_count: 1,
        target_replica_count: 2,
        priority: ArtifactClass::TelemetryArchive.repair_priority(),
        entered_at_height: 520,
    });

    // Repair coordinator pops entries: governance (priority 0), review (1), research (2), telemetry (3)
    let first = queue.pop_highest().expect("must have at least one entry");
    assert_eq!(
        first.artifact_class,
        ArtifactClass::GovernanceBundle,
        "governance must be repaired first"
    );

    let second = queue.pop_highest().expect("must have second entry");
    assert_eq!(
        second.artifact_class,
        ArtifactClass::ReviewEvidence,
        "review evidence must be repaired second"
    );

    let third = queue.pop_highest().expect("must have third entry");
    assert_eq!(
        third.artifact_class,
        ArtifactClass::ResearchOutput,
        "research output must be repaired third"
    );

    let fourth = queue.pop_highest().expect("must have fourth entry");
    assert_eq!(
        fourth.artifact_class,
        ArtifactClass::TelemetryArchive,
        "telemetry must be repaired last"
    );

    // Queue must be exhausted
    assert!(queue.pop_highest().is_none());
}

/// Hook 7: Negative — AtRisk for TelemetryArchive is lowest priority.
#[test]
fn conforms_to_artifact_spec_1_7_atrisk_telemetry_lowest_priority() {
    let mut queue = RepairQueue::new(10);

    queue.push(RepairEntry {
        artifact_root_hash: [50u8; 32],
        artifact_class: ArtifactClass::TelemetryArchive,
        current_replica_count: 1,
        target_replica_count: 2,
        priority: ArtifactClass::TelemetryArchive.repair_priority(),
        entered_at_height: 100,
    });
    queue.push(RepairEntry {
        artifact_root_hash: [51u8; 32],
        artifact_class: ArtifactClass::ReviewEvidence,
        current_replica_count: 1,
        target_replica_count: 3,
        priority: ArtifactClass::ReviewEvidence.repair_priority(),
        entered_at_height: 200,
    });

    let first = queue.pop_highest().unwrap();
    assert_eq!(first.artifact_class, ArtifactClass::ReviewEvidence);
}

/// Hook 7: Edge case — zero-replica (lost artifact) still enters repair queue with
/// target_replica_count unchanged.
#[test]
fn conforms_to_artifact_spec_1_7_atrisk_zero_replicas_enters_queue() {
    let mut queue = RepairQueue::new(10);

    // Artifact with 0 replicas (critically lost) still added to queue
    queue.push(RepairEntry {
        artifact_root_hash: [60u8; 32],
        artifact_class: ArtifactClass::GovernanceBundle,
        current_replica_count: 0,
        target_replica_count: 5,
        priority: 0,
        entered_at_height: 100,
    });

    let entry = queue.pop_highest().unwrap();
    assert_eq!(entry.current_replica_count, 0);
    assert_eq!(entry.target_replica_count, 5);
}

// ── Helpers for parallel retrieval tests ──

/// Compute SHA3-256 hash of a chunk for parallel retrieval testing.
fn leaf_hash_for_test(chunk: &[u8]) -> [u8; 32] {
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(chunk);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Generate a Merkle proof for a chunk at the given index.
fn merkle_proof_for_test(chunks: &[Vec<u8>], chunk_index: u32) -> Vec<[u8; 32]> {
    use sha3::{Digest, Sha3_256};
    let idx = chunk_index as usize;
    if chunks.is_empty() || idx >= chunks.len() {
        return vec![];
    }
    let mut level: Vec<[u8; 32]> = chunks.iter().map(|c| leaf_hash_for_test(c)).collect();
    let mut proof = Vec::new();
    let mut current_idx = idx;
    while level.len() > 1 {
        let sibling = if current_idx.is_multiple_of(2) {
            if current_idx + 1 < level.len() {
                level[current_idx + 1]
            } else {
                level[current_idx]
            }
        } else {
            level[current_idx - 1]
        };
        proof.push(sibling);
        let mut next_level = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            let left = pair[0];
            let right = if pair.len() == 2 { pair[1] } else { pair[0] };
            let mut hasher = Sha3_256::new();
            hasher.update(left);
            hasher.update(right);
            let mut node = [0u8; 32];
            node.copy_from_slice(&hasher.finalize());
            next_level.push(node);
        }
        level = next_level;
        current_idx /= 2;
    }
    proof
}

/// Verify a Merkle proof for parallel retrieval testing.
fn verify_merkle_proof_for_test(
    leaf_hash: &[u8; 32],
    chunk_index: u32,
    proof: &[[u8; 32]],
    expected_root: &[u8; 32],
) -> bool {
    use sha3::{Digest, Sha3_256};
    let mut current_hash = *leaf_hash;
    let mut current_idx = chunk_index as usize;
    for sibling in proof {
        let mut hasher = Sha3_256::new();
        if current_idx.is_multiple_of(2) {
            hasher.update(current_hash);
            hasher.update(sibling);
        } else {
            hasher.update(sibling);
            hasher.update(current_hash);
        }
        let mut node = [0u8; 32];
        node.copy_from_slice(&hasher.finalize());
        current_hash = node;
        current_idx /= 2;
    }
    current_hash == *expected_root
}
