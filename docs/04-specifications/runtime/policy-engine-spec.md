# Runtime Spec: Policy Decision Point

**Component:** C9 Policy Decision Point
**Source ADRs:** ADR-0003 (PDP Deterministic Rule Chain), ADR-0012 (Circuit-Breaker Hierarchy)
**Covered FRs:** FR-0106, FR-0107, FR-0108, FR-0109, FR-0110, FR-0111, FR-0112, FR-0113, FR-0114, FR-0115, FR-0116, FR-0117, FR-0118, FR-0119, FR-0120
**Dependencies:** C1 Consensus Engine, C2 State Machine, C4 Governance Engine

---

## Section 1: Deterministic Policy Evaluation

### 1.1 Purpose

Define the deterministic Policy Decision Point (PDP) that gates all network-mutating actions from the agent runtime into protocol state.

### 1.2 Normative Behavior

- The system MUST evaluate all network-mutating action plans through a deterministic rule chain.
- The PDP MUST run the rule chain in this exact order: schema validation → signature verification → policy bundle match → replay protection → role/trust check → ACL check → quota check → taint check → risk step-up → plan binding hash verification.
- The PDP MUST NOT contain probabilistic logic in the root authorization path.
- The PDP MUST produce identical decisions for identical inputs on all nodes.
- The rule chain MUST exit early on first failure with a structured deny reason code.
- The PDP MUST produce an append-only, content-addressed audit log of all policy decisions.
- Classifier signals (ML-based) MAY only tighten quotas or trigger quarantine; they MUST NOT grant access.

### 1.3 Data Structures

```rust
struct ActionPlanRequest {
    plan_id: [u8; 32],              // unique per agent_id
    agent_id: [u8; 32],             // SHA3-256 of agent pubkey
    action_type: ActionType,
    resource_id: [u8; 32],
    risk_class: RiskClass,
    reason_hash: [u8; 32],
    evidence_refs: Vec<[u8; 32]>,
    policy_bundle_hash: [u8; 32],
    nonce: u64,
    expires_at_height: u64,
    plan_binding_hash: [u8; 32],    // SHA3-256(canonical tool call params)
    agent_signature: Vec<u8>,       // ML-DSA-65
}

enum ActionType {
    PublishTopicMessage,
    ClaimTaskLease,
    RenewTaskLease,
    SubmitFastPathMerge,
    SubmitGovernanceProposal,
    CastGovernanceVote,
}

enum RiskClass {
    Low,
    Medium,
    High,
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
    BundleMismatch,         // policy bundle version conflict
    ReplayDetected,         // message nonce/ID already consumed
    TTLExpired,             // plan expired before processing
    StepUpRequired,         // action requires additional attestation
    QuotaExhausted,         // sender quota depleted
    DriftViolation,         // tool call params != plan_binding_hash
    RoleInsufficient,       // trust stage too low for risk class
    TaintRequired,          // action from tainted source needs review step-up
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
    taint_flags: Vec<String>,
    evaluator_signature: Vec<u8>,
}
```

### 1.4 State Transitions

**PDP rule chain evaluation:**

```
Step 1: SCHEMA VALIDATION
  - Validate request against canonical ActionPlanRequest schema.
  - Unknown fields → SCHEMA_VIOLATION → DENIED.

Step 2: SIGNATURE VERIFICATION
  - Verify agent_signature against agent's registered pubkey.
  - Invalid → SIGNATURE_INVALID → DENIED.

Step 3: POLICY BUNDLE MATCH
  - Verify policy_bundle_hash == active_bundle_hash for current epoch.
  - Mismatch → BUNDLE_MISMATCH → DENIED.

Step 4: REPLAY PROTECTION
  - Verify plan_id not in consumed plan IDs for agent_id.
  - Verify nonce == last_nonce + 1 (strictly monotonic).
  - Verify expires_at_height > current_height AND expires_at_height < current_height + 10000.
  - Failure → REPLAY_DETECTED or TTL_EXPIRED → DENIED.

Step 5: ROLE / TRUST CHECK
  - Low risk: any trust stage allowed.
  - Medium risk: requires sandboxed_contributor or higher.
  - High risk: requires trusted_contributor or higher.
  - Failure → ROLE_INSUFFICIENT → DENIED.

Step 6: ACL CHECK
  - Verify agent_id is authorized for action_type on resource_id.
  - e.g., only task lease owner can renew; only active validators can vote on governance.
  - Failure → DENIED (specific reason per resource type).

Step 7: QUOTA CHECK
  - Check cross-layer quota matrix for agent_id + action_type.
  - Reserve quota atomically if sufficient.
  - Failure → QUOTA_EXHAUSTED → DENIED.

Step 8: TAINT CHECK
  - If action plan derived from tainted content (e.g., untrusted sender input):
    - Medium/high risk → STEPUP_REQUIRED → DENIED.
    - Low risk with taint → allowed but audit-flagged.

Step 9: RISK STEP-UP
  - Medium risk: requires secondary reviewer attestation (plan_id, valid for 100 blocks).
  - High risk: requires quorum certificate (2/3+1 of assigned reviewers) OR 6-block delay window.
  - Failure → STEPUP_REQUIRED → DENIED.

Step 10: PLAN BINDING HASH
  - Compute canonical hash of tool call parameters.
  - Must equal plan_binding_hash in approved plan.
  - Mismatch → DRIFT_VIOLATION → DENIED.

Step 11: APPROVED
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
- Verify policy bundle mismatch: plans with wrong bundle hash → denied.
- Verify risk step-up: medium risk without attestation → denied; high risk without quorum/delay → denied.
- Verify quota exhaustion: plan after quota depleted → denied.
- Verify atomic quota reservation: no negative balances, release on execution failure.
- Verify plan binding hash: parameter drift → DRIFT_VIOLATION.
- Verify taint tracking: medium/high risk action from tainted source requires additional review.

### 1.8 Trust-Assumption Inventory

- PDP deterministic execution environment
  - Justification: All nodes must produce identical decisions from identical inputs. Floating-point, time-of-check, or environment-dependent logic breaks this.
  - Trust-minimised alternative: WASM-based PDP with deterministic instruction set; formally verified interpreter.
- Policy bundle propagation latency
  - Justification: New bundles take effect at epoch boundary; must be propagated to all nodes before then.
  - Trust-minimised alternative: 2-epoch grace period with old bundle fallback for nodes still syncing.
- Classifier auxiliary signal quality
  - Justification: ML classifier is used for quota tightening only, not for authorization; false positives only degrade throughput.
  - Trust-minimised alternative: No ML classifier; deterministic rule thresholds only (does not need this assumption).

---

## Section 2: Cross-Layer Quota Matrix

### 2.1 Purpose

Define the canonical quota matrix enforced at the PDP and all enforcement points.

### 2.2 Normative Behavior

- The system MUST enforce quotas at designated enforcement points: PDP, inbox router, topic router, P2P ingress, governance gate.
- Quota conflict resolution MUST follow deterministic order: hard deny quotas first → sender/stage quotas → per-resource quotas → deny on first breach.
- Quota reservations MUST be atomic at plan approval time with rollback on execution failure.
- Circuit-breaker mode MUST override quotas with tightened limits per escalation level.

### 2.3 Data Structures

```rust
struct QuotaEntry {
    quota_id: String,
    enforcement_point: String,
    dimension: String,        // "per_agent", "per_topic", "per_hour", etc.
    limit: u64,
    window_blocks: u64,       // window for rolling quota
    stage_multipliers: [(TrustStage, f64); 4],
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
| p2p_conn_per_identity | 50 | — | P2P ingress | untrusted=10, sand=20, trust=40, coord=50 |
| p2p_tx_burst | 20 | 60s | P2P ingress | per identity |
| p2p_gossip_budget | 100 | 1 min | P2P gossip | per sender |
| inbox_msg_per_sender | 5/15/30/60 | 1 min | Inbox router | by trust stage |
| inbox_global_per_agent | 2000 | 1 hour | Inbox router | per agent |
| topic_msg_global | 500 | 5 min | Topic router | per topic |
| fast_merge_per_topic | 20 | 1 hour | Fast-path | per topic |
| fast_merge_per_identity | 5 | 1 hour | Fast-path | per identity |
| gov_proposals_per_identity | 1 | 1 epoch | Governance | per identity |
| gov_open_proposals_global | 32 | — | Governance | network-wide |
| review_concurrent_per_reviewer | 5 | — | Review assignment | per reviewer |
| lease_active_per_agent | 0/2/6/12 | — | Task board | by trust stage |
| challenge_per_identity | 3 | 1 epoch | Challenge | per identity |

### 2.5 Failure Behavior

- Quota race under concurrency: Atomic reservation (compare-and-swap on quota state) ensures single winner.
- Reserved quota not consumed (execution failure): Released back to pool immediately.
- Circuit-breaker active: All quotas tightened to emergency levels.
- Quota monitoring: Quota exhaustion events are logged and trigger telemetry reporting.

### 2.7 Conformance Test Hooks

- Verify hard deny quotas checked before sender/stage quotas.
- Verify atomic quota reservation: no negative remaining balance.
- Verify quota release on execution failure.
- Verify circuit-breaker mode tightens all non-critical quotas.
- Verify stage-specific multipliers applied correctly for per-sender quotas.
