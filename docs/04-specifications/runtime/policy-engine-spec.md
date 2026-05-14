# Runtime Spec: Policy Decision Point

**Component:** C9 Policy Decision Point
**Source ADRs:** ADR-0003 (PDP Deterministic Rule Chain)
**Covered FRs:** FR-0106, FR-0107, FR-0108, FR-0109, FR-0111, FR-0117, FR-0118, FR-0119, FR-0120
**Dependencies:** C1 Consensus Engine, C2 State Machine, C4 Governance Engine

---

## Section 1: Deterministic Policy Evaluation

### 1.1 Purpose

Define the deterministic Policy Decision Point (PDP) that gates all network-mutating actions. The PDP enforces basic admission: schema validity, signature, replay protection, quota, and fee. It does NOT enforce LLM safety, intent filtering, risk scoring, taint tracking, or policy bundle verification — those are agent runtime local concerns.

### 1.2 Normative Behavior

- The system MUST evaluate all network-mutating action plans through a deterministic rule chain.
- The PDP MUST run the rule chain in this exact order: schema validation → signature verification → replay protection → quota check → fee check.
- The PDP MUST NOT contain probabilistic logic in the root authorization path.
- The PDP MUST produce identical decisions for identical inputs on all nodes.
- The rule chain MUST exit early on first failure with a structured deny reason code.
- The PDP MUST produce an append-only, content-addressed audit log of all policy decisions.
- The PDP MUST NOT filter, restrict, or reject actions based on content, intent, risk level, trust stage, taint, policy bundle, or plan binding hash. Those checks are the agent runtime's responsibility.

### 1.3 Data Structures

```rust
struct ActionPlanRequest {
    plan_id: [u8; 32],              // unique per agent_id
    agent_id: [u8; 32],             // SHA3-256 of agent pubkey
    action_type: ActionType,
    resource_id: [u8; 32],
    reason_hash: [u8; 32],
    evidence_refs: Vec<[u8; 32]>,
    nonce: u64,
    expires_at_height: u64,
    agent_signature: Vec<u8>,       // ML-DSA-65
}

enum ActionType {
    PublishTopicMessage,
    ClaimTaskLease,
    RenewTaskLease,
    CreateTask,
    SubmitFastPathMerge,
    SubmitGovernanceProposal,
    CastGovernanceVote,
}

struct ActionPlanResponse {
    plan_id: [u8; 32],
    decision: Decision,
    deny_reason: Option<DenyReason>,
    consumed_quota: Option<Vec<QuotaConsumption>>,
    approval_height: u64,
    expires_at_height: u64,
}

enum Decision {
    Approved,
    Denied,
}

enum DenyReason {
    SchemaViolation,        // message fails structural validation
    SignatureInvalid,       // cryptographic verification failed
    ReplayDetected,         // message nonce/ID already consumed
    TTLExpired,             // plan expired before processing
    QuotaExhausted,         // sender quota depleted
}

struct QuotaConsumption {
    quota_id: String,
    amount_consumed: u64,
    remaining: u64,
}

struct PolicyAuditEntry {
    entry_id: [u8; 32],         // content hash
    plan_id: [u8; 32],
    agent_id: [u8; 32],
    action_type: ActionType,
    decision: Decision,
    deny_reason: Option<DenyReason>,
    height: u64,
    evaluator_signature: Vec<u8>,
}
```

### 1.4 State Transitions

**PDP rule chain evaluation (simplified to 6 steps):**

```
Step 1: SCHEMA VALIDATION
  - Validate request against canonical ActionPlanRequest schema.
  - Unknown fields → SCHEMA_VIOLATION → DENIED.

Step 2: SIGNATURE VERIFICATION
  - Look up agent's key binding in state (see Section 3).
  - If in grace window: verify against active_pubkey; on mismatch, retry against pending_pubkey. Accept if either matches.
  - If past grace window (rotation finalized): verify only against active_pubkey.
  - If no pending rotation: verify against active_pubkey.
  - Invalid → SIGNATURE_INVALID → DENIED.

Step 3: REPLAY PROTECTION
  - Verify plan_id not in consumed plan IDs for agent_id.
  - Verify nonce == last_nonce + 1 (strictly monotonic).
  - Verify expires_at_height > current_height AND expires_at_height < current_height + 10000.
  - Failure → REPLAY_DETECTED or TTL_EXPIRED → DENIED.

Step 4: QUOTA CHECK
  - Check cross-layer quota matrix for agent_id + action_type.
  - Reserve quota atomically if sufficient.
  - Failure → QUOTA_EXHAUSTED → DENIED.

Step 5: FEE CHECK
  - Verify agent has sufficient balance for estimated tx fee.
  - EIP-1559 base fee + priority fee must be covered.
  - Failure → DENIED (insufficient funds).

Step 6: APPROVED
  - Plan state: pending → approved.
  - Quota consumed (atomically reserved).
  - Audit log entry written.
  - Consumed plan ID recorded.
```

### 1.5 Failure Behavior

- **PDP crash during evaluation:** Atomic quota reservation ensures no partial consumption. Plan state remains pending; no state mutation applied.
- **PDP overblock (denies all):** Collaboration degraded but protocol safety preserved. Detection via audit log telemetry (reject ratio spike).
- **PDP underblock (approves malicious plan):** Challenges, slashing, and reviewer attestation layers catch post-hoc. Challenge window (144 blocks) allows rollback.
- **Split-brain:** policy_bundle_hash mismatch prevents nodes from evaluating plans under different rules. Plan rejected until bundle syncs.
- **Rule chain timeout:** Evaluation must complete within 100ms (NFR-0003). If timeout approached, partial evaluation is discarded; plan denied with INTERNAL_ERROR.

### 1.6 Versioning and Compatibility

- PDP rule chain version is embedded in the policy bundle hash.
- Policy bundles activate at epoch boundaries only.
- Old policy bundle is valid for the entire epoch; new bundle takes effect at epoch N+1.
- Interface version between PDP and agent runtime (I-01) carries version number.

### 1.7 Conformance Test Hooks

- Verify PDP evaluates identical inputs → identical outputs on different nodes.
- Verify schema validation rejects unknown fields and malformed plans.
- Verify ML-DSA signature verification rejects invalid signatures.
- Verify replay protection: duplicate plan_id, wrong nonce, or expired TTL → denied.
- Verify quota exhaustion: plan after quota depleted → denied.
- Verify atomic quota reservation: no negative balances, release on execution failure.
- Verify fee check: agent with insufficient balance for fee → denied.
- Verify key rotation: both old and new keys valid during 100-block grace window; old key rejected after grace window finalization.
- Verify nonce continuity across key rotation (nonce is per agent_id, not per key).

### 1.8 Trust-Assumption Inventory

- PDP deterministic execution environment
  - Justification: All nodes must produce identical decisions from identical inputs. Floating-point, time-of-check, or environment-dependent logic breaks this.
  - Trust-minimised alternative: WASM-based PDP with deterministic instruction set; formally verified interpreter.
---

## Section 2: Cross-Layer Quota Matrix

### 2.1 Purpose

Define the canonical quota matrix enforced at the PDP and all enforcement points.

### 2.2 Normative Behavior

- The system MUST enforce quotas at designated enforcement points: PDP, inbox router, topic router, P2P ingress, governance gate.
- Quota conflict resolution MUST follow deterministic order: hard deny quotas first → sender/stage quotas → per-resource quotas → deny on first breach.
- Quota reservations MUST be atomic at plan approval time with rollback on execution failure.
- Quotas are static per governance configuration. No dynamic mode-based quota tightening exists at protocol level.

### 2.3 Data Structures

```rust
struct QuotaEntry {
    quota_id: String,
    enforcement_point: String,
    dimension: String,        // "per_agent", "per_topic", "per_hour", etc.
    limit: u64,
    window_blocks: u64,       // window for rolling quota
    stage_multipliers: [(TrustStage, (u64, u64)); 2],  // rational pair for untrusted/trusted
}

struct QuotaState {
    quota_id: String,
    consumed: u64,
    window_start_height: u64,
}
```

### 2.4 Canonical Quota Values

| Quota ID | Limit | Window | Applies To | Stage Scaling |
|---------|-------|--------|------------|---------------|
| p2p_conn_per_identity | 50 | — | P2P ingress | untrusted=10, trusted=50 |
| p2p_tx_burst | 20 | 60s | P2P ingress | per identity |
| p2p_gossip_budget | 100 | 1 min | P2P gossip | per sender |
| inbox_msg_per_sender | 5/60 | 1 min | Inbox router | untrusted=5, trusted=60 |
| inbox_global_per_agent | 2000 | 1 hour | Inbox router | per agent |
| topic_msg_global | 500 | 5 min | Topic router | per topic |
| fast_merge_per_topic | 20 | 1 hour | Fast-path | per topic |
| fast_merge_per_identity | 5 | 1 hour | Fast-path | per identity |
| gov_proposals_per_identity | 1 | 1 epoch | Governance | per identity |
| gov_open_proposals_global | 32 | — | Governance | network-wide |
| review_concurrent_per_reviewer | 5 | — | Review assignment | per reviewer |
| lease_active_per_agent | 2/6 | — | Task board | untrusted=2, trusted=6 |
| challenge_per_identity | 3 | 1 epoch | Challenge | per identity |
| task_create_per_stage | 0/10 | — | Task creation (PDP) | untrusted=0, trusted=10 |

### 2.5 Failure Behavior

- Quota race under concurrency: Atomic reservation (compare-and-swap on quota state) ensures single winner.
- Reserved quota not consumed (execution failure): Released back to pool immediately.
- Quota monitoring: Quota exhaustion events are logged and trigger telemetry reporting.

### 2.6 Versioning and Compatibility

- Quota matrix values are governance-adjustable within defined bounds per quota ID.
- Stage multiplier tables are stored in system parameters and activate at epoch boundaries.
- New quota IDs may be added via governance; existing IDs may not be removed (only zeroed to 0 limit).
- Enforcement point ordering (hard deny → sender/stage → per-resource) is protocol-wide and requires `git:head` update to change.

### 2.7 Conformance Test Hooks

- Verify hard deny quotas checked before sender/stage quotas.
- Verify atomic quota reservation: no negative remaining balance.
- Verify quota release on execution failure.
- Verify stage-specific multipliers applied correctly for per-sender quotas (2-stage model).
- Verify stage-specific multipliers applied correctly for per-sender quotas.

### 2.8 Trust-Assumption Inventory

- Quota enforcement consistency
  - Justification: Quotas are enforced at the PDP running on every node. Deterministic rule chain ensures identical enforcement.
  - Trust-minimised alternative: On-chain quota state with SMT inclusion proofs per consumption; slashing for validators that approve quota-exceeding plans.
- Stage multiplier calibration
  - Justification: Multiplier differences between trust stages are governance-set and may create unintended privilege gradients.
  - Trust-minimised alternative: Protocol-hardcoded ratio bounds (e.g., no stage gets >10x any lower stage multiplier).

---

## Section 3: Key Rotation State Finalization

### 3.1 Purpose

Define the key rotation state finalization rules for agent cryptographic keys. Covers FR-0118 and NFR-0024.

### 3.2 Normative Behavior

- The system MUST commit a key rotation transaction to state before a new agent public key is accepted for signature verification.
- The system MUST provide a deterministic 100-block grace window during which both the old and new keys are valid for signature verification.
- The system MUST reject signatures verified against a revoked (old) key after the grace window ends.
- The system MUST record every key rotation event in the append-only PDP audit log.
- The system MUST preserve agent identity (agent_id), nonce, and trust stage across key rotation.
- The system MUST allow a pending rotation to be superseded by a new rotation transaction during the grace window, restarting the 100-block window.
- The system MUST use the same ML-DSA-65 signature scheme for the new key.

### 3.3 Data Structures

```rust
struct KeyBinding {
    agent_id: [u8; 32],          // SHA3-256 of original active pubkey (stable identity)
    active_pubkey: Vec<u8>,     // ML-DSA-65 pubkey (current, always set)
    pending_pubkey: Option<Vec<u8>>,  // set during grace window
    rotation_height: Option<u64>,     // block height when rotation tx committed
    grace_end_height: Option<u64>,    // rotation_height + 100
}

enum TxType {
    // ... existing transaction types ...
    KeyRotationTx,
}

struct KeyRotationTransaction {
    agent_id: [u8; 32],
    new_pubkey: Vec<u8>,         // ML-DSA-65
    new_pubkey_hash: [u8; 32],  // SHA3-256(new_pubkey) for pre-computation
    signature: Vec<u8>,          // signed with current active_pubkey
    nonce: u64,
}
```

### 3.4 State Transitions

Key rotation uses a dual-key model: when pending_pubkey is set and current_height < grace_end_height, both old and new keys are accepted. After grace_end_height, only the new key is valid.

**Nonce preservation:** Nonce is bound to agent_id, not pubkey. Nonce continuity is maintained across rotation — the agent continues from last_nonce + 1 with the new key.

**Trust stage preservation:** Trust stage and staked AGX are bound to agent_id. Key rotation does not reset or degrade these.

### 3.5 Failure Behavior

- **Replay with old key after finalization:** Old key no longer matches active_pubkey; ML-DSA verification fails → SIGNATURE_INVALID. Audit log records the attempt for intrusion detection.
- **Double rotation during grace window:** Second KeyRotationTransaction resets grace_end_height to new commit height + 100. The original pending_pubkey is replaced. This prevents indefinite double-key windows (at most one pending rotation at a time).
- **Rotation tx signed with revoked key:** Same as replay → SIGNATURE_INVALID.
- **Rotation tx with invalid nonce:** Caught by Step 4 replay protection in the PDP rule chain before key binding lookup.
- Key rotation transactions follow standard admission rules.
- **Key rotation during incident escalation:** No special handling required; same grace window rules apply. Rotation audit trail supports incident forensics.

### 3.6 Versioning and Compatibility

- KeyRotationTransaction type is versioned; new fields are append-only.
- KeyBinding state is per-agent with no protocol-wide migration needed.
- Rotation flow is backwards-compatible: nodes unaware of pending_pubkey field (empty → STABLE state) treat it as no rotation in progress.

### 3.7 Conformance Test Hooks

- Verify both old and new keys produce valid signature verification during grace window (100 blocks).
- Verify old key is rejected after grace window finalization (block 101+ after rotation tx).
- Verify second rotation during grace window resets grace_end_height and replaces pending_pubkey.
- Verify nonce continuity: nonce after rotation equals last nonce before rotation + 1.
- Verify trust stage and agent_id are preserved across rotation.
- Verify rotation event is recorded in PDP audit log with both old and new pubkey hashes.
- Verify rotation tx signed with a non-active key (not active_pubkey and not pending_pubkey) is rejected.

### 3.8 Trust-Assumption Inventory

- ML-DSA-65 cryptographic security
  - Justification: Post-quantum signature security for all agent keys. Key rotation relies on the same primitive as all other signing operations.
  - Trust-minimised alternative: ML-DSA is itself the post-quantum choice. Hybrid with classical ECDSA possible during transition.
- 100-block grace window bounded risk
  - Justification: A compromised old key can still sign during the 100-block grace window. This is an intentional tradeoff — the grace window allows in-flight action plans to complete, preventing operational disruption during legitimate rotation. At ~10s block times, the exposure window is ~17 minutes.
  - Trust-minimised alternative: Zero-grace-window (instant rotation) would atomically revoke old key but would cause in-flight plan failures. The 100-block window is the minimal value that covers a full challenge window (144 blocks of plan validity) and allows queued plans to complete. Shorter windows (~50 blocks) increase the risk of false-positive plan rejections during rotation.

