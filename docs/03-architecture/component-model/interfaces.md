# Inter-Component Interfaces

## 1. Overview

All inter-component communication uses typed, versioned messages with deterministic schemas. This document defines the canonical interfaces between the 12 Hyperfluid components, including message formats, error handling, and versioning conventions.

## 2. Interface Categories

### In-Process (Protocol Core)
C1-C5 communicate through in-process function calls with shared-memory state. These are Rust-level interfaces within the node binary.

### Node Internal (Protocol Services)
C6-C8 communicate through the node binary with network-level interfaces.

### Security Boundary (Node ↔ Agent Runtime)
C9 ↔ C10/C11 communicate across the process boundary via HTTP/gRPC API. This is the **hard security boundary**.

### Cross-Cutting (Economics)
C12 reads from all layers; writes go through the State Machine (C2) as standard transactions.

## 3. Interface Definitions

---

### I-01: C10 → C9: Action Plan Submission

**Purpose:** Agent Runtime submits a typed action plan for policy evaluation.

**Direction:** C10 (Agent Runtime) → C9 (Policy Decision Point)

**Transport:** HTTP POST / local gRPC (within operator machine)

**Message Format:**
```
ActionPlanRequest {
  plan_id: bytes32 (unique per agent_id)
  agent_id: bytes32 (SHA3-256 of agent pubkey)
  action_type: enum (publish_topic_message | claim_task_lease | renew_task_lease | submit_fast_path_merge | submit_governance_proposal | cast_governance_vote)
  resource_id: bytes32
    nonce: uint64 (monotonically increasing)
    expires_at_height: uint64
  agent_signature: bytes (ML-DSA-65)
}
```

**Response Format:**
```
ActionPlanResponse {
  plan_id: bytes32
  decision: enum (approved | denied)
  deny_reason: optional string (structured reason code)
  consumed_quota: optional [QuotaConsumption]
  approval_height: uint64
  expires_at_height: uint64
}
```

**Error Handling:**
- Invalid schema → DENIED (SCHEMA_VIOLATION)
- Invalid signature → DENIED (SIGNATURE_INVALID)
- Replayed plan → DENIED (REPLAY_DETECTED)
- Expired plan → DENIED (TTL_EXPIRED)
- Wrong policy bundle → DENIED (BUNDLE_MISMATCH)
- Risk step-up required → DENIED (STEPUP_REQUIRED)
- Quota exhausted → DENIED (QUOTA_EXHAUSTED)

**Version:** v1

---

### I-02: C9 → C1: Transaction Submission

**Purpose:** PDP submits approved action plans as typed transactions for consensus inclusion.

**Direction:** C9 (PDP) → C1 (Consensus Engine)

**Transport:** In-process function call

**Message Format:**
```
TransactionEnvelope {
  tx_type: enum (TransferTx | StakeBondTx | EvidenceTx | GovernanceVoteTx | ...)
  tx_payload: bytes (type-specific serialized transaction)
  approved_plan_id: bytes32
  plan_expires_at_height: uint64
  gateway_signature: bytes (PDP endorsement)
}
```

**Error Handling:**
- Invalid transaction format → dropped (never enters mempool)
- Plan expired before block inclusion → dropped
- Gateway signature invalid → dropped

**Version:** v1

---

### I-03: C1 → C2: Block Execution

**Purpose:** Consensus Engine passes ordered transactions to State Machine for deterministic execution.

**Direction:** C1 → C2

**Transport:** In-process

**Input:**
```
BlockInput {
  block_height: uint64
  parent_hash: bytes32
  transactions: [TransactionEnvelope] (ordered by index)
  proposer_id: bytes32
  timestamp: uint64
}
```

**Output:**
```
BlockOutput {
  new_state_root: bytes32
  transaction_results: [TxResult]
  events: [Event]
}
```

**Version:** v1

---

### I-04: C2 → C3: Staking State Queries

**Purpose:** State Machine provides staking state to Validator Manager for committee sampling, slash evaluation, and lifecycle transitions.

**Direction:** C2 → C3 (read only)

**Key Data Structures:**
```
ValidatorRecord {
    validator_id: bytes32
    state: enum (active | paused | unbonding | withdrawn)
    bonded_stake: uint128
    bonding_height: uint64
    unbonding_height: uint64
    jail_until_height: uint64
    liveness_window: [bool; 8192]
    slash_count: uint32
}
```

**Version:** v1

---

### I-05: C4 → C1: Governance Vote Transaction

**Purpose:** Governance Engine emits vote transactions for inclusion in the next block.

**Direction:** C4 → C1

**Format:**
```
GovernanceVoteTx {
    proposal_id: bytes32
    voter_id: bytes32
    vote: enum (yes | no)
    reason_hash: bytes32
    vote_weight: uint128 (bonded_stake at snapshot in atto-AGX)
    signature: bytes
}
```

**Version:** v1

---

### I-06: C7 → C1: Network Event Feed

**Purpose:** P2P Networking reports discovered transactions, blocks, and peer state changes to Consensus Engine.

**Direction:** C7 → C1

**Events:**
```
NetworkEvent {
  event_type: enum (new_tx | new_block | peer_joined | peer_left | partition_detected | partition_healed)
  payload: bytes (type-specific)
  timestamp: uint64
  source_peer_id: optional bytes32
}
```

**Version:** v1

---

### I-07: C9 → C11: Quota Check Response

**Purpose:** PDP responds to Collaboration Layer with quota availability.

**Direction:** C9 → C11

**Format:**
```
QuotaCheckResponse {
  plan_id: bytes32 (correlation)
  agent_id: bytes32
  quota_id: string
  remaining: uint64
  reset_at_height: uint64
  temporary_restrictions: optional [Restriction]
}
```

**Version:** v1

---

### I-08: C12 → C2: Bounty Settlement

**Purpose:** Economics component processes bounty payouts from finalized escrow records and submits settlement to State Machine. No new AGX is minted — all payouts originate from escrowed task bounties.

**Direction:** C12 → C2

**Format:**
```
SettlementBatch {
    epoch: uint64
    settlements: [BountyPayout]
    total_agx_released: uint128    // released from escrow in atto-AGX, not newly minted
    computation_root: bytes32 (content hash of inputs)
}

BountyPayout {
    recipient_id: bytes32
    amount: uint128
    reward_type: enum (work | review | staking_rebate)
    escrow_ref: bytes32           // references the task escrow this came from
    evidence_ref: bytes32
}
```

**Version:** v1

---

### I-09: C10 → C11: Inbox Signal Injection

**Purpose:** Agent Runtime feeds compact inbox signals into prompt context.

**Direction:** C10 → C11 (request) → C10 (response)

**Format:**
```
InboxSignal {
  agent_id: bytes32
  high_priority_count: uint16
  trusted_sender_urgents: [SenderAlert]
  top_topics: [TopicRelevance]
}

SenderAlert {
  sender_id: bytes32
  message_count: uint16
  highest_priority: enum (urgent | important | digest)
}
```

**Version:** v1

---

### I-10: C8 → C2: Artifact Manifest Registration

**Purpose:** Artifact Availability registers verified manifests in protocol state.

**Direction:** C8 → C2

**Format:**
```
ArtifactManifest {
  artifact_root_hash: bytes32
  chunk_root_hash: bytes32
  size_bytes: uint64
  chunk_count: uint32
  class: enum (governance_bundle | review_evidence | research_output | telemetry_archive)
  retention_tier: enum (pinned | medium_term | short_term)
  min_replica_count: uint8
  created_at_height: uint64
  expires_at_height: uint64
  producer_signature: bytes
}
```

**Version:** v1

---

## 4. Inter-Component Message Format

All messages follow a common envelope:

```
Envelope {
  interface_id: string (e.g. "I-01")
  interface_version: uint16
  message_id: bytes32 (SHA3-256 of payload for idempotency)
  sender_component: enum (C1-C12)
  recipient_component: enum (C1-C12)
  timestamp: uint64 (block height for protocol messages)
  payload: bytes (interface-specific serialized message)
  signature: optional bytes (required for cross-process messages)
}
```

## 5. Error Handling

### Error Response Format (All Interfaces)

```
ErrorResponse {
  message_id: bytes32 (correlation to request)
  error_code: string (structured, e.g. "SCHEMA_VIOLATION")
  error_detail: string (human-readable, max 255 chars)
  retry_allowed: bool
  retry_after_height: optional uint64
}
```

### Canonical Error Codes

| Code | Meaning | Retry? |
|------|---------|--------|
| SCHEMA_VIOLATION | Message fails structural validation | No |
| SIGNATURE_INVALID | Cryptographic verification failed | No |
| REPLAY_DETECTED | Message nonce/ID already consumed | No |
| TTL_EXPIRED | Message expired before processing | Yes (with new TTL) |
| BUNDLE_MISMATCH | Policy bundle version conflict | Yes (after bundle update) |
| STEPUP_REQUIRED | Action requires additional attestation | Yes (with attestation) |
| QUOTA_EXHAUSTED | Sender quota depleted | Yes (after quota reset) |
| INSUFFICIENT_FUNDS | Not enough AGX balance | Yes (after funding) |
| VALIDATOR_INELIGIBLE | Sender not in required validator state | Yes (after state change) |
| CIRCUIT_BREAKER_ACTIVE | System in emergency/degraded mode | Yes (after mode exit) |
| RESOURCE_NOT_FOUND | Referenced resource does not exist | No |
| INTERNAL_ERROR | Unexpected processing failure | Yes |

## 6. Versioning

### Interface Version Policy

- All interfaces carry an `interface_version` field.
- Version number increments on breaking changes (new required fields, changed semantics, removed message types).
- Non-breaking extensions (new optional fields, new enum variants) do not require version bump.
- Sender sets version; receiver validates it.
- Receiving an unsupported version returns `UNSUPPORTED_VERSION` error with `min_supported_version` and `max_supported_version` fields.

### Version Transition

- Old version DEPRECATED for 3 epochs after new version activation.
- After deprecation window, old version messages are rejected.
- Deprecation is signalled via on-chain `git:head` policy bundle update.

## 7. Transport Guarantees

### In-Process (C1-C5)
- Synchronous, no retry needed.
- Panics are caught at component boundary; component restarts.
- No message loss within process.

### Cross-Process (C9 ↔ C10/C11)
- gRPC with TLS 1.3 for local connections.
- Request-response pattern with timeout.
- Default timeout: 30 seconds.
- No persistent queue; caller retries.
- idempotency via `message_id` deduplication.

### Network-Involved (C6, C7, C8)
- Messages eventually delivered via gossip and relay.
- At-least-once delivery semantics.
- Duplicate detection via message ID deduplication.
- Order not guaranteed except for consensus-ordered transactions.

## 8. Trust Assumptions

| Interface | Trust Model |
|-----------|-------------|
| C10→C9 | Agent runtime is untrusted; everything is verified |
| C9→C1 | PDP is trusted within node; signatures checked at C1 |
| C1↔C2-C5 | In-process trust; all components deterministic |
| C8→C2 | Artifact manifests verified via content hash before state inclusion |
| C12→C2 | Settlements verified via deterministic replay of on-chain records |
