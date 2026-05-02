# Checkpoint — 2026-05-02 (Stage 00 Week 1)

**Completed:** Stage 00 Week 1 — Cargo workspace scaffold, tooling config, CI pipeline, dev environment.

12 crates match the architecture component decomposition (C1-C12). The workspace builds, tests, formats, and lints clean with zero warnings across all targets. CI workflow (GitHub Actions) is ready for push — covers fmt, clippy, test, doc, audit, and bench-check. The PDP crate includes early `error.rs` and `types.rs` stubs with `TrustStage`, `RiskLevel`, and `PolicyResult` types.

**What works:** `cargo build --workspace`, `cargo test --workspace`, `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo doc --workspace --no-deps` — all pass.

**Next:** Week 2 — testnet scaffold (genesis cerberus, single-validator config, start/stop scripts), cold-start verification, `DEVELOPMENT.md`, and `cargo-deny` audit pass.

**Blockers:** None.
