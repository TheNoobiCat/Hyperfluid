# Security Spec: Incident Response

**Components:** C4 Governance (Post-Incident Bridge)
**Covered FRs:** FR-0142, FR-0143, FR-0144, FR-0145
**Dependencies:** C1 Consensus Engine, C2 State Machine

---

## Section 1: Congestion Response

### 1.1 Purpose

Define the protocol's congestion response. Replaces the previous multi-tier circuit-breaker system with a single EIP-1559-style dynamic base fee — the only congestion control mechanism the protocol needs.

### 1.2 Normative Behavior

- The system MUST adjust the base fee up or down each block based on block utilization.
- Base fee MUST increase by at most 12.5% per block when utilization > target.
- Base fee MUST decrease by at most 12.5% per block when utilization < target.
- No other automatic congestion mode is enforced at the protocol level.
- The system MUST NOT freeze, filter, or restrict any transaction type based on congestion state.
- No emergency mode, degraded mode, or circuit-breaker exists in the protocol.
- Agent runtimes MAY implement local rate limiting, circuit breakers, or spam filters for their own UX — these are local operator concerns, not protocol-enforced.

### 1.3 Data Structures

```rust
struct IncidentRecord {
    incident_id: [u8; 32],         // SHA3-256 of trigger evidence
    declared_at_height: u64,
    resolved_at_height: u64,       // 0 if active
    trigger_evidence_ref: [u8; 32],
}
```

### 1.4 State Transitions

Base fee adjustment per block:
```
if block_utilization > target_utilization:
    base_fee = base_fee * (1 + (excess / target) * 0.125)
else:
    base_fee = base_fee * (1 - (shortfall / target) * 0.125)
base_fee = max(base_fee, minimum_base_fee)
```

### 1.5 Failure Behavior

- **Base fee spike under high demand:** Expected behavior. Base fee rises until demand subsides. Priority fee allows faster inclusion during spikes.
- **Base fee near zero under low demand:** Minimum base fee floor prevents zero-fee spam.
- **Governance override of base fee:** Possible via standard governance proposal.

### 1.6 Versioning and Compatibility

- Base fee adjustment formula is pinned by `git:head`.
- Minimum base fee and target utilization are governance-adjustable parameters.

### 1.7 Conformance Test Hooks

- Verify base fee increases by max 12.5% per block when utilization exceeds target.
- Verify base fee decreases by max 12.5% per block when utilization is below target.
- Verify minimum base fee floor enforced.
- Verify no transaction type is filtered or frozen at any fee level.

### 1.8 Trust-Assumption Inventory

- Base fee formula correctness
  - Justification: EIP-1559 proven on Ethereum mainnet since 2021.
  - Trust-minimised alternative: None — the formula is trivially verifiable.
