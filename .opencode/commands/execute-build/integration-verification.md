**Integration verification (before marking week complete):**
Before marking any week's tasks as complete, verify each component actually functions end-to-end. Unit tests passing is NOT sufficient.

For each component you implemented this week, answer:

1. **Does it process data?** → Write an integration test that feeds it real input and verifies real output. Not just "function returns expected value for test fixture" — actual end-to-end behavior.

2. **Does it communicate?** → Write a test that actually sends/receives over a socket, pipe, or channel between two independent contexts. Not just "state machine transitions correctly."

3. **Does it persist?** → Write a test that writes to disk and reads it back. Not just "struct serializes to bytes."

4. **Does it run a loop?** → Write a test that runs the loop for N iterations and verifies state changes. Not just "loop body executes once."

5. **Is it wired into the node?** → The node binary must demonstrate the component in action. A `sleep(100ms)` counter does not demonstrate consensus.

**Failure criteria — do NOT mark week complete if:**
- Component only has unit tests of internal/pure functions with no end-to-end demonstration
- Component defines types/enums/structs but has no behavior producing observable output
- Component uses mocks, shims, or in-memory simulations where real I/O is required
- Node binary does not exercise the component

If a component fails integration verification:
1. Do NOT mark the week complete.
2. Document the gap in `docs/08-handoff/latest/open-questions.md`.
3. Implement the missing integration behavior before proceeding.
