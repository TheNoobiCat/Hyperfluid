# Checkpoint — 2026-05-05 (Pre-Stage-01 Amendment)

**Completed:** Agent tool expansion (5→9 tools), seed index creation, ADR-0013.

## Summary

### Agent Tools Expanded
The core agent tool set grew from 5 to 9 by adding `read`, `edit`, `write`, `apply_patch`. These structured file-access tools replace raw bash for common file operations, reducing error rates and token cost.

**Why:** Original 5-tool set forced agents to use bash for all file access. This was error-prone (whole-file rewrites), verbose, and harder to sandbox per-operation. ADR-0013 documents the tradeoffs.

### Seed Index Created
Physical `/ideas/` directory created at project root with `_template.md` for idea quality/structure. The directory is intentionally empty during build — maintainer populates manually after build is operational.

**Why:** The seed idea index (FR-0084) existed as a concept in research docs but had no physical representation or CLI discovery mechanism. Agents can now discover ideas via `hyperfluid idea list` and read them via `hyperfluid idea get <slug>`.

### Files Created
- `/ideas/README.md` — seed index purpose and usage
- `/ideas/_template.md` — seed idea format requirement
- `docs/03-architecture/decisions/ADR-0013-expanded-agent-tools-and-seed-index.md`

### Files Modified
| File | Change |
|------|--------|
| `docs/01-research/agents/agent-tools-spec.md` | +4 tool schemas, +`hyperfluid idea` CLI, +Tradeoff 5, updated pseudocode, updated Section 9/10 |
| `docs/04-specifications/runtime/agent-runtime-spec.md` | Section 2: expanded to 9 tools, +data structures, +failure modes, +conformance hooks |
| `docs/04-specifications/runtime/collaboration-spec.md` | Section 1.1: references physical `/ideas/` folder |
| `docs/01-research/agents/collaboration-layer-parallel-teams.md` | Section 4: updated Idea Seed Index to mention `/ideas/` folder + CLI |
| `docs/02-requirements/runtime/FR-0061-0075-agent-runtime.md` | FR-0062: 5→9 tools; FR-0068: +`hyperfluid idea` CLI |
| `docs/03-architecture/index.md` | ADR-0013 registered in ADR table |
| `docs/08-handoff/latest/build-status.md` | Pre-Stage-01 amendment table added |

### Verification
- All 14 specs remain valid (no structural changes to existing sections)
- Seed ideas not yet added (intentionally empty — maintainer adds later)
- No code files changed (spec/design amendments only)

**Next:** Stage 01 (Protocol Core) — Build Minimum Viable Chain. No downstream blockers.

**Open Questions:** None.
