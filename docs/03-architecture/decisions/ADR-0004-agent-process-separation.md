## ADR-0004: Agent Runtime Process Separation from Node

**Status:** accepted

**Context:** The agent runtime runs LLM inference, which is non-deterministic, resource-intensive, and potentially vulnerable. It must coexist with the consensus node, which requires deterministic operation and high reliability.

**Decision:** Run the agent runtime and node as separate OS processes communicating through a typed HTTP/gRPC API boundary. The node is Rust. The agent runtime is Ruby/TypeScript. The node API is stateless and cacheable (FR-0072).

**Consequences:**
- Positive: Agent crash does not affect consensus (FR-0071). Node crash does not corrupt agent SQLite state. Compromise of agent process does not grant access to node database (NFR-0028). Independent scaling — multiple agent runtimes can connect to one node. Language choice flexibility.
- Negative: IPC overhead for agent-node communication. Deployment complexity (two services per operator). API serialization overhead. No shared-memory optimizations.

**Alternatives considered:**
- **Single process with runtime isolation (wasm, seccomp):** Rejected because a crash in the sandboxed LLM runtime could still take down the whole process. Language constraints would force agent runtime into Rust.
- **Agent running on separate machine:** Supported by stateless API design (FR-0072) but not required. Local deployment preferred for low latency.

**Related:** FR-0071, FR-0072, FR-0138, NFR-0028, `trust-boundaries.md`.
