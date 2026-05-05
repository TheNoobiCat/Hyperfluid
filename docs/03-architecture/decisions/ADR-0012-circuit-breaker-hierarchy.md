## ADR-0012: Congestion Response via EIP-1559 Base Fee

**Status:** superseded

**Context:** Originally a three-tier circuit-breaker escalation hierarchy (Normal/Degraded/Emergency) was spec'd. In practice, no major blockchain uses a multi-tier circuit breaker — the complexity of 22+ thresholds, persistence windows, hysteresis, reporter quorums, and sub-circuit-breakers creates oscillation risk and false-positive mode switches without proven benefit. Ethereum's EIP-1559 (single-parameter base fee adjusting 12.5% per block) has been proven in production since 2021.

**Decision:** Remove the circuit-breaker system entirely. Congestion response is handled by the EIP-1559-style base fee adjustment built into the fee market (see `fee-market-spec.md`). No emergency mode, degraded mode, or frozen actions exist at the protocol level. Agent runtimes may implement their own local rate limiting.

**Consequences:**
- Positive: Dramatically simpler. No mode flapping, no hysteresis tuning, no reporter quorums, no staged recovery ramps. One parameter (base fee) is provably sufficient for congestion management.
- Negative: No protocol-level emergency freeze for coordinated attacks. However, the fee market naturally prices out spam under load — if a spammer can afford to pay, they're economically beneficial to the network.

**Alternatives considered:**
- **Three-tier circuit breaker (original):** Rejected. Overengineered for a problem that EIP-1559 solves with one parameter.
- **Two-tier (normal/emergency):** Rejected on same grounds — the complexity of defining objective emergency conditions and exit criteria is not justified by the threat model.

**Related:** FR-0148, FR-0149, FR-0150, `fee-market-spec.md`.
