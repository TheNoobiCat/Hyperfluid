// Conformance tests for consensus-spec.md Section 1.7 (Committee BFT)
//
// Source: docs/04-specifications/protocol/consensus-spec.md Section 1.7

use hyperfluid_consensus::types::{BlockHeader, Committee, CommitteeMode, Hash32};

fn make_header(height: u64, parent_hash: Hash32, state_root: Hash32) -> BlockHeader {
    BlockHeader {
        height,
        parent_hash,
        state_root,
        transaction_root: [0u8; 32],
        committee_id: 0,
        proposer_id: [0u8; 32],
        timestamp: height * 10,
        epoch: height / 8192,
    }
}

#[test]
fn conforms_to_consensus_spec_1_7_block_hash_deterministic() {
    let h1 = make_header(42, [1u8; 32], [2u8; 32]);
    let h2 = make_header(42, [1u8; 32], [2u8; 32]);

    let hash1 = h1.block_hash();
    let hash2 = h2.block_hash();
    assert_eq!(hash1, hash2);
    assert_ne!(hash1, [0u8; 32]);
}

#[test]
fn conforms_to_consensus_spec_1_7_block_hash_changes_with_data() {
    let h1 = make_header(42, [1u8; 32], [2u8; 32]);
    let h2 = make_header(43, [1u8; 32], [2u8; 32]);

    assert_ne!(h1.block_hash(), h2.block_hash());
}

#[test]
fn conforms_to_consensus_spec_1_7_committee_deterministic_sampling() {
    let seed = [0x42u8; 32];
    let validator_ids: Vec<Hash32> = (0..150u8).map(|i| [i; 32]).collect();
    let stakes: Vec<u128> = validator_ids.iter().map(|_| 1000u128).collect();

    let committee1 = Committee::sample(1, seed, &validator_ids, &stakes, 100, &[]);
    let committee2 = Committee::sample(1, seed, &validator_ids, &stakes, 100, &[]);

    assert_eq!(committee1.members, committee2.members);
    assert_eq!(committee1.weights, committee2.weights);
    assert_eq!(committee1.members.len(), 100);
}

#[test]
fn conforms_to_consensus_spec_1_7_committee_size_is_exactly_100() {
    let seed = [1u8; 32];
    let validator_ids: Vec<Hash32> = (0..200u8).map(|i| [i; 32]).collect();
    let stakes: Vec<u128> = vec![1000u128; 200];

    let committee = Committee::sample(0, seed, &validator_ids, &stakes, 100, &[]);
    assert_eq!(committee.members.len(), 100);
}

#[test]
fn conforms_to_consensus_spec_1_7_committee_no_duplicate_members() {
    let seed = [2u8; 32];
    let validator_ids: Vec<Hash32> = (0..150u8).map(|i| [i; 32]).collect();
    let stakes: Vec<u128> = vec![1000u128; 150];

    let committee = Committee::sample(0, seed, &validator_ids, &stakes, 100, &[]);
    let mut seen = std::collections::HashSet::new();
    for member in &committee.members {
        assert!(seen.insert(member), "duplicate committee member");
    }
}

#[test]
fn conforms_to_consensus_spec_1_7_rotation_max_overlap_20_percent() {
    let seed1 = [1u8; 32];
    let seed2 = [2u8; 32];
    let validator_ids: Vec<Hash32> = (0..200u8).map(|i| [i; 32]).collect();
    let stakes: Vec<u128> = vec![1000u128; 200];

    let c1 = Committee::sample(1, seed1, &validator_ids, &stakes, 100, &[]);
    let c2 =
        Committee::sample_with_rotation(2, seed2, &validator_ids, &stakes, 100, &c1.members, &[]);

    let set1: std::collections::HashSet<_> = c1.members.iter().collect();
    let overlap = c2.members.iter().filter(|m| set1.contains(m)).count();
    assert!(overlap <= 20, "committee overlap {} > 20 (max 20% for 100-seat committee)", overlap);
}

#[test]
fn conforms_to_consensus_spec_1_7_rotation_max_overlap_with_small_set() {
    // With 150 validators and 100 seats at 20% overlap, only 20 overlap + 50 new
    // = 70 max; 100 required. Use 180 validators minimum.
    let seed1 = [5u8; 32];
    let seed2 = [6u8; 32];
    let validator_ids: Vec<Hash32> = (0..250u8).map(|i| [i; 32]).collect();
    let stakes: Vec<u128> = vec![1000u128; 250];

    let c1 = Committee::sample(1, seed1, &validator_ids, &stakes, 100, &[]);
    let c2 =
        Committee::sample_with_rotation(2, seed2, &validator_ids, &stakes, 100, &c1.members, &[]);

    let set1: std::collections::HashSet<_> = c1.members.iter().collect();
    let overlap = c2.members.iter().filter(|m| set1.contains(m)).count();
    assert!(overlap <= 20, "committee overlap {} > 20 with 250 validators", overlap);
}

#[test]
fn conforms_to_consensus_spec_1_7_two_epoch_recency_guard() {
    let seed1 = [10u8; 32];
    let seed2 = [11u8; 32];
    let seed3 = [12u8; 32];
    let validator_ids: Vec<Hash32> = (0..200u8).map(|i| [i; 32]).collect();
    let stakes: Vec<u128> = vec![1000u128; 200];

    let c1 = Committee::sample(1, seed1, &validator_ids, &stakes, 100, &[]);
    let c2 =
        Committee::sample_with_rotation(2, seed2, &validator_ids, &stakes, 100, &c1.members, &[]);

    let ineligible: Vec<Hash32> = c1.members.to_vec();
    let c3 = Committee::sample_with_rotation(
        3,
        seed3,
        &validator_ids,
        &stakes,
        100,
        &c2.members,
        &ineligible,
    );

    for member in &c3.members {
        assert!(
            !ineligible.contains(member),
            "validator {:?} served 2 consecutive epochs but was still selected for epoch 3",
            &member[..4]
        );
    }
}

#[test]
fn conforms_to_consensus_spec_1_7_two_epoch_recency_edge_case() {
    // With 250 validators and 100 ineligible for recency, there are 150
    // eligible candidates. The two-epoch guard should prevent all 100
    // ineligible from being selected.
    let seed1 = [20u8; 32];
    let seed2 = [21u8; 32];
    let validator_ids: Vec<Hash32> = (0..250u8).map(|i| [i; 32]).collect();
    let stakes: Vec<u128> = vec![1000u128; 250];

    let c1 = Committee::sample(1, seed1, &validator_ids, &stakes, 100, &[]);
    let c2 =
        Committee::sample_with_rotation(2, seed2, &validator_ids, &stakes, 100, &c1.members, &[]);

    let ineligible: Vec<Hash32> = c1.members.to_vec();
    let c3 = Committee::sample_with_rotation(
        3,
        seed2,
        &validator_ids,
        &stakes,
        100,
        &c2.members,
        &ineligible,
    );

    for member in &c3.members {
        assert!(!ineligible.contains(member));
    }
}

#[test]
fn conforms_to_consensus_spec_1_7_committee_three_tier_stall() {
    // Full-size pool (100 total): raw thresholds apply
    assert_eq!(Committee::committee_mode(100, 100), CommitteeMode::Normal);
    assert!(Committee::can_produce(100, 100));

    // 67 active of 100 total → Normal
    assert_eq!(Committee::committee_mode(67, 100), CommitteeMode::Normal);
    assert!(Committee::can_produce(67, 100));

    // Bootstrap: 10 total validators, scaled degraded = ceil(50*10/100) = 5
    assert_eq!(Committee::committee_mode(10, 10), CommitteeMode::Normal);
    assert!(Committee::can_produce(10, 10));
    assert!(Committee::can_produce(5, 10));
    assert!(!Committee::can_produce(4, 10));

    // Bootstrap: 3 total, scaled normal=3, degraded=2
    assert_eq!(Committee::committee_mode(3, 3), CommitteeMode::Normal);
    assert_eq!(Committee::committee_mode(2, 3), CommitteeMode::Degraded);
    assert_eq!(Committee::committee_mode(1, 3), CommitteeMode::Emergency);

    // Bootstrap: 1 total → scaled normal=1, degraded=1
    assert_eq!(Committee::committee_mode(1, 1), CommitteeMode::Normal);
    assert!(Committee::can_produce(1, 1));

    // Zero active → Emergency regardless of pool size
    assert_eq!(Committee::committee_mode(0, 10), CommitteeMode::Emergency);
    assert!(!Committee::can_produce(0, 10));
}

#[test]
fn conforms_to_consensus_spec_1_7_emergency_idle_blocks_constants() {
    assert_eq!(Committee::NORMAL_THRESHOLD, 67);
    assert_eq!(Committee::DEGRADED_THRESHOLD, 50);
    assert_eq!(Committee::EMERGENCY_IDLE_BLOCKS, 500);
}

#[test]
fn conforms_to_consensus_spec_1_7_vdf_fallback_deterministic() {
    let previous_vdf_output = [0xAAu8; 32];
    let epoch_headers_hash = [0xBBu8; 32];
    let epoch_number = 42u64;
    let valid_reveals: Vec<Hash32> = vec![[0x11u8; 32], [0x22u8; 32]];

    let hash1 = Committee::compute_vdf_fallback(
        &previous_vdf_output,
        &epoch_headers_hash,
        epoch_number,
        &valid_reveals,
    );
    let hash2 = Committee::compute_vdf_fallback(
        &previous_vdf_output,
        &epoch_headers_hash,
        epoch_number,
        &valid_reveals,
    );
    assert_eq!(hash1, hash2);
    assert_ne!(hash1, [0u8; 32]);
}

#[test]
fn conforms_to_consensus_spec_1_7_vdf_fallback_changes_with_input() {
    let previous_vdf_output = [0xAAu8; 32];
    let epoch_headers_hash = [0xBBu8; 32];
    let valid_reveals: Vec<Hash32> = vec![[0x11u8; 32]];

    let hash1 = Committee::compute_vdf_fallback(
        &previous_vdf_output,
        &epoch_headers_hash,
        42,
        &valid_reveals,
    );
    let hash2 = Committee::compute_vdf_fallback(
        &previous_vdf_output,
        &epoch_headers_hash,
        43,
        &valid_reveals,
    );
    assert_ne!(hash1, hash2);
}

#[test]
fn conforms_to_consensus_spec_1_7_vdf_fallback_empty_reveals() {
    let previous_vdf_output = [0xAAu8; 32];
    let epoch_headers_hash = [0xBBu8; 32];
    let valid_reveals: Vec<Hash32> = vec![];

    let hash = Committee::compute_vdf_fallback(
        &previous_vdf_output,
        &epoch_headers_hash,
        1,
        &valid_reveals,
    );
    assert_ne!(hash, [0u8; 32]);
}

#[test]
fn conforms_to_consensus_spec_1_7_emergency_transition() {
    let seed = [0xDEu8; 32];
    let validator_ids: Vec<Hash32> = (0..200u8).map(|i| [i; 32]).collect();
    let stakes: Vec<u128> = vec![1000u128; 200];

    let committee = Committee::emergency_transition(5, seed, &validator_ids, &stakes);
    assert_eq!(committee.members.len(), 100);
    assert_eq!(committee.epoch, 5);
    assert_eq!(committee.seed, seed);
}
