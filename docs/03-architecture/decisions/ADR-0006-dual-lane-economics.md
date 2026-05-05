## ADR-0006: Fee-Market Congestion Control

**Status:** superseded

**Context:** Originally a dual-lane economics model (collaboration + control) was spec'd with 4 mempool lanes (Evidence 15%, Consensus-Control 10%, Governance 10%, Transfer 65%). In practice, no major blockchain uses lane reservation — Bitcoin and Ethereum use a single mempool with fee ordering. Lane reservation wastes 25-35% of mempool capacity under normal load and adds complexity without proven benefit.

**Decision:** Remove mempool lane reservation. The mempool is a single priority queue ordered by fee. Evidence and governance transactions receive governance-set fee discounts to ensure they clear during congestion. EIP-1559 base fee adjustment is the sole congestion mechanism.

**Consequences:**
- Positive: Simpler mempool, no wasted capacity, no dynamic reallocation logic, no lane classification complexity.
- Negative: Evidence/governance transactions compete in the same pool. Fee discounts mitigate this — if they're insufficient, these transactions may be delayed during extreme congestion (governance can adjust discounts).

**Alternatives considered:**
- **Dual-lane reservation (original):** Rejected. Wastes capacity, adds complexity, no major chain uses this.
- **Dynamic lane allocation:** Rejected on same grounds — no proven benefit over fee-ordered pool with targeted discounts.

**Related:** FR-0050, FR-0146, FR-0159, `p2p-wire-spec.md` §2.
