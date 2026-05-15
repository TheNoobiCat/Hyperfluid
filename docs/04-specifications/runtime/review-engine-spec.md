# Runtime Spec: Review Engine

**Component:** C12 Economics & Incentives (Review Market)
**Source ADRs:** ADR-0008 (Two-Phase Quality Pipeline), ADR-0017 (90/10 Payout Split)
**Covered FRs:** FR-0148, FR-0149, FR-0150, FR-0161, FR-0164, FR-0165, FR-0168, FR-0169, FR-0170, FR-0191
**Dependencies:** C1 Consensus Engine, C8 Artifact Availability, C9 Policy Decision Point

### 1.8 Trust-Assumption Inventory

- Reviewer independence constraints effectiveness
  - Justification: Stake-graph analysis and key correlation may not detect all collusion relationships.
  - Trust-minimised alternative: Economic incentives that make collusion more expensive than honesty (requires calibration).
- Challenge arbiter fairness
  - Justification: Challenge outcomes affect real economic penalties; arbiter must be auditable.
  - Trust-minimised alternative: Multi-sig arbiter panel from diverse validators with governance-enforced penalties on incorrect arbitration.

---

## Section 2: Sybil Detection

Sybil detection is handled by the trust ladder's abuse-flag mechanism (see collaboration-spec.md §3). No separate protocol-level correlation engine exists at this layer.
