## ADR-0007: Committee BFT with VDF Randomness

**Status:** accepted

**Context:** Consensus must scale to a large validator set without degrading liveness. Committee BFT bounds communication to a fixed-size committee (100 validators), but committee selection must be fair, unbiased, and resistant to manipulation.

**Decision:** Use committee-based BFT from genesis with a fixed committee size of 100 validators, stake-weighted sampling with 15% per-operator cap, and VDF-derived epoch seeds from validator commitment-reveal inputs. Partial committee overlap (max 33%) preserves liveness across epoch transitions.

**Consequences:**
- Positive: O(1) communication overhead regardless of total validator count. BFT safety with f < 33% Byzantine validators. VDF randomness prevents last-revealer bias and grinding attacks. Anti-concentration cap prevents whale capture.
- Negative: Only 100 validators actively participate in consensus per epoch; wider set pushed to staking/relay/collaboration roles. VDF evaluation requires sequential computation time > 2x reveal window. Committee transition at epoch boundary is a liveness-risk point.

**Alternatives considered:**
- **Full-set BFT (all validators vote):** Rejected because communication overhead scales quadratically, breaking liveness under rapid growth. Not practical above ~100 validators.
- **Drand-based randomness:** Rejected in decentralisation audit because it introduces an external trust dependency. VDF is self-contained within the protocol.
- **VRF-based sampling:** Considered but rejected because VRF requires secret key (can be withheld to bias selection). VDF commitment-reveal is bias-resistant.

**Related:** FR-0001, FR-0002, FR-0003, FR-0004, NFR-0016.
