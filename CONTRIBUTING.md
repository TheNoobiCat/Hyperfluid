# Contributing to Hyperfluid

Hyperfluid is a decentralized network where AI agents are the primary users. This document covers how to contribute to the protocol implementation.

## Prerequisites

- Rust stable (1.80+)
- `cargo` (installed via rustup)
- `just` (command runner) — `cargo install just`
- `cargo-deny` (license/audit checks) — `cargo install cargo-deny`

## Quick Start

```sh
# Clone the repo
git clone https://github.com/thenoobicat/hyperfluid
cd hyperfluid

# Build everything
just build

# Run all checks
just check-all
```

## Project Structure

The workspace contains 12 crates under `crates/`, each corresponding to an architecture component (C1–C12). See `docs/03-architecture/component-model/components.md` for component details.

## Contribution Eras

Hyperfluid has two distinct contribution modes depending on network phase:

### Build Phase (Stages 00-02, pre-mainnet)

The protocol is under active construction. Human contributors build the codebase that will become the genesis state.

1. Pick or create an issue.
2. Create a branch: `feature/short-description` or `fix/short-description`.
3. Make changes. Write tests alongside implementation.
4. Run `just check-all` before committing.
5. Open a PR. CI must be green.

### Post-Genesis (mainnet+)

The canonical code state is the on-chain `git:head` pointer managed by C4 (Governance Engine). GitHub `main` is a read-only snapshot mirror — merging to `main` has no effect on the network.

- **Code changes** require a governance proposal (C4), not a PR.
- **Agent-driven contributions** flow through the task board (C11), not GitHub.
- **New code ships when** the `git:head` advances to a commit where the proposal passed, was executed in the governance sandbox, and was validated by Committee BFT.

To fetch the live `git:head`: the node resolves it from the State Machine (C2) — not from any remote.

## Code Style

- `cargo fmt` (config in `rustfmt.toml`)
- `cargo clippy` with `-D warnings` (config in `clippy.toml`)
- Follow existing patterns in the crate you are modifying.
- Use `// SPEC_DEVIATION: [reason]` when deviating from a spec.

## Commit Messages

Follow conventional commits: `type(scope): description`

Types: feat, fix, docs, test, refactor, perf, chore, ci

## Spec Traceability

Every implementation change must be traceable to a Layer 4 spec section. Include the spec reference in commit messages or PR descriptions (e.g., `Spec: consensus-spec.md §2.3`).

## License

All contributions are licensed under MIT OR Apache-2.0.
