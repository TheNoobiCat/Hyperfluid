---
description: "Fix errors or implement features with full doc sync"
---
Read `BUILD-SYSTEM.md`, `GLOSSARY.md`, `TEMPLATES.md`, then:

1. **Read current state:**
   - Latest handoff: `docs/08-handoff/latest/` (prioritise most recent `checkpoint-*.md` and `build-status.md`).
   - Current stage file in `docs/05-planning/stages/` (check active week, pending/complete tasks).
   - `PROJECT-STATUS.md`.
   - All relevant spec files in `docs/04-specifications/` for the component(s) involved.
   - All relevant source code in `crates/` for affected components.
   - Architecture docs and ADRs in `docs/03-architecture/` if the fix crosses design boundaries.

2. **Diagnose:**
   - If error/bug: trace root cause. Check whether code deviates from spec (accidental bug) or spec is wrong/underspecified (design gap).
   - If feature request: check whether it's already in scope but underspecified (update spec, implement) or genuinely new (file ADR, update spec, add FR in `docs/02-requirements/`, implement).

3. **Fix or implement:**
   - Apply fix or build feature in the appropriate crate. Follow existing conventions.
   - When spec deviation is intentional, annotate with `// SPEC_DEVIATION: [reason]` and reference ADR.
   - Write or update tests.

4. **Synchronise documentation:**
   - If spec was wrong/ambiguous: update it. If trust-assumption inventory affected, update it.
   - If ADR created: register in `docs/03-architecture/index.md`.
   - If requirement added: register in `docs/02-requirements/index.md`.
   - Update `build-status.md`.
   - Create `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md`.
   - If week boundary crossed or new tasks injected, update stage file.
   - Update `PROJECT-STATUS.md`.

5. **Verify:**
   - `cargo build --workspace` passes
   - `cargo test --workspace` passes
   - `cargo fmt --all -- --check` passes
   - `cargo clippy --workspace --all-targets -- -D warnings` passes
   - `cargo doc --workspace --no-deps` passes
   - If any step fails, fix before proceeding.

6. **Report back:** Summarise what was found, code files changed, docs updated, and any remaining open questions or gaps.
