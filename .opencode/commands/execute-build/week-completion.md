When the week's tasks are complete:
1. Update stage file (mark week complete).
2. Update `build-status.md`.
3. Update `PROJECT-STATUS.md`.

4. **Stub audit:** Before CI, scan every `TxType` variant, `TaskStatus` variant, and spec-proclaimed feature touched this week. Any type definition with zero non-test behavioral code paths (no production code that transitions to it, populates it, or reads it with non-default data) is VAPORWARE. Block week completion.

5. **CI mimic** — run in parallel via 3 `build-worker` subagents:
   - **Worker A:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
   - **Worker B:** `cargo test --workspace && cargo doc --workspace --no-deps --document-private-items`
   - **Worker C:** `cargo deny check && cargo bench --workspace --no-run`

Wait for all three workers. If any fails, run the failing tool locally (not in a subagent) to get the exact error output and fix it. Re-run the full CI locally after fix to confirm everything passes.

Do not actually commit or push to github.

Then stop and wait for next prompt.
