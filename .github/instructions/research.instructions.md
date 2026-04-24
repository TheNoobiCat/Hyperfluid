# Research Document Standard (Mandatory)

This instruction file defines the only allowed format for all files under `research/**/*.md`.
Every research document must follow the exact section sequence and headings below.

## 1. Title
- Clear, technical title describing the system or topic.

## 2. Executive Summary
- 5–10 bullet points maximum.
- Explain what the system is.
- Explain why it matters.
- State the key insight or design idea.

## 3. System Overview
- High-level explanation of the system.
- What problem it solves.
- Core design philosophy.
- Key constraints.

## 4. Architecture (CRITICAL SECTION)
- Define all system components clearly.
- Explain how components interact.
- Explain data/message flow.
- Explain network topology where applicable.

### Mandatory requirements
- Include diagrams using the **mermaid skill** (flowcharts, sequence diagrams, architecture diagrams, state machines).
- ASCII diagrams acceptable as fallback only.
- Describe component responsibilities clearly.
- Explain data movement step-by-step.

## 5. Core Mechanisms
- Explain how the system works internally.
- Cover mechanisms such as routing, discovery, consensus/coordination, security model, and protocols where relevant.
- Explain why the mechanism works, not only what it does.

## 6. Design Decisions & Tradeoffs
- For each major design choice, compare alternatives (Option A vs Option B).
- Explain why one option is chosen.
- Explain what is sacrificed.
- Explain what degrades or breaks at scale.

### Mandatory requirements
- Include at least 3 major tradeoffs per document.

## 7. Failure Modes & Edge Cases
- Cover real-world failures such as network partitions, node churn, latency spikes, security attacks, and partial failures.
- For each failure mode, explain:
  - What happens.
  - Why it happens.
  - How the system handles it (or fails).

## 8. Scalability Analysis
- Analyze behavior at:
  - Small scale (10–100 nodes)
  - Medium scale (1k–10k nodes)
  - Large scale (100k+ nodes)
- Include bottlenecks, resource constraints, communication overhead, and relay/routing load where applicable.

## 9. Recommended Architecture
- Provide the final opinionated architecture choice.
- Explain why it is optimal.
- State alternatives rejected.
- Give clear technical justification.

## 10. Implementation Plan
- Provide a step-by-step implementation approach.
- Include technologies, component build order, deployment strategy, testing strategy, and scaling strategy.

## 11. Future Improvements
- List upgrades, research directions, optimizations, and long-term scaling ideas.

## Formatting Rules (Mandatory)
- Use the section headers exactly as defined above.
- Use bullet points for clarity.
- **Use the mermaid skill for diagrams**: Generate flowcharts, sequence diagrams, architecture diagrams, state machines, or class diagrams (especially for Architecture and Core Mechanisms sections).
- ASCII diagrams are acceptable as fallback only.
- Mermaid diagrams must follow the repository style used in `research/infinite-agent/infinite-agent.md`:
  - Prefer `flowchart TD` for flow/architecture diagrams and `stateDiagram-v2` for state machines.
  - Use plain technical labels (no emojis).
  - Do not use Mermaid `style`, `classDef`, theme-init blocks, or custom color directives.
  - Keep labels concise; use `<br/>` only when line breaks are needed for readability.
- Include pseudocode for complex systems.
- Avoid filler text entirely.
- Every sentence must add technical value.

## Enforcement Policy
- Documents not matching this format are non-compliant.
- Missing required sections, missing diagrams (generated via mermaid skill or ASCII fallback), or fewer than 3 major tradeoffs are non-compliant.
- Any non-compliant research document must be revised to match this specification exactly.
