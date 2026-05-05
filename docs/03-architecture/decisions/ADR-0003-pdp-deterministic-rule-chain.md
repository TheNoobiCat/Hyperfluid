## ADR-0003: Policy Decision Point — Simplified Rule Chain

**Status:** accepted (amended 2026-05-06)

**Context:** All network-mutating actions from agents must be validated before execution. The original 10-step PDP rule chain included role/trust checks, ACL checks, taint tracking, risk step-up, and plan binding hash verification — all designed to prevent LLM prompt injection attacks from reaching the protocol. In practice, these checks duplicate what the agent runtime should do locally. The protocol should only enforce basic admission: schema, signature, replay, quota, fee. LLM safety is the agent runtime's job.

**Decision:** Implement the PDP as a 5-step rule chain:

1. Schema validation
2. Signature verification (ML-DSA-65)
3. Replay protection (plan_id, nonce, TTL)
4. Quota check (cross-layer quota matrix)
5. Fee check (sufficient balance for EIP-1559 fees)

Removed: policy bundle match, role/trust check, ACL check, taint check, risk step-up, plan binding hash, RiskClass enum. None of these are the protocol's concern — the agent runtime enforces its own safety policies locally.

**Consequences:**
- Positive: Simpler, faster. Protocol does not police LLM output. Agent runtime owns its own safety. Fewer DenyReasons (5 vs 12), less state, less code.
- Negative: A compromised agent can submit any validly-signed transaction with sufficient fees. Protocol no longer provides intent-based filtering. Agent runtime sandboxing becomes the sole defense against malicious agent behavior.

**Alternatives considered:**
- **10-step original chain:** Rejected. Protocol should not be in the business of LLM safety.
- **Pure permissionless (no PDP):** Rejected. Basic admission (signature, replay, quota, fee) is always required.

**Related:** FR-0106, FR-0107, FR-0108, FR-0110, `component-model/components.md` C9.
