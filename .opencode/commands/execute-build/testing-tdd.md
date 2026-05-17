**Testing (TDD — Red → Green → Refactor → Recurse):**
- The spec's Section X.7 Conformance Test Hooks ARE the TDD stories.

**Parallelism (hook-group level):**
- Collect all conformance hooks from the relevant spec sections.
- Group hooks by spec section (same section = share types/can conflict → same worker).
- For each group, launch a `build-worker` subagent with:
  - The spec section's X.7 hooks (text)
  - Target crate path
  - Existing test patterns and file structure
  - Each worker runs RED → GREEN → REFACTOR internally per hook in its group
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
