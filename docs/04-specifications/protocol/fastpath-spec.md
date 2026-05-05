# Protocol Spec: Fast-Path Topic Protocol

**Component:** C6 Fast-Path Topic Protocol
**Source ADRs:** ADR-0008 (Two-Phase Quality Pipeline)
**Covered FRs:** FR-0031, FR-0032, FR-0033, FR-0034, FR-0035, FR-0036, FR-0037, FR-0038, FR-0039, FR-0040
**Dependencies:** C1 Consensus Engine, C2 State Machine, C4 Governance Engine

---

## Section 1: Topic-Scoped Fast-Path Merges

### 1.1 Purpose

Define the fast-path merge protocol for topic-scoped collaboration with quorum certificates, challenge windows, and promotion bridges.

### 1.2 Normative Behavior

- Fast-path merges MUST target only `topic/<id>/main` branches; they MUST NOT directly mutate canonical `git:head`.
- A quorum certificate with 2f + 1 weighted approvals from the topic snapshot set MUST be required to finalize a fast-path merge.
- At least one approver MUST be outside the primary author's operator cluster (stake-graph distance or key correlation heuristic).
- The system MUST enforce per-topic merge throughput: max 20 fast merges per topic per hour.
- Per-identity merge throughput: max 5 fast merges per identity per hour.
- Deterministic prechecks (object graph, merge reproducibility, topic scope) MUST complete before opening review windows.
- A challenge window of 144 blocks (~24 hours) MUST open after fast-path certification.
- Certified rollbacks MUST revert topic state to a prior head, scoped to the affected topic only.
- Competing certificates for the same base topic head MUST be resolved deterministically: higher approval weight first, then lower certificate hash.
- Certificate validity MUST be bound to `proposal_id` and `base_topic_head`; replay of old certificates against newer topic heads MUST be rejected.
- Topic merge outputs MAY be packaged into promotion bundles for optional canonical governance proposals.

### 1.3 Data Structures

```rust
struct FastPathProposal {
    proposal_id: [u8; 32],
    topic_id: [u8; 32],
    proposer_id: [u8; 32],
    base_topic_head: [u8; 32],      // current topic head at proposal time
    proposed_head: [u8; 32],        // proposed new topic head
    bundle_manifest_hash: [u8; 32],
    expires_at_height: u64,
    signature: Vec<u8>,
}

struct FastPathCertificate {
    proposal_id: [u8; 32],
    topic_id: [u8; 32],
    base_topic_head: [u8; 32],
    proposed_head: [u8; 32],
    approvals: Vec<ReviewerSignature>,
    aggregate_signature: Vec<u8>,    // aggregated ML-DSA signatures
    signer_set_hash: [u8; 32],
    issued_at_height: u64,
    challenge_until_height: u64,
}

struct ReviewerSignature {
    reviewer_id: [u8; 32],
    vote: ReviewerVote,
    reason_hash: [u8; 32],
    signature: Vec<u8>,
}

enum ReviewerVote {
    Approve,
    Deny,
}

struct FastPathChallengeTx {
    proposal_id: [u8; 32],
    topic_id: [u8; 32],
    challenger_id: [u8; 32],
    evidence_hash: [u8; 32],
    challenger_bond: u128,           // 20% of merge reward value in atto-AGX
    signature: Vec<u8>,
}

struct FastPathRollbackTx {
    proposal_id: [u8; 32],
    topic_id: [u8; 32],
    rollback_to_head: [u8; 32],
    arbiter_certificate: Vec<u8>,
    signature: Vec<u8>,
}

struct PromotionBundle {
    topic_id: [u8; 32],
    merge_certificate: FastPathCertificate,
    artifact_hash_chain: Vec<[u8; 32]>,
    diff_summary: [u8; 32],
    coordinator_signatures: Vec<Vec<u8>>,
}
```

### 1.4 State Transitions

**Merge lifecycle:**

```
FastPathProposalTx ─► proposed
│
├── deterministic precheck passes ─► review_window_opened
│   ├── reviewers approve (2f+1 quorum) ─► certified
│   │   ├── challenge window (144 blocks) ─► final (if unchallenged)
│   │   └── successful challenge ─► rolled_back
│   └── review window expires without quorum ─► rejected
└── deterministic precheck fails ─► rejected
```

**Review flow:**
1. Proposer submits FastPathProposalTx with topic_id, base_topic_head, proposed_head, and bundle manifest.
2. Deterministic precheck validates: object graph reaches proposed_head, merge is reproducible, scope is topic-local.
3. Review sandbox launched for each reviewer (fresh context, single review tool).
4. Reviewers produce approvals or denials.
5. On 2f+1 weighted approvals, certificate is issued. Topic head advances.
6. Challenge window opens: 144 blocks. Any eligible participant can submit FastPathChallengeTx with bond.
7. If unchallenged, final after 144 blocks.
8. If challenged, arbiter evaluation produces outcome: uphold (certificate revoked, rollback) or deny (certificate stands, challenger bond burned).

**Tie-break rule:** When two valid certificates compete for the same base_topic_head:
1. Sort by total approval weight descending.
2. If tied, sort by certificate hash ascending (bytes comparison).
3. Higher-ranked certificate wins. Lower-ranked certificate is rejected.

### 1.5 Failure Behavior

- **Reviewer crash/timeout:** Missing review does not block quorum; counted as no-vote (not penalized).
- **Insufficient reviewers:** If quorum cannot be achieved within deadline, proposal expires rejected.
- **Certificate replay:** Certificate with `base_topic_head != current topic head` is rejected at validation.
- **Challenge spam:** Challenger bond (20% of task value) is burned on failed challenge. Per-identity challenge cap prevents bulk griefing.
- **Rollback propagation:** Rollback is strictly topic-local; canonical git:head unchanged.

### 1.6 Versioning and Compatibility

- Certificate schema version is embedded in proposal_id generation.
- Review sandbox configuration is pinned by policy bundle hash.
- Topic state format compatible across protocol versions.

### 1.7 Conformance Test Hooks

- Verify fast-path merge targets only topic branches; direct git:head mutation rejected.
- Verify 2f+1 quorum required for certification.
- Verify at least one independent reviewer required.
- Verify per-topic throughput limit (20/hr) and per-identity limit (5/hr).
- Verify certificate replay against different topic head is rejected.
- Verify deterministic tie-break produces identical result on all nodes.
- Verify challenge window of 144 blocks before finality.
- Verify rollback is topic-local; canonical git:head unchanged.
- Verify promotion bundle packaging with content-addressed references.

### 1.8 Trust-Assumption Inventory

- Reviewer independence detection accuracy
  - Justification: Stake-graph analysis and key correlation heuristics may produce false negatives (related operators classified as independent).
  - Trust-minimised alternative: Declared legal entity attestation for validators; adds centralization risk.
- Challenge window sufficiency
  - Justification: 24-hour window assumes honest challenger can detect and submit evidence within that time.
  - Trust-minimised alternative: Longer challenge window (governance-adjustable parameter).
- Review sandbox isolation
  - Justification: Same assumption as governance review; reviewer must not be influenced by side channels.
  - Trust-minimised alternative: Multiple independent reviewer sandboxes on different operators.
