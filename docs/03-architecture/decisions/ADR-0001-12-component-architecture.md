## ADR-0001: 12-Component Architecture Decomposition

**Status:** accepted

**Context:** The Hyperfluid system encompasses consensus, networking, agent runtime, collaboration, security, and economics. A clear decomposition is required to define component boundaries, interfaces, and responsibilities. Without it, requirements cannot be mapped to implementation units.

**Decision:** Decompose Hyperfluid into 12 components organized across three architectural layers:

- **Protocol Core (C1-C5):** Consensus Engine, State Machine & SMT, Staking & Validator Manager, Governance Engine, Fee Market
- **Protocol Services (C6-C8):** Fast-Path Topic Protocol, P2P Networking & Connection Manager, Artifact Availability & Storage
- **Security Boundary (C9):** Policy Decision Point
- **Agent Runtime (C10-C11):** Agent Runtime, Collaboration & Inbox Layer
- **Economics (C12):** Economics & Incentives

**Consequences:**
- Positive: Every FR maps to exactly one primary component. Components have single clear responsibility. Independent scaling, testing, and replacement. Acyclic dependency graph.
- Negative: More interfaces to document and version (11 major interfaces). Interface proliferation risk under rapid evolution.

**Alternatives considered:**
- **5-6 coarse components:** Rejected because it creates god-components with mixed responsibilities (e.g., "Protocol" would combine consensus, staking, governance, and fees).
- **20+ fine components:** Rejected because excessive fragmentation would create coordination overhead without clear benefit. 12 components maps naturally to the requirement domains.

**Related:** All FR/NFR documents, `components.md`.
