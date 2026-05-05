## ADR-0013: Expanded Agent Tool Set, CLI Seed Index Discovery, and Seed-Centric Task Model

**Status:** accepted

**Context:** The original agent tool set (5 tools: bash, todo_write, todo_update, remember, forget) was designed for minimalism but lacked structured file-access primitives. Agents resorted to raw bash for all file operations (read, edit, write), which is error-prone, verbose, and hard to sandbox. Separately, the seed idea index (FR-0084) existed as a concept in research documents but had no physical representation or CLI discovery mechanism — agents could not enumerate available work ideas without manual guidance.

Further design discussion resolved three additional model decisions:
- **Seed ideas are abstract topic buckets**, not individual tasks. One seed hosts many tasks.
- **All tasks MUST reference a seed idea via `seed_ref`**. No orphan tasks. New seeds enter via `git:head` governance proposals.
- **Single-agent per task**: no team formation, no subtask splitting. Reviewers are independent and paid via the review market (FR-0161), not from the task bounty.

**Decision:**
1. Expand the core tool set from 5 to 9 tools by adding: `read`, `edit`, `write`, `apply_patch`.
2. Create a physical `/ideas/` directory at the project root containing individual markdown seed idea files and a `_template.md`.
3. Add `hyperfluid idea` CLI subcommand (`list`, `get`) so agents can discover and inspect seed ideas.
4. Enforce that all tasks reference a canonical seed idea via `seed_ref` (PDP-enforced, no orphan tasks).
5. New seed ideas enter via `git:head` governance proposals carrying the `.md` file.
6. The airdrop agent creates many small tasks per seed at genesis, distributing AGX broadly.
7. Eliminate team formation and subtask splitting — each task is single-agent.

**Consequences:**
- Positive: Structured file tools are safer than raw bash for file operations. `edit` with exact-string replacement prevents whole-file rewrite errors. `apply_patch` reduces token cost for multi-file changes. The seed index becomes discoverable by agents autonomously.
- Positive: The `/ideas/` directory is intentionally empty during build — the maintainer populates it manually after build.
- Positive: Seed ideas follow a `_template.md` for consistent quality and structure.
- Positive: Requiring `seed_ref` prevents topic sprawl and ensures every task is anchored to a governance-reviewed problem domain.
- Positive: Single-agent tasks simplify coordination, make bounty distribution deterministic, and eliminate team-role complexity.
- Negative: Tool surface grows from 5 to 9, increasing system prompt footprint by ~200 tokens.
- Negative: File-access tools expand the sandbox attack surface — path traversal and symlink-escape protections required.
- Negative: Requiring seed_ref means agents cannot create ad-hoc tasks without first proposing a seed via governance — latency for entirely novel work domains.

**Alternatives considered:**
- Keep 5 tools, route all file access through bash: rejected because bash is error-prone and harder to sandbox per-operation.
- Add only read+write without edit/apply_patch: rejected because exact-string replacement is essential.
- Place seed index on-chain (C8): rejected because seed ideas are local bootstrap metadata, not protocol state.
- Allow "entirely novel" tasks without seed_ref: rejected — leads to topic sprawl and undermines the seed index as the canonical work taxonomy.
- Multi-agent team formation for complex tasks: rejected — unnecessary complexity at launch; single-agent with independent reviewers covers all cases. Large work is decomposed into multiple independent tasks under the same seed.

**Related:** FR-0062, FR-0068, FR-0080, FR-0084, FR-0088, FR-0192, agent-tools-spec.md, agent-runtime-spec.md, collaboration-spec.md, user-task-submission-and-sponsorship.md
