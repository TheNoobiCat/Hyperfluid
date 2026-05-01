## ADR-0012: Circuit-Breaker Escalation Hierarchy

**Status:** accepted

**Context:** Under attack or extreme load, the system must automatically restrict non-critical operations to preserve safety-critical ones. Manual intervention is not viable in a decentralized network. Escalation and de-escalation must be deterministic and autonomous.

**Decision:** Implement a three-tier circuit-breaker escalation hierarchy:

- **Normal:** Full functionality. No restrictions.
- **Degraded:** Freeze new low-trust claims, tighten merge quotas, force digest-only for low-trust senders. Triggered by sustained reject_ratio breach or queue depth spike.
- **Emergency:** 3x PoW difficulty, 50% unknown-sender budgets, frozen low-trust fast-path, emergency fee floor, reserved control lanes. Triggered by finality_lag > 60s for 3 consecutive blocks + minimum independent reporters + signed evidence validity (FR-0142).

All transitions require:
1. Metric breach on **multiple metrics simultaneously** (not single metric noise)
2. **Persistence** across consecutive measurement windows (prevents transients)
3. **Minimum independent reporter count** (prevents single-source manipulation)
4. **Signed evidence validity** (prevents fabricated triggers)

De-escalation requires sustained metric normalization with **hysteresis** (exit thresholds stricter than entry thresholds). No manual override required for either direction.

**Consequences:**
- Positive: Automatic defense escalation without central operator. Hysteresis prevents mode flapping. Multi-metric triggers prevent false positives from noisy telemetry. Control lanes always survive. Temporary post-incident quotas prevent recovery backlash.
- Negative: Emergency mode is restrictive and may degrade legitimate collaboration. Tuning thresholds requires calibration from testnet data. False positives from coordinated telemetry spoofing still possible (mitigated by multi-source corroboration and independent observation reconciliation per FR-0141).

**Alternatives considered:**
- **Single tier (normal/emergency):** Rejected because binary escalation is too coarse. Degraded mode provides graduated response for moderate stress.
- **Manual override for emergency declaration:** Rejected because it introduces a central coordinator single point of failure and contradicts decentralized incident response design.
- **Per-component circuit-breakers independent:** Rejected because coordinated restrictions are needed during network-wide attacks. Independent breakers could create inconsistent application of restrictions.

**Related:** FR-0085, FR-0100, FR-0142, FR-0143, FR-0154, FR-0187, `failure-model.md` Section 3.
