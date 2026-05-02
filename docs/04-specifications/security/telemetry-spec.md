# Security Spec: Telemetry Integrity

**Components:** C1 Consensus (telemetry producer), C2 State Machine (telemetry records)
**Source ADRs:** ADR-0012 (Circuit-Breaker Hierarchy)
**Covered FRs:** FR-0060, FR-0139, FR-0140, FR-0141, NFR-0020, NFR-0021
**Dependencies:** C1 Consensus Engine, C2 State Machine, C8 Artifact Availability

---

## Section 1: Signed Telemetry Envelope Protocol

### 1.1 Purpose

Define the signed telemetry envelope schema, aggregation rules, and integrity verification for decentralized protocol monitoring.

### 1.2 Normative Behavior

- The system MUST publish signed telemetry summaries each epoch.
- Every telemetry observation MUST be encapsulated in a signed TelemetryEnvelope.
- The system MUST enforce temporal binding: each envelope MUST include block height and monotonic sequence number.
- The system MUST reject envelopes with seq_no <= last_seq_no for the same (producer_id, metric_class) pair.
- The system MUST reject envelopes with height outside current_height +/- 10 blocks.
- All telemetry envelopes MUST be signed using ML-DSA-65 keys.
- Telemetry aggregation MUST produce a single epoch-level summary per metric class.
- Telemetry integrity verification MUST be possible by any node replaying the envelope chain.

### 1.3 Data Structures

```rust
struct TelemetryEnvelope {
    envelope_id: [u8; 32],       // SHA3-256(payload || signature)
    producer_id: [u8; 32],
    metric_class: MetricClass,
    value: u64,                   // encoded metric value (format depends on class)
    height: u64,                  // block height at observation
    seq_no: u64,                  // monotonic per (producer_id, metric_class)
    signature: Vec<u8>,           // ML-DSA-65
}

enum MetricClass {
    FinalityLagMs = 0,       // milliseconds between block time and commit time
    RejectRatioBps = 1,      // basis points (1/10000) of denied plans
    FillRatioBps = 2,        // basis points of mempool utilization
    TxThroughputTps = 3,     // transactions per second
}

struct EpochTelemetrySummary {
    epoch: u64,
    metric_class: MetricClass,
    aggregated_value: u64,
    reporter_count: u32,
    outlier_flags: Vec<OutlierFlag>,
    reconciliation_status: ReconciliationStatus,
    summary_signature: Vec<u8>,   // multi-sig from aggregation quorum
}

struct OutlierFlag {
    producer_id: [u8; 32],
    z_score: f64,                  // deviation from aggregate
    envelope_ref: [u8; 32],       // reference to anomalous envelope
}

enum ReconciliationStatus {
    Consistent,                   // matches independent observation
    DiscrepancyDetected(u8),      // discrepancy percentage
    NotReconcilable,              // no independent observable exists
}
```

### 1.4 State Transitions

**Telemetry submission flow:**
1. Validator or reporting agent observes a protocol metric at a given height.
2. Constructs TelemetryEnvelope with producer_id, metric_class, value, height, seq_no (last_seq_no + 1).
3. Signs envelope with producer's ML-DSA key.
4. Broadcasts envelope via gossip or includes in block (mempool telemetry lane).
5. Receiving nodes validate: signature, height bound, seq_no monotonicity.
6. Valid envelopes stored in SMT (key prefix 0x07) for current epoch.

**Per-epoch aggregation:**
1. At epoch boundary, collect all valid envelopes for this epoch per metric class.
2. Count reporters. If count < M = max(5, committee_size / 10): mark low-confidence.
3. Compute aggregate:
   - FinalityLagMs: median of all values.
   - RejectRatioBps: trimmed mean (discard top and bottom 10% of values).
   - FillRatioBps: trimmed mean (discard top and bottom 10%).
   - TxThroughputTps: median.
4. For each envelope, compute z-score against distribution. Flag if |z| > 3.0.
5. Run reconciliation against independent observables:
   - FinalityLagMs: compare with actual block header timestamps.
   - FillRatioBps: compare with actual mempool admission logs.
   - RejectRatioBps: not reconcilable (internal PDP metric).
6. Publish EpochTelemetrySummary.
7. Prune raw envelopes after 30 days (short-term retention tier).

### 1.5 Failure Behavior

- **Reporter suppression:** Validator stops reporting during incident → suppression detection flag (producer reporting 0 envelopes during active epoch).
- **Metric spoofing:** Producer submits fabricated metric → reconciliation against block headers flags discrepancy. Repeated spoofing triggers anomaly report and potential reputation penalty.
- **Outlier gaming:** Producer submits extreme value to bias trimmed mean → trimmed mean discards top/bottom 10%; outlier flagging for z > 3.0.
- **Low reporter count:** Summary marked low-confidence. Circuit-breaker excludes that metric from automated triggers. Governance alert generated.
- **Reconciliation failure:** Aggregated metric differs from independent observable by >10% → DiscrepancyDetected. Metric excluded from trust calculations for that epoch.
- **Signature failure:** Envelope signature validation fails → envelope discarded.
- **Replay attack:** seq_no <= last_seq_no → envelope rejected.

### 1.6 Versioning and Compatibility

- Telemetry envelope schema version is embedded in the first byte of the payload.
- MetricClass enum is extensible via governance (additive only).
- Aggregation method per metric class is specified in the policy bundle.

### 1.7 Conformance Test Hooks

- Verify telemetry envelope signature validation rejects forged or invalid signatures.
- Verify seq_no replay detection: envelope with stale sequence number rejected.
- Verify height bound: envelope with height outside +/- 10 blocks rejected.
- Verify trimmed mean aggregation correctly discards top/bottom 10%.
- Verify outlier detection flags producers beyond z=3.0.
- Verify reconciliation: finality lag cross-checked against block header timestamps.
- Verify minimum reporter count M = max(5, committee_size / 10) enforced.
- Verify multi-source corroboration: single reporter cannot determine aggregated metric.

### 1.8 Trust-Assumption Inventory

- Reporter key integrity
  - Justification: Telemetry envelope signature validation depends on reporter's public key being correctly bound in protocol state.
  - Trust-minimised alternative: Validator key binding via first-spend reveal (same as account model).
- Independence of reporters
  - Justification: Aggregated telemetry assumes reporters observe independently. Colluding validators can fabricate consensus metrics.
  - Trust-minimised alternative: Random sampling of reporter set per epoch; cross-validation against block header ground truth.
- Aggregation algorithm correctness
  - Justification: Trimmed mean and median are deterministic but discarding outliers may miss genuine anomalies.
  - Trust-minimised alternative: Multi-method aggregation (median + trimmed mean + geometric mean) with cross-verification.

---

## Section 2: Independent Policy Gateway Reconciliation

### 2.1 Purpose

Define the reconciliation protocol that cross-checks aggregated telemetry against independently observable network events.

### 2.2 Normative Behavior

- The system MUST reconcile aggregated telemetry against independently observable on-chain data where possible.
- Finality lag telemetry MUST be cross-checked against actual block header timestamps.
- Fill ratio telemetry MUST be cross-checked against actual mempool admission logs.
- Reject ratio telemetry MUST NOT be self-reconciled (internal PDP metric — no independent observable).
- Discrepancies beyond 10% tolerance MUST trigger reconciliation failure flag.
- Reconciliation results MUST be published in the EpochTelemetrySummary.

### 2.3 Data Structures

```rust
struct ReconciliationReport {
    epoch: u64,
    metric_class: MetricClass,
    aggregated_telemetry_value: u64,
    independent_observable_value: u64,
    discrepancy_pct: f64,           // |agg - obs| / max(agg, obs) * 100
    status: ReconciliationStatus,
    verified_by: Vec<[u8; 32]>,    // validators that verified the reconciliation
}
```

### 2.4 Reconciliation Methods

| Metric | Aggregation Method | Independent Observable |
|--------|-------------------|----------------------|
| FinalityLagMs | Median | Block header timestamps minus commit timestamps |
| FillRatioBps | Trimmed mean (10%) | Mempool admission count / mempool capacity per block |
| RejectRatioBps | Trimmed mean (10%) | None — PDP-internal, no chain-observable equivalent |
| TxThroughputTps | Median | Transaction count per block / block interval |

### 2.5 Failure Behavior

- Finality lag discrepancy >10%: Possible clock skew or fabricated telemetry. Flag for investigation.
- Fill ratio discrepancy >10%: Possible mempool admission discrepancy between nodes. Suggests partition or selective admission.
- Reconciliation not possible (NotReconcilable): Metric excluded from circuit-breaker triggers. Relies on other metrics only.

### 2.6 Versioning and Compatibility

- Reconciliation methodology versioned in the policy bundle.
- Independent observable mappings (which metrics reconcile against which on-chain data) are governance-adjustable.
- Reconciliation discrepancy tolerance (default 10%) is a system parameter stored in protocol state.

### 2.7 Conformance Test Hooks

- Verify finality lag reconciliation matches block header timestamp differences within 10%.
- Verify fill ratio reconciliation matches mempool log analysis within 10%.
- Verify reconciliation discrepancy >10% sets DiscrepancyDetected status.
- Verify non-reconcilable metrics (reject ratio) are marked NotReconcilable.

### 2.8 Trust-Assumption Inventory

- Independent observable availability
  - Justification: Reconciliation depends on independently verifiable on-chain data. Some metrics (e.g., reject ratio) have no independent observable.
  - Trust-minimised alternative: Cross-node telemetry comparison requiring reporter diversity across operator clusters.
- Reconciliation discrepancy tolerance
  - Justification: 10% tolerance may miss subtle manipulation. Tighter tolerances increase false-positive reconciliation failures.
  - Trust-minimised alternative: Governance-adjustable tolerance with per-metric-class granularity.
