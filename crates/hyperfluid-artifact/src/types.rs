/// Artifact class defining storage requirements. Source: artifact-availability-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ArtifactClass {
    GovernanceBundle = 0,
    ReviewEvidence = 1,
    ResearchOutput = 2,
    TelemetryArchive = 3,
}

impl ArtifactClass {
    /// Discriminant as u8 for serialization/hashing.
    pub fn discriminant(self) -> u8 {
        self as u8
    }

    /// Default minimum replica count per artifact class. Source: spec Section 1.4 table.
    pub fn default_min_replicas(self) -> u8 {
        match self {
            ArtifactClass::GovernanceBundle => 5,
            ArtifactClass::ReviewEvidence => 3,
            ArtifactClass::ResearchOutput => 2,
            ArtifactClass::TelemetryArchive => 2,
        }
    }

    /// Default priority for repair queue (lower = higher priority).
    pub fn repair_priority(self) -> u8 {
        match self {
            ArtifactClass::GovernanceBundle => 0,
            ArtifactClass::ReviewEvidence => 1,
            ArtifactClass::ResearchOutput => 2,
            ArtifactClass::TelemetryArchive => 3,
        }
    }
}

/// Retention tier determining expiry behavior. Source: artifact-availability-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RetentionTier {
    Pinned = 0,
    MediumTerm = 1,
    ShortTerm = 2,
}

impl RetentionTier {
    pub fn discriminant(self) -> u8 {
        self as u8
    }
}

/// Storage manifest registered in protocol state. Source: artifact-availability-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactManifest {
    pub artifact_root_hash: [u8; 32],
    pub chunk_root_hash: [u8; 32],
    pub size_bytes: u64,
    pub chunk_count: u32,
    pub class: ArtifactClass,
    pub retention_tier: RetentionTier,
    pub min_replica_count: u8,
    pub created_at_height: u64,
    pub expires_at_height: u64,
    pub producer_signature: Vec<u8>,
}

impl ArtifactManifest {
    /// Determine if the artifact is expired at the given block height.
    pub fn is_expired(&self, current_height: u64) -> bool {
        match self.retention_tier {
            RetentionTier::Pinned => false,
            _ => {
                if self.expires_at_height == 0 {
                    return false;
                }
                current_height >= self.expires_at_height
            }
        }
    }
}

/// Replication lease binding a provider to store an artifact. Source: spec Section 1.3
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplicationLease {
    pub lease_id: [u8; 32],
    pub artifact_root_hash: [u8; 32],
    pub provider_id: [u8; 32],
    pub lease_start_height: u64,
    pub lease_end_height: u64,
    pub challenge_cadence: u16,
    pub collateral: u128,
    pub status: LeaseStatus,
}

/// Lease status states. Source: spec Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseStatus {
    Active,
    AtRisk,
    Expired,
}

/// Proof-of-possession challenge response. Source: spec Section 1.3
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProofOfPossession {
    pub lease_id: [u8; 32],
    pub chunk_index: u32,
    pub chunk_bytes: Vec<u8>,
    pub merkle_proof: Vec<[u8; 32]>,
    pub lease_signature: Vec<u8>,
    pub response_height: u64,
}

impl ProofOfPossession {
    /// Build a proof-of-possession for the given chunk.
    /// Verifies the Merkle proof against the expected chunk root before returning.
    /// Returns None if the chunk is missing or the proof does not verify against chunk_root_hash.
    pub fn build(
        chunks: &[Vec<u8>],
        chunk_index: u32,
        chunk_root_hash: [u8; 32],
        lease_id: [u8; 32],
        response_height: u64,
    ) -> Option<Self> {
        let chunk_bytes = chunks.get(chunk_index as usize).cloned()?;
        let merkle_proof = crate::chunks::merkle_proof_for_chunk(chunks, chunk_index);
        let leaf_hash = crate::chunks::hash_leaf(&chunk_bytes);
        if !crate::chunks::verify_merkle_proof(
            &leaf_hash,
            chunk_index,
            &merkle_proof,
            &chunk_root_hash,
        ) {
            return None;
        }
        Some(Self {
            lease_id,
            chunk_index,
            chunk_bytes,
            merkle_proof,
            lease_signature: vec![],
            response_height,
        })
    }
}

/// Repair queue entry. Source: spec Section 1.3
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairEntry {
    pub artifact_root_hash: [u8; 32],
    pub artifact_class: ArtifactClass,
    pub current_replica_count: u8,
    pub target_replica_count: u8,
    pub priority: u8, // 0 = highest
    pub entered_at_height: u64,
}

/// Repair queue for missing artifacts. Source: spec Section 1.3
#[derive(Debug, Clone)]
pub struct RepairQueue {
    entries: Vec<RepairEntry>,
    #[allow(dead_code)]
    max_concurrent: u8,
}

impl RepairQueue {
    pub fn new(max_concurrent: u8) -> Self {
        Self { entries: Vec::new(), max_concurrent }
    }

    /// Push a repair entry.
    pub fn push(&mut self, entry: RepairEntry) {
        self.entries.push(entry);
    }

    /// Pop the highest-priority entry (lowest priority number).
    /// Ties broken by entered_at_height (oldest first).
    pub fn pop_highest(&mut self) -> Option<RepairEntry> {
        if self.entries.is_empty() {
            return None;
        }
        // Sort by (artifact_class.repair_priority, priority, entered_at_height)
        self.entries
            .sort_by_key(|e| (e.artifact_class.repair_priority(), e.priority, e.entered_at_height));
        Some(self.entries.remove(0))
    }
}
