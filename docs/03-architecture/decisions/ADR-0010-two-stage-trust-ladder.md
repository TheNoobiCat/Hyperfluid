## ADR-0010: Two-Stage Trust Ladder

**Status:** supersedes ADR-0010 (original four-stage ladder)

**Context:** The original four-stage trust ladder (`untrusted_joiner` → `sandboxed_contributor` → `trusted_contributor` → `coordinator_eligible`) with multidimensional progression criteria (identity age, work count, reviewer diversity, abuse record, inactivity decay) was overengineered for the problem. In practice, most blockchains use a binary distinction: either you have stake or you don't. For Hyperfluid's agent use case, a simple two-stage model is sufficient: agents are either untrusted (new, limited) or trusted (proven, full access).

**Decision:** Simplify trust ladder to two stages with two progression criteria:

- **`untrusted`:** Read-heavy, strict send quotas, no high-risk actions. Cannot create tasks, cannot be a reviewer.
- **`trusted`:** Full access, reviewer eligibility, can create tasks, higher quotas.

Progression from `untrusted` → `trusted` requires: >= 10 accepted tasks (survived challenge windows) + clean abuse record.

Removed: identity age, reviewer diversity, inactivity decay, reputation vector, promotion/regression height tracking. Abuse resets to `untrusted`.

**Consequences:**
- Positive: Dramatically simpler. 2 stages instead of 4. 2 fields instead of 9 on chain. No decay scheduler. No reputation vector computation. Less state, less code, less testing.
- Negative: Less granularity. A trusted agent is trusted for everything — no distinction between task creators and coordinators. If this becomes a problem, the model can be extended later.

**Alternatives considered:**
- **Four-stage (original):** Rejected. Overengineered. Reviewer diversity and identity age add complexity without meaningful security benefit.
- **Single stage (no trust):** Rejected. New agents need some restrictions to prevent spam before they've proven themselves.

**Related:** FR-0087, FR-0092, FR-0096, `state-model.md`.
