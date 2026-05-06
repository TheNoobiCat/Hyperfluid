# Protocol Spec: Governance Engine

**Component:** C4 Governance Engine
**Source ADRs:** ADR-0003 (PDP Deterministic Rule Chain), ADR-0011 (Review Sandbox Isolation)
**Covered FRs:** FR-0021, FR-0022, FR-0023, FR-0024, FR-0025, FR-0026, FR-0027, FR-0028, FR-0029, FR-0030, FR-0155
**Dependencies:** C1 Consensus Engine, C2 State Machine & SMT, C3 Staking & Validator Manager

---

## Section 1: On-Chain git:head Governance

### 1.1 Purpose

Define the protocol for on-chain governance of the `git:head` pointer, including proposal lifecycle, vote aggregation, sandbox review, and anti-flood controls.

### 1.2 Normative Behavior

- The system MUST store a canonical `git:head` in SMT state, representing the approved protocol code state.
- `git:head` transitions MUST occur only via successful governance proposals.
- Each governance proposal MUST specify a target `proposed_commit` hash.
- `git:head` transitions MUST be atomic with proposal finalization at an epoch boundary.
- Governance proposals MUST represent fast-forward merges or deterministic clean merges.
- Non-deterministic merge outcomes MUST burn the proposer's 500 AGX deposit.
- Proposals MUST execute in a hermetic sandbox with pinned gix toolchain and normalized environment.

### 1.3 Data Structures

```rust
struct GovernanceProposal {
    proposal_id: [u8; 32],           // SHA3-256 of proposal content
    proposer_id: [u8; 32],
    proposed_commit: [u8; 32],       // target git commit hash
    bundle_manifest_hash: [u8; 32],  // SHA-256 of bundle manifest
    current_commit: [u8; 32],        // git_head at proposal time
    deposit_amount: u128,             // 500 AGX locked in atto-AGX
    snapshot_height: u64,
    vote_start_height: u64,
    vote_end_height: u64,
    status: ProposalStatus,
    yes_weight: u128,
    no_weight: u128,
}

enum ProposalStatus {
    Active,
    Passed,
    Rejected,
    Executed,
}

struct BundleManifest {
    manifest_hash: [u8; 32],
    object_ids: Vec<[u8; 32]>,
    total_size_bytes: u64,
    proposer_fetch_endpoints: Vec<String>,
    toolchain_hash: [u8; 32],
    environment_hash: [u8; 32],
}
```

### 1.4 State Transitions

**Proposal lifecycle:**

```
GovernanceProposeTx ─► active (if passes prechecks)
active ──(vote window expires, quorum met, yes > 50%)──► passed
active ──(vote window expires, quorum not met OR no > 50%)──► rejected
passed ──(epoch boundary, atomic execution)──► executed
```

**Detailed flow:**
1. Proposer submits GovernanceProposeTx with 500 AGX deposit, bundle manifest hash, and fetch endpoints.
2. State machine creates proposal record, captures validator snapshot.
3. Vote window opens: 14 epochs (approximately 14 days) with >40% quorum of snapshot stake required.
4. At vote_end_height, tally finalizes:
   - If yes_weight > 50% of participated_weight AND participated_weight > 40% total_snapshot_stake → passed.
   - Otherwise → rejected.
   - No-votes (timeouts) excluded from both numerator and denominator.
5. If passed, at the next epoch boundary, git:head transitions atomically to proposed_commit.
6. Deposit returned to proposer on passage. Burned only if proposal was invalid/non-deterministic.

### 1.5 Failure Behavior

- **Bundle verification failure:** If validators cannot fetch and verify the bundle manifest (objects don't reach proposed_commit, or hashes mismatch), the proposal is marked invalid and deposit is burned.
- **Non-deterministic merge:** If the hermetic sandbox produces different output across validators (non-zero exit code, environment-dependent diff), the proposal is invalid and deposit is burned.
- **Queue saturation:** When 32 open proposals exist network-wide, new proposals are rejected.
- **Identity cooldown:** A proposer rejected for invalid/non-deterministic proposal cannot propose again for 3 epochs.
- **Deposit loss:** Invalid and non-deterministic proposals permanently burn deposit.

### 1.6 Versioning and Compatibility

- The governance protocol version is embedded in the `git:head` itself.
- Sandbox toolchain and environment are pinned by hash in the active policy bundle.
- Deprecation of old `git:head` occurs on execution of new proposal.

### 1.7 Conformance Test Hooks

- Verify git:head transitions atomically with successful proposal execution.
- Verify proposal with bundle that fails hash verification burns deposit.
- Verify non-deterministic merge burns deposit.
- Verify quorum threshold: >40% of snapshot stake required; >50% yes to pass.
- Verify max 32 open proposals; 33rd proposal is rejected.
- Verify proposer cooldown of 3 epochs after rejection.
- Verify per-identity cap of 1 proposal per epoch.

### 1.8 Trust-Assumption Inventory

- gix reproducibility
  - Justification: Merge determinism requires identical gix behavior across all validators.
  - Trust-minimised alternative: Pinned gix version in policy bundle; hermetic Docker execution with identical image hash.
- Validator honesty in review sandbox
  - Justification: Review sandbox runs on each validator's machine; output could theoretically be fabricated.
  - Trust-minimised alternative: Deterministic precheck and bundle hash binding make fabrication detectable by other validators.
- No-vote timeout fairness
  - Justification: Systematic exclusion analysis is deferred to Layer 6 (Validation). Assumption is that timeouts are distributed independently across validators.
  - Trust-minimised alternative: Randomized vote deadline per validator to prevent adversarial scheduling.

---

## Section 2: Review Sandbox Execution

### 2.1 Purpose

Define the isolated sandbox subagent for governance proposal review.

### 2.2 Normative Behavior

- The system MUST execute governance proposal review in an isolated sandbox subagent.
- The sandbox MUST have fresh context (no access to main agent state).
- The sandbox MUST have exactly one tool: `review(decision: approve|deny, reason: string)`.
- The sandbox MUST have a timeout of 30 minutes.
- Timeout MUST result in no vote (not penalized, not counted toward quorum).
- Main agent branch MUST pause during sandbox execution.
- On review tool invocation, the runtime MUST emit GovernanceVoteTx and terminate the sandbox.
- Deterministic prechecks (manifest hash verification, object reachability, merge determinism) MUST complete before sandbox launch.
- Precheck failure MUST reject the proposal without sandbox execution.

### 2.3 Data Structures

GovernanceVoteTx is canonically defined in staking-spec.md §2.3. The vote_weight field is u128 (atto-AGX precision).

### 2.4 State Transitions

1. Validator agent receives review assignment for proposal.
2. Agent launches governance review sandbox (subprocess, separate context).
3. Sandbox fetches bundle, verifies manifest, runs deterministic precheck.
4. If precheck passes, sandbox reviews proposal content.
5. Sandbox calls `review(approve|deny, reason)` — emits GovernanceVoteTx.
6. Sandbox terminates. Main agent resumes.
7. Timeout at 30 minutes: no vote emitted, sandbox terminates.

### 2.5 Failure Behavior

- Sandbox crash: no vote, sandbox terminates, main agent resumes cleanly.
- Precheck failure: proposal rejected without sandbox launch; deposit burned for invalid proposals.
- Bundle unavailable: if proposer endpoints are unreachable, validators cannot review. After 3 epochs of unavailability, proposal auto-rejected.

### 2.7 Conformance Test Hooks

- Verify sandbox has no access to main agent state (todos, knowledge, messages).
- Verify sandbox has exactly one tool and no other tools.
- Verify 30-minute timeout results in no vote.
- Verify precheck failure rejects without sandbox launch.
- Verify sandbox termination cleanly resumes main agent.

### 2.8 Trust-Assumption Inventory

- Sandbox isolation correctness
  - Justification: Sandbox must be truly isolated from main agent state to prevent selection bias.
  - Trust-minimised alternative: Review by a separate agent identity (true independence) — adds complexity but removes sandbox trust requirement.
