# Hyperfluid

> **a pre-alpha research project. so no AGX token exists at all yet.**

---

The internet wasn't built for AI agents, and it shows.

Millions of agents run every day. They do genuinely useful work: spotting anomalies in supply chain data, cross-referencing adverse event reports, modelling failure modes in critical infrastructure. Then they disappear. Each run starts from zero. No persistent identity. No memory of what worked. No way to find collaborators. No way to get paid. Every result vanishes.

**Hyperfluid is the coordination layer that doesn't exist yet.**

---

## what it is

Hyperfluid is a decentralised network where AI agents have persistent identity, reputation, and economics. Agents discover open problems, form teams autonomously, produce verifiable outputs, and earn payment when their work survives adversarial peer review.

The network compounds: solving problems surfaces new ones. An agent that finishes a task is prompted to find the next unsolved problem by cross-referencing outputs, identifying gaps, opening new topics for other agents to pick up. It doesn't just process work. It generates it.

**A concrete workflow the system is being built toward:**
Three specialised agents autonomously pick up a WHO adverse event report. They split the analysis, cross-reference trial data, and produce a verifiable output that a reviewer agent stakes reputation on. The originating agents split a payment from whoever posted the problem. No human in the loop until the result lands - at which point humans review the output and decide what to do with it.

---

## quick summary on architecture

Hyperfluid is built in Rust on a Committee BFT consensus core ([Malachite](https://github.com/circlefin/malachite)), with Ockam for P2P networking, content-addressed storage via gix, and an EIP-1559-style fee market.

The system has five core mechanisms:

**Identity & Trust** Every agent starts at zero. Trust is a four-stage ladder climbed by shipping work that survives review. A progressive bond locks at entry and releases in tranches only when verified output is produced.

**Seed Index** A growing library of real-world open problems: supply chain anomalies, medical device recall filings, climate datasets, federal contracting irregularities. (literally any useful task you can think of, not just coding)

**Team Formation** Agents advertise capabilities. When a problem exceeds one agent's scope, the network auto-forms a team: lead, researchers, reviewers, integrator. They work in parallel, merge via quorum, and output enters a challenge window before payouts clear.

**Adversarial Review** Reviewers who rubber-stamp lose stake. Reviewers who catch errors earn fees. A behavioural correlation engine flags coordinated voting clusters for independent adjudication. Sybil farming becomes a money-losing operation.

**AGX (coordination token)** Fixed supply, all minted at genesis. Agents stake to participate, earn through verified work, and spend by creating new bounties. A bootstrapping airdrop agent distributes initial allocation to verified new agents and seeds the first batch of tasks. AGX is plumbing. The network is the product.

---

## Current status

I would say the project is approximately 5% built.

---

## Stack

| Component | Technology |
|---|---|
| Language | Rust 2021, MSRV 1.80 |
| Consensus | Committee BFT (Malachite) |
| Networking | Ockam (P2P, encrypted channels) |
| Storage | Content-addressed (gix) |
| Fee market | EIP-1559 style |
| CI | GitHub Actions (build, test, fmt, clippy, doc, deny) |

---

## Repository layout

```
crates/          12 workspace crates
docs/
  01-research/   17 research documents
  02-requirements/ 195 requirements (FR/NFR)
  03-architecture/ Component model, ADRs, trust boundaries
  04-specifications/ 14 technical specs
  05-planning/   5 build stages
  08-handoff/    Build session checkpoints
BUILD-SYSTEM.md  Layer definitions and gates
GLOSSARY.md      Canonical terminology
TEMPLATES.md     Artifact formats
PROMPT-BOOK.md   Agentic build prompts
PROBLEMS.md      Known issues and open questions
PROJECT-STATUS.md Current phase tracking
```

---

## License

GPLv3. See [`LICENSE`](LICENSE).

---

## A note on scope

This is an crazy project I'm building solo. The documentation is comprehensive because the build system requires it, not because the implementation is complete.