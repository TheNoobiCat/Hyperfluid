---
description: "Run a fresh-perspective code audit to find undocumented bugs"
---
Read `BUILD-SYSTEM.md`, `GLOSSARY.md`, then:

1. **Read known-state inventory** (skip already-documented bugs):
   - Every file in `docs/08-handoff/latest/` — `build-status.md`, all `checkpoint-*.md`. Pay attention to "Known Issues" sections and any `open-questions.md`.
   - `PROJECT-STATUS.md` — blockers and gaps sections.
   - All stage files in `docs/05-planning/stages/` — note any "Risk Areas" or deferred items.

2. **Read source material** (canonical "what should be true"):
   - Every spec in `docs/04-specifications/` — all sections including trust-assumption inventories.
   - Architecture docs in `docs/03-architecture/` — especially `interfaces.md`, `failure-model.md`, `state-model.md`.
   - Requirements in `docs/02-requirements/` for high-level intent.

3. **Read every line of code** in every crate under `crates/`, including tests.

4. **Cross-reference code against specs and architecture** — check for:
   - Logic errors, off-by-one, inverted conditionals, incorrect state transitions, integer overflow/underflow
   - Spec deviations (skip intentional `// SPEC_DEVIATION:` comments)
   - Missed error handling: `.unwrap()`, `.expect()`, `panic!()`, ignored `Result`, missing `?`, catch-all match arms
   - Security: missing signature verification, unvalidated input, reentrancy, shared mutable state, hardcoded secrets
   - Type/representation errors: wrong enum variant, incorrect field mapping, wrong units
   - Cross-crate inconsistencies: same concept defined differently across crates
   - Concurrency: shared state without `Mutex`/`RwLock`, locks held across await, race windows
   - Dead/unreachable code, unused functions/imports/variables
   - Test bugs: no assertions, trivial passes, wrong setup

5. **Filter against known-state** — skip bugs already documented in checkpoints, build-status "Known Issues", open-questions.md, or PROJECT-STATUS gaps.

6. **Fix every new bug found** — apply fix in appropriate crate. If fix reveals spec gap, update spec and note in change log. If architecture decision needed, file ADR.

7. **Document and verify:**
   - Create `docs/01-research/_audit-bugs-YYYY-MM-DD.md` with summary, severity breakdown, per-bug details, systemic patterns
   - Update `build-status.md`
   - Create `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md`
   - Update `PROJECT-STATUS.md`
   - `cargo build --workspace` passes
   - `cargo test --workspace` passes
   - `cargo fmt --all -- --check` passes
   - `cargo clippy --workspace --all-targets -- -D warnings` passes
   - `cargo doc --workspace --no-deps` passes
