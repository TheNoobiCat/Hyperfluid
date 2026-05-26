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
            // Pinned artifacts never expire — they are retained indefinitely.
            RetentionTier::Pinned => false,
            // MediumTerm and ShortTerm expire at expires_at_height.
            // If expires_at_height is 0 (sentinel for "no expiry set"), treat as not expired.
            RetentionTier::MediumTerm | RetentionTier::ShortTerm => {
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
    /// Lease signature binding this proof to a specific lease.
    /// # SPEC_DEVIATION
    /// Stored but NOT verified by the artifact crate. Callers (e.g., governance,
    /// fastpath) MUST verify via `Identity::verify_with_pubkey()`.
    pub lease_signature: Vec<u8>,
    pub response_height: u64,
}

impl ProofOfPossession {
    /// Build a proof-of-possession for the given chunk.
    /// Verifies the Merkle proof against the expected chunk root before returning.
    /// Returns None if the chunk is missing or the proof does not verify against chunk_root_hash.
    ///
    /// # SPEC_DEVIATION: lease signature verification delegated to caller
    ///
    /// The artifact crate is a pure storage layer and does not perform cryptographic
    /// verification of lease signatures. Callers MUST use
    /// `hyperfluid_p2p::identity::Identity::verify_with_pubkey()` to verify the
    /// `lease_signature` before accepting a `ProofOfPossession` as valid.
    /// This separation avoids pulling the heavy `ml-dsa` / `hyperfluid-p2p` dependency
    /// into the storage crate.
    ///
    /// # Panics
    ///
    /// In debug builds, panics if `lease_signature` is empty, since a valid signature
    /// should always be provided by the caller.
    pub fn build(
        chunks: &[Vec<u8>],
        chunk_index: u32,
        chunk_root_hash: [u8; 32],
        lease_id: [u8; 32],
        response_height: u64,
        lease_signature: Vec<u8>,
    ) -> Option<Self> {
        debug_assert!(
            !lease_signature.is_empty(),
            "lease signature should be provided by caller; see SPEC_DEVIATION note"
        );
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
            lease_signature,
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
    max_concurrent: u8,
    in_progress_count: u8,
}

impl RepairQueue {
    pub fn new(max_concurrent: u8) -> Self {
        Self { entries: Vec::new(), max_concurrent, in_progress_count: 0 }
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

    /// Returns `true` if a new repair can be scheduled without exceeding
    /// the maximum concurrent repair limit.
    pub fn can_schedule_repair(&self) -> bool {
        self.in_progress_count < self.max_concurrent
    }

    /// Attempt to pop the highest-priority entry and mark it as in-progress.
    /// Returns `Err` with a description if at capacity or no entries are pending.
    pub fn try_schedule_repair(&mut self) -> Result<RepairEntry, &'static str> {
        if !self.can_schedule_repair() {
            return Err("maximum concurrent repairs reached");
        }
        let entry = self.pop_highest().ok_or("no pending repairs")?;
        self.in_progress_count += 1;
        Ok(entry)
    }

    /// Mark a repair as completed, freeing a slot for the next repair.
    pub fn finish_repair(&mut self) {
        self.in_progress_count = self.in_progress_count.saturating_sub(1);
    }
}
