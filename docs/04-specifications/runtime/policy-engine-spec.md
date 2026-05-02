# Runtime Spec: Policy Decision Point

**Component:** C9 Policy Decision Point
**Source ADRs:** ADR-0003 (PDP Deterministic Rule Chain), ADR-0012 (Circuit-Breaker Hierarchy)
**Covered FRs:** FR-0106, FR-0107, FR-0108, FR-0109, FR-0110, FR-0111, FR-0112, FR-0113, FR-0114, FR-0115, FR-0116, FR-0117, FR-0118, FR-0119, FR-0120, FR-0121, FR-0122, FR-0123, FR-0124, FR-0125, FR-0126, FR-0127, FR-0128, FR-0129, FR-0130, FR-0131, FR-0132, FR-0133, FR-0134, FR-0135
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
  - Look up agent's key binding in state (see Section 3).
  - If in grace window: verify against active_pubkey; on mismatch, retry against pending_pubkey. Accept if either matches.
  - If past grace window (rotation finalized): verify only against active_pubkey. Pending_pubkey is now active; old key is revoked.
  - If no pending rotation: verify against active_pubkey.
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
- Verify key rotation: both old and new keys valid during 100-block grace window; old key rejected after grace window finalization.
- Verify nonce continuity across key rotation (nonce is per agent_id, not per key).

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

### 2.6 Versioning and Compatibility

- Quota matrix values are governance-adjustable within defined bounds per quota ID.
- Stage multiplier tables are stored in system parameters and activate at epoch boundaries.
- New quota IDs may be added via governance; existing IDs may not be removed (only zeroed to 0 limit).
- Enforcement point ordering (hard deny → sender/stage → per-resource) is protocol-wide and requires `git:head` update to change.

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

**Key rotation lifecycle:**

```
State: STABLE
  - active_pubkey only (no pending rotation)
  - Signature verification uses active_pubkey

  Agent submits KeyRotationTransaction ──→ State: GRACE_WINDOW

State: GRACE_WINDOW (100 blocks)
  - active_pubkey = old key, pending_pubkey = Some(new_key)
  - Signature verification: try active_pubkey first; on fail, try pending_pubkey
  - grace_end_height = commit_height + 100

  New rotation tx submitted during grace → grace_end_height = new_height + 100 (restart)
  Current block >= grace_end_height → State: ROTATION_FINALIZED

State: ROTATION_FINALIZED
  - active_pubkey = pending_pubkey
  - pending_pubkey = None
  - Old key permanently revoked
  - Rotation event written to audit log
  → returns to STABLE with new active_pubkey
```

**Signature verification during each phase:**

| Phase | Verification rule |
|-------|-------------------|
| STABLE (no pending) | Verify against active_pubkey only |
| GRACE_WINDOW | Verify against active_pubkey; on fail, retry pending_pubkey. Accept if either matches. |
| ROTATION_FINALIZED | Verify against active_pubkey only (old key revoked, will fail) |

**Nonce preservation:** Nonce is bound to agent_id, not pubkey. Nonce continuity is maintained across rotation — the agent continues from last_nonce + 1 with the new key.

**Trust stage preservation:** Trust stage, reputation, and staked AGX are bound to agent_id. Key rotation does not reset or degrade these.

### 3.5 Failure Behavior

- **Replay with old key after finalization:** Old key no longer matches active_pubkey; ML-DSA verification fails → SIGNATURE_INVALID. Audit log records the attempt for intrusion detection.
- **Double rotation during grace window:** Second KeyRotationTransaction resets grace_end_height to new commit height + 100. The original pending_pubkey is replaced. This prevents indefinite double-key windows (at most one pending rotation at a time).
- **Rotation tx signed with revoked key:** Same as replay → SIGNATURE_INVALID.
- **Rotation tx with invalid nonce:** Caught by Step 4 replay protection in the PDP rule chain before key binding lookup.
- **Key rotation during circuit-breaker mode:** Rotation transactions are not subject to circuit-breaker quotas. Agents must be able to rotate compromised keys during emergencies.
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

---

## Section 4: Prompt Injection Defense

### 4.1 Purpose

Define the deterministic policy controls, attack corpus registry, and evaluation framework for prompt injection defense at the protocol boundary. Covers FR-0121 through FR-0135.

### 4.2 Normative Behavior

- The system MUST treat all inbound payloads (DM, topic messages, documentation, web content, code) as untrusted at ingress regardless of sender identity.
- The system MUST NOT allow untrusted text to directly trigger network-mutating actions without passing through the deterministic PDP rule chain.
- The system MUST use classifier signals (e.g., ModernBERT-style) as auxiliary scoring inputs only; classifiers MUST NOT serve as root authorization truth.
- The system MUST maintain a versioned attack corpus registry with deterministic scenario schemas, expected policy outcomes, and severity weights.
- The system MUST support deterministic scenario replay with reproducible traces using seeded randomness.
- The system MUST gate runtime promotion on dual metrics: safety (ASR, UER) and productivity (FPR-deny, P99 latency, task throughput delta).
- The system MUST run canary scenario subsets periodically in production-like environments with drift budget alerting.
- The system MUST feed discovered attack vectors from security incidents back into the attack corpus within defined SLAs.
- The system MUST maintain rotating hidden scenario subsets to prevent runtime overfitting.
- The system MUST sign evaluation telemetry with ML-DSA to prevent compromised runtimes from under-reporting unsafe executions.
- The system MUST support staged policy bundle rollout with automatic rollback on canary metric breach.
- The system MUST require additional review certificates for high-risk actions derived from tainted content (see Section 1, Step 8: Taint Check).
- The system MUST block actions that pass schema validation but violate resource ACL, quota caps, or risk-step-up requirements.
- The system MUST apply identical policy gate evaluation to actions from any sender regardless of apparent trust level.
- The system MUST detect multi-turn delayed trigger payloads by evaluating each action independently of benign conversation history.
- The system MUST sanitize role confusion payloads (e.g., "ignore previous instructions", "system:") from untrusted content before context insertion.

### 4.3 Data Structures

```rust
struct AttackScenario {
    scenario_id: [u8; 32],
    corpus_version_hash: [u8; 32],
    ingress_channel: IngressChannel,
    initial_state_hash: [u8; 32],
    interaction_script: Vec<InteractionStep>,
    expected_outcome: ExpectedOutcome,
    expected_reason_code: Option<DenyReason>,
    severity_weight: f64,                // 0.0 to 1.0
}

enum IngressChannel {
    DirectMessage,
    TopicMessage,
    DocumentAttachment,
    WebContent,
    CodeInput,
}

struct InteractionStep {
    step_index: u32,
    sender_role: SenderRole,
    content: Vec<u8>,
    asserted_identity: Option<[u8; 32]>,
}

enum SenderRole {
    System,
    User,
    Assistant,
}

enum ExpectedOutcome {
    Deny,
    AllowLowRisk,
    AllowWithStepUp,
}

struct AttackCorpus {
    corpus_version: u64,
    scenarios: Vec<AttackScenario>,
    hidden_pool: Vec<AttackScenario>,     // not visible to runtime developers
    created_at_height: u64,
    last_updated_height: u64,
    corpus_hash: [u8; 32],               // Merkle root over all scenarios
}

struct ScenarioRunResult {
    scenario_id: [u8; 32],
    run_seed: [u8; 32],                  // derived from initial_state_hash
    trace: Vec<TraceEvent>,
    outcome: ExpectedOutcome,
    actual_outcome: ExpectedOutcome,
    match_result: bool,                   // expected == actual
    latency_ms: u64,
    policy_decisions: Vec<ActionPlanResponse>,
}

struct TraceEvent {
    step_index: u32,
    event_type: TraceEventType,
    data: Vec<u8>,
    height: u64,
}

enum TraceEventType {
    PolicyEvaluation,
    ToolCall,
    TaintPropagation,
    QuotaCheck,
}

struct DualMetricGate {
    max_asr: f64,                  // Acceptable Safety Rate ceiling
    max_uer: f64,                  // Unsafe Execution Rate ceiling
    max_fpr_deny: f64,             // False Positive Rate (deny) ceiling
    max_p99_latency_ms: u64,       // P99 policy evaluation latency ceiling
    task_throughput_delta_pct: f64, // maximum allowed throughput degradation
}

struct CanaryRunConfig {
    interval_blocks: u64,          // how often to run canaries
    scenario_subset_hash: [u8; 32],
    drift_budget_asr: f64,         // max allowed ASR increase
    drift_budget_uer: f64,         // max allowed UER increase
}
```

### 4.4 State Transitions

**Attack corpus lifecycle:**
1. Initial corpus seeded from known injection patterns at genesis.
2. Governance proposals may add new scenarios (FR-0123 acceptance criteria: deterministic schemas with expected outcomes).
3. Attack vectors discovered during incidents enter the corpus via incident-to-corpus feedback (post-incident review exports evidence as new scenarios).
4. Hidden scenario pool rotates on a schedule (governance-adjustable; default every 30 days). Rotation replaces 20% of hidden scenarios with newly generated ones.
5. Corpus version bumps on any scenario addition. Version hash references the Merkle root over all scenarios.

**Scenario runner execution flow:**
1. Load scenario from corpus by scenario_id.
2. Derive deterministic seed: SHA3-256(initial_state_hash || corpus_version_hash || scenario_id).
3. Initialize PDP state snapshot from initial_state_hash.
4. For each InteractionStep in interaction_script:
   a. Feed content to agent runtime via specified ingress_channel.
   b. Record agent's tool call attempts and PDP evaluations.
   c. Record trace event (PolicyEvaluation, ToolCall, TaintPropagation, QuotaCheck).
5. Classify actual_outcome: Deny (all mutations blocked), AllowLowRisk (only low-risk approved), AllowWithStepUp (high-risk with valid step-up certificate).
6. Compare actual_outcome to expected_outcome. Record match_result.
7. Archive ScenarioRunResult with content-addressed run_id.

**Release gating pipeline:**
1. On policy bundle update proposal, run full corpus against proposed policy.
2. Compute ASR = denied_should_deny / total_should_deny, UER = allowed_should_deny / total_should_deny.
3. Compute FPR-deny = denied_should_allow / total_should_allow, P99 latency, task throughput delta vs baseline.
4. If any metric exceeds DualMetricGate thresholds: block release. Record all failures by severity.
5. Severity-weighted failures may hard-fail even if aggregate passes.

**Canary drift detection:**
1. At configured interval, select canary scenario subset from corpus.
2. Run scenarios against current production PDP+policy.
3. Compute rolling ASR/UER over last N canary runs.
4. If rolling ASR/n > baseline * (1 + drift_budget_asr): trigger alert. Export investigation bundle.
5. Archive canary results for trend analysis.

**Incident-to-corpus feedback loop:**
1. On incident resolution (PostIncidentReport published), security review identifies new attack vectors.
2. Attack vectors converted to AttackScenario format with severity_weight >= 0.7.
3. Scenarios submitted via GovernanceProposeTx for corpus addition.
4. SLA: new attack families added to corpus within 7 epochs of incident resolution.

### 4.5 Failure Behavior

- **Corpus blind spot:** An attack vector not in the corpus will not be caught by scenario runner. Mitigation: regular hidden pool rotation, incident-to-corpus feedback loop.
- **Metric gaming:** Runtimes may optimize for known benchmarks. Mitigation: hidden scenario subsets (FR-0128), canary drift detection, telemetry signing prevents data manipulation.
- **Overblocking release:** Policy update may pass safety metrics but degrade productivity. Mitigation: dual metric gating (FR-0125), staged rollout with canary subset (FR-0130), automatic rollback on threshold breach.
- **Telemetry tampering:** Compromised runtime under-reports unsafe executions. Mitigation: signed telemetry envelopes (FR-0129), independent policy gateway reconciliation (FR-0141).
- **Multi-turn delayed trigger:** Injection spread across multiple benign turns before activation. Mitigation: each action plan evaluated independently; no cumulative trust accumulation from prior benign turns.
- **Role confusion bypass:** Payload mimics system instructions. Mitigation: deterministic pattern filtering on untrusted content; system prompt identity block is append-only and protected from message-level modification.
- **Trusted channel compromise:** Compromised trusted sender delivers injection payload. Mitigation: all senders pass identical policy gate regardless of trust stage (FR-0133).

### 4.6 Versioning and Compatibility

- Attack corpus version is pinned in policy bundle for reproducible evaluation.
- Scenario schema is append-only; new fields ignored by old runners.
- DualMetricGate thresholds are governance-adjustable within protocol bounds.
- Canary run interval and drift budgets are system parameters stored in protocol state.
- Hidden pool rotation schedule is governance-adjustable.

### 4.7 Conformance Test Hooks

- Verify inbound payload from any sender (including trusted) is marked untrusted at ingress.
- Verify classifier signals tighten quotas but cannot authorize execution.
- Verify attack corpus scenario replay produces deterministic traces from same seed + state hash.
- Verify dual metric gate blocks release when ASR > threshold OR UER > threshold.
- Verify dual metric gate blocks release when FPR-deny exceeds ceiling.
- Verify canary drift detection alerts when rolling ASR exceeds baseline + drift_budget.
- Verify incident-to-corpus feedback adds new scenarios to corpus within SLA.
- Verify staged rollout rolls back automatically on canary metric breach.
- Verify taint-aware policy: high-risk action from tainted source requires step-up certificate.
- Verify schema-conformant but ACL-violating actions are blocked.
- Verify identical policy gate evaluation for all senders (no trust-based bypass).
- Verify multi-turn delayed trigger: each action evaluated independently; no history-based trust.
- Verify role confusion payloads are sanitized before context insertion.
- Verify signed eval telemetry cannot be forged (invalid signature → rejected).

### 4.8 Trust-Assumption Inventory

- Classifier model quality
  - Justification: Classifier (ModernBERT-style) is used for quota tightening and quarantine, not root authorization. False positives degrade throughput but never create security gaps.
  - Trust-minimised alternative: No ML classifier; deterministic rule thresholds only.
- Attack corpus completeness
  - Justification: The corpus cannot enumerate all possible injection attacks. Unknown attack vectors will not be caught by scenario runner.
  - Trust-minimised alternative: Continuous red-team engagement and incident-to-corpus feedback loop; hidden scenario rotation prevents overfitting to known patterns.
- Deterministic PDP as root guard
  - Justification: Prompt injection defense relies on the PDP rule chain being consistently enforced on all nodes. Any PDP bypass is a security boundary violation.
  - Trust-minimised alternative: Formal verification of PDP rule chain determinism; adversarial testing against PDP implementation.
