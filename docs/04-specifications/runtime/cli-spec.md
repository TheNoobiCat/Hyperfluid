# Runtime Spec: `hyperfluid` CLI

**Component:** C10 Agent Runtime
**Source ADRs:** ADR-0013 (Expanded Agent Tools and CLI Seed Index Discovery)
**Covered FRs:** FR-0068, FR-0069, FR-0199
**Dependencies:** C9 Policy Decision Point, C2 State Machine, C11 Collaboration & Inbox

---

## Section 1: CLI Command Taxonomy

### 1.1 Purpose

Define the complete `hyperfluid` CLI command tree for all network-mutating and query actions. The CLI is the sole interface for agents and operators to interact with protocol state.

### 1.2 Normative Behavior

- The system MUST expose a single `hyperfluid` binary.
- All CLI output MUST be machine-parseable JSON.
- The CLI spec MUST be embedded verbatim in the agent system prompt (see agent-runtime-spec.md §3).
- No runtime command discovery — the spec in the prompt is the canonical reference.
- The CLI covers: transactions, queries, task operations, review operations, governance, agent self-management, and idea seed index discovery.
- All network-mutating commands MUST route through the Policy Decision Point (PDP).

### 1.3 Command Tree

```
hyperfluid
├── tx
│   ├── transfer     --to <address> --amount <atto-agx>
│   ├── stake        --action <bond|renew|unbond|withdraw> --amount <atto-agx>
│   ├── delegate     --action <delegate|undelegate|withdraw|set-commission>
│   │                   --to <validator> [--amount <atto-agx>] [--commission-rate <0-20>]
│   ├── identity     --reveal-pubkey <pubkey>
│   ├── governance   --propose <proposal-file>
│   └── evidence     --submit <evidence-file>
│
├── query
│   ├── balance      --address <address>
│   ├── account      --address <address>
│   ├── nonce        --address <address>
│   ├── validator    --address <address>
│   ├── committee    --epoch <epoch>
│   ├── proposal     --id <proposal-id>
│   ├── task         --id <task-id> [--topic <topic>]
│   ├── review       --id <review-id>
│   ├── trust-stage  --address <address>
│   ├── block        --height <height>
│   ├── git-head     [--branch <branch>]
│   └── fee-estimate --tx-type <type>
│
├── task
│   ├── list         [--topic <topic>] [--status <status>] [--seed-ref <seed>]
│   ├── get          --id <task-id>
│   ├── claim        --id <task-id>
│   ├── release      --id <task-id>
│   ├── submit       --title <title> --description-file <path> --bounty <atto-agx>
│   │                   --seed-ref <seed> [--required-skills <hash>] [--sponsor]
│   ├── heartbeat    --id <task-id> [--progress-hash <hash>]
│   ├── lease        --id <task-id> --action <extend|release>
│   └── split        --id <parent-task-id> --children <child-spec-json>
│
├── review
│   ├── list         [--status <status>] [--task-id <task-id>]
│   ├── submit       --id <assignment-id> --verdict <accept|reject> --evidence <hash>
│   ├── challenge    --id <review-id> --evidence <hash>
│   └── claim-rewards
│
├── governance
│   ├── list         [--status <status>]
│   ├── get          --id <proposal-id>
│   ├── vote         --id <proposal-id> --choice <yes|no>
│   ├── fetch-bundle --id <proposal-id>
│   └── verify       --id <proposal-id>
│
├── fastpath
│   ├── list         [--topic <topic>] [--status <status>]
│   ├── propose      --topic <topic> --proposed-head <hash> --manifest <hash>
│   ├── approve      --id <proposal-id>
│   ├── challenge    --id <proposal-id> --evidence <hash>
│   └── status       --id <proposal-id>
│
├── agent
│   ├── list-skills
│   ├── load-skill   <skill-name>
│   ├── status
│   └── key-info
│
└── idea
    ├── list         [--topic <topic>]
    └── get          --ref <seed-ref>
```

### 1.4 State Transitions

- Read-only commands (query, list, get, status, key-info) do not mutate state.
- Mutating commands construct an `ActionPlanRequest`, route through PDP, and submit to the node API.
- On success: the command returns `{ "ok": true, "result": { ... } }`.
- On failure: the command returns `{ "ok": false, "error": { "code": "...", "message": "..." } }`.

### 1.5 Failure Behavior

- Invalid flags or arguments: structured error with usage hint.
- PDP rejection: return the `DenyReason` code from the rule chain.
- Network error: retry with 3-second backoff, max 3 attempts.
- Timeout: return `{ "ok": false, "error": { "code": "TIMEOUT", "message": "..." } }`.

### 1.6 Versioning and Compatibility

- CLI commands and flags are append-only within major protocol versions.
- Removing or renaming a flag requires a governance proposal.
- The CLI spec in the system prompt is pinned to policy bundle hash.

### 1.7 Conformance Test Hooks

- Verify `hyperfluid query balance` returns correct balance for known address.
- Verify `hyperfluid task submit` with valid args constructs and submits an action plan.
- Verify `hyperfluid task submit` with missing `--seed-ref` is rejected with structured error.
- Verify `hyperfluid idea list` returns at least the entries from the seed index.
- Verify CLI output is valid JSON for all commands.
- Verify all mutating commands route through PDP (rejected plans return structured deny).

### 1.8 Trust-Assumption Inventory

- CLI binary integrity
  - Justification: Agent relies on local `hyperfluid` binary to construct correct action plans.
  - Trust-minimised alternative: Multi-platform binary verification via `git:head` governance.
- Node API availability
  - Justification: CLI requires node API to submit transactions and queries.
  - Trust-minimised alternative: Local transaction construction with async submission.
