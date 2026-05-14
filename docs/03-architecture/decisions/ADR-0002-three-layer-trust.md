## ADR-0002: Three-Zone Security Architecture

**Status:** accepted

**Context:** Hyperfluid includes both a deterministic protocol core and an LLM-powered agent runtime. The LLM output is inherently non-deterministic and potentially vulnerable to prompt injection. A security architecture is needed that prevents agent compromise from affecting protocol safety.

**Decision:** Implement a three-zone trust architecture:

1. **Zone 1 (Trusted):** Protocol core — deterministic, replicated, no external dependencies. Can read/write SMT state. No LLM involvement.
2. **Zone 2 (Semi-Trusted):** Policy Decision Point — deterministic rule chain that gates all agent-to-protocol mutations. Runs in node process but logically isolated.
3. **Zone 3 (Untrusted):** Agent runtime — runs LLMs, executes bash, loads skills. Separate OS process. All network mutations must pass through Zone 2.

**Consequences:**
- Positive: Agent compromise cannot affect consensus safety (FR-0138, NFR-0028). Deterministic policy gating prevents prompt injection from executing network actions (FR-0121). Process separation enables independent language choices (Rust for node, Ruby/TypeScript for agent).
- Negative: Added IPC overhead for every network-mutating action. Additional deployment complexity. Two codebases to maintain.

**Alternatives considered:**
- **Monolithic agent-node:** Rejected because agent crash would stall consensus, and compromised agents could corrupt protocol state.
- **ML-based policy boundary:** Rejected as primary gate because probabilistic classifiers can be bypassed via adversarial inputs (FR-0122). ML used only as auxiliary signal.

**Related:** FR-0071, FR-0106, FR-0121, FR-0138, NFR-0028, `trust-boundaries.md`.
