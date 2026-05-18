# Checkpoint: 2026-05-18 — Gap Fill Round (Collaboration Crate)

## Summary
Filled the open GAP NOTE for Stage 02 tracking inbox budgets, topic decay, abuse evidence, and replay prevention (FR-0093, FR-0094, FR-0095, FR-0101, FR-0175) by building the `hyperfluid-collaboration` crate from empty scaffold.

## Gaps Investigated

| Gap | Source | Still Present? | Resolved? |
|-----|--------|---------------|-----------|
| FR-0093 (global inbox budget) | stage-02:82 | Yes (empty scaffold) | YES |
| FR-0094 (topic message budget) | stage-02:82 | Yes (empty scaffold) | YES |
| FR-0095 (abuse evidence + quarantine) | stage-02:82 | Yes (empty scaffold) | YES |
| FR-0101 (topic decay lifecycle) | stage-02:82 | Yes (empty scaffold) | YES |
| FR-0175 (replay prevention) | stage-02:82 | Yes (empty scaffold) | YES |

## Verification Evidence

| Gap | Evidence | Status |
|-----|----------|--------|
| FR-0093 | `inbox::tests::global_inbox_budget_enforced_fr0093`, `per_sender_quota_untrusted_fr0093` | PASS |
| FR-0094 | `inbox::tests::topic_budget_enforced_fr0094` | PASS |
| FR-0095 | `inbox::tests::abuse_evidence_accumulation_fr0095`, `abuse_triggers_quarantine_fr0095` | PASS |
| FR-0101 | `topic::tests::topic_lifecycle_fr0101`, `topic_decay_to_stale_fr0101`, `topic_decay_to_archived_fr0101`, `activity_resets_decay` | PASS |
| FR-0175 | `replay::tests::fresh_submission_accepted_fr0175`, `replayed_submission_rejected_fr0175` | PASS |

## Files Changed

| File | Action |
|------|--------|
| `crates/hyperfluid-collaboration/Cargo.toml` | Added dependencies |
| `crates/hyperfluid-collaboration/src/types.rs` | NEW — 22 types, 5 enums, 5 impl blocks |
| `crates/hyperfluid-collaboration/src/task_board.rs` | NEW — TaskBoard with lifecycle, leases, shadow claims (10 tests) |
| `crates/hyperfluid-collaboration/src/inbox.rs` | NEW — InboxSystem with quotas, priority routing, abuse (10 tests) |
| `crates/hyperfluid-collaboration/src/topic.rs` | NEW — TopicRegistry with lifecycle decay (8 tests) |
| `crates/hyperfluid-collaboration/src/trust.rs` | NEW — TrustLadder with promotion, whitewash guard (10 tests) |
| `crates/hyperfluid-collaboration/src/replay.rs` | NEW — ReplayProtection with freshness nonce (6 tests) |
| `crates/hyperfluid-collaboration/src/lib.rs` | Updated re-exports |

## Remaining Gaps

| Gap | Blocker |
|-----|---------|
| Malachite BFT effect handler (~300 lines) | No BFT consensus loop |
| Malachite clatter network bridge (~500 lines) | No BFT gossip transport |
| Malachite Host actor (~400 lines) | No BFT integration |
| Slashing execution + reward distribution | Deferred to Stage 03 |
| Full 24h soak test | Deferred to Stage 03 |
| Economics crate (C12) | Not yet built — planned for Week 5-6 |
| Review engine integration | Not yet wired — planned for Week 5-6 |
| Sybil detection | Not yet built — planned for Week 5-6 |
