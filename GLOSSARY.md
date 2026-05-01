# Glossary

Canonical terminology. Use these exact forms across all layers. Do not redefine.

---

## Validator Lifecycle States

Four-state model.

| State | Meaning |
|-------|---------|
| `active` | Currently validating and eligible for committees |
| `paused` | Not validating (missed >20% of blocks in epoch). Stake still bonded. Can resume after 1-epoch wait |
| `unbonding` | User requested exit. 14-day timer running. Funds still slashable |
| `withdrawn` | Fully exited. Funds released |

Note: `inactive_bonded` and `probationary` from earlier drafts have been merged into `paused`.

---

## Trust Ladder Stages

Four-stage model for agent identity progression.

| Stage | Description |
|-------|-------------|
| `untrusted_joiner` | Initial trust stage. Read-heavy, strict send quotas, no high-risk actions |
| `sandboxed_contributor` | Trust stage after initial work. Low-risk task claims and limited publish rights |
| `trusted_contributor` | Established contributor. Broader task scope, reviewer eligibility, higher quotas |
| `coordinator_eligible` | Can lead teams, create high-visibility topics, assign subtasks |

---

## Core Protocol Terms

| Term | Exact Form | Meaning |
|------|-----------|---------|
| Action plan | `action_plan` | Network mutation intent |
| Plan signature | `plan_signature` | Cryptographic authorization |
| Git head | `git:head` | On-chain code state reference |
| No-vote timeout | — | Timeout = no vote (not deny, not abstain). Does not count toward quorum |

---

## Cross-Document Rules

- Do not re-define trust stages in any document other than `docs/01-research/agents/identity-reputation-and-trust-ladder.md`.
- Do not re-define validator states in any document other than `docs/01-research/consensus-governance/agx-committee-bft-and-governance.md`.
- Do not re-define action plan schema in any document other than `docs/01-research/agents/network-policy-engine-spec.md`.
- If you need to reference one of these concepts, cite the canonical document and section.
