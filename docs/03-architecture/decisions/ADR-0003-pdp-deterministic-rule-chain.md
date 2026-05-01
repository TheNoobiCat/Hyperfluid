## ADR-0003: Policy Decision Point as Deterministic Rule Chain

**Status:** accepted

**Context:** All network-mutating actions from agents must be validated before execution. The validation mechanism must be deterministic, auditable, and resistant to adversarial inputs. ML-based approaches are probabilistic and can be bypassed.

**Decision:** Implement the Policy Decision Point (C9) as a deterministic 8-step rule chain with no ML in the root authorization path:

1. Schema validation (action plan field validation)
2. Signature verification (ML-DSA-65)
3. Policy bundle activation check (hash match, epoch boundary)
4. Replay protection (plan_id uniqueness, nonce monotonicity, TTL)
5. Role check (risk class vs trust stage)
6. ACL check (action_type allowed on resource_id)
7. Quota check (cross-layer quota matrix)
8. Risk step-up check (attestation/quorum for medium/high risk)

**Consequences:**
- Positive: Decisions are identical on all nodes given same input. Audit log is deterministic and replayable. Cannot be gamed through adversarial prompt engineering. O(1) evaluation time per plan. Classifier signals can tighten quotas but cannot authorize or deny execution.
- Negative: Less flexibility for novel action patterns that don't fit existing action types. New action types require governance proposal. Rule chain cannot adapt to context beyond the 8 defined steps.

**Alternatives considered:**
- **ModernBERT classifier as primary authorizer:** Rejected because classifiers are probabilistic (can produce different results on different nodes) and can be bypassed via adversarial inputs. FR-0122 limits classifiers to auxiliary signals only.
- **Multi-factor LLM review as gate:** Rejected because LLM output is non-deterministic and introduces latency. LLM review is applied in the Review Sandbox (after deterministic gates pass) for governance and topic merge evaluation.

**Related:** FR-0106, FR-0113, FR-0122, `component-model/components.md` C9.
