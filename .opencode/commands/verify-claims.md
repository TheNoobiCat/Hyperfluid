---
description: "Verify handoff claims against code reality — truth-in-advertising audit"
---

IMPORTANT: This is NOT a bug hunt. Do NOT look for undocumented bugs or subtle logic errors — that is `audit.md`. This command only verifies that claimed states (COMPLETE, DONE, PASS, test counts, spec conformance) match actual code. Fix mismatches but do not go hunting for new bugs outside the claim scope.

Read `BUILD-SYSTEM.md`, `GLOSSARY.md`, then:

## Phase 0: Prerequisite — integration gaps gate

State reads allowed. Read:
- `docs/08-handoff/latest/build-status.md` — INTEGRATION GAPS section.
- `docs/08-handoff/latest/open-questions.md` — open blocks.

If any gap in INTEGRATION GAPS is marked OPEN or CRITICAL AND the gap blocks a component that the current claims say is COMPLETE, do NOT proceed with verification of that component's claims. Instead, note it as BLOCKED and skip to Phase 7 with a "cannot verify — blocked by open integration gap" verdict.

---

## Phase 1: Extract all claims from handoff docs

Read every file in `docs/08-handoff/latest/` in parallel (at most 3 files per `read` call). Also read `PROJECT-STATUS.md`. Extract every explicit claim into a structured inventory:

| Field | Description |
|-------|-------------|
| `id` | Sequential `C-001`, `C-002`, ... |
| `type` | One of: `COMPLETE_CLAIM`, `TEST_COUNT`, `PASS_CLAIM`, `SPEC_CONFORMANCE`, `INTEGRATION_CLAIM` |
| `source` | File path + line number |
| `claim_text` | Verbatim text of the claim |
| `expected_reality` | What the claim asserts as true (e.g., "PDP has 58 tests", "hyperfluid-agent is fully wired", "cargo clippy passes with zero warnings") |
| `deferred` | Set to `true` if the claim is immediately followed by a DEFERRED marker, or if the component is listed under a DEFERRED heading. Skip these. |

**Claim extraction rules:**
- A `COMPLETE_CLAIM` is any row in a task-status table marked `Complete`, or any heading/section containing `COMPLETE` or `DONE`.
- A `TEST_COUNT` is any explicit number followed by "tests" (e.g., "58 tests", "57/57 workspace tests pass").
- A `PASS_CLAIM` is any verification table row marked `PASS`.
- A `SPEC_CONFORMANCE` is any claim containing "conformance", "implements", "matches spec", "follows spec", or referencing specific spec hooks.
- An `INTEGRATION_CLAIM` is any claim containing "wired", "integrated", "exercises", "end-to-end", or "integration test".
- **Skip** any claim that references a DEFERRED task, or the header/context immediately precedes the word "DEFERRED" or "Deferred".

Output the full claim inventory as a table. Then proceed.

---

## Phase 2: Complete-Claim Verification

Farm one `build-worker` subagent per COMPLETE_CLAIM in parallel (batch up to 4 per worker if claims target the same crate). Each receives:
- The claim text and source
- The component/crate/module path from the claim
- The relevant spec sections (from `docs/04-specifications/`), architecture docs (from `docs/03-architecture/`), and requirements (from `docs/02-requirements/`) that define what "complete" means for this component

Each worker returns a structured verdict:

```json
{
  "claim_id": "C-001",
  "verdict": "PASS" | "FAIL" | "PARTIAL" | "UNVERIFIABLE",
  "evidence": [
    {"type": "EXISTS" | "MISSING" | "STUB" | "UNWIRED", "path": "path:line", "detail": "..."}
  ],
  "reality_short": "One-line summary of what the code actually does"
}
```

**Verdict rules:**
- **PASS** — The named type/function/module exists, accepts the spec-defined parameters, returns the spec-defined types, is wired into a production code path, and has at least one non-trivial test.
- **FAIL** — The named thing does not exist, or exists but is a stub (returns `unimplemented!()`, `todo!()`, hardcoded default with no computation), or is never called from any production path.
- **PARTIAL** — Exists and is partially wired but some spec-defined behavior is missing or has a `SPEC_DEVIATION` that skips critical functionality.
- **UNVERIFIABLE** — Cannot determine because the claim is too vague or the component is blocked by a known integration gap (from Phase 0).

Wait for all workers. Aggregate into a unified findings table.

---

## Phase 3: Test Count & Quality Verification

Farm one `build-worker` subagent per crate in `crates/` in parallel. Each receives its crate path. Each returns:

```json
{
  "crate": "hyperfluid-pdp",
  "claimed_test_count": 58,
  "actual_test_count": 57,
  "count_verdict": "PASS" | "INFLATED" | "DEFLATED",
  "tests_with_zero_assertions": [
    {"name": "test_load_chunk_not_found", "path": "store.rs:268"}
  ],
  "trivially_true_assertions": [
    {"name": "test_identity_peer_id", "path": "identity.rs:220", "code": "assert_eq!(id.peer_id(), id.peer_id())"}
  ],
  "negative_test_count": 12,
  "total_test_count": 58,
  "negative_ratio": 0.21,
  "negative_ratio_verdict": "PASS" | "LOW" | "CRITICAL",
  "mocked_only_tests": [
    {"name": "...", "detail": "Uses zero-valued inputs for all non-trivial fields"}
  ],
  "tests_that_dont_test_what_they_say": []
}
```

**Quality rules:**
- `count_verdict = INFLATED` if actual < claimed. `DEFLATED` if actual > claimed by more than 10% (means doc is stale, not a lie, but still flag it).
- A test has **zero assertions** if the `#[test]` body contains no `assert!`, `assert_eq!`, `assert_ne!`, `should_panic`, `Result::Err` unwrap, or explicit `assert` macro.
- A test is **mocked-only** if every non-trivial struct field in its test data is set to `0`, `false`, `vec![]`, `Vec::new()`, `None`, or `""` — meaning no real data ever flows through the code paths under test.
- `negative_ratio_verdict = CRITICAL` if negative tests (names containing "error", "fail", "reject", "invalid", "bad", "wrong", "empty", "nonexistent", "corrupt", "tampered") are <5% of total. `LOW` if 5-15%. `PASS` if >=15%.

Wait for all workers. Aggregate into a unified test-quality table.

---

## Phase 4: Dead / Stub Code Scan

Farm one `build-worker` subagent per crate in parallel. Each receives its crate path. Each returns:

```json
{
  "crate": "hyperfluid-consensus",
  "spec_deviations": [
    {"path": "driver.rs:120", "detail": "ml_dsa_verify is stub — always returns true"}
  ],
  "unused_pub_fns": [
    {"fn": "compute_offline_metrics", "path": "driver.rs:350", "callers_found": 0}
  ],
  "accepted_but_ignored_params": [
    {"fn": "submit_tx", "path": "driver.rs:200", "param": "nonce", "detail": "nonce parameter is logged but never validated"}
  ],
  "wildcard_swallows": [
    {"path": "rule_chain.rs:88", "detail": "match arm `_ => true` in PDP validation — passes all unknown tx types"}
  ],
  "config_keys_not_read": [],
  "placeholder_fields": [
    {"struct": "BlockHeader", "path": "types.rs:42", "field": "signer_set_hash", "value": "[0u8; 32]", "detail": "Zero hash in every production path (never populated from real data)"}
  ],
  "stub_functions": [
    {"fn": "establish_secure_channel", "path": "secure_channel.rs:100", "detail": "Returns Ok(()) without establishing any connection"}
  ],
  "dead_code_verdict": "PASS" | "HAS_DEAD" | "HAS_STUBS"
}
```

**Detection rules:**
- **Unused pub fn:** grep for `pub fn <name>`. Then grep for `<name>(` across ALL crates (excluding the definition file and test files). Zero non-test callers = unused.
- **Accepted but ignored params:** grep for functions that accept a parameter but never reference it in the body (beyond assignment to `_`).
- **Wildcard swallows:** In non-test code, grep for `_ => true` and `_ => false` in match expressions that handle protocol-level enums. Flag every one — these are almost always lazy "done" claims.
- **Placeholder fields:** grep for struct literals where fields are set to `[0u8; N]`, `0u64`, `0u128`, `Address32::default()`, `Hash32::default()`, `vec![]`, `None` — these are staging fields in "complete" code.
- **Stub functions:** Functions that do nothing except return `Ok(())`, `true`, `false`, or a hardcoded value with no side effects and no computation on inputs.
- **Config keys not read:** Cross-reference documented CLI flags or config keys in `build-status.md` or spec files against actual `clap` attributes or config-reading code. Flag any that are documented but never read.

Wait for all workers. Aggregate findings.

---

## Phase 5: Spec-Formula Cross-Check (local, no subagent)

Run locally (not in subagent). For each of these 5 core formulas, extract the spec text and the code implementation, then compare structurally:

| Formula | Spec Source | Code Location |
|---------|------------|---------------|
| EIP-1559 base fee adjustment | `fee-market-spec.md` | `crates/hyperfluid-fee-market/src/lib.rs` |
| Committee stake-weighted sampling | `consensus-spec.md` §1.4 | `crates/hyperfluid-consensus/src/types.rs` |
| PDP quota effective limit | `policy-engine-spec.md` §2.5 | `crates/hyperfluid-pdp/src/quota.rs` |
| Reward distribution (90/10 split) | `review-engine-spec.md` §1.5 | `crates/hyperfluid-state/src/state_machine.rs` |
| State root computation (SMT) | `consensus-spec.md` §2.2 | `crates/hyperfluid-state/src/smt.rs` |

**Comparison method:**
1. Read the spec formula section. Extract the mathematical expression (terms, operators, order of operations).
2. Read the code implementation. Extract the same expression.
3. Check for:
   - Missing terms (spec says X but code never uses X)
   - Extra terms (code adds Y that spec doesn't mention — flag as SPEC_DEVIATION or undocumented feature)
   - Wrong operator (spec says `ceil(N * threshold / total)` but code uses integer division without `div_ceil`)
   - Wrong type (spec says `u128` but code uses `u64` — precision loss)
   - Missing edge-case handling (spec mentions overflow/underflow/zero-divisor guard but code lacks it)
   - Different parameter names suggesting unit mismatch

Output a table:
| Formula | Verdict | Issue |
|---------|---------|-------|
| Base fee adjustment | PASS | Code matches spec |
| Committee sampling | DRIFT | Code uses `div_ceil` but spec says `floor` |
| ... | ... | ... |

**Verdicts:** `PASS`, `DRIFT` (minor mismatch), `WRONG` (semantic difference), `UNVERIFIABLE` (spec too vague).

---

## Phase 6: Integration Path Verification

Farm one `build-worker` per component that has any `INTEGRATION_CLAIM` or `COMPLETE_CLAIM` with integration claims (e.g., "PDP wired into node binary", "P2P TCP transport integrated").

Each worker receives:
- The claim
- The component crate path
- The node binary path (`crates/hyperfluid-node/`)
- All other crate paths

Each returns:

```json
{
  "claim_id": "C-012",
  "component": "PDP",
  "verdict": "WIRED" | "LIBRARY_ONLY" | "TEST_ONLY" | "UNUSED",
  "callers": {
    "production": ["hyperfluid-node/src/main.rs:300", "hyperfluid-consensus/src/driver.rs:150"],
    "test_only": ["hyperfluid-pdp/src/tests.rs:45"]
  },
  "integration_tests_found": 2,
  "verdict_reason": "PDP is actually called from the block production loop in driver.rs and main.rs, with 2 integration tests exercising it through the node binary."
}
```

**Verdict rules:**
- `WIRED` — Component's public API is called from at least one non-test production path (main.rs, driver.rs, server startup, etc.) AND has at least one integration test (a test that starts the component through its public interface with real I/O).
- `LIBRARY_ONLY` — Component compiles and has unit tests, but is never called from any production code path (only called from test code or other tests).
- `TEST_ONLY` — Component is only exercised via `#[cfg(test)]` callers; zero production callers.
- `UNUSED` — Component crate compiles but is never linked into the binary, or all its public functions are dead.

Wait for all workers. Aggregate.

---

## Phase 7: Report and Fix

### 7a: Build the unified verdict table

Combine all findings from Phases 2-6 into a single table:

| # | Type | Claim | Source | Verdict | Evidence Detail | Severity |
|---|------|-------|--------|---------|----------------|----------|
| 1 | COMPLETE_CLAIM | P2P TCP transport | build-status.md:87 | **HONEST** | Functions exist, wired into main.rs, 53 tests | info |
| 2 | COMPLETE_CLAIM | PDP fully wired | build-status.md:664 | **LIAR** | PDP rule_chain.rs wildcard _ => true on line 88 swallows 4 unknown tx types | critical |
| 3 | TEST_COUNT | 58 PDP tests | build-status.md:657 | **INFLATED** | Actual count: 57 tests (57 < 58) | medium |
| 4 | TEST_QUALITY | PDP test suite | store.rs:268 | **VACUOUS** | 3 tests have zero assertions — test_load_chunk_not_found at store.rs:268 | medium |
| 5 | SPEC_FORMULA | Committee sampling | types.rs | **DRIFT** | Code uses div_ceil, spec uses floor (may be intentional, needs spec update) | low |
| 6 | INTEGRATION | P2P wired | build-status.md:93 | **LIAR** | TCP listener only bound in test code, not in main.rs | critical |
| 7 | DEAD_CODE | compute_offline_metrics | driver.rs:350 | **LAZY** | Function is pub but has zero callers across all crates | medium |

**Severity classification:**
- **critical** — Claim says COMPLETE but component is entirely stub/unwired/never linked
- **high** — Claim is verifiably wrong (wrong test count, missing functionality, wildcard swallowing in critical path)
- **medium** — Claim is partially wrong (dead code, placeholder fields, minor spec drift)
- **low** — Claim is slightly stale or imprecise (test count slightly off, cosmetic drift)
- **info** — Claim verified exactly correct

**Verdict classification:**
- **LIAR** — Claim says COMPLETE but reality is stub, unwired, or non-existent
- **INFLATED** — Test counts or coverage numbers are wrong by >5%
- **VACUOUS** — Tests exist but assert nothing or are trivially true
- **LAZY** — Dead code, placeholder fields, ignored parameters in "complete" components
- **DRIFT** — Spec formula or interface diverged from implementation
- **HONEST** — Claim matches reality exactly

### 7b: Fix all non-trivial findings

For every finding with severity `critical`, `high`, or `medium`:

1. If it's a **LIAR** claim (component claimed COMPLETE but is actually stub/unwired):
   - Update `build-status.md` to downgrade the claim from COMPLETE to IN PROGRESS or PARTIAL
   - Add a note in the claim's section: "**(Corrected by verify-claims: component has stub/unwired code — see findings)**"
   - Fix the stub by either implementing the real behavior or adding `// SPEC_DEVIATION: [reason]` if intentionally deferred
   - If intentionally deferred, move it to the DEFERRED section and remove from the COMPLETE claim

2. If it's a **VACUOUS** test (zero assertions):
   - Add the missing assertion. If you cannot determine what correct assertion should be, replace the test with `#[ignore]` and add a comment `// TODO: add assertion — no-op test discovered by verify-claims`

3. If it's a **LAZY** finding (dead code, placeholder fields):
   - Remove dead functions (the fn, not the entire module)
   - Replace `[0u8; 32]`/`0u64`/`None` placeholder fields with either real computation or `// SPEC_DEVIATION: placeholder — deferred per [reason]`
   - Wire ignored parameters or mark them with `let _ = param` to silence

4. If it's a **DRIFT** finding (spec vs code mismatch):
   - If the code is correct, update the spec to match
   - If the spec is correct, fix the code to match
   - If uncertain, file an ADR and add `// SPEC_DEVIATION`

5. If it's an **INFLATED** test count:
   - Update `build-status.md` and the offending checkpoint file to reflect the correct count

### 7c: Update documentation

- Create `docs/08-handoff/latest/checkpoint-YYYY-MM-DD-verify-claims.md` containing:
  - Summary of the verify-claims run
  - The full verdict table (7a)
  - Per-finding resolution (what was fixed and how)
  - Updated test counts and statuses

- Update `docs/08-handoff/latest/build-status.md`:
  - Downgrade any LIAR claims
  - Correct any INFLATED test counts
  - Add "Verified by verify-claims YYYY-MM-DD" to each claim that passed

- Update `PROJECT-STATUS.md`:
  - Update "Last updated"
  - Add a "Claims Verification" section with a link to the checkpoint
  - Move any newly-discovered block or blocker into the "Blockers" section

---

## Phase 8: CI Mimic

Run in parallel via 3 `build-worker` subagents:

- **Worker A:** `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
- **Worker B:** `cargo test --workspace && cargo doc --workspace --no-deps --document-private-items`
- **Worker C:** `cargo deny check && cargo bench --workspace --no-run`

Wait for all three. If any fails, run the failing tool locally (not in a subagent) to get the exact error output and fix it. Re-run the full CI locally after fixing to confirm everything passes. Do not actually commit or push to github.

---

## Self-Improve the Build Process

After documenting all findings, review the systemic patterns. For each pattern, check whether the current TDD cycle in `.opencode/commands/execute-build/checkpoint.md` (the determinism sweep) or `.opencode/commands/execute-build/testing-tdd.md` (the TDD checklist) would have caught it during initial implementation (not during this audit).

If a systemic pattern would NOT have been caught by any existing guard, **append it to `.opencode/commands/execute-build/checkpoint.md`** as a new line at the end (before the file-save line). Use this exact format:

```
- **short-name guard:** [one-sentence description of what to check and why]. [Concrete grep/scan command or manual instruction] — [consequence of not doing this].
```

**Guard examples (not specific finding references):**
- `- **test-assertion guard:** After writing a `#[test]`, verify the body contains at least one `assert`/`assert_eq`/`assert_ne` call. A `#[test]` that runs code without asserting anything is a no-op — it cannot fail and provides zero verification. Run `Select-String -Pattern "^\s*#\[test\]" -Path "$crate/**/*.rs" -Context 0,10 | Select-String -NotMatch "assert!"` to find assertion-free tests.`
- `- **claim-consistency guard:** When a task-status table in `build-status.md` says "N tests", verify the actual `#[test]` count matches N. Run `Select-String -Pattern "^\s*#\[test\]" $(Get-ChildItem "crates/$crate/**/*.rs" | Resolve-Path -Relative) | Measure-Object | Select-Object -ExpandProperty Count`.`
- `- **wildcard-swallow guard:** In non-test `match` expressions over protocol enums (TxType, ActionType, DenyReason, TaskStatus), grep for `_ =>` before every checkpoint. A wildcard arm in protocol dispatch means the compiler will not warn you when a new variant is added — new behavior silently defaults to whatever the wildcard says. Grep for `_\s*=>\s*(true|false|Ok\(|Err\(|return)` in non-test code. Each wildcard must be justified or replaced with explicit arms.`
- `- **production-caller guard:** Before marking a component as `COMPLETE`, verify its `pub fn` API is called from at least one non-test production path. Grep the crate's public functions across `crates/hyperfluid-node/` and other crate `src/` dirs (not tests). Zero production callers = the component is a library, not a completed integration.`
- `- **placeholder-field guard:** After adding a struct representing protocol state, verify every field is populated from a non-default value in at least one production path. Grep for `[0u8; 32]`, `0u64`, `0u128`, `Hash32::default()`, `vec![]`, `None` in struct constructor expressions. Placeholder fields in "complete" components are staging artifacts — either implement the real derivation or add `// SPEC_DEVIATION: [reason]`.`

Keep guards generic — describe the pattern to catch, not the specific finding. Do not reference claim IDs or past audit findings.
