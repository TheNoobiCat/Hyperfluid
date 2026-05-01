## FR-0021: On-Chain `git:head` Governance

**Category:** Governance

**Statement:** The system shall store and transition a canonical `git:head` on-chain, representing the protocol's approved code state, with changes governed by stake-weighted voting.

**Rationale:** Creates auditable, protocol-native upgrade authority instead of social-layer repo control. See `agx-committee-bft-and-governance.md` Section 5 (Governance determinism).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 172-199
- `decentralization-and-stack-benchmark.md` Section 9 (Recommended Architecture)

**Acceptance Criteria:**
- [ ] `git:head` is stored in the SMT state root and updated only via successful governance proposals.
- [ ] Each governance proposal specifies a target `proposed_commit` hash.
- [ ] `git:head` transitions atomically with proposal finalization.

**Dependencies:** FR-0010, FR-0016
**Tags:** must-have

---

## FR-0022: Deterministic Governance Proposal Validation

**Category:** Governance

**Statement:** The system shall validate that governance proposals represent fast-forward or deterministic clean merges, with non-deterministic outcomes burning the proposer deposit.

**Rationale:** Prevents environment-dependent governance divergence. See `agx-committee-bft-and-governance.md` Section 5 (Governance determinism).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 172-184
- `decentralization-and-stack-benchmark.md` Section 7 (Governance execution split)

**Acceptance Criteria:**
- [ ] Proposals must be fast-forward merges or deterministic clean merges.
- [ ] Hermetic sandbox execution with pinned gix/toolchain and normalized environment.
- [ ] Non-deterministic merge outcome burns the 500 AGX proposer deposit.

**Dependencies:** FR-0021
**Tags:** must-have

---

## FR-0023: Proposal Bundle Manifest Verification

**Category:** Governance

**Statement:** The system shall require validators to fetch git objects directly from proposer endpoints, recompute manifest hash, and verify every object ID against the manifest before merge simulation.

**Rationale:** Prevents proposer equivocation (serving different bundles to different validators). See `agx-committee-bft-and-governance.md` Section 5 (Governance determinism).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 174-184

**Acceptance Criteria:**
- [ ] `GovernanceProposeTx` includes `bundle_manifest_hash` and `proposer_fetch_endpoints`.
- [ ] Validators verify fetched object graph reaches exactly `proposed_commit`.
- [ ] Inconsistent bundles from proposer cause proposal invalidation and deposit burn.

**Dependencies:** FR-0022
**Tags:** must-have

---

## FR-0024: Governance Proposal Deposit and Cooldown

**Category:** Governance

**Statement:** The system shall require a 500 AGX deposit for governance proposals, burn it on invalid/non-deterministic proposals, and enforce a 3-epoch cooldown after rejection.

**Rationale:** Economic cost prevents governance spam and griefing. See `agx-committee-bft-and-governance.md` Section 5 (Default protocol parameters).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 121, 218
- `agx-economics-and-adversarial-incentives.md` Section 7 (Governance griefing)

**Acceptance Criteria:**
- [ ] `GovernanceProposeTx` locks 500 AGX deposit.
- [ ] Invalid or non-deterministic proposal burns deposit permanently.
- [ ] Rejected proposal triggers 3-epoch cooldown before same identity can propose again.
- [ ] Maximum 32 open proposals network-wide at any time.

**Dependencies:** FR-0021, FR-0016
**Tags:** must-have

---

## FR-0025: Governance Vote Window and Quorum

**Category:** Governance

**Statement:** The system shall define explicit governance vote windows, require quorum from `active` validators at snapshot, and finalize based on stake-weighted majority.

**Rationale:** Ensures decisions represent active participating stake. See `agx-committee-bft-and-governance.md` Section 5 (Governance determinism).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 172-199

**Acceptance Criteria:**
- [ ] Vote window duration is specified in blocks per proposal.
- [ ] Quorum threshold requires at least 40% of snapshot stake to participate.
- [ ] Proposal passes with >50% of participating stake voting yes.
- [ ] Vote includes optional `reason_hash` linking to content-addressed review rationale.

**Dependencies:** FR-0016, FR-0021
**Tags:** must-have

---

## FR-0026: Review Sandbox for Governance Proposals

**Category:** Governance

**Statement:** The system shall execute governance proposal review in an isolated sandbox subagent with fresh context, fixed system prompt, single `review` tool, and a 30-minute timeout.

**Rationale:** Isolates review from main agent context to prevent prompt injection and ensure focused evaluation. See `agx-committee-bft-and-governance.md` Section 5, lines 185-197.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 185-197
- `index.md` (Review Timeout Semantics)

**Acceptance Criteria:**
- [ ] Main agent branch pauses during review sandbox execution.
- [ ] Sandbox has exactly one tool: `review(decision: approve|deny, reason)`.
- [ ] Sandbox timeout of 30 minutes results in no vote (not penalized).
- [ ] On `review(...)` call, runtime emits `GovernanceVoteTx` and terminates sandbox.

**Dependencies:** FR-0021
**Tags:** should-have

---

## FR-0027: Deterministic Precheck Gating

**Category:** Governance

**Statement:** The system shall run deterministic prechecks (bundle/object verification, gix merge checks) before starting any review sandbox; failures short-circuit with no sandbox launch.

**Rationale:** Saves computation on obviously invalid proposals. See `agx-committee-bft-and-governance.md` Section 5, lines 185-186.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 185-186

**Acceptance Criteria:**
- [ ] Precheck validates manifest hash, object reachability, and merge determinism.
- [ ] Precheck failure marks review as failed without sandbox execution.
- [ ] Precheck results are logged for audit.

**Dependencies:** FR-0023, FR-0026
**Tags:** must-have

---

## FR-0028: Governance Anti-Flood Controls

**Category:** Governance

**Statement:** The system shall enforce governance anti-flood controls: max 32 open proposals network-wide, 1 proposal per identity per epoch, and a reserved governance lane.

**Rationale:** Prevents governance queue saturation by spam or griefing. See `agx-committee-bft-and-governance.md` Section 5 (Swarm hardening profile).

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 215-218
- `agx-economics-and-adversarial-incentives.md` Section 7 (Governance griefing)

**Acceptance Criteria:**
- [ ] `max_open_governance_proposals` = 32 network-wide.
- [ ] `max_proposals_per_identity_per_epoch` = 1.
- [ ] `proposal_cooldown_after_reject` = 3 epochs.
- [ ] Governance lane reserves 10% of mempool capacity.

**Dependencies:** FR-0024
**Tags:** must-have

---

## FR-0029: No-Vote Timeout Fairness

**Category:** Governance

**Statement:** The system shall not penalize validators for no-vote timeouts, and systematic exclusion analysis shall be deferred to Layer 6 (Validation) with an explicit assumption note.

**Rationale:** Distinguishes unavailability from active denial; fairness proof requires validation simulation. See `agx-committee-bft-and-governance.md` Section 5, lines 140-144.

**Source Research:**
- `agx-committee-bft-and-governance.md` Section 5, lines 140-144
- `PROJECT-STATUS.md` (Research Gaps: No-vote timeout fairness proof)

**Acceptance Criteria:**
- [ ] No-vote does not affect validator reputation or stake.
- [ ] Quorum calculation excludes no-votes.
- [ ] Documentation contains assumption: systematic exclusion analysis deferred to validation phase.

**Dependencies:** FR-0018
**Tags:** should-have

---

## FR-0030: Post-Incident Governance Bridge

**Category:** Governance

**Statement:** The system shall export immutable incident evidence bundles to governance for root-cause parameter or code updates after incident resolution.

**Rationale:** Closes the loop between incident response and protocol evolution. See `decentralized-incident-response-and-recovery.md` Section 4.

**Source Research:**
- `decentralized-incident-response-and-recovery.md` Section 4 (Architecture)
- `decentralized-incident-response-and-recovery.md` Section 5, lines 71-78

**Acceptance Criteria:**
- [ ] Evidence archive exports finalized incident timeline and proofs as governance-accessible bundle.
- [ ] Bundle is content-addressed and signed by incident classifiers.
- [ ] Governance can reference bundle hash in parameter change proposals.

**Dependencies:** FR-0021
**Tags:** should-have
