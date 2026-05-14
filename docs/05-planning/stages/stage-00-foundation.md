# Stage 00: Foundation

## Inputs
- From previous stage: Layer 4 specifications (14 specs, frozen), Layer 3 architecture (12 components, 12 ADRs), Layer 2 requirements (202 FR/NFR).
- External: Rust toolchain stable, `cargo`, `git`, `just` (command runner), CI platform (GitHub Actions or equivalent).

## Outputs
- Monorepo Cargo workspace with crate boundaries matching architecture component decomposition.
- `justfile` with build, test, lint, fmt, bench, doc targets.
- CI pipeline: format check, clippy, test, coverage, doc build, benchmark regression.
- Local single-node testnet scaffold: start/stop scripts, genesis block generation, faucet.
- Developer environment: `shell.nix` or `devcontainer` for reproducible builds.
- Dependency audit baseline: `cargo-deny` config, supply-chain policy.
- `CODEOWNERS` and PR template.

## Exit Criteria
- [ ] `cargo build` passes on all targets from clean checkout.
- [ ] `cargo test` passes (initially empty or boilerplate).
- [ ] `just fmt` and `just lint` pass with zero warnings.
- [ ] CI pipeline runs on every push and PR.
- [ ] Single-node testnet boots, produces blocks, and stops cleanly.
- [ ] Dependency licenses audited; no GPL/AGPL conflicts identified.
- [ ] `cargo-deny` passes for advisories, bans, licenses, sources.
- [ ] All risks documented and acceptable.
- [ ] Next stage inputs prepared.

## Duration Estimate
1–2 weeks. Extend to 2 weeks if dependency resolution or CI platform integration proves difficult.

## Dependencies
- None. This stage requires only the repo, Rust tooling, and CI platform access.
- External: `just` installation on CI runners.

## Week-by-Week Breakdown

### Week 1 [x] COMPLETE (2026-05-02)

- [x] Create Cargo workspace with crate scaffold for each of the 12 architecture components.
- [x] Configure root `Cargo.toml` with workspace members, dependencies, profiles (dev, release, bench).
- [x] Write `justfile` with standard targets: `build`, `test`, `fmt`, `lint`, `bench`, `doc`, `clean`, `audit`.
- [x] Set up `rustfmt.toml`, `clippy.toml` with project conventions.
- [x] Add `deny.toml` (cargo-deny) with license/advisory/bans/sources configuration.
- [x] Add `CODEOWNERS`, PR template (`.github/PULL_REQUEST_TEMPLATE.md`), `CONTRIBUTING.md`.
- [x] Create `.github/workflows/ci.yml` pipeline (fmt, clippy, test, doc, audit, bench).
- [x] Create `.devcontainer/devcontainer.json` for reproducible environment.
- [x] PDP crate seeded with early `error.rs` and `types.rs` (TrustStage, RiskLevel, PolicyResult).
- [x] `.gitignore` updated with comprehensive Rust entries.

### Week 2 [x] COMPLETE (2026-05-04)

1. [x] Core types in `hyperfluid-state` (KeyPrefix, Account, SMTNode, InclusionProof), `hyperfluid-consensus` (Committee, BlockHeader, Block, TransactionEnvelope, TxType, GenesisConfig), `hyperfluid-staking` (ValidatorRecord, ValidatorState, SlashRecord, SystemParameters, GovernanceVoteTx).
2. [x] `hyperfluid-node` binary crate: genesis block generation, config loading, stub consensus loop, `--gen-genesis` flag, clean shutdown.
3. [x] Local testnet scaffold: `config/testnet-single.toml`, `scripts/testnet/start.ps1`, `scripts/testnet/stop.ps1`.
4. [x] Full cold-start verification: `cargo build`, `cargo test` (21 tests), `cargo fmt --check`, `cargo clippy`, `cargo doc`, node boots and stops cleanly.
5. [x] `DEVELOPMENT.md` developer onboarding guide.
6. [x] `cargo-deny` installed and passing: advisories ok, bans ok, licenses ok, sources ok.
7. [x] `.gitignore` updated with `genesis.json`.

## Risk Areas
- **Dependency license conflict:** Some Rust crypto crates use AGPL or GPL. Mitigation: `cargo-deny` configured upfront; evaluate alternatives before adding any dependency past Stage 00.
- **CI runner resource limits:** Full `cargo build` of workspace may exceed free-tier CI limits. Mitigation: `sccache` or CI-native caching; measure build times in Week 1 to decide if paid CI tiers are needed.
- **Testnet scaffold diverges from production deployment:** Mitigation: scaffold uses same config format, genesis layout, and key format as planned production deployment. No special-case shortcuts.

## Spec References
No Layer 4 specs are implemented at this stage. The outputs (workspace, CI, testnet scaffold) are infrastructure that will host the implementation in later stages.

## Upstream Dependencies for Next Stage
- Cargo workspace must be stable (crate names, dependency versions, feature flags decided).
- CI must be green.
- Testnet scaffold must boot and produce blocks before Stage 01 can validate consensus progress.
