---
description: "Read-only scan for production-unready code — stubs, skipped validation, placeholders, dead weight"
---

Read-only scan. Makes no changes. Finds code that compiles but would not ship to production.

Read `PROJECT-STATUS.md` and `docs/08-handoff/latest/build-status.md` to determine which components are claimed as done for the current stage. Then farm one `build-worker` per crate in `crates/` in parallel. Each worker scans all `src/` files (not `tests/`) and returns findings for its crate.

Each worker checks these patterns and returns structured results:

## 1. Signature / crypto stubs

Anywhere the code claims to verify a signature, identity, or proof but doesn't actually do it. Look for:
- Functions that always return `true` for signature verification
- Comments like `stub`, `not yet wired`, `deferred`, `placeholder`, `mock` on crypto/identity operations
- Signatures built with empty bytes (`vec![]`) instead of real cryptographic output
- Verification functions that log or print but return success regardless
- `pdp_bypass` or similar development flags that turn off security checks in production paths

**Severity:** critical — these are the #1 "would not ship" pattern.

## 2. Unimplemented / todo in production code

Every `unimplemented!()`, `todo!()`, or `panic!("not implemented")` in a `src/` file. Also flag any function whose body is literally just `unimplemented!()` or `todo!()`.

**Severity:** critical if in a code path that executes during normal operation. medium if in an error handler or rarely-triggered path.

## 3. Critical unwrap/expect in non-startup code

All `.unwrap()` and `.expect()` calls in production `src/` files. Exclude ones that only run during startup/initialization (bind, listen, config load, genesis setup). Flag any in:
- State machine transaction handlers
- Network I/O handlers
- Crypto operations
- Consensus message processing
- Lock acquisition on shared state

**Severity:** high — panics in these paths crash the node.

## 4. Hardcoded zero hashes / identities

Places where critical identifiers, hashes, or addresses are hardcoded to `[0u8; 32]`, `[0u8; N]`, `0u64`, `0u128` instead of being derived from real computation. Exclude:
- Buffer initialization (reading into a zeroed buffer is fine)
- Genesis block setup (genesis values can be zero)
- Sentinel comparisons (checking `if x == [0u8; 32]` is fine)
- `Default::default()` on types where zero is the correct default

Flag every instance where a hash, identity, root hash, or signature is *assigned* `[0u8; 32]` in non-genesis, non-buffer code.

**Severity:** high if the zero value is used in a cryptographic operation or state commitment. medium if it's a metadata field.

## 5. Mock/default fallback in production paths

Where the production binary silently uses a mock implementation because the real one is behind an opt-in feature flag. Look for:
- `#[cfg(not(feature = "..."))]` that selects a mock as the default
- `cfg!(feature = ...)` fallthroughs where missing feature = mock
- `StubProvider` or `Mock*` types that are the default when config is incomplete

**Severity:** critical — the binary claims to be production but uses toy implementations.

## 6. Empty function bodies

Functions in production code that accept inputs but do nothing with them — return `Ok(())`, `true`, `false`, or a hardcoded constant. Exclude:
- Trivial getters/setters
- Logging-only handlers
- Functions annotated with `#[allow(dead_code)]` or behind `#[cfg(test)]`

**Severity:** medium if it's a handler that should do work. low if it's explicitly a no-op by design.

## 7. Wildcard swallows in protocol dispatch

Non-test `match` expressions over protocol enums (TxType, ActionType, DenyReason, TaskStatus, TrustStage) where a wildcard arm returns a constant (`_ => true`, `_ => false`, `_ => Ok(())`, `_ => Err(...)`). These silently handle any new variant with the same behavior, making the compiler useless for catching missing cases.

**Severity:** high if in a validation/dispatch path. medium if in a logging/metrics path.

## 8. Dead code in "complete" components

Functions marked `pub` that are defined but never called in any non-test production path. Exclude functions marked `#[allow(dead_code)]` (already acknowledged). Also find `#[allow(dead_code)]` annotations in components claimed as COMPLETE — the allow attribute itself is an admission of dead weight.

**Severity:** medium — dead code is noise but signals the component isn't truly wired.

## Output format

Each worker returns:

```json
{
  "crate": "hyperfluid-consensus",
  "findings": [
    {
      "id": "F-001",
      "pattern": "signature_stub",
      "file": "src/driver.rs",
      "line": 1117,
      "severity": "critical",
      "title": "ML-DSA signature verification is stub — always passes",
      "code_snippet": "/// Signature verification (step 2) is a stub — real ML-DSA-65 checking deferred to Week 9-10.",
      "why_it_matters": "Every transaction is accepted without cryptographic signature check. An attacker can forge any transaction."
    }
  ],
  "stats": {
    "critical": 1,
    "high": 3,
    "medium": 5,
    "low": 2
  }
}
```

Wait for all workers. Aggregate into a single report grouped by severity. Do NOT make any changes — just output the report.

## Report format

Output:

```
# verify-claims: Production-Readiness Gaps
Date: YYYY-MM-DD
Stage: [current stage from PROJECT-STATUS.md]

## Critical (would not ship)
| # | Crate | File:Line | Pattern | Issue |
|---|-------|-----------|---------|-------|
...

## High (will break in production)
...

## Medium (dead weight / noise)
...

## Low (cosmetic)
...

## Summary
- Critical: N
- High: N
- Medium: N
- Low: N
- Total: N
```

Write this in a new file called verify-claims-report-[current date].md inside .opencode/verify-claims-reports/

No CI mimic. No doc updates. No fixes. Read-only.
