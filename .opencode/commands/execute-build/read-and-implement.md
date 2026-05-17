Read:
1. Current week's stage file — enumerate task list.
2. All specifications referenced for this week.
3. `docs/08-handoff/latest/build-status.md` (what's already done).

**Delegation pattern (task-level parallelism):**
- Group tasks by **independence**:
  - Different crate + different spec section = independent (can parallelize)
  - Same crate = serial (shared types, merge conflicts)
  - Same spec section, different crate = depends on crate coupling
- For each independent task group, launch a `build-worker` subagent with:
  - Relevant spec section text (not the full spec file — the section only)
  - Target crate path
  - Reference to existing code conventions (error types, module structure)
  - Result format: `[files_changed, summary, passes, cross_cutting_concerns]`
- Wait for all workers. Collect results.
- **Post-aggregation cross-cutting check** (run locally, not in a subagent):
  - `cargo check --workspace` — catches broken intra-workspace deps
  - `grep` for any type/field name that appears in two changed files but differs (type drift)
  - If a worker flagged a `cross_cutting_concern`, inspect manually

Implement serial tasks directly. Implement delegated tasks via subagents. Follow the spec exactly.
