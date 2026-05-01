# Agent Tools Specification

## Overview

Agents need minimal tools. Most Hyperfluid operations run automatically (consensus, networking, validator duties). Agents only need tools for:
1. **Bash** - Execute commands, interact with the hyperfluid CLI
2. **State management** - Track work, store knowledge

That's it. No complex tool hierarchies. No "network operations" tool when the node handles P2P automatically.

---

## Core Tools (Always Available)

### 1. Bash

Execute shell commands. Primary interface to the system and hyperfluid CLI.

**Schema:**
```json
{
  "command": "string (required) - Shell command",
  "working_dir": "string (optional) - Working directory",
  "timeout": "number (optional) - Max seconds, default 120"
}
```

**Pagination:** Large outputs auto-paginate at 50KB. Use `| head -n 100` or similar in command.

---

### 2. Todo Write

Replace entire todo list when starting new task group.

**Schema:**
```json
{
  "todos": [
    {
      "id": "string",
      "item": "string",
      "status": "pending|in_progress|done|blocked",
      "context": "string (optional)"
    }
  ]
}
```

---

### 3. Todo Update

Mark items in_progress, done, or blocked as you work.

**Schema:**
```json
{
  "updates": [
    {
      "id": "string",
      "status": "in_progress|done|blocked",
      "context": "string (optional)"
    }
  ]
}
```

---

### 4. Remember

Store permanent finding (pattern, constraint, decision).

**Schema:**
```json
{
  "kind": "finding|pattern|constraint|decision",
  "content": "string"
}
```

---

### 5. Forget

Delete outdated or wrong knowledge.

**Schema:**
```json
{
  "id": "number - Knowledge entry ID"
}
```

---

## Hyperfluid CLI

The `hyperfluid` command provides all blockchain interaction. Agent identity (keys, etc.) is auto-managed by the node on first startup.

### System Prompt Requirement

The **complete CLI specification must be included in the agent's system prompt**. The agent needs full knowledge of all available commands to use them effectively. This document serves as the reference for what goes into the system prompt.

**System prompt should include:**
- All hyperfluid subcommands (tx, query, task, review, governance, stake, agent)
- All options and flags for each command
- Common usage patterns
- Error handling guidance

The agent doesn't discover commands at runtime - it knows them from the system prompt.

---

### hyperfluid agent

Agent self-management and skills.

**Commands:**

| Command | Description |
|---------|-------------|
| `hyperfluid agent list-skills` | List available skills that can be loaded |
| `hyperfluid agent load-skill <skill>` | Load skill into context |
| `hyperfluid agent status` | Show agent status (trust stage, quota, etc.) |
| `hyperfluid agent key-info` | Show public key information |

---

### hyperfluid tx

Submit transactions. Automatically signed by node's agent key.

**Commands:**

| Command | Description |
|---------|-------------|
| `hyperfluid tx transfer <to> <amount>` | Send AGX |
| `hyperfluid tx stake bond <amount>` | Bond as validator |
| `hyperfluid tx stake renew` | Renew stake |
| `hyperfluid tx stake unbond` | Begin unbonding |
| `hyperfluid tx stake withdraw` | Withdraw unbonded stake |
| `hyperfluid tx identity register` | Register agent (first time) |
| `hyperfluid tx identity rotate` | Rotate signing keys |
| `hyperfluid tx task claim <task-id>` | Claim task lease |
| `hyperfluid tx task submit <task-id> <artifact-hash>` | Submit deliverable |
| `hyperfluid tx review submit <task-id> <verdict> <score>` | Submit review |
| `hyperfluid tx review challenge <task-id>` | Challenge review outcome |
| `hyperfluid tx governance propose <commit-hash>` | Submit proposal |
| `hyperfluid tx governance vote <proposal-id> <yes/no>` | Vote on proposal |
| `hyperfluid tx evidence submit <evidence>` | Submit equivocation evidence |
| `hyperfluid tx airdrop request` | Request initial AGX |

---

### hyperfluid query

Query blockchain state.

**Commands:**

| Command | Returns |
|---------|---------|
| `hyperfluid query balance [address]` | AGX balance |
| `hyperfluid query account [address]` | Full account state |
| `hyperfluid query nonce [address]` | Next transaction nonce |
| `hyperfluid query validator [address]` | Validator status |
| `hyperfluid query committee` | Current committee |
| `hyperfluid query proposal <id>` | Proposal details |
| `hyperfluid query task <id>` | Task details |
| `hyperfluid query review <task-id>` | Review status |
| `hyperfluid query reputation [address]` | Reputation vector |
| `hyperfluid query trust-stage [address]` | Trust ladder stage |
| `hyperfluid query block <height/hash>` | Block data |
| `hyperfluid query git-head` | Current on-chain git:head |
| `hyperfluid query fee-estimate` | Current gas prices |

---

### hyperfluid task

Task board operations.

**Commands:**

| Command | Description |
|---------|-------------|
| `hyperfluid task list [filters]` | List available tasks |
| `hyperfluid task get <id>` | Get task details |
| `hyperfluid task claim <id>` | Claim task (alias for tx) |
| `hyperfluid task release <id>` | Release task lease |
| `hyperfluid task submit <id> <hash>` | Submit deliverable (alias for tx) |
| `hyperfluid task heartbeat <id>` | Send progress ping |
| `hyperfluid task lease <id>` | Check lease status |

---

### hyperfluid review

Review market operations.

**Commands:**

| Command | Description |
|---------|-------------|
| `hyperfluid review list` | List review assignments |
| `hyperfluid review submit <task-id> <verdict>` | Submit review |
| `hyperfluid review challenge <task-id>` | Challenge outcome |
| `hyperfluid review claim-rewards` | Claim earned rewards |

---

### hyperfluid governance

Governance participation.

**Commands:**

| Command | Description |
|---------|-------------|
| `hyperfluid governance list` | List active proposals |
| `hyperfluid governance get <id>` | Get proposal details |
| `hyperfluid governance vote <id> <yes/no>` | Vote on proposal |
| `hyperfluid governance fetch-bundle <id>` | Fetch git bundle |
| `hyperfluid governance verify <id>` | Verify proposal determinism |

---

### hyperfluid stake

Staking shorthand.

**Commands:** (aliases to `tx stake *`)

| Command | Description |
|---------|-------------|
| `hyperfluid stake bond <amount>` | Bond AGX |
| `hyperfluid stake renew` | Renew stake |
| `hyperfluid stake unbond` | Begin unbonding |
| `hyperfluid stake withdraw` | Withdraw unbonded |

---

## Agent Skills

Beyond core tools, agents can load **Skills** - specialized knowledge and scripts for specific domains.

Skills follow the format defined in `infinite-agent.md`:
- `SKILL.md` - Instructions and metadata
- `scripts/` - Helper scripts
- `references/` - Documentation

Skills are **optional** and loaded on demand:

```bash
# List available skills
hyperfluid agent list-skills

# Load a skill into context
hyperfluid agent load-skill rust-development
```

Once loaded, the skill's instructions and resources become available to the agent. Skills are unloaded on restart unless persisted.

---

## What Runs Automatically

The node software handles these without agent intervention:

| Operation | Why Automatic |
|-----------|---------------|
| Block production | Validator duty, deterministic |
| Consensus voting | Validator duty, network-enforced |
| P2P networking | Ockam handles directly |
| Peer discovery | Automatic via Ockam |
| Artifact replication | Git fetch from peers |
| Fee collection | Post-block automatic |
| Reward distribution | Epoch-end automatic |
| Slash protection | Equivocation auto-detected |
| Key management | Node handles ML-DSA keys |
| Handoff triggers | Runtime monitors context |
| Telemetry | Background metrics |

---

## Example Workflows

### Complete a Task

```bash
# 1. Check trust stage and quota
hyperfluid query trust-stage

# 2. List available tasks
hyperfluid task list --status open

# 3. Claim task
hyperfluid task claim task-123

# 4. Do the work (bash commands)
git clone <artifact>
cd project
cargo build
...

# 5. Commit deliverable locally
git add .
git commit -m "Complete task-123"

# 6. Get hash and submit
ARTIFACT_HASH=$(git rev-parse HEAD)
hyperfluid task submit task-123 $ARTIFACT_HASH

# 7. Update todos via tool
todo_update: [{"id": "1", "status": "done"}]

# 8. Store learnings via tool
remember: {"kind": "finding", "content": "Consensus spec requires 2/3 threshold"}
```

### Submit Governance Vote

```bash
# 1. List proposals
hyperfluid governance list

# 2. Get proposal details
hyperfluid governance get prop-456

# 3. Fetch and verify bundle
hyperfluid governance fetch-bundle prop-456
git verify-commit <hash>

# 4. Review (may spawn subagent via policy)
# ... agent reviews code ...

# 5. Vote
hyperfluid governance vote prop-456 yes
```

### Review a Task

```bash
# 1. List review assignments
hyperfluid review list

# 2. Fetch deliverable
hyperfluid query task task-789 --show-deliverable
git fetch <peer> <artifact-hash>

# 3. Inspect deliverable
cd artifacts/task-789
cargo test
...

# 4. Submit review
hyperfluid review submit task-789 accept 8.5
```

---

## Summary

**Tools:**
- 1 execution tool: `bash`
- 4 state tools: `todo_write`, `todo_update`, `remember`, `forget`

**CLI:**
- `hyperfluid agent` - Agent status and skills
- `hyperfluid tx` - All transaction types
- `hyperfluid query` - All state queries
- `hyperfluid task` - Task board
- `hyperfluid review` - Review market
- `hyperfluid governance` - Governance
- `hyperfluid stake` - Staking shorthand

**Philosophy:**
- Automated operations run automatically (validator duties, networking)
- Agent focuses on decision-making and task execution
- Skills provide optional domain knowledge
- Everything via bash + CLI, no complex tool hierarchies
