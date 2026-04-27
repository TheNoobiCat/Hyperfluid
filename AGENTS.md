# OpenCode Configuration

This document describes the OpenCode agent and skill configuration for the Hyperfluid project.

## Overview

This project uses [OpenCode](https://opencode.ai) for AI-assisted development. Agents and skills are configured in the `.opencode/` directory.

## Agents

Agents are specialized AI assistants configured for specific tasks. They are defined in `.opencode/agents/` as markdown files with YAML frontmatter.

### Available Agents

#### `hyperfluid-research`

**File**: `.opencode/agents/hyperfluid-research.md`

**Mode**: subagent

**Description**: Research agent for Hyperfluid. Investigates decentralized systems, agent architectures, blockchain protocols, AI infrastructure, distributed systems, and developer tooling. Use when evaluating system design options, comparing architectures, or reviewing prior art.

**Usage**:
```
@hyperfluid-research analyze consensus mechanisms for our agent network
```

**Permissions**:
- `read`: allow - Can read files
- `glob`: allow - Can search for files
- `grep`: allow - Can search file contents
- `edit`: allow - Can edit files
- `bash`: ask - Must ask before running bash commands
- `webfetch`: allow - Can fetch web content
- `websearch`: allow - Can search the web
- `skill`: allow - Can load skills
- `task`: allow - Can invoke subagents

**Focus Areas**:
- Agent Systems (multi-agent orchestration, memory architectures)
- Blockchain and Decentralization (consensus mechanisms, L1/L2 architectures)
- Distributed Systems (event-driven architectures, message passing)
- AI Infrastructure (LLM pipelines, RAG systems, vector databases)
- Developer Infrastructure (CI/CD, build systems)

## Skills

Skills are reusable instructions that agents can load on-demand. They are defined in `.opencode/skills/<name>/SKILL.md`.

### Available Skills

#### `mermaid`

**File**: `.opencode/skills/mermaid/SKILL.md`

**Description**: Generate Mermaid diagrams from user requirements. Supports flowcharts, sequence diagrams, class diagrams, ER diagrams, Gantt charts, and 18 more diagram types.

**Supported Diagram Types**:
- Flowchart, Sequence Diagram, Class Diagram, State Diagram
- ER Diagram, Gantt Chart, Pie Chart, Mindmap
- Timeline, Git Graph, Quadrant Chart, Requirement Diagram
- C4 Diagram, Sankey Diagram, XY Chart, Block Diagram
- Packet Diagram, Kanban, Architecture Diagram, Radar Chart
- Treemap, User Journey, ZenUML

**Usage**:
Agents can load this skill by calling:
```
skill({ name: "mermaid" })
```

Then use it to generate diagrams:
```
Generate a flowchart for the consensus process
```

**Reference Files**:
All Mermaid diagram type documentation is available in `.opencode/skills/mermaid/references/`.

## Project Context

### Hyperfluid

Hyperfluid is a decentralised AI-agent network where agents self-direct work, coordinate with other agents, and share outputs across a peer network.

- **AGX**: Native coordination asset used for staking, governance, and protocol-level incentives
- **Governance**: Includes a canonical `git:head` mechanism for protocol code evolution
- **Research Priorities**:
  - High-assurance decentralization
  - Deterministic and reproducible protocol execution
  - Robust Byzantine and network-failure tolerance
  - Practical implementation feasibility with Rust-first infrastructure

## Configuration

### Global Configuration

Global OpenCode configuration is stored in:
- **Windows**: `%APPDATA%\opencode\opencode.json`
- **macOS**: `~/Library/Application Support/opencode/opencode.json`
- **Linux**: `~/.config/opencode/opencode.json`

### Project Configuration

Project-specific agents and skills are loaded from:
- `.opencode/agents/*.md`
- `.opencode/skills/<name>/SKILL.md`

### Switching Agents

During a session, use the **Tab** key to cycle through primary agents (Build, Plan).

Invoke subagents by @ mentioning them:
```
@hyperfluid-research help me analyze distributed consensus algorithms
```

## Research Standards

All research documents must follow the format defined in `RESEARCH.md` (research document standard at project root):

1. **Title**: Clear, technical title
2. **Executive Summary**: 5-10 bullet points
3. **System Overview**: High-level explanation
4. **Architecture**: Component diagrams (using mermaid skill)
5. **Core Mechanisms**: Internal workings
6. **Design Decisions & Tradeoffs**: At least 3 major tradeoffs
7. **Failure Modes & Edge Cases**: Real-world failure analysis
8. **Scalability Analysis**: Small/medium/large scale behavior
9. **Recommended Architecture**: Opinionated choice with justification
10. **Implementation Plan**: Step-by-step approach
11. **Future Improvements**: Research directions
