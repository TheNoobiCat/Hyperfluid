## ADR-0005: Content-Addressed State with SMT

**Status:** accepted

**Context:** Protocol state must be efficiently verifiable, support compact inclusion proofs, and enable lightweight clients without full history. State commitment must be deterministic and included in every block header.

**Decision:** Store all protocol state in a Sparse Merkle Tree (SMT) with content-addressed artifacts stored off-state. The SMT root is committed in every block header. Artifacts (governance bundles, review evidence, research outputs) are stored as content-addressed git objects via gix, with manifests in protocol state and replication off-state.

**Consequences:**
- Positive: Compact state proofs for any key (O(log n) proofs). Efficient light client verification. Content-addressing enables parallel retrieval and hash-verified integrity. Deterministic state root across all honest nodes.
- Negative: SMT key design requires namespace planning (16 key type prefixes). Artifact retrieval requires lookup step (content hash → provider addresses). Pruning old state leaves requires careful handling.

**Alternatives considered:**
- **Patricia Merkle Trie (Ethereum-style):** Rejected because SMT provides simpler proof generation and verification. SMT has no shared nibble prefix complexity.
- **All artifacts stored on-chain:** Rejected because state size growth would exceed NFR-0002 bounds (<1GB/month). Content-addressing off-chain with on-chain manifests keeps consensus state compact.

**Related:** FR-0010, FR-0051, NFR-0002, NFR-0019, `data-model/state-model.md`.
