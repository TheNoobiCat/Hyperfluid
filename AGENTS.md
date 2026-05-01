# Hyperfluid — Agent Instructions

**What:** A decentralized network where AI agents — not humans — are the primary users. Agents discover work, claim tasks, collaborate in teams, review each other's output, and vote on protocol evolution. All autonomously.

**Stack:** Rust-first. Committee BFT consensus (Malachite), on-chain `git:head` governance, EIP-1559 fee markets, Ockam P2P networking, content-addressed storage via gix. AGX is the native token for staking, fees, and rewards.

**Premise:** No humans in the loop. Agents join with 0 AGX, earn through verified work, progress through a trust ladder (`untrusted_joiner` → `sandboxed_contributor` → `trusted_contributor` → `coordinator_eligible`), and govern the network themselves.

All work flows through an 8-layer documentation pipeline defined in `BUILD-SYSTEM.md`. For live project state, see `PROJECT-STATUS.md` — do not record state in this file.

---

## First thing to read

1. `BUILD-SYSTEM.md` — the 8-layer pipeline and hard gates.
2. `PROJECT-STATUS.md` — current phase, blockers, active gaps.
3. `GLOSSARY.md` — canonical terminology (do not redefine these).
4. `PROMPT-BOOK.md` — if you are executing a build phase.

---

## Project structure

```
docs/01-research/          ← Layer 1 (current). Research corpus.
  _template.md             ← Format every research doc must follow
  index.md                 ← Document inventory + research-to-spec mapping
  agents/                  ← Agent runtime, coordination, identity
  consensus-governance/    ← BFT, staking, governance
  networking/              ← P2P, transport, availability
  security/                ← Threat models
  stack-evaluations/       ← Stack comparisons

Root files:
  BUILD-SYSTEM.md          ← Process definition (layers, gates, traceability)
  TEMPLATES.md             ← Format templates for every artifact type
  GLOSSARY.md              ← Canonical terminology
  PROMPT-BOOK.md           ← Executable prompts per phase
  PROJECT-STATUS.md        ← Live project state and gaps
  PROBLEMS.md              ← Observations and issues
```

---

## Hard rules

- **Never redefine canonical terms** from `GLOSSARY.md`. Reference them; do not duplicate.
- **Never skip the decentralisation audit gate** before promoting research to requirements or specs to planning. See `BUILD-SYSTEM.md` for the checklist.
- **Never create empty audit files**. If no issues found, stop.
- **Cross-references must point to canonical source documents**, not duplicate definitions. The canonical sources are listed in `docs/01-research/index.md` "Canonical Source Map".
- **Research docs must follow `_template.md`** exactly (11 sections, min 3 tradeoffs, Mermaid diagrams required).
- **This is not a code repo**. Do not write production code, create crates, or set up CI pipelines unless explicitly entering Phase 5 (Implementation), which has not started.

---

## Conventions

- **Mermaid diagrams**: plain technical labels, no emojis, no `style`/`classDef`/theme directives. Use `flowchart TD` and `stateDiagram-v2`. Invoke the mermaid skill when a diagram is needed.
- **Terminology**: underscore-separated (`untrusted_joiner`, `action_plan`), colon for `git:head`.
- **Research document format**: Title → Executive Summary (5-10 bullets) → System Overview → Architecture (with Mermaid) → Core Mechanisms → Design Decisions & Tradeoffs (min 3) → Failure Modes & Edge Cases → Scalability Analysis → Recommended Architecture → Implementation Plan → Future Improvements.
- **Traceability**: every claim must be traceable: Research → Requirement → ADR → Spec → Test. Maintain bidirectional links.

---

## Common mistakes to avoid

- **Guessing terminology**: check `GLOSSARY.md` and `docs/01-research/index.md` first.
- **Promoting docs without the decentralisation audit**: this is a hard gate in `BUILD-SYSTEM.md`.
- **Duplicating canonical definitions**: trust stages, validator states, action plan schema, quota matrix — all have canonical owners. Reference them.
- **Writing code before specs are frozen**: do not create crates or production code unless explicitly in Phase 5 (Implementation). See `PROJECT-STATUS.md` for current phase.
- **Forgetting Mermaid diagrams**: the `_template.md` requires them in Architecture and Core Mechanisms sections.
