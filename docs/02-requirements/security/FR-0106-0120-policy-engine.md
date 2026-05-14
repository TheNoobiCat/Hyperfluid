## FR-0106: Typed Network Action Plans

**Category:** Security

**Statement:** The system shall require typed network action plans for all network-mutating operations, with schema validation, signature verification, and deterministic policy evaluation.

**Rationale:** Converts injection defense from heuristic text interpretation to enforceable control logic. See `network-policy-engine-spec.md` Section 2 (Executive Summary).

**Source Research:**
- `network-policy-engine-spec.md` Section 5 (Action plan schema)
- `prompt-injection-and-network-policy-boundary.md` Section 5 (Typed network action plan schema)

**Acceptance Criteria:**
- [ ] Action plan schema includes: plan_id, agent_id, action_type, resource_id, nonce, expires_at_height, agent_signature.
- [ ] Free text alone never executes network-mutating tools.
- [ ] Schema validation is deterministic across all nodes.

**Dependencies:** FR-0007
**Tags:** must-have

---

## FR-0107: Tool-Call Verification

**Category:** Security

**Statement:** The system shall verify that each network tool call is properly signed and authorized by a valid action plan.

**Rationale:** Prevents unauthorized tool calls. Tool-call binding hash verification is an agent runtime local concern — the protocol only enforces signature and plan validity.

**Source Research:**
- `network-policy-engine-spec.md` Section 5

**Acceptance Criteria:**
- [ ] Gateway verifies agent signature on action plan.
- [ ] Approved plans are single-use and transition to consumed state after execution.
- [ ] Any action without a valid approved plan is rejected.

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
- [ ] PDP runs deterministic rule chain: schema, signature, replay, quota, fee.
- [ ] No probabilistic classifiers in root authorization path.
- [ ] Returns structured deny reason code on any failure.

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
