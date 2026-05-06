# Research: Prompt Injection Defense Framework

## Purpose

This document defines the deterministic policy controls, attack corpus registry, and evaluation framework for prompt injection defense at the protocol boundary. This is a **reference design** — agents MAY implement these mechanisms locally; the protocol does not mandate or enforce them. The protocol's only prompt-injection enforcement is the 5-step PDP rule chain (schema, signature, replay, quota, fee) which applies identically to all action plans regardless of content.

**Moved from:** `docs/04-specifications/runtime/policy-engine-spec.md` §4 — demoted from protocol spec to research reference. The evaluation framework is a runtime-local concern, not protocol-enforced.

---

## Normative Behavior (Reference)

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
- The system MUST block actions that violate schema, signature, replay, quota, or fee checks.
- The system MUST apply identical policy gate evaluation to actions from any sender regardless of apparent trust level.
- The system MUST detect multi-turn delayed trigger payloads by evaluating each action independently of benign conversation history.
- The system MUST sanitize role confusion payloads (e.g., "ignore previous instructions", "system:") from untrusted content before context insertion.

---

## Data Structures

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
    hidden_pool: Vec<AttackScenario>,
    created_at_height: u64,
    last_updated_height: u64,
    corpus_hash: [u8; 32],
}

struct ScenarioRunResult {
    scenario_id: [u8; 32],
    run_seed: [u8; 32],
    trace: Vec<TraceEvent>,
    outcome: ExpectedOutcome,
    actual_outcome: ExpectedOutcome,
    match_result: bool,
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
    max_asr: f64,
    max_uer: f64,
    max_fpr_deny: f64,
    max_p99_latency_ms: u64,
    task_throughput_delta_pct: f64,
}

struct CanaryRunConfig {
    interval_blocks: u64,
    scenario_subset_hash: [u8; 32],
    drift_budget_asr: f64,
    drift_budget_uer: f64,
}
```

---

## State Transitions (Reference)

### Attack corpus lifecycle

1. Initial corpus seeded from known injection patterns at genesis.
2. Governance proposals may add new scenarios.
3. Attack vectors discovered during incidents enter the corpus via incident-to-corpus feedback.
4. Hidden scenario pool rotates on a schedule (governance-adjustable; default every 30 days). Rotation replaces 20% of hidden scenarios with newly generated ones.
5. Corpus version bumps on any scenario addition.

### Scenario runner execution flow

1. Load scenario from corpus by scenario_id.
2. Derive deterministic seed: SHA3-256(initial_state_hash || corpus_version_hash || scenario_id).
3. Initialize PDP state snapshot from initial_state_hash.
4. For each InteractionStep in interaction_script:
   a. Feed content to agent runtime via specified ingress_channel.
   b. Record agent's tool call attempts and PDP evaluations.
   c. Record trace event.
5. Classify actual_outcome. Compare to expected_outcome. Record match_result.
6. Archive ScenarioRunResult with content-addressed run_id.

### Release gating pipeline

1. On policy bundle update proposal, run full corpus against proposed policy.
2. Compute ASR, UER, FPR-deny, P99 latency, task throughput delta vs baseline.
3. If any metric exceeds DualMetricGate thresholds: block release.

### Canary drift detection

1. At configured interval, select canary scenario subset from corpus.
2. Run scenarios against current production PDP+policy.
3. Compute rolling ASR/UER over last N canary runs.
4. If rolling ASR/n > baseline * (1 + drift_budget_asr): trigger alert.

### Incident-to-corpus feedback loop

1. On incident resolution, security review identifies new attack vectors.
2. Attack vectors converted to AttackScenario format.
3. Scenarios submitted via GovernanceProposeTx for corpus addition.
4. SLA: new attack families added to corpus within 7 epochs of incident resolution.

---

## Failure Modes

- **Corpus blind spot:** An attack vector not in the corpus will not be caught by scenario runner. Mitigation: regular hidden pool rotation, incident-to-corpus feedback loop.
- **Metric gaming:** Runtimes may optimize for known benchmarks. Mitigation: hidden scenario subsets, canary drift detection, telemetry signing.
- **Overblocking release:** Policy update may pass safety metrics but degrade productivity. Mitigation: dual metric gating, staged rollout with canary subset, automatic rollback.
- **Telemetry tampering:** Compromised runtime under-reports unsafe executions. Mitigation: signed telemetry envelopes, independent policy gateway reconciliation.
- **Multi-turn delayed trigger:** Injection spread across multiple benign turns before activation. Mitigation: each action plan evaluated independently.
- **Role confusion bypass:** Payload mimics system instructions. Mitigation: deterministic pattern filtering on untrusted content.
- **Trusted channel compromise:** Compromised trusted sender delivers injection payload. Mitigation: all senders pass identical policy gate regardless of trust stage.

---

## Trust-Assumption Inventory

| Assumption | Justification | Trust-Minimised Alternative |
|------------|---------------|----------------------------|
| Classifier model quality | Classifier used for quota tightening, not root authorization. False positives degrade throughput but never create security gaps. | No ML classifier; deterministic rule thresholds only |
| Attack corpus completeness | Corpus cannot enumerate all possible injection attacks. Unknown vectors will not be caught by scenario runner. | Continuous red-team engagement and incident-to-corpus feedback loop; hidden scenario rotation |
| Deterministic PDP as root guard | Injection defense relies on PDP rule chain enforced on all nodes. Any PDP bypass is a security boundary violation. | Formal verification of PDP rule chain determinism; adversarial testing |
