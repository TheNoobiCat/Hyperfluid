# Checkpoint — 2026-05-05 (Seed-Centric Model + Single-Agent Tasks)

**Completed:** Full design alignment across 15 files. Seed ideas are now explicitly abstract topic buckets, seed_ref is required for all tasks, new seeds enter via governance, and single-agent execution is the law.

## What changed

### Core design decisions codified

1. **Seed ideas are abstract topic buckets** — not individual tasks. A seed like "Engineering assistance" hosts many small, claimable tasks. The airdrop agent creates many tasks per seed at genesis to distribute AGX broadly.

2. **All tasks MUST reference a seed idea via `seed_ref`** — PDP-enforced. No orphan tasks. The `user-task-submission-and-sponsorship.md` "entirely novel — no seed reference required" path has been removed.

3. **New seed ideas enter via `git:head` governance proposals** — the proposer submits the `.md` file following `_template.md`. Validators review and vote. Once accepted, the seed becomes canonical.

4. **Single-agent per task** — no team formation, no subtask splitting. Reviewers are independent, paid via the review market (FR-0161), not from the task bounty. The worker gets the full escrowed bounty.

5. **Bounty field removed from seed idea template** — bounties are per-task, not per-seed. The template now has "Example tasks" to show what kinds of work the seed would contain.

### Files modified (15 total)

| File | Changes |
|------|---------|
| `/ideas/_template.md` | Rewritten: removed bounty, added topic-bucket description, example tasks, governance entry note |
| `/ideas/README.md` | Rewritten: seed=bucket model, governance creation, airdrop many-tasks, seed_ref required |
| `user-task-submission-and-sponsorship.md` | seed_ref now REQUIRED (was optional); "entirely novel" removed; failure scenario rewritten; PDP rules updated; CLI args updated; Relationship section rewritten |
| `collaboration-layer-parallel-teams.md` | Seed index updated to topic-bucket model; team formation replaced with single-agent; Team Coordinator replaced with Review Assignment |
| `collaboration-spec.md` | seed_ref added to Task struct (required); team formation removed; single-agent model added; conformance hooks updated |
| `agent-runtime-spec.md` | System prompt (§3.2) now instructs agents on seed requirement and seed browsing |
| `agx-economics-and-adversarial-incentives.md` | Clarified many-tasks-per-seed model; updated airdrop agent description |
| `FR-0080` | Changed from "Dynamic Team Formation" to "Single-Agent Task Claiming" |
| `FR-0081` | Now requires topic to reference a valid canonical seed idea |
| `FR-0084` | Rewritten: seed=bucket, seed_ref required, governance entry, tags changed to must-have |
| `FR-0088` | Changed from "Task Splitting and Subtasks" to "Single-Agent Task Execution" |
| `FR-0192` | Clarified many small tasks per seed, not one task per seed |
| `GLOSSARY.md` | Added "Seed idea" entry; updated "Airdrop agent" entry |
| `ADR-0013` | Updated with seed-centric model decisions, renamed |
| `phase-02-status.md` | Terminology unified (parent_seed_ref→seed_ref), gap resolved |
| `architecture/index.md` | ADR-0013 title updated |
| `requirements/index.md` | FR-0080/0088 descriptions updated |

### Critical contradiction resolved

`user-task-submission-and-sponsorship.md` line 157 previously said:
> "User submissions can also be entirely novel — no seed reference required."

This directly contradicted the seed-centric model. Now reads:
> "All tasks MUST reference a seed idea. This is enforced by the PDP."

### Verification

- `cargo build --workspace` — PASS
- `cargo test --workspace` (23/23) — PASS
- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `cargo doc --workspace --no-deps` — PASS

No code files changed. Documentation-only amendment.

**Next:** Stage 01 (Protocol Core). All pre-Stage-01 amendments complete.

**Open Questions:** None.
