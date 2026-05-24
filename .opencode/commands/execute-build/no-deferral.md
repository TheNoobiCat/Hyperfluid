**Hard rule: Never defer a task that can be built now.**

A task MUST be built immediately unless ALL three conditions are true:
1. An upstream crate, type, function, or trait literally does not exist in the codebase
   (not "isn't wired yet" — "does not compile")
2. No stub, mock, or minimal placeholder could be built instead
3. The blocking dependency is cited with exact file:line

If ANY of these is false → build the task. Do not defer.

"Not on the critical path" is NEVER a valid reason to defer.
"Medium priority" or "low priority" tags are scheduling hints, not skip signals.

A task is "deferred" only when building it would literally not compile or
would produce dead code with zero callers because the caller doesn't exist yet.

When the week's task list is enumerated, every task that does not meet
all three blocking conditions MUST appear in the delegation plan.

After week completion, any task left unbuilt that was buildable is a process
violation. Report it in the week-completion summary.
