---
description: "Execute the current build task from the stage plan"
---
Read `BUILD-SYSTEM.md` (Layer 5, Layer 8), `TEMPLATES.md` (Checkpoint contract), then the latest handoff (`docs/08-handoff/latest/`, prioritising most recent `checkpoint-*.md` and `build-status.md`). If previous agent left unfinished work, complete it first.

Read:
1. Current week's stage file.
2. The specification referenced for that week.
3. `docs/08-handoff/latest/build-status.md` (what's already done).

Implement in the appropriate crate/directory. Follow the spec exactly.

**When a spec is ambiguous or contradictory:**
1. Do not guess. Check `docs/08-handoff/latest/open-questions.md` — if a ruling exists, follow it.
2. If no ruling exists, make a decision, document it as an ADR in `docs/03-architecture/decisions/`, and file a spec change request.
3. Implement with a `// SPEC_DEVIATION: [reason]` comment.

**When a requirement gap is discovered mid-implementation:**
1. STOP. Do not paper over the gap.
2. File the gap in `docs/08-handoff/latest/open-questions.md` with:
   - The spec section that is underspecified
   - What is missing
   - Whether it blocks the current task or can be deferred
3. If it blocks, escalate to spec revision. Track in `PROJECT-STATUS.md`.

**Stop rule (trust assumptions):**
If you discover a trust assumption or centralised dependency not listed in the spec's trust-assumption inventory:
1. STOP implementation of that component.
2. File in `open-questions.md`.
3. Escalate to spec revision before continuing.

**Testing (TDD — Red → Green → Refactor → Recurse):**
- The spec's Section X.7 Conformance Test Hooks ARE the TDD stories.
- For each hook, in order:
   1. **RED:** Write a failing test that asserts the hook's behavior.
   2. **GREEN:** Write minimum code to make the test pass.
   3. **REFACTOR:** Clean up while keeping the test green.
   4. **RECURSE:** After GREEN, grep for the SAME bug pattern across ALL crates. If found, file a new issue or fix inline before moving to the next hook.
- **Every conformance test must have at least one NEGATIVE assertion** (wrong input → correct rejection). If the hook only tests positive behavior, add an explicit edge-case subtest.
- `cargo test` before any implementation must fail on the new test.
- Test file convention: `crates/<component>/tests/` mirroring spec sections.
- Test naming: `conforms_to_<spec>_<section>_<short_description>`.
- Conformance matrix entries are derived from these tests (they ARE the evidence).

**Checkpoint cadence:**
- Create a checkpoint after each passing green test, not just week boundaries.
- A checkpoint is one line per test: `hook <name> — PASS`.
- At week boundaries, before summarising, run a **determinism sweep** on any new protocol-level code:
  - `grep -rn "as f64\|as f32\|f64::\|f32::" crates/` — flag any floating-point in deterministic paths
  - `grep -rn "Instant::now\|SystemTime::now\|thread_rng\|rand::random" crates/` — flag wall-clock/random sources in protocol logic
  - Verify all new `HashMap`/`HashSet` usages in protocol code don't leak iteration order into consensus decisions
- File as `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md`.

When the week's tasks are complete:
1. Update stage file (mark week complete).
2. Update `build-status.md`.
3. Update `PROJECT-STATUS.md`.

Then stop and wait for next prompt.
