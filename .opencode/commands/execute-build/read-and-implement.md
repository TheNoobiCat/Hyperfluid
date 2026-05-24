Read:
1. Current week's stage file — enumerate task list.
2. All specifications referenced for this week.
3. `docs/08-handoff/latest/build-status.md` (what's already done).

**No-deferral gate (from no-deferral.md):**
Before grouping tasks, classify every task in the week's stage file:
- BUILDABLE: can be built now → MUST appear in the delegation plan
- BLOCKED: dependency doesn't exist → document exact file:line that's missing
Every BUILDABLE task MUST be built. Every BLOCKED task MUST have a file:line citation.
If you cannot produce the citation, the task is BUILDABLE and must be built.

**Model awareness:** The orchestrator (this agent) runs on a powerful pro model. Subagents
(`build-worker`) run on `deepseek-v4-flash` — a smaller, cheaper model with lower intelligence and less power.
The worker will NOT infer, extrapolate, or "figure out" missing context.
Prompts must be fully specified: explicit types, exact file paths, concrete behavior, zero
ambiguity. Treat each prompt as code for a function-caller, not instructions for a thinking
assistant.

**Delegation pattern (task-level parallelism):**
- Group tasks by **independence**:
  - Different crate + different spec section = independent (can parallelize)
  - Same crate = serial (shared types, merge conflicts)
  - Same spec section, different crate = depends on crate coupling
- For each independent task group, launch a `build-worker` subagent. The prompt MUST include
  ALL of the following (the worker will NOT infer missing items):

  1. **Exact files to read** — absolute paths (worker will not search for them)
  2. **Spec section text** — verbatim MUST/SHOULD/MAY behavior excerpts
  3. **Target crate path** — exact filesystem path(s) to modify
  4. **Exact types to use** — concrete type names, fields, signatures (e.g. `Vec<u8>`, not "a byte sequence")
  5. **Exact module path** — e.g. `crates/hyperfluid-storage/src/journal/wal.rs`, not "storage crate"
  6. **Existing patterns to follow** — 1-2 reference implementations (file + line)
  7. **Error handling convention** — which error type / variant to use or create
  8. **Testing convention** — test file path, naming pattern, which X.7 hooks to cover
  9. **Result format** — the `[files_changed, summary, passes, cross_cutting_concerns]` schema
  10. **"Do not" boundary** — what the worker must NOT touch (e.g. "do not add dependencies")

- Wait for all workers. Collect results.
- **Post-aggregation cross-cutting check** (run locally, not in a subagent):
  - `cargo check --workspace` — catches broken intra-workspace deps
  - `grep` for any type/field name that appears in two changed files but differs (type drift)
  - If a worker flagged a `cross_cutting_concern`, inspect manually

Implement serial tasks directly. Implement delegated tasks via subagents. Follow the spec exactly.
