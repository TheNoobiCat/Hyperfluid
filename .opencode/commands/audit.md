---
description: "Run a fresh-perspective code audit to find undocumented bugs"
---
Read `BUILD-SYSTEM.md`, `GLOSSARY.md`, then:

1. **Read known-state inventory** (skip already-documented bugs):
   - Every file in `docs/08-handoff/latest/` — `build-status.md`, all `checkpoint-*.md`. Pay attention to "Known Issues" sections and any `open-questions.md`.
   - `PROJECT-STATUS.md` — blockers and gaps sections.
   - All stage files in `docs/05-planning/stages/` — note any "Risk Areas" or deferred items.
   - Build a skip-list of already-documented bugs to pass to workers.

2. **Read source material** (canonical "what should be true") — farm in parallel:
   Launch a `build-worker` subagent per domain. Each reads its files and returns a structured summary:
   - **Worker A:** protocol specs (`docs/04-specifications/protocol/`)
   - **Worker B:** runtime specs (`docs/04-specifications/runtime/`)
   - **Worker C:** storage + security specs (`docs/04-specifications/storage/`, `docs/04-specifications/security/`)
   - **Worker D:** architecture docs (`docs/03-architecture/`)
   - **Worker E:** requirements (`docs/02-requirements/`)
   Each returns: `[domain, key_claims: [...], trust_assumptions: [...], cross_cutting_flags: [...]]`
   Wait for all workers. Aggregate into a unified source-material reference.

3. **Read every line of code** — farm per-crate in parallel:
   Launch a `build-worker` subagent per crate under `crates/`. Each returns:
   `[crate, functions_found: N, types_found: [list], suspicious_patterns: [...]]`

4. **Cross-reference code against specs and architecture** — farm per spec-crate pair in parallel:
   Map each spec to its implementing crate(s) (from stage files or build-status). Launch a `build-worker` per pair. Each checks:
   - Logic errors, off-by-one, inverted conditionals, incorrect state transitions, integer overflow/underflow
   - Spec deviations (skip intentional `// SPEC_DEVIATION:` comments)
   - Missed error handling: `.unwrap()`, `.expect()`, `panic!()`, ignored `Result`, missing `?`, catch-all match arms
   - Security: missing signature verification, unvalidated input, reentrancy, shared mutable state, hardcoded secrets
   - Type/representation errors: wrong enum variant, incorrect field mapping, wrong units
   - Determinism violations: f64/f32 in protocol code, HashMap iteration order leakage, wall-clock or random sources in consensus paths
   - Monetary type invariant: every atto-AGX field must be u128 — grep for u64 in monetary-adjacent fields
   - Concurrency: shared state without `Mutex`/`RwLock`, locks held across await, race windows
   - Dead/unreachable code, unused functions/imports/variables
   - Test bugs: no assertions, trivial passes, wrong setup, missing negative/edge-case tests
   Each returns findings scoped to its pair. Wait for all workers.

   **Post-aggregation cross-cutter checks** (run locally, not in subagent):
   - Cross-crate type drift: grep for the same type/enum name in two different crate `src/` dirs — flag if field layouts differ
   - Combine findings across all spec-crate pairs, deduplicate, build a unified finding list

5. **Filter against known-state** — skip bugs already documented in checkpoints, build-status "Known Issues", open-questions.md, or PROJECT-STATUS gaps.

6. **Fix every new bug found**:
   - Apply fix in appropriate crate. If fix reveals spec gap, update spec and note in change log. If architecture decision needed, file ADR.
   - After each fix, grep for the same root-cause pattern across all crates. Fix other instances before marking resolved.

7. **Self-improve the build process:**
   After documenting all bugs, review the systemic patterns you found. For each pattern, check whether the current TDD cycle in `.opencode/commands/execute-build.md` would have caught it during initial implementation (not during this audit). Look at these sections specifically:
   - The TDD checklist (RED → GREEN → REFACTOR → RECURSE steps)
   - The negative assertion requirement
   - The determinism sweep in the checkpoint cadence
   
   If a systemic pattern would NOT have been caught by any existing step, add a generic guard to `execute-build.md`. Guard examples (not specific bug references):
   - A grep/scan step for a class of type or representation error
   - A structural test requirement (e.g., "every function that takes X must also be tested with Y")
   - A validation check to add to the determinism sweep or pre-checkpoint verification
   
   Keep guards generic (e.g., "grep for the old type across all crates after any type migration" — not "change u64 to u128"). Do not reference specific bug numbers or past audit findings.

8. **Document and verify:**
   - Create `docs/01-research/_audit-bugs-YYYY-MM-DD.md` with summary, severity breakdown, per-bug details, systemic patterns, and any process changes made to execute-build.md
   - Update `build-status.md`
   - Create `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md`
   - Update `PROJECT-STATUS.md`
   - **CI mimic** — run in parallel via 3 `build-worker` subagents:
      - **Worker A:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
      - **Worker B:** `cargo test --workspace && cargo doc --workspace --no-deps --document-private-items`
      - **Worker C:** `cargo deny check && cargo bench --workspace --no-run`
   
   Wait for all three. If any fails, run the failing tool locally (not in a subagent) to get the exact error output and fix it. Do not actually commit or push to github.
