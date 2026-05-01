## FR-0106: Typed Network Action Plans

**Category:** Security

**Statement:** The system shall require typed network action plans for all network-mutating operations, with schema validation, signature verification, and deterministic policy evaluation.

**Rationale:** Converts injection defense from heuristic text interpretation to enforceable control logic. See `network-policy-engine-spec.md` Section 2 (Executive Summary).

**Source Research:**
- `network-policy-engine-spec.md` Section 5 (Action plan schema)
- `prompt-injection-and-network-policy-boundary.md` Section 5 (Typed network action plan schema)

**Acceptance Criteria:**
- [ ] Action plan schema includes: plan_id, agent_id, action_type, resource_id, risk_class, reason_hash, evidence_refs, policy_bundle_hash, nonce, expires_at_height, agent_signature.
- [ ] Free text alone never executes network-mutating tools.
- [ ] Schema validation is deterministic across all nodes.

**Dependencies:** FR-0007
**Tags:** must-have

---

## FR-0107: Tool-Call Binding Hash Verification

**Category:** Security

**Statement:** The system shall verify that each network tool call matches its approved action plan via canonical binding hash computed from tool_name, normalized params, resource_id, and action_type.

**Rationale:** Prevents parameter substitution attacks after plan approval. See `network-policy-engine-spec.md` Section 5 (Tool-call binding).

**Source Research:**
- `network-policy-engine-spec.md` Section 5, lines 144-149
- `prompt-injection-and-network-policy-boundary.md` Section 5 (Tool-call binding rule)

**Acceptance Criteria:**
- [ ] Gateway computes canonical hash of tool call parameters.
- [ ] Hash must equal `plan_binding_hash` in approved plan.
- [ ] Any parameter drift invalidates execution.
- [ ] Approved plans are single-use and transition to consumed state after execution.

**Dependencies:** FR-0106
**Tags:** must-have

---

## FR-0108: Replay Protection for Action Plans

**Category:** Security

**Statement:** The system shall enforce replay protection via unique plan_id per agent, strictly monotonic nonce, TTL enforcement, and consumed plan tracking.

**Rationale:** Prevents replay attacks using captured network traffic. See `network-policy-engine-spec.md` Section 5 (Replay protection).

**Source Research:**
- `network-policy-engine-spec.md` Section 5, lines 117-123
- `PROJECT-STATUS.md` (Research Gaps: Plan replay protection E2E)

**Acceptance Criteria:**
- [ ] plan_id must be unique per agent_id.
- [ ] Nonce is strictly monotonic (last_nonce + 1).
- [ ] TTL: expires_at_height must be > current and < current + 10,000; default 1,000-5,000 blocks.
- [ ] Consumed plan IDs are tracked in protocol state.

**Dependencies:** FR-0106, FR-0008
**Tags:** must-have

---

## FR-0109: Signed Policy Bundle Activation

**Category:** Security

**Statement:** The system shall activate policy bundles only at epoch boundaries, signed by governance quorum, with no grace periods or height-based activation windows.

**Rationale:** Ensures deterministic decision reproducibility across decentralized nodes. See `network-policy-engine-spec.md` Section 5 (Policy bundle activation).

**Source Research:**
- `network-policy-engine-spec.md` Section 5, lines 111-116

**Acceptance Criteria:**
- [ ] Bundle is valid from next epoch start after governance approval.
- [ ] Plan includes `policy_bundle_hash`; plans referencing inactive bundles are rejected.
- [ ] Validators cache bundle validity for current epoch.

**Dependencies:** FR-0106, FR-0021
**Tags:** must-have

---

## FR-0110: Risk Class Step-Up Controls

**Category:** Security

**Statement:** The system shall enforce step-up controls for medium and high risk actions: medium requires secondary reviewer attestation; high requires quorum certificate or delay window plus attestation.

**Rationale:** Adds defense-in-depth for safety-critical operations. See `network-policy-engine-spec.md` Section 5 (Step-up controls).

**Source Research:**
- `network-policy-engine-spec.md` Section 5, lines 125-143
- `prompt-injection-and-network-policy-boundary.md` Section 5 (Risk classes)

**Acceptance Criteria:**
- [ ] Risk classes: low, medium, high.
- [ ] Medium: attestation signed by reviewer_eligible role, binds to plan_id, expires after 100 blocks.
- [ ] High: quorum 2/3+1 of assigned committee OR minimum 6-block delay.
- [ ] Step-up certificate is single-use, non-transferable, bound to original plan_id.

**Dependencies:** FR-0106
**Tags:** must-have

---

## FR-0111: Cross-Layer Quota Matrix

**Category:** Security

**Statement:** The system shall enforce the canonical cross-layer quota matrix with deterministic conflict resolution: hard deny quotas first, then sender/stage quotas, then per-resource quotas, deny on first breach.

**Rationale:** Coordinated quota enforcement across networking, inbox, fast-path, and governance layers. See `network-policy-engine-spec.md` Section 5 (Cross-layer quota matrix).

**Source Research:**
- `network-policy-engine-spec.md` Section 5, lines 150-173
- `index.md` (Quota Matrix)

**Acceptance Criteria:**
- [ ] All canonical quota IDs from spec are enforced at designated enforcement points.
- [ ] Conflict resolution rule is deterministic.
- [ ] Quota violations return structured deny reason codes.

**Dependencies:** FR-0106
**Tags:** must-have

---

## FR-0112: Policy Decision Audit Log

**Category:** Security

**Statement:** The system shall maintain an append-only audit log of all policy decisions with plan_id, decision, reason code, block height, and evaluator signature.

**Rationale:** Enables post-hoc reproducibility and dispute resolution. See `network-policy-engine-spec.md` Section 4 (Audit and Evidence Log).

**Source Research:**
- `network-policy-engine-spec.md` Section 4 (Architecture)
- `prompt-injection-and-network-policy-boundary.md` Section 4 (Audit Log)

**Acceptance Criteria:**
- [ ] Every policy decision is logged with structured reason code.
- [ ] Log is content-addressed and signed.
- [ ] Log supports efficient querying by plan_id, agent_id, and block range.

**Dependencies:** FR-0106
**Tags:** must-have

---

## FR-0113: Deterministic Policy Decision Point (PDP)

**Category:** Security

**Statement:** The system shall evaluate policy decisions deterministically with structured deny reason codes, no model calls, and early exit on failure.

**Rationale:** Model-based policy is probabilistic and bypassable. See `network-policy-engine-spec.md` Section 4 (Policy Decision Point).

**Source Research:**
- `network-policy-engine-spec.md` Section 4 (Component responsibilities)
- `prompt-injection-and-network-policy-boundary.md` Section 6, Tradeoff 3

**Acceptance Criteria:**
- [ ] PDP runs deterministic rule chain: schema, signature, bundle, replay, role, ACL, quota, risk.
- [ ] No probabilistic classifiers in root authorization path.
- [ ] Returns structured deny reason code on any failure.

**Dependencies:** FR-0106
**Tags:** must-have

---

## FR-0114: Taint Tracking for Sensitive Actions

**Category:** Security

**Statement:** The system shall track provenance of untrusted content in agent context and require additional review for action plans derived from tainted output.

**Rationale:** Prevents untrusted content from silently influencing high-risk actions. See `prompt-injection-and-network-policy-boundary.md` Section 5 (Taint Tracker).

**Source Research:**
- `prompt-injection-and-network-policy-boundary.md` Section 5 (Taint Tracker)
- `prompt-injection-and-network-policy-boundary.md` Section 6, Tradeoff 4

**Acceptance Criteria:**
- [ ] Inbound content is tagged untrusted by default.
- [ ] Taint propagates through agent memory to derived action plans.
- [ ] High-risk actions from tainted sources require additional reviewer certificate.
- [ ] Taint labels are included in audit log.

**Dependencies:** FR-0106
**Tags:** must-have

---

## FR-0115: Tool Output Sanitization Pipeline

**Category:** Security

**Statement:** The system shall sanitize all tool outputs deterministically: size limit (100KB), content-type validation, HTML/JS stripping, Unicode normalization (NFC), pattern filtering, and markdown escape for untrusted sources.

**Rationale:** Tool outputs are untrusted by default and can carry injection payloads. See `prompt-injection-and-network-policy-boundary.md` Section 5 (Tool output sanitization).

**Source Research:**
- `prompt-injection-and-network-policy-boundary.md` Section 5, lines 149-159

**Acceptance Criteria:**
- [ ] Output truncated to 100KB max.
- [ ] HTML scripts and event handlers stripped.
- [ ] Unicode normalized to NFC; suspicious characters (bidi, homoglyphs) flagged.
- [ ] Known injection prefixes blocked.
- [ ] Markdown escaped for untrusted source tier.

**Dependencies:** FR-0062
**Tags:** must-have

---

## FR-0116: Cumulative Risk Scoring

**Category:** Security

**Statement:** The system shall implement cumulative risk scoring to detect policy bypass attempts via tool chaining, with audit-triggered throttling.

**Rationale:** Sequence of low-risk actions can approximate high-risk outcomes. See `prompt-injection-and-network-policy-boundary.md` Section 7 (Policy bypass attempt via tool chaining).

**Source Research:**
- `prompt-injection-and-network-policy-boundary.md` Section 7 (Policy bypass attempt via tool chaining)

**Acceptance Criteria:**
- [ ] Per-workflow budget tracks cumulative risk.
- [ ] Anomalous chains trigger temporary throttling.
- [ ] Cumulative scoring is deterministic and logged.

**Dependencies:** FR-0106
**Tags:** should-have

---

## FR-0117: Atomic Quota Reservations

**Category:** Security

**Statement:** The system shall implement atomic quota reservations at plan approval time, with rollback on execution failure.

**Rationale:** Prevents quota race conditions under high concurrency. See `network-policy-engine-spec.md` Section 7 (Quota race under high concurrency).

**Source Research:**
- `network-policy-engine-spec.md` Section 7 (Quota race under high concurrency)

**Acceptance Criteria:**
- [ ] Quota is reserved atomically during PDP evaluation.
- [ ] Reserved quota is released if execution fails.
- [ ] No negative quota balances possible.

**Dependencies:** FR-0111
**Tags:** must-have

---

## FR-0118: Key Rotation State Finalization

**Category:** Security

**Statement:** The system shall finalize key binding updates on-chain/in-state before new signatures are accepted.

**Rationale:** Prevents signature validation disagreements during key rotation. See `network-policy-engine-spec.md` Section 7 (Signature key rotation mismatch).

**Source Research:**
- `network-policy-engine-spec.md` Section 7 (Signature key rotation mismatch)

**Acceptance Criteria:**
- [ ] Key rotation transaction is committed to state before new key is active.
- [ ] Signatures with old key are rejected after grace window.
- [ ] Grace window is deterministic (e.g., 100 blocks).

**Dependencies:** FR-0005
**Tags:** must-have

---

## FR-0119: Policy Bundle Split-Brain Prevention

**Category:** Security

**Statement:** The system shall include `policy_bundle_hash` in every action plan and reject plans referencing bundles not active locally.

**Rationale:** Prevents evaluation disagreements during policy update propagation. See `network-policy-engine-spec.md` Section 7 (Policy bundle split-brain).

**Source Research:**
- `network-policy-engine-spec.md` Section 7 (Policy bundle split-brain)

**Acceptance Criteria:**
- [ ] Action plan rejected if `policy_bundle_hash` != local active bundle hash.
- [ ] Bundle activation at epoch boundary is deterministic globally.
- [ ] Propagation lag does not cause permanent fragmentation.

**Dependencies:** FR-0109
**Tags:** must-have

---

## FR-0120: Network Action Type Taxonomy

**Category:** Security

**Statement:** The system shall define a minimal, versioned network action type taxonomy including: publish_topic_message, claim_task_lease, renew_task_lease, submit_fast_path_merge, submit_governance_proposal, cast_governance_vote.

**Rationale:** Minimal taxonomy reduces policy sprawl and attack surface. See `prompt-injection-and-network-policy-boundary.md` Section 5 (Typed network action plan schema).

**Source Research:**
- `prompt-injection-and-network-policy-boundary.md` Section 5, lines 63-69
- `network-policy-engine-spec.md` Section 10 (Implementation Plan)

**Acceptance Criteria:**
- [ ] Action types are enumerated and versioned.
- [ ] Unknown action types are rejected at schema validation.
- [ ] New types require governance approval.

**Dependencies:** FR-0106
**Tags:** must-have
