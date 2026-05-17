---
description: "Execute the current build task from the stage plan"
---

This command is split into multiple files. You must read **all of the files** listed below before doing anything else. 

1. `.opencode/commands/execute-build/preamble.md`
2. `.opencode/commands/execute-build/read-and-implement.md`
3. `.opencode/commands/execute-build/spec-ambiguity.md`
4. `.opencode/commands/execute-build/requirement-gap.md`
5. `.opencode/commands/execute-build/stop-rule.md`
6. `.opencode/commands/execute-build/testing-tdd.md`
7. `.opencode/commands/execute-build/checkpoint.md`
8. `.opencode/commands/execute-build/integration-verification.md`
9. `.opencode/commands/execute-build/week-completion.md`

After you have read every single file, follow each file in order.

**Parallelization note:** Steps 2 (read-and-implement) and 6 (testing-tdd) may farm independent work items to `build-worker` subagents. Steps 9 (week-completion) farms CI checks in parallel. See each step file for delegation patterns.