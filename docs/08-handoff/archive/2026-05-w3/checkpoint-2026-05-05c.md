# Checkpoint — 2026-05-05 (User Task Submission Pipeline Propagation)

**Completed:** Propagated `user-task-submission-and-sponsorship.md` through the full 8-layer pipeline. 10 new files/artifacts created, 11 existing files updated.

## What changed

### Layer 2 — Requirements
7 new FRs added (FR-0194–FR-0200):
- FR-0194: `task_create` Action Plan Type
- FR-0195: Task Creation Trust-Stage Quotas (0/3/10/30)
- FR-0196: Agent Sponsorship Model (agent-as-proxy)
- FR-0197: Task Discovery via Gossip/DHT (`TaskCreated` events)
- FR-0198: Task Cancellation Fee (1% of bounty, min 1 AGX)
- FR-0199: `hyperfluid task submit` CLI Command
- FR-0200: Telegram Sponsored Task Submission

Total requirements: 202 (172 FR + 30 NFR). Index updated.

### Layer 3 — Architecture
- ADR-0014: User Task Submission and Agent Sponsorship (new)
- Data model updated: TASK entity now includes `seed_ref`, `metadata_hash`, `sponsor_id`, `requester_pubkey`; `bounty_agx` type corrected to u128
- Architecture index registered

### Layer 4 — Specifications
- `policy-engine-spec.md`: `TaskCreate` added to ActionType enum; `InvalidSeedRef` + `InsufficientFunds` added to DenyReason; `task_create_per_stage` added to quota table
- `consensus-spec.md`: `TaskCreateTx` added to TxType; task creation state transition (PDP → fee → escrow → record → gossip event) added
- `collaboration-spec.md` §1.2: task submission pipeline reference added
- `agent-runtime-spec.md` §3.2: `hyperfluid task submit` CLI in system prompt; §5.1: sponsored submission context added

### Layer 5 — Planning
- `stage-02-agent-runtime.md` updated: task submit CLI in Week 3-4; task creation quotas, discovery via gossip/DHT in Week 5-6; team formation removed; output and exit criteria updated

### Verification
- `cargo build --workspace` — PASS
- `cargo test --workspace` (23/23) — PASS
- `cargo fmt --all -- --check` — PASS
- `cargo clippy --workspace --all-targets -- -D warnings` — PASS
- `cargo doc --workspace --no-deps` — PASS

**All layers now traceable from `user-task-submission-and-sponsorship.md` through to Stage 02 implementation plan.**

**Next:** Stage 01 (Protocol Core). All pre-Stage-01 amendments and pipeline propagation complete.

**Open Questions:** None.
