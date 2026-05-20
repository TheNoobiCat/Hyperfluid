**Testing (TDD — Red → Green → Refactor → Recurse):**
- The spec's Section X.7 Conformance Test Hooks ARE the TDD stories.

**Model awareness:** Subagents (`build-worker`) run on `deepseek-v4-flash` — a smaller, cheaper
model with limited reasoning and **no autonomy**. The worker will NOT infer missing context.
Every prompt must be fully specified with exact file paths, concrete type names, and explicit
behavior expectations. See `read-and-implement.md` for the template.

**Parallelism (hook-group level):**
- Collect all conformance hooks from the relevant spec sections.
- Group hooks by spec section (same section = share types/can conflict → same worker).
- For each group, launch a `build-worker` subagent. The prompt MUST include:
  1. **Exact test files to read** — absolute paths to existing test patterns
  2. **X.7 hooks (verbatim)** — the conformance test hooks from the spec
  3. **Target crate path** — exact filesystem path(s) to modify
  4. **Exact types under test** — concrete type names, function signatures
  5. **Testing conventions** — test file path, naming pattern (`conforms_to_*`)
  6. **Existing pattern reference** — 1-2 analogous test files (file + line)
  7. **Positive/negative/edge requirement** — each hook needs all three
  8. **Result format** — `[files_changed, summary, passes, cross_cutting_concerns]`
  9. **"Do not" boundary** — what the worker must NOT touch
- Wait for all workers. Collect results.
- **Post-aggregation steps** (run locally, not in subagent):
  - `cargo test --workspace` — catches test interaction issues between workers
  - **RECURSE:** grep for the SAME bug pattern across ALL crates. If found, fix inline or file issue.
  - Verify no test shims leaked into library code (grep `thread_local!`, `RefCell`, mock patterns in `crates/*/src/`)

**Production code vs test code boundary:**
- Library code (`src/*.rs`) MUST use real implementations: real randomness (`getrandom`), real network I/O, real crypto.
- Test code (`#[cfg(test)]` modules, `tests/` files) MAY use deterministic seeds, mocks, and in-memory simulations.
- NEVER put test shims, deterministic RNGs, `thread_local!` globals, or mock implementations in library code and call it "done".
- If a conformance test requires deterministic behavior, put the deterministic logic in the test, not the library.
- The default feature flag MUST ship production-ready code, not mocks. Mocks are opt-in only.
- A `// SPEC_DEVIATION` comment is NOT a free pass to ship test-quality code as production code. If the deviation makes the component unusable in production, it must be tracked as a blocker, not a completed task.
- **Every conformance test must have at least one NEGATIVE assertion** (wrong input → correct rejection). If the hook only tests positive behavior, add an explicit edge-case subtest.
- `cargo test` before any implementation must fail on the new test.
- Test file convention: `crates/<component>/tests/` mirroring spec sections.
- Test naming: `conforms_to_<spec>_<section>_<short_description>`.
- Conformance matrix entries are derived from these tests (they ARE the evidence).
