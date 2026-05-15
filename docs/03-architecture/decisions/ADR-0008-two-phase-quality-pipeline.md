## ADR-0008: Two-Phase Quality Pipeline

**Status:** supersedes ADR-0008 (original three-phase pipeline)

**Context:** The original three-phase pipeline had objective checks (Phase 1), independent review (Phase 2), and challenge window (Phase 3). In practice, the objective checks are redundant — task verification is deterministic by the task spec itself, and the reviewer set can check the same things. Quality-weighted payout formulas add complexity without clear benefit over fixed payout. The 4 independence constraints (operator cluster, temporal spread, stake spread, pair frequency) overengineer reviewer selection — one constraint (no same-operator reviewers, detectable via stake-graph cluster IDs) is sufficient.

**Decision:** Simplify review to 2 phases with fixed payout:

1. **Independent review:** Protocol-assigned reviewers (min 3) with one independence constraint: no reviewer shares an operator cluster with any other reviewer or the worker (verified via stake-graph analysis). Fixed payout: 90% of escrowed bounty goes to the worker, 10% split equally among all reviewers who submit a timely verdict (approve or deny). Verdict-independent payout eliminates approval bias.
2. **Challenge window:** 144-block window after review completion. Anyone can submit a challenge with 20% bond. If challenge succeeds, challenger receives the bond + reward from clawed-back worker and incorrect reviewer shares. If challenge fails, bond is burned.

Removed: Phase 1 (objective checks), quality-weighted payout, clawback mechanism, temporal spread constraint, stake spread constraint, pair frequency cap, reputation decay/inactivity tracking.

**Consequences:**
- Positive: Simpler. Fewer constraints, no quality formula, no reputation decay. Lower barrier for reviewers. Less state, less code.
- Negative: No quality differentiation in payouts — reviewers are paid equally regardless of thoroughness. If this becomes a problem, reviewer reputation (strike count only) can be added later.

**Alternatives considered:**
- **Three-phase (original):** Rejected. Objective checks are redundant with reviewer verification. Quality-weighted payout is overengineered — fixed bounty split is simpler and less gameable.
- **Single-phase (review only, no challenge):** Rejected. Challenge window is essential for fraud correction — without it, a majority of colluding reviewers could steal bounties.

**Related:** FR-0161, FR-0148, FR-0149. Payout split superseded by ADR-0017 (90/10 with verdict-independent reviewer payout). Note: "clawback mechanism" in the removed list refers to quality-weighted scoring clawback. Challenge-based clawback (FR-0172) is retained and is a distinct mechanism.
