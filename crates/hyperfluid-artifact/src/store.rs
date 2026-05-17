//! Disk-backed content-addressed storage for artifact chunks.
//!
//! Implements on-disk persistence with SHA3-256 content verification
//! on both write and read paths. Content-addressed directory fanout
//! prevents too many files in a single directory.
//!
//! Source: docs/04-specifications/storage/artifact-availability-spec.md Section 1.2

use sha3::{Digest, Sha3_256};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// StoreError
// ---------------------------------------------------------------------------

/// Errors that can occur during storage operations.
#[derive(Debug)]
pub enum StoreError {
    /// An I/O error from the underlying filesystem.
    Io(io::Error),
    /// The hash of stored/read data did not match the expected hash.
    HashMismatch { expected: [u8; 32], actual: [u8; 32] },
    /// The requested chunk was not found on disk.
    NotFound,
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StoreError::Io(e) => write!(f, "store I/O error: {}", e),
            StoreError::HashMismatch { expected, actual } => {
                write!(
                    f,
                    "hash mismatch: expected {}, got {}",
                    hex::encode(expected),
                    hex::encode(actual)
                )
            }
            StoreError::NotFound => write!(f, "chunk not found on disk"),
        }
    }
}

impl std::error::Error for StoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            StoreError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for StoreError {
    fn from(e: io::Error) -> Self {
        StoreError::Io(e)
    }
}

// ---------------------------------------------------------------------------
// StoreConfig
// ---------------------------------------------------------------------------

/// Configuration for the on-disk artifact store.
#[derive(Debug, Clone)]
pub struct StoreConfig {
    /// Root directory for chunk storage.
    pub storage_dir: PathBuf,
}

impl Default for StoreConfig {
    fn default() -> Self {
        StoreConfig { storage_dir: PathBuf::from("./data/chunks") }
    }
}

// ---------------------------------------------------------------------------
// Path construction
// ---------------------------------------------------------------------------

/// Compute the content-addressed path for a chunk.
///
/// Directory fanout: `{storage_dir}/{hash[0]}/{hash[0..1]}/{full_hex}`
///
/// - Level 1: first byte in hex (2 chars)
/// - Level 2: first two bytes in hex (4 chars)
/// - Leaf: full 32-byte hash in hex (64 chars)
pub fn chunk_path(storage_dir: &Path, hash: &[u8; 32]) -> PathBuf {
    let hex_full = hex::encode(hash);
    let level1 = &hex_full[0..2]; // hash[0]
    let level2 = &hex_full[0..4]; // hash[0..1]
    storage_dir.join(level1).join(level2).join(hex_full)
}

// ---------------------------------------------------------------------------
// Store / Load / Query operations
// ---------------------------------------------------------------------------

/// Write a chunk to disk at its content-addressed path.
///
/// # Integrity
/// Computes SHA3-256 of `data` and verifies it matches `hash` *before*
/// writing to disk (content-addressed verification per Integration Gate).
///
/// Parent directories are created automatically if they do not exist.
pub fn store_chunk(config: &StoreConfig, hash: &[u8; 32], data: &[u8]) -> Result<(), StoreError> {
    // 1. Compute the hash of the data we were given
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let computed_hash: [u8; 32] = hasher.finalize().into();

    // 2. Verify it matches the expected content hash
    if computed_hash != *hash {
        return Err(StoreError::HashMismatch { expected: *hash, actual: computed_hash });
    }

    // 3. Build the content-addressed path
    let path = chunk_path(&config.storage_dir, hash);

    // 4. Create parent directories
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 5. Write to disk
    fs::write(&path, data)?;

    Ok(())
}

/// Load a chunk from disk by its content hash.
///
/// # Integrity
/// After reading from disk, the SHA3-256 of the read bytes is computed
/// and verified against the expected `hash`. Returns `HashMismatch` if
/// on-disk data has been corrupted.
pub fn load_chunk(config: &StoreConfig, hash: &[u8; 32]) -> Result<Vec<u8>, StoreError> {
    let path = chunk_path(&config.storage_dir, hash);

    // 1. Read from disk
    let data = match fs::read(&path) {
        Ok(d) => d,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            return Err(StoreError::NotFound);
        }
        Err(e) => return Err(StoreError::Io(e)),
    };

    // 2. Verify integrity (Integration Gate: hash on read)
    let mut hasher = Sha3_256::new();
    hasher.update(&data);
    let computed_hash: [u8; 32] = hasher.finalize().into();

    if computed_hash != *hash {
        return Err(StoreError::HashMismatch { expected: *hash, actual: computed_hash });
    }

    Ok(data)
}

/// Return `true` if a chunk with the given hash exists on disk.
pub fn chunk_exists(config: &StoreConfig, hash: &[u8; 32]) -> bool {
    let path = chunk_path(&config.storage_dir, hash);
    path.is_file()
}

/// Delete a chunk from disk by its content hash.
///
/// Returns `NotFound` if the chunk does not exist.
/// Does *not* clean up empty parent directories.
pub fn delete_chunk(config: &StoreConfig, hash: &[u8; 32]) -> Result<(), StoreError> {
    let path = chunk_path(&config.storage_dir, hash);

    match fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Err(StoreError::NotFound),
        Err(e) => Err(StoreError::Io(e)),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: compute SHA3-256 of arbitrary bytes.
    fn sha3_256(data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha3_256::new();
        hasher.update(data);
        hasher.finalize().into()
    }

    /// Helper: create a StoreConfig pointed at a temp directory.
    fn temp_config() -> (StoreConfig, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("create temp dir");
        let config = StoreConfig { storage_dir: dir.path().join("chunks") };
        (config, dir)
    }

    // ------------------------------------------------------------------
    // test_store_and_load_chunk
    // ------------------------------------------------------------------

    #[test]
    fn test_store_and_load_chunk() {
        let (config, _dir) = temp_config();

        let data1 = b"chunk data number one - hello world!";
        let data2 = b"another chunk of data, completely different";
        let data3 = b"third chunk, short";

        let hash1 = sha3_256(data1);
        let hash2 = sha3_256(data2);
        let hash3 = sha3_256(data3);

        // Store all three
        store_chunk(&config, &hash1, data1).expect("store chunk 1");
        store_chunk(&config, &hash2, data2).expect("store chunk 2");
        store_chunk(&config, &hash3, data3).expect("store chunk 3");

        // Verify they exist
        assert!(chunk_exists(&config, &hash1));
        assert!(chunk_exists(&config, &hash2));
        assert!(chunk_exists(&config, &hash3));

        // Load them back
        let loaded1 = load_chunk(&config, &hash1).expect("load chunk 1");
        let loaded2 = load_chunk(&config, &hash2).expect("load chunk 2");
        let loaded3 = load_chunk(&config, &hash3).expect("load chunk 3");

        assert_eq!(loaded1, data1);
        assert_eq!(loaded2, data2);
        assert_eq!(loaded3, data3);
    }

    // ------------------------------------------------------------------
    // test_store_chunk_hash_mismatch
    // ------------------------------------------------------------------

    #[test]
    fn test_store_chunk_hash_mismatch() {
        let (config, _dir) = temp_config();

        let data = b"actual data being stored";
        let wrong_hash = sha3_256(b"different data");

        let result = store_chunk(&config, &wrong_hash, data);
        match result {
            Err(StoreError::HashMismatch { .. }) => {} // expected
            other => panic!("expected HashMismatch, got {:?}", other),
        }

        // Verify nothing was written
        let real_hash = sha3_256(data);
        assert!(!chunk_exists(&config, &real_hash));
    }

    // ------------------------------------------------------------------
    // test_load_chunk_not_found
    // ------------------------------------------------------------------

    #[test]
    fn test_load_chunk_not_found() {
        let (config, _dir) = temp_config();

        let non_existent_hash = sha3_256(b"no such chunk exists");

        let result = load_chunk(&config, &non_existent_hash);
        match result {
            Err(StoreError::NotFound) => {} // expected
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // test_store_chunk_overwrite
    // ------------------------------------------------------------------

    #[test]
    fn test_store_chunk_overwrite() {
        let (config, _dir) = temp_config();

        let data_v1 = b"version one of the chunk content";
        let data_v2 = b"version two - different bytes";

        let hash1 = sha3_256(data_v1);
        let hash2 = sha3_256(data_v2);

        // Store first version
        store_chunk(&config, &hash1, data_v1).expect("store v1");

        // Now store something different at the SAME hash — this is impossible
        // for different content (different hash), so we test overwriting the
        // SAME content under the SAME hash.
        store_chunk(&config, &hash1, data_v1).expect("store same content again");

        let loaded = load_chunk(&config, &hash1).expect("load after overwrite");
        assert_eq!(loaded, data_v1);

        // Store under a different hash — should create a new file
        store_chunk(&config, &hash2, data_v2).expect("store different hash");

        // Both should exist independently
        assert!(chunk_exists(&config, &hash1));
        assert!(chunk_exists(&config, &hash2));

        assert_eq!(load_chunk(&config, &hash1).unwrap(), data_v1);
        assert_eq!(load_chunk(&config, &hash2).unwrap(), data_v2);
    }

    // ------------------------------------------------------------------
    // test_chunk_content_addressing_verification
    // ------------------------------------------------------------------

    #[test]
    fn test_chunk_content_addressing_verification() {
        let (config, dir) = temp_config();
        let storage_dir = dir.path().join("chunks");

        let data = b"persistence and integrity verification test data";
        let hash = sha3_256(data);

        // Write
        store_chunk(&config, &hash, data).expect("write chunk");

        // Simulate restart: drop the config, create a fresh one pointing at
        // the same storage directory, and read back.
        drop(config);

        let config2 = StoreConfig { storage_dir };

        // Read back
        let loaded = load_chunk(&config2, &hash).expect("read after restart");
        assert_eq!(loaded, data);

        // Verify hash of loaded data matches
        let recomputed = sha3_256(&loaded);
        assert_eq!(recomputed, hash);
    }

    // ------------------------------------------------------------------
    // test_chunk_path_structure
    // ------------------------------------------------------------------

    #[test]
    fn test_chunk_path_structure() {
        let hash = sha3_256(b"path test");
        let storage_dir = Path::new("/store");
        let path = chunk_path(storage_dir, &hash);

        let hex_full = hex::encode(hash);
        let expected = storage_dir.join(&hex_full[0..2]).join(&hex_full[0..4]).join(&hex_full);

        assert_eq!(path, expected);
    }

    // ------------------------------------------------------------------
    // test_delete_chunk
    // ------------------------------------------------------------------

    #[test]
    fn test_delete_chunk() {
        let (config, _dir) = temp_config();

        let data = b"delete me";
        let hash = sha3_256(data);

        store_chunk(&config, &hash, data).expect("store");
        assert!(chunk_exists(&config, &hash));

        delete_chunk(&config, &hash).expect("delete");
        assert!(!chunk_exists(&config, &hash));
    }

    // ------------------------------------------------------------------
    // test_delete_chunk_not_found
    // ------------------------------------------------------------------

    #[test]
    fn test_delete_chunk_not_found() {
        let (config, _dir) = temp_config();

        let hash = sha3_256(b"ghost chunk");
        let result = delete_chunk(&config, &hash);
        match result {
            Err(StoreError::NotFound) => {} // expected
            other => panic!("expected NotFound, got {:?}", other),
        }
    }

    // ------------------------------------------------------------------
    // test_hash_mismatch_on_load__corrupted_file
    // ------------------------------------------------------------------

    #[test]
    fn test_hash_mismatch_on_load_corrupted_file() {
        let (config, _dir) = temp_config();

        let data = b"original pristine content";
        let hash = sha3_256(data);

        store_chunk(&config, &hash, data).expect("store");

        // Corrupt the file on disk by overwriting with different content
        // using a DIFFERENT hash so the content-addressed path is new.
        // We can't easily corrupt at the same path since the path IS the hash.
        // Instead, we write garbage to a path that would match the hash
        // but has different content — which is possible only by writing
        // raw bytes directly.
        //
        // Actually, to simulate disk corruption we overwrite the file at
        // the content-addressed path with corrupted bytes.
        let path = chunk_path(&config.storage_dir, &hash);
        std::fs::write(&path, b"corrupted!!!").expect("overwrite with corruption");

        let result = load_chunk(&config, &hash);
        match result {
            Err(StoreError::HashMismatch { .. }) => {} // expected
            other => panic!("expected HashMismatch on corrupted load, got {:?}", other),
        }
    }
}
