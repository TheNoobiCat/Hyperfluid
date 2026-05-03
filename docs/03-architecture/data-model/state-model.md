# Data Model: Protocol State

## 1. Executive Summary

Hyperfluid's protocol state is stored in a Sparse Merkle Tree (SMT) committed in every block header. This document defines all core entities, their fields, types, and relationships. The state model supports efficient inclusion proofs, compact witnesses, and deterministic convergence across all honest nodes.

## 2. Entity Relationship Diagram

```mermaid
erDiagram
    ACCOUNT ||--o{ VALIDATOR : "stakes as"
    ACCOUNT ||--o{ IDENTITY : "controls"
    ACCOUNT {
        bytes32 account_id PK "SHA3-256 of ML-DSA pubkey"
        uint64 balance "AGX in atto-AGX"
        uint64 nonce "strictly monotonic"
        bytes32 pubkey_hash "revealed on first spend"
        bytes pubkey "revealed on first spend"
    }
    VALIDATOR {
        bytes32 validator_id PK "same as account_id"
        string state "active | paused | unbonding | withdrawn"
        uint64 bonded_stake "total bonded AGX"
        uint64 bonding_height "height of StakeBondTx"
        uint64 unbonding_height "height of UnbondRequestTx"
        uint64 jail_until_height "0 if not jailed"
        bytes liveness_bitmap "8192 bits window"
        uint32 slash_count "total slash events"
        uint32 missed_blocks "current window count"
        uint64 last_renew_height "height of last StakeRenewTx"
    }
    SLASH_RECORD {
        bytes32 slash_id PK "hash of evidence + validator"
        bytes32 validator_id FK "references VALIDATOR"
        string fault_type "equivocation | liveness | other"
        uint64 slash_amount "AGX slashed"
        uint64 slash_height "block of slash"
        bytes32 evidence_ref "content hash of evidence"
    }
    GOVERNANCE_PROPOSAL {
        bytes32 proposal_id PK "hash of proposal content"
        bytes32 proposer_id FK "references ACCOUNT"
        bytes32 proposed_commit "git commit hash"
        bytes32 bundle_manifest_hash "sha256 of bundle manifest"
        bytes32 current_commit "current git_head at proposal"
        uint64 deposit_amount "AGX locked"
        uint64 snapshot_height "validator snapshot height"
        uint64 vote_start_height "voting begins"
        uint64 vote_end_height "voting ends"
        string status "active | passed | rejected | executed"
        uint64 yes_weight "total yes votes"
        uint64 no_weight "total no votes"
    }
    GOVERNANCE_VOTE {
        bytes32 proposal_id PK,FK
        bytes32 voter_id PK,FK
        string vote "yes | no"
        uint64 vote_weight "bonded stake at snapshot"
        bytes32 reason_hash "content hash of rationale"
        bytes signature "ML-DSA-65"
    }
    COMMITTEE {
        uint64 epoch PK "epoch number"
        bytes32 seed "VDF-derived seed"
        bytes members "list of validator_id"
        bytes weights "corresponding stake weights"
    }
    ARTIFACT_MANIFEST {
        bytes32 artifact_root_hash PK
        bytes32 chunk_root_hash "Merkle root over chunks"
        uint64 size_bytes "total artifact size"
        uint32 chunk_count "number of chunks"
        string class "governance_bundle | review_evidence | research_output | telemetry_archive"
        string retention_tier "pinned | medium_term | short_term"
        uint8 min_replica_count "target replica count"
        uint64 created_at_height "registration height"
        uint64 expires_at_height "pruning height"
        bytes32 producer_signature "ML-DSA-65"
    }
    REPLICATION_LEASE {
        bytes32 lease_id PK "hash of provider+artifact+height"
        bytes32 artifact_root_hash FK "references ARTIFACT_MANIFEST"
        bytes32 provider_id "provider account_id"
        uint64 lease_start_height "lease begins"
        uint64 lease_end_height "lease expires"
        uint16 challenge_cadence "blocks between proofs"
        uint64 collateral "AGX locked"
        string status "active | at_risk | expired"
    }
    TASK {
        bytes32 task_id PK "content hash of task spec"
        string topic_id "topic reference"
        bytes32 primary_owner "agent_id with active lease"
        bytes32 funder "agent_id that created and funded the bounty"
        string status "open | claimed | in_progress | blocked | done"
        uint64 bounty_agx "escrowed reward for completion"
        uint64 created_at_height "task creation height"
        uint64 lease_expires_height "primary lease expiry"
        bytes32 required_skills_hash "content hash of skill list"
        string escrow_status "locked | released | refunded | clawed_back"
    }
    REVIEW_ASSIGNMENT {
        bytes32 assignment_id PK "hash of task+reviewer+epoch"
        bytes32 task_id FK "references TASK"
        bytes32 reviewer_id "reviewer agent_id"
        bytes32 review_record_ref "content hash of review"
        string verdict "approve | deny"
        uint8 quality_score "0-100 normalized"
        bytes32 objective_check_ref "reference to objective pass/fail"
        uint64 challenged_at_height "0 if unchallenged"
        string challenge_outcome "pending | upheld | overturned"
    }
    TELEMETRY_ENVELOPE {
        bytes32 envelope_id PK "hash of payload+sig"
        bytes32 producer_id "reporting validator/agent"
        string metric_class "finality_lag | reject_ratio | fill_ratio | tx_throughput"
        uint64 value "encoded metric value"
        uint64 height "block height at observation"
        uint64 seq_no "monotonic per producer per class"
        bytes signature "ML-DSA-65"
    }
    INCIDENT_RECORD {
        bytes32 incident_id PK "hash of trigger evidence"
        string mode "normal | emergency"
        uint64 declared_at_height "incident start height"
        uint64 resolved_at_height "0 if active"
        bytes32 trigger_evidence_ref "content hash of trigger proof"
        uint8 reporter_count "independent reporters"
        string exit_reason "escrow | timeout | metrics_normalized"
    }
    AIRDROP_POOL {
        bytes32 pool_id PK "singleton"
        uint64 total_allocated "10,000,000 AGX"
        uint64 remaining "currently unclaimed"
        uint64 airdrops_this_epoch "count this epoch"
        uint64 epoch_cap "max per epoch"
        string status "active | sunset"
    }
    TRUST_STAGE {
        bytes32 agent_id PK "references IDENTITY"
        string stage "untrusted_joiner | sandboxed_contributor | trusted_contributor | coordinator_eligible"
        uint64 identity_age_blocks "blocks since registration"
        uint32 accepted_work_count "tasks completed"
        uint32 review_diversity_count "distinct reviewers"
        uint32 abuse_flags "active abuse markers"
        bytes reputation_vector "delivery, review, liveness, safety scores"
        uint64 last_promotion_height "height of last stage change"
        uint64 last_regression_height "height of last demotion"
    }
    SYSTEM_PARAMETERS {
        uint64 epoch_length "blocks per epoch"
        uint64 committee_size "validator count"
        uint64 min_stake "AGX minimum"
        uint64 bond_delay "blocks"
        uint64 unbond_delay "blocks"
        uint64 challenge_window "blocks"
        uint64 max_governance_proposals "32"
        uint64 proposal_deposit "500 AGX"
        uint64 airdrop_amount "100 AGX per agent"
        uint64 airdrop_pool_total "10,000,000 AGX"
        bytes32 git_head "current git commit hash"
    }
    CIRCUIT_BREAKER_STATE {
        bytes32 cb_id PK "singleton"
        string mode "normal | degraded | emergency"
        uint64 entered_at_height "mode start height"
        uint64 metrics_window_start "current window start"
        uint32 reject_ratio_pct "current rejected %"
        uint32 fill_ratio_pct "mempool fill %"
        uint32 finality_lag_ms "current lag"
        uint8 sustained_windows "consecutive breach windows"
    }
    ACTION_PLAN {
        bytes32 plan_id PK,FK "unique per agent"
        bytes32 agent_id FK "references ACCOUNT"
        string action_type "publish_topic_message | claim_task_lease | etc"
        bytes32 resource_id "target resource"
        string risk_class "low | medium | high"
        bytes32 policy_bundle_hash "active bundle at submission"
        uint64 nonce "monotonically increasing"
        uint64 expires_at_height "plan TTL"
        string status "pending | approved | consumed | denied"
        bytes32 deny_reason "if denied, reason hash"
    }
    id1["VALIDATOR"] ||--o{ id2["SLASH_RECORD"] : "receives"
    id2["ACCOUNT"] ||--o| id1["VALIDATOR"] : "is"
    id3["GOVERNANCE_PROPOSAL"] ||--o{ id4["GOVERNANCE_VOTE"] : "receives"
    id1["VALIDATOR"] ||--o{ id4["GOVERNANCE_VOTE"] : "casts"
    id5["ARTIFACT_MANIFEST"] ||--o{ id6["REPLICATION_LEASE"] : "has"
    id7["TASK"] ||--o{ id8["REVIEW_ASSIGNMENT"] : "receives"
    id9["TRUST_STAGE"] ||--|| id10["ACCOUNT"] : "describes"
    id11["ACTION_PLAN"] ||--|| id10["ACCOUNT"] : "submitted_by"
```

## 3. State Organization

### SMT Key Schema

The SMT maps 32-byte keys to values. Keys are structured with a type prefix:

| Key Prefix | Entity | Key Derivation |
|-----------|--------|---------------|
| `0x01` | ACCOUNT | `SHA3-256(0x01 || account_id)` |
| `0x02` | VALIDATOR | `SHA3-256(0x02 || validator_id)` |
| `0x03` | GOVERNANCE_PROPOSAL | `SHA3-256(0x03 || proposal_id)` |
| `0x04` | COMMITTEE | `SHA3-256(0x04 || epoch_be_bytes)` |
| `0x05` | ARTIFACT_MANIFEST | `SHA3-256(0x05 || artifact_root_hash)` |
| `0x06` | TASK | `SHA3-256(0x06 || task_id)` |
| `0x07` | TELEMETRY_ENVELOPE | `SHA3-256(0x07 || height || producer_id || seq_no)` |
| `0x08` | SYSTEM_PARAMETERS | `SHA3-256(0x08)` (singleton) |
| `0x09` | CIRCUIT_BREAKER_STATE | `SHA3-256(0x09)` (singleton) |
| `0x0A` | TRUST_STAGE | `SHA3-256(0x0A || agent_id)` |
| `0x0B` | ACTION_PLAN | `SHA3-256(0x0B || agent_id || plan_id)` |
| `0x0C` | AIRDROP_POOL | `SHA3-256(0x0C)` (singleton) |
| `0x0D` | REPLICATION_LEASE | `SHA3-256(0x0D || lease_id)` |
| `0x0E` | INCIDENT_RECORD | `SHA3-256(0x0E || incident_id)` |
| `0x0F` | REVIEW_ASSIGNMENT | `SHA3-256(0x0F || assignment_id)` |

## 4. Core Entity Descriptions

### ACCOUNT

Every cryptographic identity maps to one ACCOUNT. Created on first inbound transfer or airdrop. Balance is in atto-AGX (10^18 units per AGX). Nonce prevents replay attacks. Public key is revealed on first spend to enable address compression.

**Lifecycle:** Created on first funded transaction → Active (perpetual) → Can be pruned if balance = 0 and nonce = 0 for 100,000 blocks.

### VALIDATOR

Extends ACCOUNT with four-state lifecycle: `active` → `paused` → `unbonding` → `withdrawn`. Only `active` validators are eligible for committee membership. Liveness is tracked via a 8,192-bit (1,024 byte) bitmap covering one liveness window.

**State Transitions:**
- `StakeBondTx` with >= 1,000 AGX → creates VALIDATOR in `active` (after bond_delay)
- Miss >20% blocks in window → `active` → `paused` (with 0.1% slash)
- Repeated breach within 3 windows → escalate to 1% slash
- `StakeRenewTx` + 1 epoch wait → `paused` → `active`
- `UnbondRequestTx` → `active` or `paused` → `unbonding` (14-day timer)
- After unbond_delay expiry + `WithdrawUnbondedTx` → `unbonding` → `withdrawn`
- Equivocation evidence → immediate 10% slash + jail 30 days (`active` → `paused`)

### GOVERNANCE_PROPOSAL

Created via `GovernanceProposeTx` with 500 AGX deposit. References a `proposed_commit` to update `git:head`. Validators vote via `GovernanceVoteTx`. Requires >40% quorum of snapshot stake and >50% yes votes to pass.

**Lifecycle:** proposed → `active` (vote window) → `passed` or `rejected` → if passed, `executed` at epoch boundary.

### COMMITTEE

Determined per epoch via VDF-derived seed from validator commitment-reveal inputs. Contains exact member set (100 validators) and stake weights. Max 15% per operator with anti-split detection. 67% rotation between consecutive epochs (max 33% overlap).

### ARTIFACT_MANIFEST

Content-addressed metadata for stored artifacts. The `artifact_root_hash` is the canonical hash of serialized manifest. The `chunk_root_hash` is a Merkle root over ordered chunk hashes. Retention tiers determine default expiry and minimum replica counts.

### TASK

Collaboration unit on the task board. Has soft lease lifecycle: `open` → `claimed` → `in_progress` → `blocked` → `done`. Lease TTL is 20 minutes with 5-minute heartbeat interval. Shadow claims permitted after 8-minute grace window. Auto-takeover promotes best shadow claimant on primary lease expiry.

### TRUST_STAGE

Four-stage trust ladder per agent: `untrusted_joiner` → `sandboxed_contributor` → `trusted_contributor` → `coordinator_eligible`. Promotion requires minimum identity age, accepted work count, reviewer diversity, and clean abuse record. Severe abuse can demote by up to 2 stages.

### CIRCUIT_BREAKER_STATE

System-wide singleton tracking the current operational mode. Three modes: `normal`, `degraded`, `emergency`. Transitions are triggered by multi-metric breaches (reject ratio, fill ratio, finality lag) sustained over consecutive windows. Hysteresis prevents mode flapping. Emergency mode applies deterministic parameter overrides.

### ACTION_PLAN

Immutable record of every network-mutating action submitted by agents. Each plan is single-use (transitions `approved` → `consumed` on execution). Plans reference the active policy bundle hash to prevent split-brain evaluation. Denied plans record structured reason codes for audit.

## 5. Relationships

### Key Relationships

1. **ACCOUNT → VALIDATOR:** 1:0..1 (not all accounts are validators)
2. **VALIDATOR → SLASH_RECORD:** 1:many (a validator can receive multiple slashes)
3. **GOVERNANCE_PROPOSAL → GOVERNANCE_VOTE:** 1:many (a proposal receives many votes)
4. **ARTIFACT_MANIFEST → REPLICATION_LEASE:** 1:many (one manifest, many provider leases)
5. **TASK → REVIEW_ASSIGNMENT:** 1:many (up to 5 concurrent reviewers per task)
6. **ACCOUNT → ACTION_PLAN:** 1:many (an agent submits many plans)
7. **ACCOUNT → TRUST_STAGE:** 1:1 (every agent identity has one trust stage)
8. **COMMITTEE → VALIDATOR:** many:many (validators participate in multiple committees across epochs)

### Cross-Entity Validations

- A `GovernanceVoteTx` is rejected if `voter_id` is not in `active` validators at `snapshot_height`.
- A `StakeBondTx` is rejected if `account_id` already has a VALIDATOR in non-`withdrawn` state.
- An `ACTION_PLAN` with `risk_class = high` requires `trust_stage >= trusted_contributor`.
- An `ARTIFACT_MANIFEST` registration is rejected if `expires_at_height` exceeds class retention maximum.

## 6. State Size Projections

| Entity | Size (bytes) | Growth Rate | Projected @ 1 Year |
|--------|-------------|-------------|-------------------|
| ACCOUNT | ~300 | 1k/day | ~109 MB |
| VALIDATOR | ~500 | 100/day | ~18 MB |
| GOVERNANCE_PROPOSAL | ~400 | 32/day | ~4.7 MB |
| ARTIFACT_MANIFEST | ~300 | 10k/day | ~1.1 GB |
| TASK | ~350 | 50k/day | ~6.4 GB |
| TELEMETRY_ENVELOPE | ~250 | pruned after 30d | ~2.7 GB (rolling) |
| TRUST_STAGE | ~200 | 1k/day | ~73 MB |
| ACTION_PLAN | ~350 | 100k/day | ~12.8 GB |

**Total projected (with pruning):** ~2-3 GB at 1 year, within the NFR-0002 bound of <1GB/month growth.

Archived entities (TELEMETRY_ENVELOPE, expired ARTIFACT_MANIFEST, consumed ACTION_PLAN) are pruned by archive nodes after retention windows expire. Validating nodes keep only the SMT root and current state.

## 7. Determinism Guarantees

- All entity values are serialized in canonical binary format (no non-deterministic fields in hash inputs).
- Timestamps in entity data are expressed as block heights, not wall-clock time (deterministic replay).
- Merkle tree construction uses deterministic ordering: keys sorted lexicographically before tree insertion.
- State transitions are ordered by transaction inclusion index within each block.
- Governance merge execution uses hermetic sandbox with pinned toolchain/environment (FR-0022, NFR-0023).

## 8. Serialization Format

All protocol state values use **SCALE (Simple Concatenated Aggregate Little-Endian)** encoding, consistent with Malachite's encoding layer. This provides:

- Deterministic byte output for identical values
- Compact representation (no field names in wire format)
- Fixed-size integers in little-endian for performance
- Variable-length collections prefixed with compact-encoded length
