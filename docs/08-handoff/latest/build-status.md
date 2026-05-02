# Build Status — Stage 00 (Foundation)

**Last updated:** 2026-05-02
**Stage:** 00 — Foundation

## Week 1: Complete

| Task | Status |
|------|--------|
| Cargo workspace with 12 crate scaffolds | Complete |
| Root `Cargo.toml` with profiles, workspace deps | Complete |
| `justfile` (build, test, fmt, lint, bench, doc, clean, audit) | Complete |
| `rustfmt.toml` (edition 2021, stable-only features) | Complete |
| `clippy.toml` | Complete |
| `deny.toml` (cargo-deny: advisories, licenses, bans, sources) | Complete |
| `.github/CODEOWNERS` | Complete |
| `.github/PULL_REQUEST_TEMPLATE.md` | Complete |
| `CONTRIBUTING.md` | Complete |
| `.github/workflows/ci.yml` (fmt, clippy, test, doc, audit, bench-check) | Complete |
| `.devcontainer/devcontainer.json` | Complete |
| `.gitignore` updated (comprehensive Rust entries) | Complete |
| PDP crate: `error.rs` and `types.rs` stub modules with TrustStage/RiskLevel | Complete |

## Verification

| Check | Result |
|-------|--------|
| `cargo build --workspace` | PASS (12 crates) |
| `cargo test --workspace` | PASS (12/12) |
| `cargo fmt --all -- --check` | PASS (clean) |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS (zero warnings) |
| `cargo doc --workspace --no-deps` | PASS |

## Week 2: Pending

- [ ] Testnet scaffold (genesis cerberus, single-validator config, start/stop scripts)
- [ ] Full cold-start verification (`git clone` -> `cargo build` -> `cargo test` -> local testnet boots)
- [ ] `DEVELOPMENT.md` (developer onboarding guide)
- [ ] Verify `cargo-deny` passes (needs `cargo-deny` installed)

## Exit Criteria Status

| Criterion | Status |
|-----------|--------|
| `cargo build` from clean checkout | PASS |
| `cargo test` passes | PASS |
| `just fmt` and `just lint` pass zero warnings | PASS |
| CI pipeline runs on push/PR | READY (workflow file created; needs repo push to verify) |
| Single-node testnet boots | NOT STARTED (Week 2) |
| Dependency licenses audited; no GPL/AGPL | NOT RUN (needs cargo-deny binary) |
| All risks documented and acceptable | IN PROGRESS |
| Next stage inputs prepared | PENDING |

## Known Issues

1. `cargo-deny` (audit) not yet run — requires `cargo install cargo-deny` on a machine with the binary. CI workflow includes the `cargo-deny-action` so it will run on push. Blocked on: toolchain not on this machine.
2. PDP crate has stub `error.rs` + `types.rs` with `thiserror` dependency. Lightweight and intentional — these types will grow in Stage 02.
