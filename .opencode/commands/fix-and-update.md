---
description: "Fix errors or implement features with full doc sync"
---
Read the prompt left by the user at the end of this message, then read `BUILD-SYSTEM.md`, `GLOSSARY.md`, `TEMPLATES.md`, then:

1. **Read current state:**
   - Latest handoff: `docs/08-handoff/latest/` (prioritise most recent `checkpoint-*.md` and `build-status.md`).
   - Current stage file in `docs/05-planning/stages/` (check which week is active and what tasks are pending/complete).
   - `PROJECT-STATUS.md`.
   - All relevant spec files in `docs/04-specifications/` — read the spec(s) for the component(s) involved in the error or feature request.
   - All relevant source code in `crates/` for the affected components.
   - Architecture docs and ADRs in `docs/03-architecture/` if the fix crosses design boundaries.

2. **Diagnose the user's message:**
   - If error/bug: trace root cause. Check whether the code deviates from the spec (accidental bug) or the spec itself is wrong/underspecified (design gap).
   - If feature request: check whether this is:
     - **Already in scope** but under-specified in the spec → update spec, then implement.
     - **Genuinely new** → file an ADR in `docs/03-architecture/decisions/ADR-XXXX-description.md`, update the affected spec section, and if the feature introduces new requirements, add a FR file in `docs/02-requirements/`. Then implement.

3. **Fix or implement:**
   - Apply the fix or build the feature in the appropriate crate. Follow existing code conventions (error types, naming, module structure).
   - When a spec deviation is intentional or forced by implementation constraints, annotate with `// SPEC_DEVIATION: [reason]` and reference the ADR if one was created.
   - Write or update tests that cover the fix/feature.

4. **Synchronise documentation:**
   - If spec was wrong/ambiguous: update the spec file in `docs/04-specifications/`. If the change affects the trust-assumption inventory (section X.8), update it.
   - If an ADR was created: register it in `docs/03-architecture/index.md` (add to the ADR table).
   - If a requirement was added: register it in `docs/02-requirements/index.md`.
   - Update `build-status.md`: mark affected tasks, add new entries for any new work done.
   - Create `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md` summarising: what broke, root cause, what was fixed/changed, and any new gaps or open questions.
   - If a week boundary was crossed or new tasks were injected into the stage plan, update the stage file's week-by-week breakdown.
   - Update `PROJECT-STATUS.md`: record the fix, update "Next Actions" and "Last updated".

5. **CI mimic** — replicate CI exactly: `RUSTFLAGS="-D warnings"`, auto-fix first, then strict check.

   ```powershell
   # Phase 1 — auto-fix what clippy and rustfmt can handle
   cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged -- -D warnings 2>$null
   cargo fmt --all

   # Phase 2 — strict verification matching CI environment
   $env:RUSTFLAGS = "-D warnings"
   cargo fmt --all -- --check ; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
   cargo clippy --workspace --all-targets -- -D warnings ; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
   cargo test --workspace ; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
   cargo doc --workspace --no-deps --document-private-items ; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
   cargo deny check ; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
   cargo bench --workspace --no-run ; if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
   Write-Output "ALL CI CHECKS PASSED"
   ```

   If `cargo clippy --fix` doesn't resolve all lints (e.g. `manual_checked_ops` has no auto-fix), phase 2 will catch the remainder. Fix them and re-run from phase 1. Do not commit or push to github.

6. **Report back:** Summarise what was found, what code files were changed, what docs were updated, and any remaining open questions or gaps.

The prompt from the user: 