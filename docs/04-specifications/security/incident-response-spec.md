# Security Spec: Incident Response & Recovery

**Components:** C12 Economics (Circuit-Breaker), C4 Governance (Post-Incident Bridge)
**Source ADRs:** ADR-0012 (Circuit-Breaker Escalation Hierarchy)
**Covered FRs:** FR-0142, FR-0143, FR-0144, FR-0145, FR-0154
**Dependencies:** C1 Consensus Engine, C2 State Machine, C4 Governance Engine, Telemetry System

---

## Section 1: Incident State Machine

### 1.1 Purpose

Define the decentralized incident detection, declaration, and response protocol including the circuit-breaker mode escalation hierarchy.

### 1.2 Normative Behavior

- The system MUST implement a binary incident mode: Normal / Emergency.
- Mode transitions MUST be fully automatic; no human intervention required for either direction.
- Incident declaration MUST require: metric breach + persistence across consecutive windows + minimum independent reporter count + signed evidence validity.
- The system MUST NOT require a central authority for incident declaration or resolution.
- Incident records MUST be signed, content-addressed, and stored in protocol state (SMT key prefix 0x0E).
- Emergency mode MUST apply deterministic parameter overrides network-wide.
- Recovery MUST be staged with temporary post-incident quotas.

### 1.3 Data Structures

```rust
struct IncidentRecord {
    incident_id: [u8; 32],         // SHA3-256 of trigger evidence
    mode: IncidentMode,
    declared_at_height: u64,
    resolved_at_height: u64,       // 0 if active
    trigger_evidence_ref: [u8; 32],
    reporter_count: u8,             // independent reporters
    exit_reason: Option<ExitReason>,
}

enum IncidentMode {
    Normal,
    Emergency,
}

enum ExitReason {
    EscrowTimeout,          // escrow period expired
    MetricsNormalized,      // all metrics within normal bounds
    GovernanceOverride,     // governance proposal resolved the incident
}

struct CircuitBreakerState {
    cb_id: [u8; 32],              // singleton key
    mode: CircuitBreakerMode,
    entered_at_height: u64,
    metrics_window_start: u64,
    reject_ratio_pct: u32,
    fill_ratio_pct: u32,
    finality_lag_ms: u32,
    sustained_windows: u8,         // consecutive breach windows
}

enum CircuitBreakerMode {
    Normal,
    Degraded,
    Emergency,
}

struct EmergencyOverrides {
    pow_difficulty_multiplier: f64,      // 3.0x
    unknown_sender_budget_pct: u8,       // 50% of normal
    frozen_actions: Vec<ActionType>,     // low-trust fast-path, new task claims
    emergency_fee_floor: u64,            // atto-AGX — [TUNE] 10x normal minimum
    reserved_lane_pct: u8,               // 100% (evidence + control lanes reserved)
}

struct RecoveryConfig {
    stabilization_windows_required: u8,  // 3 consecutive normal windows to exit
    exit_hysteresis_multiplier: f64,     // exit thresholds are 0.7x entry thresholds
    post_incident_quota_duration: u64,   // blocks of temporary quotas after recovery
    post_incident_quota_multiplier: f64, // 0.5x normal until duration expires
}
```

### 1.4 State Transitions

**Incident trigger conditions:**

| Metric | Normal → Degraded | Degraded → Emergency | Exit (all modes) |
|--------|-------------------|---------------------|-------------------|
| FinalityLagMs | > 30s sustained 3 windows | > 60s sustained 3 windows | < 20s sustained 3 windows |
| RejectRatioBps | > 5000 (50%) sustained 3 windows | > 8000 (80%) sustained 3 windows | < 3000 (30%) sustained 3 windows |
| FillRatioBps | > 8000 (80%) sustained 2 windows | > 9500 (95%) sustained 2 windows | < 6000 (60%) sustained 2 windows |

**Trigger logic:** All metrics must breach simultaneously for mode escalation. A single metric breach does not escalate. Multi-metric breach AND persistence across consecutive windows (3 for most triggers) → mode escalation. Hysteresis: exit thresholds are stricter than entry thresholds (0.7x multiplier).

**Mode transition flow:**

```
Normal
  → Degraded [multi-metric breach + sustain 3 windows]
    → freeze new low-trust claims, tighten quotas, digest-only for low-trust
    → auto-recover when metrics normalize for 3 windows
  
  → Emergency [multi-metric breach + sustain 3 windows]
    → Emergency parameter overrides apply
    → 3x PoW difficulty, 50% unknown-sender budgets, frozen low-trust fast-path
    → emergency fee floor, 100% reserved evidence/control lanes
    → auto-recover when metrics normalize for 3 windows
```

**Incident declaration algorithm:**
1. Telemetry aggregation at epoch boundary produces EpochTelemetrySummary.
2. CircuitBreakerState evaluates metrics against thresholds.
3. If ALL trigger metrics breach and persistence requirement met:
   a. Count independent reporters corroborating the breach.
   b. If reporter count >= M (max(5, committee_size / 10)):
      - Create signed IncidentRecord.
      - Transition mode.
      - Apply parameter overrides.
4. If reporter_count < M: escalation blocked (insufficient corroboration). Alert for manual review.

**Incident resolution algorithm:**
1. After mode transition, continue monitoring metrics each epoch.
2. If ALL metrics normalize (below exit thresholds) for stabilization_windows_required:
   a. Create resolution IncidentRecord with exit_reason = MetricsNormalized.
   b. Transition to lower mode (Emergency → Degraded, Degraded → Normal).
   c. Apply post-incident temporary quotas for post_incident_quota_duration blocks.
3. Governance can override via proposal (exit_reason = GovernanceOverride).

### 1.5 Failure Behavior

- **False-positive emergency trigger:** Hysteresis (stricter exit thresholds, sustained normalization requirement) prevents rapid mode flapping. Multi-metric trigger prevents single-metric noise from triggering.
- **False alarm campaign:** Validators submitting fabricated incident evidence → post-incident review detects fabricated evidence → reputation penalty and temporary reporting restriction (FR-0144).
- **Emergency mode stall:** System stuck in emergency because metrics won't normalize → post-incident governance bridge exports evidence bundle for root-cause parameter update (FR-0030).
- **Recovery traffic surge:** Staged ramp-up with temporary post-incident quotas (0.5x normal) prevents backlog shock immediately after restrictions lift.
- **No reporter quorum:** If reporter count is insufficient, incident cannot be declared. Natural throttle against false alarms but also delays legitimate incident response. Reporter incentive alignment needed.

### 1.6 Versioning and Compatibility

- Circuit-breaker thresholds are governance-adjustable within defined bounds.
- Emergency parameter overrides are deterministic and defined in policy bundle.
- Incident record schema versioned for backward compatibility.

### 1.7 Conformance Test Hooks

- Verify emergency mode entry requires: finality_lag > 60s for 3 consecutive blocks + multi-metric corroboration + minimum reporter count.
- Verify emergency mode exit requires: finality_lag < 30s for 10 consecutive blocks + all metrics normalized.
- Verify emergency mode parameter overrides apply deterministically and network-wide.
- Verify evidence and control lanes maintain guaranteed capacity in emergency mode.
- Verify low-trust fast-path actions are frozen in emergency mode.
- Verify false-alarm reporters are penalized post-incident.
- Verify recovery staged ramp-up applies temporary post-incident quotas.
- Verify no human authority required for any mode transition.

### 1.8 Trust-Assumption Inventory

- Multi-metric trigger sufficiency
  - Justification: Three metrics must breach simultaneously; coordinated attack could fabricate all three.
  - Trust-minimised alternative: Multi-source corroboration requires reporters from distinct operator clusters (not just independent keys).
- Incident evidence integrity
  - Justification: IncidentRecord is signed by declaring reporters; evidence must be independently verifiable.
  - Trust-minimised alternative: Merkle proof of telemetry envelope chain from reporter set at trigger height.
- Automatic recovery fairness
  - Justification: Auto-recovery thresholds may be too aggressive or too conservative. Calibration needed. [TUNE]
  - Trust-minimised alternative: Governance vote required for recovery (adds latency but ensures human verification).

---

## Section 2: Recovery Staged Ramp-Up

### 2.1 Purpose

Define the staged recovery protocol after emergency mode exit to prevent backlog shock.

### 2.2 Normative Behavior

- The system MUST apply temporary post-incident quotas after emergency mode exit.
- Post-incident quotas MUST be 50% of normal quotas for the recovery duration.
- Recovery duration MUST be 3 epochs (approximately 3 days).
- The system MUST process deferred low-priority operations in FIFO order during recovery.
- The system MUST monitor stabilization window metrics before allowing full normalization.
- Recovery quota multipliers MUST be deterministic and predefined.

### 2.3 Data Structures

```rust
struct RecoveryState {
    exited_emergency_at_height: u64,
    recovery_end_height: u64,
    current_quota_multiplier: f64,    // starts at 0.5, linear ramp to 1.0
    deferred_operations_count: u64,
    stabilization_checks_passed: u32,
}

struct PostIncidentReport {
    incident_id: [u8; 32],
    exit_reason: ExitReason,
    total_duration_blocks: u64,
    metrics_at_recovery: EpochTelemetrySummary,
    evidence_bundle_hash: [u8; 32],   // for governance bridge
}
```

### 2.4 State Transitions

**Recovery ramp-up schedule:**

```
Phase 1 (Epoch 1 after exit): quota_multiplier = 0.5
  - Process critical deferred operations only
  - Monitor metrics for stabilization

Phase 2 (Epoch 2 after exit): quota_multiplier = 0.75
  - Resume normal low-priority processing
  - Validate no metric recurrence

Phase 3 (Epoch 3 after exit): quota_multiplier = 1.0
  - Full normalization
  - Publish PostIncidentReport
  - Export evidence bundle to governance
```

**Rollback during recovery:** If any metric breaches threshold during recovery, re-enter emergency mode immediately (no persistence requirement during recovery phase).

### 2.5 Failure Behavior

- Metric recurrence during recovery: Immediate re-entry to emergency mode. No grace period.
- Deferred operation backlog: Processed at reduced rate during recovery. If backlog exceeds mempool capacity before full normalization, emergency mode extensible by governance.
- Post-incident report publication: Required within 1 epoch of full recovery. Failure to publish (missing reporter quorum) delays governance bridge but does not affect system operation.

### 2.6 Versioning and Compatibility

- Recovery ramp-up schedule (epoch durations, quota multipliers) is stored in system parameters.
- Post-incident quota multiplier values are governance-adjustable within bounds (0.25x-1.0x).
- Recovery phase durations are governance-adjustable; shortening below 1 epoch requires emergency governance path.

### 2.7 Conformance Test Hooks

- Verify post-incident quotas at 50% for first epoch after emergency exit.
- Verify linear ramp to 100% over 3-epoch recovery period.
- Verify deferred operations processed in FIFO order.
- Verify immediate re-entry to emergency on metric breach during recovery.

### 2.8 Trust-Assumption Inventory

- Recovery metric monitoring reliability
  - Justification: Recovery depends on accurate telemetry during the ramp-up phases. Fabricated telemetry during recovery could trigger false re-entry.
  - Trust-minimised alternative: Multi-metric corroboration during recovery; additional reporter independence requirements during recovery phase.
- Deferred operation backlog bounds
  - Justification: Backlog during emergency may exceed post-incident processing capacity, causing indefinite delayed operations.
  - Trust-minimised alternative: Per-operation-class maximum backlog size with oldest-first eviction beyond bounds.
