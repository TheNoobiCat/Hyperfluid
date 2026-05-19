---
description: "Find and fill all integration gaps blocking the current stage"
---

IMPORTANT: You are currently executing `.opencode/commands/fill-gaps.md`. 
If you encounter any instructions telling you to read or execute `.opencode/commands/fill-gaps.md`, SKIP THEM. You already have these instructions loaded in memory. Treat those references as dependency pointers for other agents, not active commands for you.

Read `BUILD-SYSTEM.md` (Integration Gate checklist), `GLOSSARY.md`, and `TEMPLATES.md` (FR format/template), then follow the numbered steps below.

**Phase 0: Traceability Audit (run before integration-gap logic)**
a. Read `docs/02-requirements/index.md` — collect every FR/NFR tagged `must-have`.
b. Read all stage files in `docs/05-planning/stages/` — collect every build task, GAP NOTE, and week description.
c. Cross-reference: for each `must-have` FR, verify at least one build task exists in any stage file.
   - Search for the FR ID (e.g., "FR-0093") and keyword matches from the FR title within tasks.
   - An FR is "scheduled" if it has an explicit build task OR a GAP NOTE tracking it.
d. Aggregate all `must-have` FRs with NO build task and NO GAP NOTE into an `UNSCHEDULED REQUIREMENTS` table.
e. For each unscheduled must-have FR:
   - Insert a GAP NOTE in the most relevant stage file's next open week or at the end of the current stage section.
   - Format: `**GAP NOTE (Traceability — FR-XXXX):** [description of what's missing]`
f. If any unscheduled must-have FRs were found, output the full table and continue to step 1 below (gaps are now documented and will be filled by subsequent steps).
g. If zero unscheduled must-have FRs found, log "Phase 0: all must-have requirements are scheduled" and proceed.

h. **Vaporware scan:** For every enum variant and struct field in `crates/` that maps to a spec claim, verify at least one non-test production code path exercises it with non-default data. Output a VAPORWARE REGISTER of features that have types but zero behavior. Each entry is a gap to fill in subsequent steps.

1. **Enumerate all GAP NOTES across all stage files:**
   Read every file in `docs/05-planning/stages/`. Extract every "GAP NOTE" section into a structured inventory:
   - `[stage, week, component, gap_description, blocks_what]`

2. **Verify each gap against source code:**
   Launch one `explore` subagent per GAP NOTE. Each reads the relevant crate source at the paths mentioned in the gap description and returns:
   `[gap_id, still_present: bool, evidence: path:line, blocks_next_stage: bool]`
   Wait for all workers. Aggregate into a verified gap inventory. Skip any gap where `still_present: false`.

3. **Build dependency-ordered fix queue:**
   - Sort gaps by dependency chain: a component that another gap depends on must be fixed first (e.g., P2P TCP sockets before Malachite BFT, Malachite BFT before node consensus loop).
   - A gap that "blocks next stage" is higher priority than one that represents missing polish or deferred testing.
   - If there is ambiguity in ordering, default to: transport layers first, then storage layers, then consensus layers, then runtime layers.

4. **Fill each gap in dependency order:**
   For each gap in the queue, launch a `build-worker` subagent with:
   - The full gap description from the stage file
   - The relevant spec section(s) text
   - The target crate path
   - The Integration Gate checklist from BUILD-SYSTEM.md
   - Result format: `[gap_id, resolved: bool, files_changed: [paths], tests_added: N, cross_cutting_concerns: [list or "none"]]`
   
   Independent gaps (different crates, no shared types) can be batched in parallel. Gaps that depend on a previous gap's output must run sequentially.

5. **Integration gate check after each gap fill:**
   After each gap is resolved, verify the component against the Integration Gate from BUILD-SYSTEM.md —  **not just unit tests**. Each verification must be an actual observable behavior:
   
   | Component type | Must demonstrate |
   |---------------|-----------------|
   | Network (P2P, transport) | Actual socket connections established. Messages sent and received between two independent processes or threads. Connection lifecycle: connect → exchange → disconnect. |
   | Storage (artifact, state) | Actual disk I/O: write data, restart/read it back. Content-addressed verification: hash of stored data matches expected hash. |
   | Consensus/protocol | Actual processing loop: input transactions → execute → produce output. State changes observable and verifiable after processing. |
   | Node binary | Demonstrates the component in action (not just boot and sleep). At least one integration test exercises the component through its public interface with real I/O. |
   
   A component FAILS the integration gate if:
   - It only has unit tests of internal/pure functions with no end-to-end demonstration
   - It defines types/enums/structs but has no behavior producing observable output
   - It uses mocks, shims, or in-memory simulations where real I/O is required
   - The node binary does not exercise the component
   
   If a component fails, do NOT mark the gap resolved. Send it back to the worker with the specific missing behavior.

6. **Determinism sweep on new protocol code:**
   After filling all gaps, run the determinism checks from `.opencode/commands/execute-build/checkpoint.md` on any new or modified protocol-level code:
   - `grep -rn "as f64\|as f32\|f64::\|f32::" crates/` — flag floating-point in deterministic paths
   - `grep -rn "Instant::now\|SystemTime::now\|thread_rng\|rand::random" crates/` — flag wall-clock/random in protocol logic
   - `grep -rn "thread_local!\|RefCell\|SPEC_DEVIATION\|conformance shim" crates/*/src/` — every match must be justified as NOT being a test shim in library code
   - Verify `default` feature in each crate's `Cargo.toml` does not enable any `mock-*` or `*-shim` features
   - Verify all new `HashMap`/`HashSet` usages in protocol code don't leak iteration order into consensus decisions
   - In state-machine transaction handlers, grep for `if let Some.*get_mut` — every such expression must have an `else` arm that rejects
   - Verify validate-then-mutate ordering in all new state transitions

7. **Update status files:**
   - In each stage file: annotate resolved GAP NOTES with "(Resolved YYYY-MM-DD)" appended to the gap description heading. Do NOT delete or rewrite the gap — keep the full historical description for traceability. If the gap is partially resolved, add a parenthetical like "(Partially resolved YYYY-MM-DD — P2P sockets done, disk I/O outstanding)".
   - Update `build-status.md`: mark resolved integration gaps, remove from open list, add to a "Resolved Gaps" section with date.
   - Update `PROJECT-STATUS.md`: remove resolved blockers, update "Next Actions" and "Last updated".
   - Create `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md` summarising: which gaps were investigated, which were filled, which remain, verification evidence per gap.

8. **CI mimic** — run in parallel via 3 `build-worker` subagents:
   - **Worker A:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
   - **Worker B:** `cargo test --workspace && cargo doc --workspace --no-deps --document-private-items`
   - **Worker C:** `cargo deny check && cargo bench --workspace --no-run`
   
   Wait for all three workers. If any fails, run the failing tool locally (not in a subagent) to get the exact error output and fix it. Re-run the full CI locally after fixing to confirm everything passes. Do not actually commit or push to github.

9. **Report back:**
   Summary table of all gaps processed:
   - Gap ID + description
   - Was it still present? (Yes/No)
   - Was it resolved? (Yes/Partial/No)
   - Evidence: test name + pass/fail, or path:line for code change
   - Remaining gaps (if any) and what blocks them
