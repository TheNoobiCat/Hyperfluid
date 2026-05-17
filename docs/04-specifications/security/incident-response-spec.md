# Incident Response & Congestion Control

**Components:** C4 Governance (Post-Incident Bridge)
**Covered FRs:** FR-0144, FR-0145
**Dependencies:** C1 Consensus Engine, C2 State Machine

---

## Section 1: Congestion Response

### 1.1 Purpose

Define the EIP-1559 base fee adjustment formula for congestion control. No emergency mode, degraded mode, or circuit-breaker hierarchy exists — the single dynamic base fee is the sole congestion mechanism.

### 1.2 Normative Behavior

- The system MUST adjust the base fee up or down each block based on block utilization.
- Base fee MUST increase by at most 12.5% per block when utilization > target.
- Base fee MUST decrease by at most 12.5% per block when utilization < target.
- No other automatic congestion mode is enforced at the protocol level.
- The system MUST NOT freeze, filter, or restrict any transaction type based on congestion state.
- Agent runtimes MAY implement local rate limiting, circuit breakers, or spam filters for their own UX — these are local operator concerns, not protocol-enforced.

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
