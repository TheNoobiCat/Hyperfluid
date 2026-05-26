//! Verification tests for production-readiness fixes F-29, F-63, F-64, F-87.
//!
//! Each fix has at least one positive assertion (correct usage succeeds) and
//! one negative assertion (incorrect usage is rejected).

use hyperfluid_artifact::{
    compute_chunk_merkle_root, verify_proof_of_possession, ArtifactClass, ArtifactManifest,
    ProofOfPossession, RepairEntry, RepairQueue, RetentionTier, EMPTY_MERKLE_ROOT,
};

// ═══════════════════════════════════════════════════════════════════
// F-29: lease_signature in ProofOfPossession::build
// ═══════════════════════════════════════════════════════════════════

/// Positive: build accepts a non-empty lease signature and returns Some.
#[test]
fn fix_f29_positive_accepts_lease_signature() {
    let chunks: Vec<Vec<u8>> = vec![b"data_a".to_vec(), b"data_b".to_vec()];
    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let chunk_root = compute_chunk_merkle_root(&chunk_refs);

    let proof = ProofOfPossession::build(&chunks, 0, chunk_root, [42u8; 32], 100, vec![0xABu8; 64])
        .expect("build must succeed with valid inputs and a non-empty signature");

    assert_eq!(proof.lease_signature, vec![0xABu8; 64]);
    assert_eq!(proof.chunk_index, 0);
}

/// Positive: proof with a lease signature still verifies via Merkle proof.
#[test]
fn fix_f29_positive_verify_proof_with_signature() {
    let chunks: Vec<Vec<u8>> = vec![b"aaa".to_vec(), b"bbb".to_vec(), b"ccc".to_vec()];
    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let chunk_root = compute_chunk_merkle_root(&chunk_refs);

    let proof = ProofOfPossession::build(&chunks, 2, chunk_root, [1u8; 32], 200, vec![99u8; 64])
        .expect("build must succeed");

    assert!(
        verify_proof_of_possession(&proof, &chunk_root),
        "Merkle proof must verify even with a lease signature present"
    );
}

/// Negative: build returns None for out-of-bounds chunk index.
#[test]
fn fix_f29_negative_oob_chunk_index_returns_none() {
    let chunks: Vec<Vec<u8>> = vec![b"only".to_vec()];
    let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
    let chunk_root = compute_chunk_merkle_root(&chunk_refs);

    let proof = ProofOfPossession::build(
        &chunks,
        5, // OOB
        chunk_root,
        [1u8; 32],
        100,
        vec![1u8; 64],
    );
    assert!(proof.is_none(), "OOB chunk index must return None");
}

/// Negative: build returns None when Merkle proof doesn't match chunk_root_hash.
#[test]
fn fix_f29_negative_wrong_root_returns_none() {
    let chunks: Vec<Vec<u8>> = vec![b"chunk_a".to_vec(), b"chunk_b".to_vec()];
    let wrong_root = [0xFFu8; 32]; // Not the actual Merkle root

    let proof = ProofOfPossession::build(&chunks, 0, wrong_root, [1u8; 32], 100, vec![1u8; 64]);
    assert!(proof.is_none(), "build must fail when chunk_root_hash does not match");
}

// ═══════════════════════════════════════════════════════════════════
// F-63: Empty Merkle tree returns sentinel instead of [0u8; 32]
// ═══════════════════════════════════════════════════════════════════

/// Positive: empty chunk list returns EMPTY_MERKLE_ROOT.
#[test]
fn fix_f63_positive_empty_returns_sentinel() {
    let chunks: Vec<&[u8]> = vec![];
    let root = compute_chunk_merkle_root(&chunks);
    assert_eq!(root, *EMPTY_MERKLE_ROOT, "empty tree must return sentinel");
}

/// Positive: single chunk root is NOT the sentinel (ensures no accidental collision).
#[test]
fn fix_f63_positive_single_chunk_not_sentinel() {
    let chunks: Vec<&[u8]> = vec![b"some_real_data"];
    let root = compute_chunk_merkle_root(&chunks);
    assert_ne!(root, *EMPTY_MERKLE_ROOT, "single chunk must not collide with sentinel");
}

/// Negative: a non-empty tree never equals the sentinel.
#[test]
fn fix_f63_negative_non_empty_not_sentinel() {
    let chunks: Vec<&[u8]> = vec![b"a", b"b", b"c"];
    let root = compute_chunk_merkle_root(&chunks);
    assert_ne!(root, *EMPTY_MERKLE_ROOT, "non-empty tree must not equal sentinel");
}

/// Positive: sentinel is deterministic (same hash every time).
#[test]
fn fix_f63_positive_sentinel_is_deterministic() {
    let empty: Vec<&[u8]> = vec![];
    let root1 = compute_chunk_merkle_root(&empty);
    let root2 = compute_chunk_merkle_root(&empty);
    assert_eq!(root1, root2, "sentinel must be deterministic across calls");
    assert_eq!(root1, *EMPTY_MERKLE_ROOT);
}

/// Positive: sentinel is a known value (pre-computed).
#[test]
fn fix_f63_positive_sentinel_known_value() {
    // SHA3-256(b"HYPERFLUID_EMPTY_MERKLE_TREE") — pre-computed
    let expected: [u8; 32] = [
        0xaf, 0x9c, 0x4f, 0xa8, 0xff, 0xfb, 0xf9, 0x12, 0x19, 0x7c, 0x64, 0x60, 0xf6, 0x33, 0x38,
        0x9d, 0xd5, 0x58, 0xd2, 0xd2, 0xb4, 0xdd, 0xc9, 0xe1, 0xbb, 0x86, 0x23, 0x32, 0xb6, 0x6b,
        0x60, 0xce,
    ];
    assert_eq!(*EMPTY_MERKLE_ROOT, expected);
}

// ═══════════════════════════════════════════════════════════════════
// F-64: is_expired uses explicit match arms (no wildcard)
// ═══════════════════════════════════════════════════════════════════

/// Positive: Pinned artifact never expires.
#[test]
fn fix_f64_positive_pinned_never_expires() {
    let manifest = ArtifactManifest {
        artifact_root_hash: [0u8; 32],
        chunk_root_hash: [0u8; 32],
        size_bytes: 42,
        chunk_count: 1,
        class: ArtifactClass::GovernanceBundle,
        retention_tier: RetentionTier::Pinned,
        min_replica_count: 5,
        created_at_height: 0,
        expires_at_height: 0,
        producer_signature: vec![],
    };
    // Even at extremely high block heights Pinned must not expire.
    assert!(!manifest.is_expired(0));
    assert!(!manifest.is_expired(10_000_000));
}

/// Positive: ShortTerm expires at the correct height.
#[test]
fn fix_f64_positive_mediumterm_expires() {
    let manifest = ArtifactManifest {
        artifact_root_hash: [1u8; 32],
        chunk_root_hash: [1u8; 32],
        size_bytes: 100,
        chunk_count: 2,
        class: ArtifactClass::ResearchOutput,
        retention_tier: RetentionTier::MediumTerm,
        min_replica_count: 2,
        created_at_height: 50,
        expires_at_height: 200,
        producer_signature: vec![],
    };
    assert!(!manifest.is_expired(199), "not yet expired at 199");
    assert!(manifest.is_expired(200), "expired at expiry height");
    assert!(manifest.is_expired(300), "expired after expiry height");
}

/// Positive: ShortTerm expires at the correct height.
#[test]
fn fix_f64_positive_shortterm_expires() {
    let manifest = ArtifactManifest {
        artifact_root_hash: [2u8; 32],
        chunk_root_hash: [2u8; 32],
        size_bytes: 50,
        chunk_count: 1,
        class: ArtifactClass::TelemetryArchive,
        retention_tier: RetentionTier::ShortTerm,
        min_replica_count: 2,
        created_at_height: 10,
        expires_at_height: 100,
        producer_signature: vec![],
    };
    assert!(!manifest.is_expired(99));
    assert!(manifest.is_expired(100));
    assert!(manifest.is_expired(101));
}

/// Positive: expires_at_height == 0 means "no expiry" for non-pinned tiers.
#[test]
fn fix_f64_positive_zero_expiry_never_expires() {
    let manifest = ArtifactManifest {
        artifact_root_hash: [3u8; 32],
        chunk_root_hash: [3u8; 32],
        size_bytes: 10,
        chunk_count: 1,
        class: ArtifactClass::ReviewEvidence,
        retention_tier: RetentionTier::MediumTerm,
        min_replica_count: 3,
        created_at_height: 0,
        expires_at_height: 0,
        producer_signature: vec![],
    };
    assert!(!manifest.is_expired(0));
    assert!(!manifest.is_expired(1_000_000));
}

/// Negative: Pinned artifact must NOT expire even with expires_at_height set.
#[test]
fn fix_f64_negative_pinned_ignores_expiry_height() {
    let manifest = ArtifactManifest {
        artifact_root_hash: [4u8; 32],
        chunk_root_hash: [4u8; 32],
        size_bytes: 1,
        chunk_count: 1,
        class: ArtifactClass::GovernanceBundle,
        retention_tier: RetentionTier::Pinned,
        min_replica_count: 5,
        created_at_height: 0,
        expires_at_height: 100, // Even though expiry is set, Pinned ignores it
        producer_signature: vec![],
    };
    assert!(!manifest.is_expired(9999), "Pinned artifacts must never expire");
}

// ═══════════════════════════════════════════════════════════════════
// F-87: RepairQueue max_concurrent enforcement
// ═══════════════════════════════════════════════════════════════════

/// Positive: can_schedule_repair returns true when under capacity.
#[test]
fn fix_f87_positive_can_schedule_under_capacity() {
    let mut queue = RepairQueue::new(5);
    assert!(queue.can_schedule_repair(), "brand-new queue must allow scheduling");
    // Schedule 3 repairs, still under 5
    for _ in 0..3 {
        queue.push(RepairEntry {
            artifact_root_hash: [0u8; 32],
            artifact_class: ArtifactClass::ResearchOutput,
            current_replica_count: 0,
            target_replica_count: 2,
            priority: 0,
            entered_at_height: 100,
        });
        let _ = queue.try_schedule_repair().expect("must schedule under capacity");
    }
    assert!(queue.can_schedule_repair(), "3 of 5 slots used, must still allow");
}

/// Positive: try_schedule_repair pops the highest priority entry.
#[test]
fn fix_f87_positive_try_schedule_pops_highest_priority() {
    let mut queue = RepairQueue::new(3);
    queue.push(RepairEntry {
        artifact_root_hash: [10u8; 32],
        artifact_class: ArtifactClass::TelemetryArchive,
        current_replica_count: 1,
        target_replica_count: 2,
        priority: 10,
        entered_at_height: 100,
    });
    queue.push(RepairEntry {
        artifact_root_hash: [20u8; 32],
        artifact_class: ArtifactClass::GovernanceBundle,
        current_replica_count: 1,
        target_replica_count: 5,
        priority: 0,
        entered_at_height: 200,
    });

    let entry = queue.try_schedule_repair().expect("must schedule");
    assert_eq!(entry.artifact_root_hash, [20u8; 32], "governance bundle must be scheduled first");
}

/// Positive: finish_repair frees a slot.
#[test]
fn fix_f87_positive_finish_repair_frees_slot() {
    let mut queue = RepairQueue::new(2);
    queue.push(RepairEntry {
        artifact_root_hash: [1u8; 32],
        artifact_class: ArtifactClass::GovernanceBundle,
        current_replica_count: 1,
        target_replica_count: 5,
        priority: 0,
        entered_at_height: 100,
    });
    queue.push(RepairEntry {
        artifact_root_hash: [2u8; 32],
        artifact_class: ArtifactClass::ReviewEvidence,
        current_replica_count: 1,
        target_replica_count: 3,
        priority: 1,
        entered_at_height: 200,
    });
    let _ = queue.try_schedule_repair().expect("first");
    let _ = queue.try_schedule_repair().expect("second");
    assert!(!queue.can_schedule_repair(), "at capacity (2 of 2)");

    queue.finish_repair();
    assert!(queue.can_schedule_repair(), "slot freed after finish");
}

/// Negative: try_schedule_repair fails when at max_concurrent capacity.
#[test]
fn fix_f87_negative_rejects_when_at_capacity() {
    let mut queue = RepairQueue::new(1);
    queue.push(RepairEntry {
        artifact_root_hash: [1u8; 32],
        artifact_class: ArtifactClass::GovernanceBundle,
        current_replica_count: 1,
        target_replica_count: 5,
        priority: 0,
        entered_at_height: 100,
    });
    let _ = queue.try_schedule_repair().expect("first must succeed");

    // Push another entry — queue is at capacity
    queue.push(RepairEntry {
        artifact_root_hash: [2u8; 32],
        artifact_class: ArtifactClass::ResearchOutput,
        current_replica_count: 1,
        target_replica_count: 2,
        priority: 5,
        entered_at_height: 200,
    });

    let result = queue.try_schedule_repair();
    assert!(result.is_err(), "must reject when max_concurrent = 1 and one repair in progress");
}

/// Negative: try_schedule_repair fails when no pending entries.
#[test]
fn fix_f87_negative_rejects_empty_queue() {
    let mut queue = RepairQueue::new(5);
    let result = queue.try_schedule_repair();
    assert!(result.is_err(), "must reject when no pending entries");
}

/// Negative: max_concurrent = 0 always rejects.
#[test]
fn fix_f87_negative_zero_max_concurrent_rejects_all() {
    let mut queue = RepairQueue::new(0);
    queue.push(RepairEntry {
        artifact_root_hash: [1u8; 32],
        artifact_class: ArtifactClass::GovernanceBundle,
        current_replica_count: 1,
        target_replica_count: 5,
        priority: 0,
        entered_at_height: 100,
    });
    assert!(!queue.can_schedule_repair(), "zero max must never allow scheduling");
    let result = queue.try_schedule_repair();
    assert!(result.is_err(), "zero max must reject all repairs");
}
