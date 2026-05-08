use sha3::{Digest, Sha3_256};

/// Split data into `n` roughly equal chunks for testing.
/// Padding: last chunk may be shorter if data doesn't divide evenly.
pub fn chunk_bytes_for_test(data: &[u8], n: usize) -> Vec<Vec<u8>> {
    if n == 0 {
        return vec![];
    }
    let chunk_size = data.len().div_ceil(n);
    let mut chunks = Vec::with_capacity(n);
    for i in 0..n {
        let start = i * chunk_size;
        if start >= data.len() {
            chunks.push(vec![]);
        } else {
            let end = std::cmp::min(start + chunk_size, data.len());
            chunks.push(data[start..end].to_vec());
        }
    }
    chunks
}

/// Hash a single chunk via SHA3-256.
fn hash_leaf(chunk: &[u8]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    Digest::update(&mut hasher, chunk);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Hash a parent node = SHA3-256(left || right).
fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha3_256::new();
    Digest::update(&mut hasher, left);
    Digest::update(&mut hasher, right);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

/// Compute the Merkle root over ordered chunk hashes.
/// Source: artifact-availability-spec.md Section 1.2
pub fn compute_chunk_merkle_root(chunk_refs: &[&[u8]]) -> [u8; 32] {
    if chunk_refs.is_empty() {
        return [0u8; 32];
    }

    let mut level: Vec<[u8; 32]> = chunk_refs.iter().map(|c| hash_leaf(c)).collect();

    while level.len() > 1 {
        let mut next_level = Vec::with_capacity(level.len().div_ceil(2));
        for pair in level.chunks(2) {
            if pair.len() == 2 {
                next_level.push(hash_pair(&pair[0], &pair[1]));
            } else {
                next_level.push(hash_pair(&pair[0], &pair[0]));
            }
        }
        level = next_level;
    }

    level[0]
}

/// Generate a Merkle proof for a chunk at `chunk_index`.
/// Returns the list of sibling hashes from leaf to root.
pub fn merkle_proof_for_chunk(chunks: &[Vec<u8>], chunk_index: u32) -> Vec<[u8; 32]> {
    let idx = chunk_index as usize;
    if chunks.is_empty() || idx >= chunks.len() {
        return vec![];
    }

    let mut level: Vec<[u8; 32]> = chunks.iter().map(|c| hash_leaf(c.as_slice())).collect();
    let mut proof = Vec::new();

    let mut current_idx = idx;

    while level.len() > 1 {
        let sibling = if current_idx % 2 == 0 {
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
            if pair.len() == 2 {
                next_level.push(hash_pair(&pair[0], &pair[1]));
            } else {
                next_level.push(hash_pair(&pair[0], &pair[0]));
            }
        }
        level = next_level;
        current_idx /= 2;
    }

    proof
}

/// Verify a Merkle inclusion proof.
/// `leaf_hash` is the hash of the claimed chunk.
/// `chunk_index` is the chunk's position in the original ordered list.
/// `proof` is the sibling hashes from leaf to root.
/// `expected_root` is the Merkle root to verify against.
pub fn verify_merkle_proof(
    leaf_hash: &[u8; 32],
    chunk_index: u32,
    proof: &[[u8; 32]],
    expected_root: &[u8; 32],
) -> bool {
    let mut current_hash = *leaf_hash;
    let mut current_idx = chunk_index as usize;

    for sibling in proof {
        if current_idx % 2 == 0 {
            current_hash = hash_pair(&current_hash, sibling);
        } else {
            current_hash = hash_pair(sibling, &current_hash);
        }
        current_idx /= 2;
    }

    current_hash == *expected_root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_root_single_chunk() {
        let data = b"test_chunk_data";
        let root = compute_chunk_merkle_root(&[data.as_slice()]);
        let expected = hash_leaf(data);
        assert_eq!(root, expected);
    }

    #[test]
    fn merkle_root_two_chunks() {
        let chunks: Vec<&[u8]> = vec![b"chunk_a", b"chunk_b"];
        let root = compute_chunk_merkle_root(&chunks);
        let expected = hash_pair(&hash_leaf(b"chunk_a"), &hash_leaf(b"chunk_b"));
        assert_eq!(root, expected);
    }

    #[test]
    fn merkle_root_three_chunks() {
        let chunks: Vec<&[u8]> = vec![b"a", b"b", b"c"];
        let root = compute_chunk_merkle_root(&chunks);
        let ab = hash_pair(&hash_leaf(b"a"), &hash_leaf(b"b"));
        let cc = hash_pair(&hash_leaf(b"c"), &hash_leaf(b"c"));
        let expected = hash_pair(&ab, &cc);
        assert_eq!(root, expected);
    }

    #[test]
    fn proof_verifies_correctly() {
        let chunks: Vec<Vec<u8>> = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec(), b"d".to_vec()];
        let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
        let root = compute_chunk_merkle_root(&chunk_refs);

        for i in 0..chunks.len() {
            let proof = merkle_proof_for_chunk(&chunks, i as u32);
            let leaf = hash_leaf(&chunks[i]);
            assert!(
                verify_merkle_proof(&leaf, i as u32, &proof, &root),
                "proof must verify for chunk {}",
                i
            );
        }
    }

    #[test]
    fn proof_fails_for_wrong_chunk() {
        let chunks: Vec<Vec<u8>> = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec(), b"d".to_vec()];
        let chunk_refs: Vec<&[u8]> = chunks.iter().map(|c| c.as_slice()).collect();
        let root = compute_chunk_merkle_root(&chunk_refs);

        let proof = merkle_proof_for_chunk(&chunks, 0);
        let wrong_leaf = hash_leaf(b"wrong_data");
        assert!(!verify_merkle_proof(&wrong_leaf, 0, &proof, &root));
    }

    #[test]
    fn chunk_bytes_for_test_splits_correctly() {
        let data = b"ABCDEFGH";
        let chunks = chunk_bytes_for_test(data, 3);
        assert_eq!(chunks.len(), 3);
        let combined: Vec<u8> = chunks.iter().flat_map(|c| c.iter()).copied().collect();
        assert_eq!(combined, data.to_vec());
    }
}
