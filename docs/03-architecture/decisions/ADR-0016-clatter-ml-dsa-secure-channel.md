## ADR-0016: clatter + ml-dsa Secure Channel Stack (replaces Ockam)

**Status:** accepted

**Context:** Hyperfluid's p2p-wire-spec.md mandates "Ockam SecureChannel + Transport" as the end-to-end encryption layer for peer-to-peer communication. However:

1. Ockam v0.150.0 is unresolvable from crates.io — its transitive dependency `core2` v0.4.0 is yanked. No resolved version exists at current crate index.
2. Even if resolvable, Ockam is architecturally misaligned — it is a full networking framework (Nodes, Workers, Routing, Vaults, Credential Authorities) that wants to own the transport layer. Hyperfluid already has its own connection state machine, DHT, gossip, and mempool.
3. Ockam has ~400+ transitive dependencies vs the actual cryptographic primitive it provides: a Noise XX handshake with X25519 + ChaCha20-Poly1305.
4. Hyperfluid's spec requires post-quantum signatures (ML-DSA-65 per trust-boundaries.md and consensus-spec.md). Ockam provides no PQ key exchange — its secure channel uses classical X25519 ECDH, creating asymmetric security where signatures survive quantum attacks but session keys do not.
5. The p2p-wire-spec.md Section 1.8 already acknowledges: "Noise protocol framework with ML-DSA — same trust model" as an alternative.

See `docs/01-research/stack-evaluations/clatter-vs-ockam-secure-channel.md` for the full evaluation (4 tradeoffs, 6 failure scenarios, scalability analysis).

**Decision:** Replace Ockam with the **clatter** crate (v2.2.0, PQ-capable Noise framework) for the secure channel key exchange and symmetric encryption, paired with the **ml-dsa** crate (v0.1.0-rc.11, FIPS 204, RustCrypto) for identity signatures.

**Stack composition:**

| Layer | Crate | Version | Function |
|-------|-------|---------|----------|
| Identity signatures | `ml-dsa` | v0.1.0-rc.11 | ML-DSA-65 keypairs, sign/verify handshake transcript |
| Key exchange | `clatter` | v2.2.0 | Noise hybrid XX (X25519 DH + ML-KEM-768 KEM) |
| Symmetric AEAD | `clatter` (built-in) | — | ChaCha20-Poly1305 via TransportState |
| PeerId derivation | `sha3` | (existing) | SHA3-256(ML-DSA pubkey) |

**Handshake: Noise_hybridXX_X25519+MLKEM768_ChaChaPoly_SHA256**
- 3-message mutual-auth handshake identical in round-trips to classical Noise XX
- X25519 DH provides classical forward secrecy; ML-KEM-768 KEM provides post-quantum resistance
- ML-DSA-65 signatures on handshake transcript bind identity to the session (prevents identity misbinding)
- Hybrid construction: if either primitive holds, the session remains secure

**Consequences:**
- Positive: Dependency footprint reduced by ~40x (~10 deps vs Ockam's 400+). Synchronous API maps 1:1 to existing `SecureChannel` trait (establish/seal/open). True hybrid PQ security for both key exchange and signatures. MIT licensed — vendorable. Matches spec's own stated alternative ("Noise protocol framework with ML-DSA").
- Negative: clatter is a single-maintainer project (jmlepisto, 57 commits, 39 GitHub stars) with no formal cryptographic audit. ML-DSA public keys are large (~1,952 bytes) adding ~2KB to handshake messages. ML-KEM-768 ciphertexts add another ~1KB per handshake.
- Mitigation: Pre-vendor clatter source at integration time (MIT license permits). Commission formal audit before Stage 04 (Mainnet). The hybrid construction means classical X25519 security persists even if ML-KEM falls. The `snow` crate is documented as fallback (classical-only, no PQ key exchange).

**Alternatives considered:**
- **Ockam v0.150.0 (status quo):** Rejected — unresolvable from crates.io (yanked transitive dep `core2`). Even if fork-fixed, architectural mismatch (wants to own networking, Hyperfluid has its own) and 400+ dependency supply chain.
- **snow only (classical Noise):** Rejected — no PQ key exchange. Would leave session keys vulnerable to quantum attack while signatures are PQ. Creates asymmetric security posture.
- **snow + manual ML-KEM integration:** Rejected — custom hybrid key exchange protocol design is the most common source of cryptographic vulnerabilities. clatter already implements this correctly, verified against PQNoise test vectors.
- **Raw pqcrypto primitives without Noise framework:** Rejected — Noise encapsulates decades of authenticated key-exchange engineering (transcript hashing, rekey mechanics, nonce handling) that are trivially gotten wrong in a bespoke implementation.
- **Fix Ockam by forking and patching:** Rejected — Ockam is 400+ crates. The maintenance burden of a fork is orders of magnitude larger than adopting clatter.

**Related:** ADR-0007 (Committee BFT, ML-DSA requirement), ADR-0001 (C7 P2P Networking), `docs/01-research/stack-evaluations/clatter-vs-ockam-secure-channel.md`, `docs/04-specifications/protocol/p2p-wire-spec.md`, `docs/04-specifications/protocol/consensus-spec.md`
