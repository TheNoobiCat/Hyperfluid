## ADR-0017: 90/10 Payout Split with Verdict-Independent Reviewer Compensation

**Status:** accepted

**Context:** The payout model had two contradictory descriptions across the doc set. The review-engine-spec.md said "bounty is split equally among approving reviewers" while the collaboration-spec.md and FR-0080 said "reviewers are paid via the review market, not from the task bounty." Neither specified a concrete split ratio, and paying reviewers only on approval created an approval-bias incentive: reviewers who correctly deny bad work earn nothing, while rubber-stampers earn every time.

**Decision:**

1. **90/10 split:** On successful completion, review, and challenge window, 90% of the escrowed task bounty goes to the worker. 10% is split equally among all reviewers who submitted a timely verdict.

2. **Verdict-independent payout:** Reviewers are paid for submitting a timely verdict (approve or deny), not for approving. This eliminates the rubber-stamping incentive. Correctness is enforced through the challenge window — if a challenge succeeds, incorrect reviewers are clawed back.

3. **Settlement is never provisional:** "Provisional settlement" language is removed. Funds remain in escrow until the challenge window closes. At that point, a single atomic transfer distributes 90% to the worker and 10% to timely reviewers. If a challenge succeeds before window close, the challenger is rewarded from clawed-back shares and remaining funds return to the funder.

**Consequences:**
- Positive: No approval bias — reviewers are paid for doing the work, not for agreeing.
- Positive: No separate review market pool needed — simplifies the economics crate.
- Positive: Single escrow flow — funds move exactly once (at final settlement), eliminating the need for locked/spendable balance states.
- Positive: Simple split math — fewer reviewers each get a larger slice of the 10%. Proportional reward-cap downgrade in scarcity fallbacks is straightforward.
- Negative: 10% pool with 3 reviewers is ~3.33% each. For small bounties (< 10 AGX), reviewer payout may not justify the work. Mitigation: reviewer collateral scaled to bounty percentage keeps the economics aligned.

**Alternatives considered:**
- **Separate review market pool (previous design):** Rejected. Requires protocol issuance or inflation to fund reviewers, contradicts the genesis-only mint (FR-0153a).
- **Approval-only payout (previous design):** Rejected. Creates approval bias that rewards rubber-stamping.
- **Quality-weighted adaptive rewards:** Rejected by ADR-0008 as overengineered. Fixed payout is simpler.
- **85/15 or 95/5 split:** Not materially different. 90/10 is round, intuitive, and splits the difference.

**Related:** FR-0153, FR-0161, FR-0172, review-engine-spec.md, collaboration-spec.md, ADR-0008.
