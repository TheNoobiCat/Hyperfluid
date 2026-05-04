# Hyperfluid Development Guide

Developer onboarding and day-to-day workflow for the Hyperfluid codebase.

## Prerequisites

- **Rust** stable 1.80+ (`rustup toolchain install stable`)
- **Git** (`git`)
- **just** command runner (`cargo install just`)
- **cargo-deny** supply-chain audit (`cargo install cargo-deny`)

Recommended (optional):
- **sccache** for faster CI builds
- **VS Code** with `rust-analyzer` extension

## Quick Start

```powershell
# Clone and build
git clone https://github.com/thenoobicat/hyperfluid
cd hyperfluid
cargo build --workspace

# Run all checks
just check-all

# Run local testnet
.\scripts\testnet\start.ps1

# Stop local testnet
.\scripts\testnet\stop.ps1
```

## Project Structure

```
hyperfluid/
  crates/
    hyperfluid-consensus/    # C1: Committee BFT, block production
    hyperfluid-state/        # C2: State Machine & SMT
    hyperfluid-staking/      # C3: Validator lifecycle, slashing
    hyperfluid-governance/   # C4: On-chain git:head governance
    hyperfluid-fee-market/   # C5: EIP-1559 fee market
    hyperfluid-fastpath/     # C6: Fast-Path Topic Protocol
    hyperfluid-p2p/          # C7: P2P networking
    hyperfluid-artifact/     # C8: Content-addressed storage
    hyperfluid-pdp/          # C9: Policy Decision Point
    hyperfluid-agent/        # C10: Agent Runtime
    hyperfluid-collaboration/# C11: Collaboration & Inbox
    hyperfluid-economics/    # C12: Economics & Incentives
    hyperfluid-node/         # Full node binary
  config/
    testnet-single.toml      # Single-validator testnet genesis
  scripts/
    testnet/
      start.ps1              # Start local testnet
      stop.ps1               # Stop local testnet
  docs/                      # 8-layer documentation pipeline
```

## Daily Workflow

```powershell
# Format code
just fmt

# Auto-fix formatting
just fmt-fix

# Lint (clippy with deny warnings)
just lint

# Run tests
just test

# Run doc tests
just test-doc

# Build documentation
just doc

# Full CI check (build + test + fmt + lint + audit)
just ci
```

## Code Conventions

- **Edition:** Rust 2021
- **Formatting:** `rustfmt.toml` (stable features only)
- **Linting:** `clippy.toml` (strict, `-D warnings`)
- **Naming:** `snake_case` for modules/functions, `CamelCase` for types, `UPPER_SNAKE` for constants
- **Crate naming:** `hyperfluid-{domain}` (e.g., `hyperfluid-consensus`)
- **Error handling:** `thiserror` for library errors; `anyhow`/`tracing` for application-level context
- **Serialization:** `serde` with `Serialize`/`Deserialize` derives; TOML for config files
- **Hashes:** `[u8; 32]` alias `Hash32`; computed via `SHA3-256`

## Testing

- Unit tests live in `#[cfg(test)] mod tests {}` blocks within source files
- Integration tests go in `crates/{crate}/tests/`
- Test the spec, not the implementation — every test should be traceable to a spec section
- Run all tests: `just test`
- Conformance tests (Stage 03) are tracked in `docs/06-validation/conformance/`

## Spec-Driven Development

Implementation follows the Layer 4 specifications. Before writing code:

1. Identify the target spec section
2. Read the normative behavior (Section X.2)
3. Implement the data structures exactly as defined (Section X.3)
4. Write tests against conformance test hooks (Section X.7)
5. If you find an ambiguity, file it in `docs/08-handoff/latest/open-questions.md`

Specs are frozen. Post-freeze changes require a governance proposal.

## Architecture Reference

| Component | Crate | Layer |
|-----------|-------|-------|
| C1 Consensus Engine | `hyperfluid-consensus` | Protocol Core |
| C2 State Machine & SMT | `hyperfluid-state` | Protocol Core |
| C3 Staking & Validator Manager | `hyperfluid-staking` | Protocol Core |
| C4 Governance Engine | `hyperfluid-governance` | Protocol Core |
| C5 Fee Market | `hyperfluid-fee-market` | Protocol Core |
| C6 Fast-Path Topic Protocol | `hyperfluid-fastpath` | Protocol Services |
| C7 P2P Networking | `hyperfluid-p2p` | Protocol Services |
| C8 Artifact Availability | `hyperfluid-artifact` | Protocol Services |
| C9 Policy Decision Point | `hyperfluid-pdp` | Security Boundary |
| C10 Agent Runtime | `hyperfluid-agent` | Runtime |
| C11 Collaboration & Inbox | `hyperfluid-collaboration` | Runtime |
| C12 Economics & Incentives | `hyperfluid-economics` | Economics |

## Glossary

Canonical terms are defined in `GLOSSARY.md`. Never redefine them. Key terms:

- `action_plan` — network mutation intent
- `git:head` — on-chain code state reference
- Trust stages: `untrusted_joiner` → `sandboxed_contributor` → `trusted_contributor` → `coordinator_eligible`
- Validator states: `active` → `paused` → `unbonding` → `withdrawn`

## CI Pipeline

GitHub Actions runs on every push and PR:
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `cargo doc --workspace --no-deps`
- `cargo deny check`

## Dependency Audit

```powershell
# Run audit
just audit

# Update advisory database
just audit-update
```

Licenses allowed: Apache-2.0, MIT, BSD-2-Clause, BSD-3-Clause, CC0-1.0, ISC, Unlicense. GPL and AGPL are denied. See `deny.toml` for full configuration.
