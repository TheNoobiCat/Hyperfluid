# Prompt Book: Build Hyperfluid

Step-by-step prompts to build the system. Use this in a fresh chat.

note: these prompts are a good baseline but i pretty much always need to add additional specific details to them etc

---

## Where to read first

Before executing any phase:
1. Read `BUILD-SYSTEM.md` for layer definitions, gates, and traceability rules.
2. Read `TEMPLATES.md` for all artifact formats (research, FR-XXXX, spec, stage, checkpoint).
3. Read `GLOSSARY.md` for canonical terminology.
4. Read `PROJECT-STATUS.md` for current project state, phase status, blockers, and spec inventory. **Always update `PROJECT-STATUS.md` after completing any phase.**
5. **Read the latest handoff** (`docs/08-handoff/latest/checkpoint-*.md` and `docs/08-handoff/latest/build-status.md` if they exist). If the previous agent left unfinished work, complete it before starting the current phase. Handoff files are the canonical state of what has been done and what is next.

This document contains only what is *unique to each phase*. Do not redefine formats that already live in `TEMPLATES.md`.

---

## Phase 0: Research Audit

Use this prompt when starting fresh or before promoting research to requirements.

### Prompt:

Audit all research documents. Read `BUILD-SYSTEM.md` (Decentralisation Audit Gate) and `GLOSSARY.md` for context, then read the latest handoff (`docs/08-handoff/latest/`) if it exists, then read every file in `docs/01-research/` recursively and check for:

1. Contradictions between documents (validator states, action plans, trust stages, slashing conditions)
2. Terminology inconsistencies (check against `GLOSSARY.md`)
3. Missing or broken cross-references
4. Unresolved TODOs or obvious gaps
5. **Decentralisation audit:** Run the checklist defined in `BUILD-SYSTEM.md`. Do not re-state the checklist here — execute it. If issues are found, create `docs/01-research/_overengineered.md` with source file, issue summary, explanation, and proposed fix. Do not create this file if no issues are found.

Create `research-audit-report.md` with:
- Summary of issues found (severity: blocking, warning, minor)
- Specific file locations and line references
- Recommended fixes
- Decentralisation audit findings (reference `_overengineered.md` if created)
- GO/NO-GO recommendation for proceeding to Phase 1

When complete, update `PROJECT-STATUS.md`: set Phase 0 status, record any blockers found, update "Next Actions" and "Last updated" date.

---

## Phase 1: Extract Requirements

Use this prompt to create all requirements from research.

### Prompt:

Create Layer 2 (Requirements). Read `BUILD-SYSTEM.md` (Layer 2), `TEMPLATES.md` (FR-XXXX format), and the latest handoff (`docs/08-handoff/latest/`) if it exists, then read all files in `docs/01-research/` and extract requirements:

1. Identify every "shall", "must", "should", or implied requirement.
2. Convert to numbered FR-XXXX or NFR-XXXX. Follow the format in `TEMPLATES.md` exactly.
3. Tag with source research file and section.
4. Define 1-3 acceptance criteria per requirement (measurable/testable).
5. **Decentralisation review:** Scan every requirement for language that introduces centralised coordination, external trust, or unverifiable economic enforcement. Flag as `[DECENTRALISATION-RISK]` and require a trust-minimised rewrite before spec creation.

Create directory structure:
- `docs/02-requirements/index.md` (master list with links)
- `docs/02-requirements/protocol/FR-XXXX-*.md`
- `docs/02-requirements/runtime/FR-XXXX-*.md`
- `docs/02-requirements/security/FR-XXXX-*.md`
- `docs/02-requirements/economics/FR-XXXX-*.md`

Create 60-200 total requirements covering all domains. Group related requirements into single files where logical.

When complete, update `docs/08-handoff/latest/phase-01-status.md` with count and gaps, and update `PROJECT-STATUS.md`: set Phase 1 status, record any new gaps, update "Next Actions" and "Last updated" date.

---

## Phase 2: Architecture Definition

Use this prompt to create architecture from requirements.

### Prompt:

Create Layer 3 (Architecture). Read `BUILD-SYSTEM.md` (Layer 3), `TEMPLATES.md` (ADR format), and the latest handoff (`docs/08-handoff/latest/`) if it exists, then read all files in `docs/02-requirements/` and create architecture documents:

Create these files:

1. `docs/03-architecture/index.md` — Overview, component list, navigation.
2. `docs/03-architecture/component-model/components.md` — Component definitions, responsibilities, owned state, interfaces, dependencies. Include Mermaid diagram.
3. `docs/03-architecture/component-model/interfaces.md` — Inter-component contracts, message formats, error handling, versioning.
4. `docs/03-architecture/data-model/state-model.md` — Core entities, fields, types, relationships. Include entity relationship diagram.
5. `docs/03-architecture/trust-boundaries.md` — Security zones, in-protocol vs local-only, sandboxed vs unsandboxed.
6. `docs/03-architecture/failure-model.md` — System-level failure scenarios and cascading failure prevention.

Map every requirement to a component. If a requirement doesn't fit, flag it for requirement revision.

Record every significant decision as an ADR in `docs/03-architecture/decisions/ADR-XXXX-*.md`.

When complete, update `docs/08-handoff/latest/phase-02-status.md`, and update `PROJECT-STATUS.md`: set Phase 2 status, record component/ADR counts, update "Next Actions" and "Last updated" date.

---

## Phase 3: Write Specifications

Use this prompt to create detailed technical specifications.

### Prompt:

Create Layer 4 (Specifications). Read `BUILD-SYSTEM.md` (Layer 4), `TEMPLATES.md` (Specification Section format), `GLOSSARY.md`, and the latest handoff (`docs/08-handoff/latest/`) if it exists, then read `docs/03-architecture/` and `docs/02-requirements/` and write all specs.

Target specs (read `PROJECT-STATUS.md` section "Layer 4 Spec Inventory" for the canonical list):
- `protocol/consensus-spec.md`
- `protocol/staking-spec.md`
- `protocol/governance-spec.md`
- `protocol/p2p-wire-spec.md`
- `protocol/fastpath-spec.md`
- `protocol/fee-market-spec.md`
- `storage/state-sync-spec.md`
- `storage/artifact-availability-spec.md`
- `runtime/agent-runtime-spec.md`
- `runtime/policy-engine-spec.md`
- `runtime/review-engine-spec.md`
- `runtime/collaboration-spec.md`
- `security/telemetry-spec.md`
- `security/incident-response-spec.md`

Every spec **must** include a trust-assumption inventory (section X.8 per `TEMPLATES.md`). Use exact numbers, not placeholders. Uncertain parameters: mark `[TUNE]` with a reasonable default.

When complete, update `docs/08-handoff/latest/phase-03-status.md` with spec inventory and any `[TUNE]` parameters, and update `PROJECT-STATUS.md`: set Phase 3 status, mark specs as complete in the inventory table, update "Next Actions" and "Last updated" date.

---

## Phase 4: Create Build Stages

Use this prompt to create the implementation roadmap.

### Prompt:

Create Layer 5 (Planning). Read `BUILD-SYSTEM.md` (Layer 5), `TEMPLATES.md` (Stage format), and the latest handoff (`docs/08-handoff/latest/`) if it exists, then read `docs/04-specifications/` and create week-by-week build stages:

Create these files:

1. `docs/05-planning/index.md` — Overview, stage summary table, status tracker.
2. `docs/05-planning/stages/stage-00-foundation.md` — Pre-coding, 1-2 weeks.
3. `docs/05-planning/stages/stage-01-protocol-core.md` — 6-8 weeks. Consensus, staking, wire, storage.
4. `docs/05-planning/stages/stage-02-agent-runtime.md` — 6-8 weeks. Runtime, policy, review.
5. `docs/05-planning/stages/stage-03-validation.md` — 4-6 weeks. Conformance, adversarial, load, security.
6. `docs/05-planning/stages/stage-04-mainnet-prep.md` — 4-6 weeks. Operations, monitoring, incident response.

Be realistic about timelines. If uncertain, estimate on the longer side.

When complete, update `docs/08-handoff/latest/phase-04-status.md`, and update `PROJECT-STATUS.md`: set Phase 4 status, update "Next Actions" and "Last updated" date.

---

## Phase 5: Execute Build

Use these prompts when actually implementing.

### Prompt Template for Implementation:

Execute current build task. Read `BUILD-SYSTEM.md` (Layer 5, Layer 8), `TEMPLATES.md` (Checkpoint contract), then read the latest handoff (`docs/08-handoff/latest/`, prioritising the most recent `checkpoint-*.md` and `build-status.md`). If the previous agent left unfinished work, complete it first.

Read:
1. Current week's stage file.
2. The specification referenced for that week.
3. `docs/08-handoff/latest/build-status.md` (to see what's already done).

Implement in the appropriate crate/directory. Follow the spec exactly.

**When a spec is ambiguous or contradictory:**
1. Do not guess. Check `docs/08-handoff/latest/open-questions.md` — if a ruling exists, follow it.
2. If no ruling exists, make a decision, document it as an ADR in `docs/03-architecture/decisions/`, and file a spec change request.
3. Implement the decision with a `// SPEC_DEVIATION: [reason]` comment.

**When a requirement gap is discovered mid-implementation:**
1. STOP. Do not paper over the gap.
2. File the gap in `docs/08-handoff/latest/open-questions.md` with:
   - The spec section that is underspecified
   - What is missing
   - Whether it blocks the current task or can be deferred
3. If it blocks the current task, escalate to a spec revision. Track in `PROJECT-STATUS.md`.

**Stop rule (trust assumptions):**
If you discover a trust assumption or centralised dependency not listed in the spec's trust-assumption inventory:
1. STOP implementation of that component.
2. File in `open-questions.md`.
3. Escalate to spec revision before continuing.

**Testing (TDD — Red → Green → Refactor):**
- The spec's Section X.7 Conformance Test Hooks ARE the TDD stories.
- For each hook, in order:
  1. **RED:** Write a failing test that asserts the hook's behavior.
  2. **GREEN:** Write minimum code to make the test pass.
  3. **REFACTOR:** Clean up while keeping the test green.
- `cargo test` before any implementation must fail on the new test.
- Test file convention: `crates/<component>/tests/` mirroring spec sections.
- Test naming: `conforms_to_<spec>_<section>_<short_description>`.
- Conformance matrix entries are derived from these tests (they ARE the evidence).

**Checkpoint cadence:**
- Create a checkpoint after each passing green test, not just week boundaries.
- A checkpoint is one line per test: `hook <name> — PASS`.
- At week boundaries, summarise: total new tests, what works, what's next, blockers.
- File as `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md`.

When the week's tasks are complete:
1. Update stage file (mark week complete).
2. Update `build-status.md`.
3. Update `PROJECT-STATUS.md`.

Then stop and wait for next prompt.

---

## Quick Start Guide

In a fresh chat, determine current state:

1. Does `research-audit-report.md` exist?
   NO -> Run Phase 0
   YES -> Continue

2. Has the decentralisation audit passed (`_overengineered.md` reviewed and fixed if it exists)?
   NO -> Run Phase 0 decentralisation audit
   YES -> Continue

3. Does `docs/02-requirements/` have FR-*.md files?
   NO -> Run Phase 1
   YES -> Continue

4. Does `docs/03-architecture/` have component docs?
   NO -> Run Phase 2
   YES -> Continue

5. Does `docs/04-specifications/` have spec files?
   NO -> Run Phase 3
   YES -> Continue

6. Does `docs/05-planning/stages/` have stage files?
   NO -> Run Phase 4
   YES -> Run Phase 5 (Implementation)

---

## Utility A: Fix-and-Update

Use this when encountering a build error, runtime crash, test failure, or when a new feature/capability is needed mid-build. Paste the error message, stack trace, test output, or feature description as your message, then run this prompt. It will fix the code AND synchronise all related documentation in one cycle.

### Prompt:

Read the prompt left by the user at the end of this message, then read `BUILD-SYSTEM.md`, `GLOSSARY.md`, `TEMPLATES.md`, then:

1. **Read current state:**
   - Latest handoff: `docs/08-handoff/latest/` (prioritise most recent `checkpoint-*.md` and `build-status.md`).
   - Current stage file in `docs/05-planning/stages/` (check which week is active and what tasks are pending/complete).
   - `PROJECT-STATUS.md`.
   - All relevant spec files in `docs/04-specifications/` — read the spec(s) for the component(s) involved in the error or feature request.
   - All relevant source code in `crates/` for the affected components.
   - Architecture docs and ADRs in `docs/03-architecture/` if the fix crosses design boundaries.

2. **Diagnose the user's message:**
   - If error/bug: trace root cause. Check whether the code deviates from the spec (accidental bug) or the spec itself is wrong/underspecified (design gap).
   - If feature request: check whether this is:
     - **Already in scope** but under-specified in the spec → update spec, then implement.
     - **Genuinely new** → file an ADR in `docs/03-architecture/decisions/ADR-XXXX-description.md`, update the affected spec section, and if the feature introduces new requirements, add a FR file in `docs/02-requirements/`. Then implement.

3. **Fix or implement:**
   - Apply the fix or build the feature in the appropriate crate. Follow existing code conventions (error types, naming, module structure).
   - When a spec deviation is intentional or forced by implementation constraints, annotate with `// SPEC_DEVIATION: [reason]` and reference the ADR if one was created.
   - Write or update tests that cover the fix/feature.

4. **Synchronise documentation:**
   - If spec was wrong/ambiguous: update the spec file in `docs/04-specifications/`. If the change affects the trust-assumption inventory (section X.8), update it.
   - If an ADR was created: register it in `docs/03-architecture/index.md` (add to the ADR table).
   - If a requirement was added: register it in `docs/02-requirements/index.md`.
   - Update `build-status.md`: mark affected tasks, add new entries for any new work done.
   - Create `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md` summarising: what broke, root cause, what was fixed/changed, and any new gaps or open questions.
   - If a week boundary was crossed or new tasks were injected into the stage plan, update the stage file's week-by-week breakdown.
   - Update `PROJECT-STATUS.md`: record the fix, update "Next Actions" and "Last updated".

5. **Verify:**
   - `cargo build --workspace` passes.
   - `cargo test --workspace` passes (new and existing tests).
   - `cargo fmt --all -- --check` passes.
   - `cargo clippy --workspace --all-targets -- -D warnings` passes.
   - `cargo doc --workspace --no-deps` passes.
   - If any verification step fails, fix the issue before proceeding.

6. **Report back:** Summarise what was found, what code files were changed, what docs were updated, and any remaining open questions or gaps.

The prompt from the user: 

---

## Utility B: Code Audit (Silent Bugs)

Use this during or after Phase 5 build execution to find bugs in implemented code that are NOT already documented in handoffs, checkpoints, or build-status. This is a fresh-perspective audit, not a progress review — it intentionally ignores "what's done vs what's left" and focuses only on undiscovered defects.

### Prompt:

Read `BUILD-SYSTEM.md`, `GLOSSARY.md`, then:

1. **Read known-state inventory** (so you know what bugs/issues are already documented and should be skipped):
   - Every file in `docs/08-handoff/latest/` — `build-status.md`, all `checkpoint-*.md`. Pay attention to "Known Issues" sections and any `open-questions.md`.
   - `PROJECT-STATUS.md` — blockers and gaps sections.
   - All stage files in `docs/05-planning/stages/` — note any "Risk Areas" or deferred items.

2. **Read source material** (the canonical "what should be true"):
   - Every spec in `docs/04-specifications/` — read all sections including trust-assumption inventories. These define correct behaviour.
   - Architecture docs in `docs/03-architecture/` — especially `interfaces.md` (message formats, error codes), `failure-model.md` (failure scenarios), `state-model.md` (state transitions).
   - Requirements in `docs/02-requirements/` for high-level intent.

3. **Read every line of code:**
   - Walk every file in every crate under `crates/`. Read tests too — test bugs are bugs.

4. **Cross-reference code against specs and architecture to find bugs:**

   For each component, check systematically:

   - **Logic errors:** Wrong comparison operator, off-by-one, inverted conditional, missing negation, incorrect state transition in a match/if chain, integer overflow/underflow (checked vs unchecked arithmetic).
   - **Spec deviations:** Behaviour that diverges from the spec. Distinguish intentional `// SPEC_DEVIATION:` comments (skip these — they're documented choices) from accidental deviations (REPORT).
   - **Missed error handling:** `.unwrap()`, `.expect()`, or `panic!()` calls on fallible operations; ignored `Result` return values; missing `?` propagation; catch-all `match` arms that hide errors.
   - **Security issues:** Missing or incorrect signature verification, input not validated against schema, missing bounds checks on untrusted data, reentrancy, shared mutable state without synchronisation, hardcoded secrets/keys.
   - **Type/representation errors:** Wrong enum variant used, incorrect field mapping between wire format and internal struct, field omitted during serialisation/deserialisation, wrong units (milliseconds vs height, nanoAGX vs AGX).
   - **Cross-crate inconsistencies:** Two crates defining the same concept differently (e.g., `TrustStage` ordering differs between `hyperfluid-pdp` and `hyperfluid-agent`), incompatible type definitions, mismatched wire format expectations.
   - **Concurrency errors:** Shared state accessed without `Mutex`/`RwLock`, `async` functions that hold locks across await points, incorrect `Send`/`Sync` bounds, race window between check-and-act operations.
   - **Dead or unreachable code:** Unused functions, dead match arms, unreachable `panic!`, imports that are never used, variables assigned but never read.
   - **Test bugs:** Tests that don't actually assert anything (no `assert!`/`assert_eq!`), tests that pass trivially due to wrong setup, tests that don't match spec behaviour.

5. **Filter the bug list:**
   Compare every candidate bug against the known-state inventory from step 1. If the bug is ALREADY documented in a checkpoint, build-status "Known Issues", open-questions.md, or PROJECT-STATUS gaps section — SKIP IT. Only report genuinely NEW discoveries.

6. **Fix every new bug found:**
   - Apply the fix in the appropriate crate.
   - If the fix reveals a spec gap or ambiguity, update the relevant spec in `docs/04-specifications/` and note the fix in the spec change log.
   - If the fix requires an architecture decision (e.g., changing an interface), file an ADR in `docs/03-architecture/decisions/` and update `docs/03-architecture/index.md`.

7. **Document and verify:**
   - Create `docs/01-research/_audit-bugs-YYYY-MM-DD.md` with:
     - Summary: total bugs found and fixed, severity breakdown (critical, major, minor).
     - For each bug: file path + line number, severity, what code did vs what it should do (cite spec section), root cause category (from step 4 list), and what was changed.
     - Systemic patterns (e.g., "all crates use `.unwrap()` on deserialisation").
   - Update `build-status.md` to reflect fixes applied.
   - Create `docs/08-handoff/latest/checkpoint-YYYY-MM-DD.md` summarising the audit scope and fixes.
   - Update `PROJECT-STATUS.md`: record the audit, update "Next Actions", "Last updated".
   - `cargo build --workspace` passes.
   - `cargo test --workspace` passes.
   - `cargo fmt --all -- --check` passes.
   - `cargo clippy --workspace --all-targets -- -D warnings` passes.
   - `cargo doc --workspace --no-deps` passes.
