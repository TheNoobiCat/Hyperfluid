## ADR-0006: Dual-Lane Economics (Collaboration + Control)

**Status:** accepted

**Context:** The network must support both high-volume collaboration traffic (task claims, messages, merges) and safety-critical control traffic (evidence, governance, emergency signals). Spam in one traffic class must not stall the other.

**Decision:** Implement dual-lane economics with explicit separation:

- **Collaboration lane:** Low baseline cost, trust-adjusted quotas, subject to circuit-breaker tightening. Handles task operations, topic merges, messages.
- **Control lane:** Reserved mempool capacity (35% total: evidence 15%, consensus-control 10%, governance 10%), higher collateral requirements, always available regardless of circuit-breaker state.

**Consequences:**
- Positive: Safety-critical operations survive spam floods (NFR-0008 requires >80% baseline throughput under 10x malicious sender ratio, with critical lanes at 100%). Gas market pricing differentiates collaboration from control. Governance lane never starved.
- Negative: More complex fee market with separate congestion handling per lane. Control lane reservation reduces throughput for collaboration during normal operation.

**Alternatives considered:**
- **Single unified lane:** Rejected because spam can starve governance and evidence. EIP-1559 fee alone cannot distinguish safety-critical from non-critical traffic.
- **Dynamic lane allocation:** Rejected because it could be gamed to expand collaboration lane at expense of control lane during attack. Dynamic reallocation only moves toward critical lanes, never away (FR-0050).

**Related:** FR-0050, FR-0152, NFR-0008, `components.md` C5.
