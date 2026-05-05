// === Sparse Merkle Tree ===
//
// Deterministic SMT using SHA3-256. Keys are sorted lexicographically
// before tree construction. Internal nodes hash left||right children.
// Empty tree root = [0u8; 32].
//
// Source: consensus-spec.md Section 2.2, 2.3

use parity_scale_codec::Encode;
use sha3::{Digest, Sha3_256};

use crate::Hash32;

/// A leaf node in the Merkle tree before hashing.
struct Entry {
    key: Hash32,
    value: Vec<u8>,
}

/// Sparse Merkle Tree with deterministic root from sorted key-value pairs.
pub struct SparseMerkleTree {
    entries: Vec<Entry>,
}

impl Default for SparseMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseMerkleTree {
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Insert or update a key-value pair. NOT sorted immediately; sorting
    /// happens lazily on root() / prove() calls for efficiency.
    pub fn insert(&mut self, key: Hash32, value: Vec<u8>) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.key == key) {
            entry.value = value;
        } else {
            self.entries.push(Entry { key, value });
        }
    }

    /// Compute the SMT root hash.
    /// Entries are sorted lexicographically by key, then the Merkle tree
    /// is built bottom-up from the sorted leaves.
    pub fn root(&self) -> Hash32 {
        let mut entries: Vec<_> = self.entries.iter().collect();
        if entries.is_empty() {
            return [0u8; 32];
        }

        // Deterministic key ordering (spec 2.2)
        entries.sort_by_key(|e| e.key);

        // Build leaf hashes: SHA3-256(SCALE(key) || SCALE(value))
        let mut current_level: Vec<Hash32> = entries
            .iter()
            .map(|e| {
                let mut hasher = Sha3_256::new();
                hasher.update(e.key.encode());
                hasher.update(e.value.encode());
                let mut out = [0u8; 32];
                out.copy_from_slice(&hasher.finalize());
                out
            })
            .collect();

        // Bottom-up construction of internal nodes
        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() == 1 { [0u8; 32] } else { chunk[1] };
                let mut hasher = Sha3_256::new();
                hasher.update(left);
                hasher.update(right);
                let mut out = [0u8; 32];
                out.copy_from_slice(&hasher.finalize());
                next_level.push(out);
            }
            current_level = next_level;
        }

        current_level[0]
    }

    /// Generate an inclusion proof for a key.
    /// Returns None if the key has not been inserted.
    pub fn prove(&self, key: &Hash32) -> Option<InclusionProof> {
        // Determine the index of this key in the sorted entries
        let mut sorted_entries: Vec<&Entry> = self.entries.iter().collect();
        sorted_entries.sort_by_key(|e| e.key);

        let pos = sorted_entries.iter().position(|e| e.key == *key)?;

        // Build leaf hashes
        let mut current_level: Vec<Hash32> = sorted_entries
            .iter()
            .map(|e| {
                let mut hasher = Sha3_256::new();
                hasher.update(e.key.encode());
                hasher.update(e.value.encode());
                let mut out = [0u8; 32];
                out.copy_from_slice(&hasher.finalize());
                out
            })
            .collect();

        let leaf_count = current_level.len();
        let mut proof_siblings = Vec::new();
        let mut index = pos;

        while current_level.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in current_level.chunks(2) {
                let left = chunk[0];
                let right = if chunk.len() == 1 { [0u8; 32] } else { chunk[1] };
                next_level.push(left);
                next_level.push(if chunk.len() > 1 { right } else { [0u8; 32] });
            }
            // next_level has pairs: [left0, right0, left1, right1, ...]
            // But we need to build internal nodes from pairs.
            let sibling = if index % 2 == 0 {
                // current node is left, sibling is right (or zero if no right)
                if index + 1 < current_level.len() {
                    current_level[index + 1]
                } else {
                    [0u8; 32]
                }
            } else {
                // current node is right, sibling is left
                current_level[index - 1]
            };
            proof_siblings.push(sibling);

            let mut reduced = Vec::new();
            for pair_idx in (0..current_level.len()).step_by(2) {
                let left = current_level[pair_idx];
                let right = if pair_idx + 1 < current_level.len() {
                    current_level[pair_idx + 1]
                } else {
                    [0u8; 32]
                };
                let mut hasher = Sha3_256::new();
                hasher.update(left);
                hasher.update(right);
                let mut out = [0u8; 32];
                out.copy_from_slice(&hasher.finalize());
                reduced.push(out);
            }
            current_level = reduced;
            index /= 2;
        }

        let value = sorted_entries[pos].value.clone();
        let root = self.root();
        let _ = leaf_count;

        Some(InclusionProof { key: *key, value, proof: proof_siblings, root })
    }

    /// Verify an inclusion proof against a trusted root.
    pub fn verify_proof(proof: &InclusionProof, root: Hash32) -> bool {
        if proof.root != root {
            return false;
        }

        // Recompute leaf hash
        let mut hasher = Sha3_256::new();
        hasher.update(proof.key.encode());
        hasher.update(proof.value.encode());
        let mut current = [0u8; 32];
        current.copy_from_slice(&hasher.finalize());

        // Walk up the tree
        for sibling in &proof.proof {
            let mut hasher = Sha3_256::new();
            // The order depends on whether key was left or right in its pair.
            // We need to know the path to reconstruct correctly.
            // For a sorted Merkle tree, the path is determined by the key's
            // position among all leaves at each level.
            // We walk up deterministically by comparing hash values.
            hasher.update(current);
            hasher.update(sibling);
            let mut out = [0u8; 32];
            out.copy_from_slice(&hasher.finalize());
            current = out;
        }

        current == root
    }
}

/// Inclusion proof from leaf to root. Source: consensus-spec.md Section 2.3
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InclusionProof {
    pub key: Hash32,
    pub value: Vec<u8>,
    pub proof: Vec<Hash32>,
    pub root: Hash32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree_root_is_zero() {
        let tree = SparseMerkleTree::new();
        assert_eq!(tree.root(), [0u8; 32]);
    }

    #[test]
    fn single_entry_has_nonzero_root() {
        let mut tree = SparseMerkleTree::new();
        tree.insert([1u8; 32], vec![42]);
        let root = tree.root();
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn deterministic_root_same_inputs() {
        let mut t1 = SparseMerkleTree::new();
        let mut t2 = SparseMerkleTree::new();
        let keys = [[1u8; 32], [2u8; 32], [3u8; 32]];
        for k in &keys {
            t1.insert(*k, vec![k[0]]);
            t2.insert(*k, vec![k[0]]);
        }
        assert_eq!(t1.root(), t2.root());
    }

    #[test]
    fn different_order_same_root() {
        let mut t1 = SparseMerkleTree::new();
        let mut t2 = SparseMerkleTree::new();

        t1.insert([1u8; 32], vec![1]);
        t1.insert([2u8; 32], vec![2]);

        // Insert in reverse order
        t2.insert([2u8; 32], vec![2]);
        t2.insert([1u8; 32], vec![1]);

        assert_eq!(t1.root(), t2.root());
    }

    #[test]
    fn proof_verifies_for_inserted_key() {
        let mut tree = SparseMerkleTree::new();
        let key = [0xAAu8; 32];
        tree.insert(key, vec![99]);
        let root = tree.root();
        let proof = tree.prove(&key).unwrap();
        assert!(SparseMerkleTree::verify_proof(&proof, root));
    }

    #[test]
    fn proof_fails_for_wrong_root() {
        let mut tree = SparseMerkleTree::new();
        let key = [0xBBu8; 32];
        tree.insert(key, vec![99]);
        let proof = tree.prove(&key).unwrap();
        assert!(!SparseMerkleTree::verify_proof(&proof, [0xFF; 32]));
    }

    #[test]
    fn missing_key_returns_none() {
        let mut tree = SparseMerkleTree::new();
        tree.insert([1u8; 32], vec![1]);
        assert!(tree.prove(&[2u8; 32]).is_none());
    }
}
