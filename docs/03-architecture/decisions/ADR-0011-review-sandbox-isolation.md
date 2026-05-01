## ADR-0011: Review Sandbox Isolation

**Status:** accepted

**Context:** Both governance proposal review and fast-path topic merge review require agents to evaluate proposals. If reviewers use their main agent context, prompt injection payloads in the proposal could influence the review decision. Review must be isolated from the reviewer's normal agent state.

**Decision:** Execute all governance and topic merge reviews in isolated sandbox subprocesses with:

- **Fresh context:** No access to main agent's messages, knowledge, todos, or history
- **Single tool:** Only `review(decision: approve|deny, reason)` tool available
- **Fixed system prompt:** Reviewer identity + proposal content only
- **Timeout:** 30 minutes; timeout = no vote (not penalized per FR-0018)
- **Deterministic lifecycle:** Main agent branch pauses during sandbox; sandbox termination resumes main branch deterministically
- **Deterministic precheck first:** Bundle/object verification and merge checks run before sandbox launch; failures short-circuit (FR-0027, FR-0035)

**Consequences:**
- Positive: Prevents prompt injection from influencing review decisions. Review decisions are based on proposal content alone, not reviewer's accumulated biases. Deterministic precheck saves computation on obviously invalid proposals. Timeout semantics prevent Review Sandbox stalls from blocking the reviewer agent indefinitely.
- Negative: Reviewers cannot use accumulated domain knowledge during review. Startup latency for sandbox creation (NFR-0007: P99 < 2s). Single-tool constraint limits depth of review.

**Alternatives considered:**
- **Review in main agent context:** Rejected because prompt injection in proposal content could manipulate the reviewer. Taint tracking (FR-0114) helps but does not fully isolate.
- **Fully automated review (no agent):** Rejected because some review dimensions (design quality, architecture coherence) require agent-level reasoning. Automated objective checks (Phase 1 of FR-0163) cover deterministic validation.

**Related:** FR-0026, FR-0087, NFR-0007, `trust-boundaries.md` Section 6.
