// === Sparse Merkle Tree ===
//
// True SMT with SHA3-256 hasher. Leaf position is determined by key bits
// (0 = left, 1 = right). Empty subtrees use zero hash. Supports inclusion
// and exclusion proofs.
//
// Wraps the nervosnetwork/sparse-merkle-tree crate internally.
// Source: consensus-spec.md Section 2.2, 2.3

use sha3::Digest;

use sparse_merkle_tree::{
    default_store::DefaultStore,
    traits::{Hasher, Value},
    MerkleProof, SparseMerkleTree as InnerSmt, H256,
};

use crate::Hash32;

// ── SHA3-256 Hasher ─────────────────────────────────────────────────

#[derive(Clone, Default)]
struct Sha3Hasher {
    state: sha3::Sha3_256,
}

impl Hasher for Sha3Hasher {
    fn write_h256(&mut self, h: &H256) {
        self.state.update(h.as_slice());
    }

    fn write_byte(&mut self, b: u8) {
        self.state.update([b]);
    }

    fn finish(self) -> H256 {
        let mut out = [0u8; 32];
        out.copy_from_slice(&self.state.finalize());
        H256::from(out)
    }
}

// ── Value wrapper for Vec<u8> ───────────────────────────────────────

#[derive(Default, Clone)]
struct SmValue(Vec<u8>);

fn hash_value(value: &[u8]) -> H256 {
    if value.is_empty() {
        return H256::zero();
    }
    let mut hasher = sha3::Sha3_256::new();
    hasher.update(value);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    H256::from(out)
}

impl Value for SmValue {
    fn to_h256(&self) -> H256 {
        hash_value(&self.0)
    }

    fn zero() -> Self {
        SmValue(vec![])
    }
}

// ── Type aliases ────────────────────────────────────────────────────

type SmtStore = DefaultStore<SmValue>;
type InnerTree = InnerSmt<Sha3Hasher, SmValue, SmtStore>;

// ── Public API ──────────────────────────────────────────────────────

/// Sparse Merkle Tree with SHA3-256 hasher.
pub struct SparseMerkleTree {
    inner: InnerTree,
}

impl Default for SparseMerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

impl SparseMerkleTree {
    pub fn new() -> Self {
        Self { inner: InnerTree::default() }
    }

    /// Insert or update a key-value pair.
    pub fn insert(&mut self, key: Hash32, value: Vec<u8>) {
        let hkey = H256::from(key);
        let hval = SmValue(value);
        debug_assert!(self.inner.update(hkey, hval).is_ok(), "SMT insert failed");
    }

    /// Compute the SMT root hash.
    pub fn root(&self) -> Hash32 {
        let mut out = [0u8; 32];
        out.copy_from_slice(self.inner.root().as_slice());
        out
    }

    /// Generate an inclusion proof for a key.
    /// Returns an exclusion proof if the key does not exist.
    pub fn prove(&self, key: &Hash32) -> Option<InclusionProof> {
        let hkey = H256::from(*key);

        let value = self.inner.get(&hkey).ok()?;
        let proof = self.inner.merkle_proof(vec![hkey]).ok()?;
        let root = self.root();

        Some(InclusionProof { key: *key, value: value.0, proof, root })
    }

    /// Verify an inclusion or exclusion proof against a trusted root.
    pub fn verify_proof(proof: &InclusionProof, root: Hash32) -> bool {
        if proof.root != root {
            return false;
        }
        let hkey = H256::from(proof.key);
        let hval_hash = hash_value(&proof.value);
        let hroot = H256::from(root);

        proof.proof.clone().verify::<Sha3Hasher>(&hroot, vec![(hkey, hval_hash)]).unwrap_or(false)
    }
}

/// Inclusion or exclusion proof for a single key.
#[derive(Debug, Clone)]
pub struct InclusionProof {
    pub key: Hash32,
    pub value: Vec<u8>,
    pub root: Hash32,
    proof: MerkleProof,
}

impl PartialEq for InclusionProof {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.value == other.value && self.root == other.root
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_tree_root_is_deterministic() {
        let tree = SparseMerkleTree::new();
        assert_eq!(tree.root(), tree.root());
    }

    #[test]
    fn single_entry_has_deterministic_root() {
        let mut tree = SparseMerkleTree::new();
        tree.insert([1u8; 32], vec![42]);
        let root = tree.root();
        assert_eq!(root, tree.root());
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
    fn missing_key_returns_exclusion_proof() {
        let mut tree = SparseMerkleTree::new();
        tree.insert([1u8; 32], vec![1]);
        let root = tree.root();

        let missing = [2u8; 32];
        let proof = tree.prove(&missing);
        assert!(proof.is_some(), "true SMT must produce exclusion proof for missing key");

        let proof = proof.unwrap();
        assert_eq!(proof.key, missing);
        assert!(proof.value.is_empty(), "exclusion proof must have empty value");
        assert!(
            SparseMerkleTree::verify_proof(&proof, root),
            "exclusion proof must verify against root"
        );
    }

    #[test]
    fn multi_leaf_proof_verifies() {
        let mut tree = SparseMerkleTree::new();
        let keys: [Hash32; 5] =
            [[0x01u8; 32], [0x02u8; 32], [0x03u8; 32], [0x04u8; 32], [0x05u8; 32]];
        for (i, k) in keys.iter().enumerate() {
            tree.insert(*k, vec![i as u8; 16]);
        }
        let root = tree.root();
        for k in &keys {
            let proof = tree.prove(k).expect("proof must exist for inserted key");
            assert!(
                SparseMerkleTree::verify_proof(&proof, root),
                "proof verification failed for key {:?}",
                k
            );
        }
        let mut tree2 = SparseMerkleTree::new();
        for (i, k) in keys.iter().enumerate() {
            tree2.insert(*k, vec![(i + 100) as u8; 16]);
        }
        let root2 = tree2.root();
        assert_ne!(root, root2);
    }
}
