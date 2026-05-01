## ADR-0009: EIP-1559 Fee Market

**Status:** accepted

**Context:** The network needs a transaction fee mechanism that prevents spam, provides efficient price discovery, and compensates validators without perverse incentives. Fixed-fee models allow cheap spam; auction-only models have unpredictable pricing.

**Decision:** Implement an EIP-1559 style dynamic fee market with:

- **Base fee:** Adjusts per-block based on prior block utilization (target 50% full). Max 12.5% increase per block. Base fee is burned (deflationary pressure).
- **Priority fee:** Optional tip for faster inclusion, paid to block proposer.
- **Minimum fee floor:** Prevents total fee collapse during low demand.
- **Staked validator rebates:** Fee rebates distributed to staked validators proportionally each epoch.
- **Per-sender mempool limits:** Prevent single sender from filling blocks.

**Consequences:**
- Positive: Proven anti-spam mechanism. Efficient price discovery without auction overhead. Fee burn provides deflationary pressure and aligns incentives. Per-sender limits prevent fee market manipulation.
- Negative: Base fee volatility during demand spikes (capped at 12.5% per block). Fee burn reduces validator revenue compared to pure-tip model. Requires accurate block utilization estimation.

**Alternatives considered:**
- **Fixed fee:** Rejected because it provides no spam deterrence at scale and no congestion-adaptive pricing.
- **Pure auction (first-price):** Rejected because it creates inefficient price discovery and MEV incentives. EIP-1559 base fee provides predictable minimum cost.
- **Free transactions with quota only:** Rejected because quota alone cannot prevent spam without economic cost. Fee + quota provides layered defense.

**Related:** FR-0146, FR-0147, FR-0159, NFR-0013, `components.md` C5.
