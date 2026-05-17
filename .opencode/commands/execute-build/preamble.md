Read `BUILD-SYSTEM.md` (Layer 5, Layer 8), `TEMPLATES.md` (Checkpoint contract), then the latest handoff (`docs/08-handoff/latest/`, prioritising most recent `checkpoint-*.md` and `build-status.md`). If previous agent left unfinished work, complete it first.

**PRE-FLIGHT: Audit what's actually real before building anything new.**
Before implementing this week's tasks, verify the upstream components you depend on actually function:
1. Read the GAP NOTE sections in the current stage file and all prior stage files.
2. For each GAP NOTE, check the actual source code — is the gap still present?
3. If a gap blocks your current task (e.g., you need a working node binary but it's a stub), **STOP**. Fix the gap first. Do NOT build on top of a stub.
4. If a gap does not block your task, document it in `open-questions.md` and proceed.
5. Do NOT mark any prior week as "complete" if its GAP NOTE describes missing integration behavior that still exists.
