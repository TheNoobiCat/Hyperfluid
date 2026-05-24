When the week's tasks are complete:
1. Update stage file (mark week complete).
2. Update `build-status.md`.
3. Update `PROJECT-STATUS.md`.

4. **Stub audit:** Before CI, scan every `TxType` variant, `TaskStatus` variant, and spec-proclaimed feature touched this week. Any type definition with zero non-test behavioral code paths (no production code that transitions to it, populates it, or reads it with non-default data) is VAPORWARE. Block week completion.

5. **Guard Enforcement Sweep** — launch a single `build-worker` subagent to execute every guard in `.opencode/commands/execute-build/checkpoint.md` systematically:
   - Read `.opencode/commands/execute-build/checkpoint.md` into the prompt verbatim.
   - For each guard (listed below as numbered items counting from guard #4), execute and report:

   ```
   [guard #, status, file:line if FAIL, note]
   ```

   Status meanings:
   - `PASS` — pattern absent, or found and justified (e.g. in `#[cfg(test)]`, behind a documented SPEC_DEVIATION)
   - `FAIL` — violation found with exact file:line
   - `MANUAL` — guard marked `[MANUAL]` or requires human judgment the subagent cannot automate

   **Tool assignment per guard category:**

   Pure grep guards — use the Grep tool with the exact regex from the guard prose:
   - #4 floating-point in protocol code: `as f64|as f32|f64::|f32::`
   - #5 wall-clock/random in protocol: `Instant::now|SystemTime::now|thread_rng|rand::random`
   - #6 test shims in library code: `thread_local!|RefCell|SPEC_DEVIATION|conformance shim`
   - #12 get_mut without rejecting else: `if let Some.*get_mut`
   - #15 field rename drift in specs: grep spec `.md` files for the old field name (context: check field names changed this week)
   - #16 field-population: grep struct field names across non-test `src/` for write-sites
   - #18 float in spec data structures: `f64|f32` in `docs/04-specifications/`
   - #23 snapshot_state parity: grep `compute_state_root` and `snapshot_state` for collection iterators and key prefixes
   - #26 fail-closed validation: `is_none()|is_err()` in validation paths — verify the default branch is restrictive
   - #27 panic/assert in production: `panic!|assert!` in `crates/*/src/` (exclude `tests/`)
   - #29 duplicated logic: grep for duplicate `Vec<QuotaEntry>` or `fn compute_` definitions across crates
   - #31 atomic ordering: `Ordering::Relaxed` in non-test code
   - #32 vaporware: for each TxType/TaskStatus variant added this week, grep across `crates/*/src/`
   - #33 checked-math overflow: `checked_mul.*unwrap_or(0)|checked_div.*unwrap_or(0)`
   - #34 truncating casts: `as u32|as u16|as u8` in protocol code where the source side is a wider integer
   - #35 async-JoinHandle discard: `let _ = .*\.await` in main/loop code
   - #36 mutex-poison masking: `if let Ok\(guard\) = .*lock\(\)` without an else/err branch
   - #37 dead-field read-side: grep struct field names for read-sites in non-test production code

   File-inspection guards — use Read to open the target file(s), then Grep to verify the condition:
   - #7 default features: Read each crate's `Cargo.toml` `[features]` section
   - #8 HashMap iteration leak: Grep for `HashMap|HashSet` in `crates/*/src/`, then Read the surrounding function to check if iteration order feeds into consensus
   - #9 HashMap return type: Grep for `pub fn.*HashMap` in protocol crate sources
   - #14 SMT root completeness: Read `compute_state_root()` in `crates/hyperfluid-state/src/state_machine.rs`, compare entity fields in the StateMachine struct
   - #19 spec structural completeness: Read each spec file header in `docs/04-specifications/`, verify §1.3 and §1.4 sections exist

   The worker does NOT fix code, edit files, add dependencies, or write tests. Report only.

   **Wait for the worker.** For each FAIL: fix the violation, re-run that guard locally with Grep to confirm. For each MANUAL: hand-verify, mark PASS or fix. All guards must be PASS before proceeding to step 6. If guard conditions in the prose are unclear, update the guard text in `checkpoint.md` after resolving the issue.

6. **CI mimic** — run auto-fix first, then check strictly.

   **Phase 1 (auto-fix, local only):**
   ```powershell
   cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged 2>$null
   cargo fmt --all
   ```

   **Phase 2 (parallel check):** Run strict verification in parallel via 3 `build-worker` subagents:
   - **Worker A:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
   - **Worker B:** `cargo test --workspace && cargo doc --workspace --no-deps --document-private-items`
   - **Worker C:** `cargo deny check && cargo bench --workspace --no-run`

Wait for all three workers. If any fails, run the failing tool locally (not in a subagent) to get the exact error output and fix it. Re-run the full CI locally after fix to confirm everything passes.

Do not actually commit or push to github.

Then stop and wait for next prompt.
