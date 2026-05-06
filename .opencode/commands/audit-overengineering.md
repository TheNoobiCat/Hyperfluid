---
description: "Audit specs for overengineering, unnecessary complexity, and stale features"
---

Read `GLOSSARY.md`, then:

1. **Scan all non-research docs** (`docs/02-requirements/`, `docs/03-architecture/`, `docs/04-specifications/`, `docs/05-planning/`) for:

   **Stale/dead features:**
   - Fields, structs, enums, or key prefixes referencing deleted features
   - Requirements or specs marked "(removed)" or "(superseded)" still in the index
   - Types or interfaces defined in two places with different fields (drift risk)

   **Unnecessary complexity:**
   - Hardcoded magic numbers with no rationale (thresholds, percentages, limits)
   - Percentage allocations that sum to more than 100%
   - Overly elaborate state machines for what should be a simple flag
   - Taxonomies/enums with variants that are never referenced outside their definition
   - Features described as "optional" or "nice-to-have" with full specs, conformance tests, and trust inventories

   **Nonsense:**
   - Self-referential dependencies (FR requiring itself)
   - Definitions of things that contradict the spec's own normative behavior
   - Sections that say "this doesn't exist" instead of just not mentioning it
   - Comments documenting design decisions that were rejected (not the decision, the rejection)

2. **For each finding**, check:
   - Is it actually dead (feature was removed, file was kept)?
   - Is it false precision (8192 because 2^13, not because it's the right number)?
   - Is it speculative engineering (designed for scale/attack that doesn't exist yet)?

3. **Output** a table with:
   - File and line
   - The overengineered thing
   - What to do: DELETE, SIMPLIFY (describe how), or FLAG (needs design decision)

4. **Apply the fixes** unless the finding is FLAG'd. For FLAG'd items, write to `PROJECT-STATUS.md` under "Open Design Questions".
