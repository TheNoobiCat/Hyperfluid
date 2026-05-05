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

        let mut proof_siblings = Vec::new();
        let mut sibling_is_left = Vec::new();
        let mut index = pos;

        while current_level.len() > 1 {
            let is_left = index % 2 == 0;
            let sibling = if is_left {
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
            // Record whether the sibling was on the LEFT (true) or RIGHT (false).
            // If current is right (odd), sibling is left. If current is left (even), sibling is right.
            sibling_is_left.push(!is_left);

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

        Some(InclusionProof { key: *key, value, proof: proof_siblings, sibling_is_left, root })
    }

    /// Verify an inclusion proof against a trusted root.
    pub fn verify_proof(proof: &InclusionProof, root: Hash32) -> bool {
        if proof.root != root {
            return false;
        }

        if proof.proof.len() != proof.sibling_is_left.len() {
            return false;
        }

        // Recompute leaf hash
        let mut hasher = Sha3_256::new();
        hasher.update(proof.key.encode());
        hasher.update(proof.value.encode());
        let mut current = [0u8; 32];
        current.copy_from_slice(&hasher.finalize());

        // Walk up the tree
        for (sibling, is_left) in proof.proof.iter().zip(proof.sibling_is_left.iter()) {
            let mut hasher = Sha3_256::new();
            if *is_left {
                // sibling was the left child, current was the right child
                hasher.update(sibling);
                hasher.update(current);
            } else {
                // current was the left child, sibling was the right child
                hasher.update(current);
                hasher.update(sibling);
            }
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
    /// true if the sibling at this level was the LEFT child (current was RIGHT).
    /// This is needed to reconstruct the correct parent hash ordering.
    pub sibling_is_left: Vec<bool>,
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

    #[test]
    fn multi_leaf_proof_verifies_at_even_and_odd_positions() {
        let mut tree = SparseMerkleTree::new();
        // Insert 5 leaves to exercise both even and odd proof positions at multiple levels
        let keys: [Hash32; 5] =
            [[0x01u8; 32], [0x02u8; 32], [0x03u8; 32], [0x04u8; 32], [0x05u8; 32]];
        for (i, k) in keys.iter().enumerate() {
            tree.insert(*k, vec![i as u8; 16]);
        }
        let root = tree.root();
        // Verify proof for EVERY key (ensures both even and odd positions work)
        for k in &keys {
            let proof = tree.prove(k).expect("proof must exist for inserted key");
            assert!(
                SparseMerkleTree::verify_proof(&proof, root),
                "proof verification failed for key at position {:?}",
                k
            );
        }
        // Verify that wrong value gives wrong root
        let mut tree2 = SparseMerkleTree::new();
        for (i, k) in keys.iter().enumerate() {
            tree2.insert(*k, vec![(i + 100) as u8; 16]); // different values
        }
        let root2 = tree2.root();
        assert_ne!(root, root2);
    }
}
