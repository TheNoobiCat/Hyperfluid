// === State Sync Protocol ===
//
// Source: docs/04-specifications/storage/state-sync-spec.md

use parity_scale_codec::Encode;
use sha3::{Digest, Sha3_256};

use crate::smt::SparseMerkleTree;
use crate::state_machine::StateMachine;
use crate::Hash32;

/// Checkpoint snapshot of state at a given height. Source: state-sync-spec.md Section 1.3
pub struct Snapshot {
    pub epoch: u64,
    pub height: u64,
    pub state_root: Hash32,
    pub block_hash: Hash32,
    pub sst_keys: Vec<(Hash32, Vec<u8>)>,
    pub merkle_proof_batch: Vec<crate::smt::InclusionProof>,
}

/// Synchronization mode. Source: state-sync-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    Full,
    Snap,
    CatchUp,
}

/// State sync progress tracker. Source: state-sync-spec.md Section 1.3
#[derive(Debug, Clone)]
pub struct SyncState {
    pub mode: SyncMode,
    pub current_height: u64,
    pub target_height: u64,
    pub validated_roots: u32,
    pub last_validated_block: Hash32,
}

/// Capture a state snapshot from a StateMachine at a given height.
///
/// Serialises all accounts into the snapshot's `sst_keys` using SCALE encoding.
/// Builds an SMT from the accounts and commits the root.
pub fn snapshot_state(sm: &StateMachine, epoch: u64, height: u64, block_hash: Hash32) -> Snapshot {
    let mut smt = SparseMerkleTree::new();
    let mut sst_keys = Vec::new();

    for (account_id, account) in sm.accounts_iter() {
        let key = crate::state_key(crate::KeyPrefix::Account, account_id);
        let value = account.encode();
        smt.insert(key, value.clone());
        sst_keys.push((key, value));
    }

    let state_root = smt.root();

    Snapshot { epoch, height, state_root, block_hash, sst_keys, merkle_proof_batch: Vec::new() }
}

/// Rebuild an SMT root from a set of (key, value) pairs and return the root.
pub fn build_smt_from_keys(keys: &[(Hash32, Vec<u8>)]) -> Hash32 {
    let mut smt = SparseMerkleTree::new();
    for (key, value) in keys {
        smt.insert(*key, value.clone());
    }
    smt.root()
}

/// Verify that a state root matches the quorum of peer-reported roots.
/// Returns true if at least `min_matching` peers report a root matching `expected_root`.
pub fn verify_state_root_quorum(
    expected_root: Hash32,
    peer_roots: &[Hash32],
    min_matching: u32,
) -> bool {
    let matches = peer_roots.iter().filter(|r| **r == expected_root).count() as u32;
    matches >= min_matching
}

/// Compute a checksum over snapshot keys for integrity verification.
/// Uses SHA3-256 over: count, key, value for each tuple.
pub fn compute_state_checksum(keys: &[(Hash32, Vec<u8>)]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    Digest::update(&mut hasher, (keys.len() as u64).to_le_bytes());
    for (key, value) in keys {
        Digest::update(&mut hasher, key);
        Digest::update(&mut hasher, (value.len() as u64).to_le_bytes());
        Digest::update(&mut hasher, value);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Verify that a set of snapshot keys matches a given checksum.
pub fn verify_snapshot_checksum(keys: &[(Hash32, Vec<u8>)], expected_checksum: Hash32) -> bool {
    compute_state_checksum(keys) == expected_checksum
}
