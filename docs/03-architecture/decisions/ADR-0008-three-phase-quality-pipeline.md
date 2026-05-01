## ADR-0008: Three-Phase Quality Pipeline

**Status:** accepted

**Context:** Task outputs must be evaluated for quality before rewards are distributed. A pipeline is needed that balances speed (fast payouts for honest work) with robustness (fraud correction for malicious submissions).

**Decision:** Implement a three-phase quality pipeline:

1. **Objective verification:** Deterministic checks (tests pass/fail, reproducibility, policy conformance) produce a normalized pass/fail vector signed by the verifier. Runs automatically on submission.
2. **Independent review market:** Protocol-assigned reviewers (with independence constraints) submit scored verdicts. Reviewers bond collateral; accurate minority opinions earn higher rewards.
3. **Challenge window:** 144-block window after review completion. Anyone can submit challenges with collateral; loser-pays policy. Final settlement only after window closes.

**Consequences:**
- Positive: Automated phase 1 catches deterministic failures cheaply. Phase 2 provides expert evaluation with anti-collusion controls. Phase 3 provides fraud correction. Provisional settlement enables fast payouts with clawback path.
- Negative: Three phases add latency to final settlement (144 blocks = ~24 hours). Reviewer assignment complexity (independence constraints, load caps, fallback logic). Challenge spam defense adds economic cost for legitimate challengers.

**Alternatives considered:**
- **Two-phase (objective + review, no challenge):** Rejected because review alone is insufficient for fraud correction. Collusion or reviewer error could finalize incorrect outcomes.
- **Pure review market (no objective phase):** Rejected because deterministic checks should run first to filter clearly invalid submissions before consuming reviewer attention.

**Related:** FR-0162, FR-0163, FR-0148, `components.md` C12.
