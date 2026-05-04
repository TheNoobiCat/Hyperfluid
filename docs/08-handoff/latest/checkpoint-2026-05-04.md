# Checkpoint — 2026-05-04 (Stage 00 Week 2 Complete)

**Completed:** Stage 00 Week 2 — Testnet scaffold, developer docs, dependency audit.

Core types implemented across three crates following spec data structures exactly: `hyperfluid-state` (C2: KeyPrefix, Account, SMTNode, InclusionProof, SHA3-256 helpers), `hyperfluid-consensus` (C1: Committee, BlockHeader, Block, TransactionEnvelope, TxType, GenesisConfig with testnet-single default), `hyperfluid-staking` (C3: ValidatorRecord, ValidatorState, SlashRecord, FaultType, SystemParameters, GovernanceVoteTx, VoteOption). 21 unit tests pass across all crates.

New `hyperfluid-node` binary crate added: boots with TOML genesis config, produces genesis block (height 0), enters stub consensus loop with epoch boundary detection, supports `--gen-genesis` flag for genesis.json export, clean shutdown on signal. Same config format and genesis layout as production deployment.

Testnet scaffold includes `config/testnet-single.toml` (single-validator genesis), `scripts/testnet/start.ps1` and `scripts/testnet/stop.ps1`. `DEVELOPMENT.md` covers onboarding, workflow, conventions, and architecture reference.

`cargo-deny` installed and passing: advisories ok, bans ok, licenses ok, sources ok. Removed unused `bincode` (RUSTSEC-2025-0141, unmaintained); SCALE encoding deferred to Stage 01 per spec.

**Verification:** `cargo build`, `cargo test` (21 tests), `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo doc --no-deps`, `cargo deny check` — all PASS. Node binary boots, writes genesis.json, enters consensus loop, handles kill signal. Cold-start verified.

**Next:** Stage 01 (Protocol Core) — Consensus Engine, State Machine & SMT, Staking, Fee Market, P2P Networking, Artifact Storage. Build Minimum Viable Chain.

**Blockers:** None. Stage 00 complete.
