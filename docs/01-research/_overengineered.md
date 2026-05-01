# Decentralisation Audit: Found Issues

## 1. Per-IP rate limits and connection caps
**Source:** `consensus-governance/agx-committee-bft-and-governance.md` (Section 5, Rate limiting) and `networking/ockam-decentralized-network-architecture.md` (Section 5, Swarm-resistant ingress controls)

**Summary:** Hard per-IP and per-ASN connection/transaction limits are used as anti-spam and anti-Sybil controls.

**Why it doesn't fit:** Decentralised networks cannot assume one IP equals one identity. Honest agents behind NAT, VPNs, Tor, mobile carriers, or cloud egress points share IP addresses. Per-IP limits therefore either throttle legitimate participants or are trivially bypassed by adversaries with diverse address ranges. This is an infrastructure-control assumption that collapses under the normal topology of a permissionless P2P network.

**Proposed fix:** Replace per-IP limits with staking-credential and reputation-weighted rate tiers. Base admission on the stronger identifier (on-chain identity + ML-DSA signature with nonce checks), not network-layer fingerprints. Apply PoW as the cost-of-entry for unknown identities. Keep connection-level resource bounds (max file descriptors, socket counts) purely as local DoS protection, not as protocol policy. Acknowledged caveat: the handshake window before identity is established remains a real attack surface and needs careful rate limiting at the implementation layer, but that is a local implementation detail, not a protocol-level Sybil control.

---

## 2. IP-address anti-Sybil for the airdrop faucet
**Source:** `consensus-governance/agx-economics-and-adversarial-incentives.md` (Section 5, New agent onboarding)

**Summary:** The autonomous airdrop agent uses IP-address uniqueness to prevent Sybil claims.

**Why it doesn't fit:** Using IP addresses as a Sybil-resistance mechanism relies on a centralised view of network topology that does not exist in a decentralised system. It either excludes legitimate agents behind shared infrastructure or is easily gamed via proxies, VPNs, and botnets. It silently depends on an external, trusted IP-allocation layer that the protocol does not control.

**Proposed fix:** Replace IP gating with a proof-of-unique-humanity or proof-of-work puzzle that is identity-bound. For example: require each new identity to post a signed interactive challenge-response (e.g., solve a deterministic puzzle seeded by the agent's pubkey) and impose a small but non-zero AGX bond. Cap the number of airdrops per *epoch* rather than per IP, and require a minimum on-chain "birth block" delay before the first claim can be spent. The bond is the actual anti-Sybil mechanism — it must be sized meaningfully relative to the airdrop value so that pre-registration farming is unprofitable. The birth-block delay alone does not prevent farming; the bond cost is what does the work.

---

## 3. Manual reviewer assignment and manual review flags
**Source:** `agents/proof-of-work-quality-and-review-markets.md` (Section 8, Reviewer assignment parameters)

**Summary:** Low reviewer pools trigger "manual assignment required" and tasks are "flagged for manual review."

**Why it doesn't fit:** Hyperfluid is explicitly agent-native with "no humans in the loop." Introducing manual assignment or manual review gates assumes a human administrator or central coordinator that the architecture denies exists. A trustless system must have a deterministic fallback—such as a wider auto-assigned panel or extended deadline—rather than halting for human intervention.

**Proposed fix:** Remove "manual assignment" triggers entirely. Replace with deterministic fallbacks: if the eligible pool falls below the auto-assignment threshold, widen the reviewer eligibility criteria (e.g., relax the minimum reviewer pool from 50 to the current pool size), extend the assignment deadline, or reduce the required reviewer count for that task with a proportional downgrade in reward cap. Flag tasks with reduced reviewer sets for a mandatory challenge window extension, not for human review. Add an absolute minimum floor: never review with fewer than 3 reviewers. If the pool drops below this floor, return the task to the open queue rather than risk capture by a single reviewer with inflated weight.

---

## 4. Protocol-level economic enforcement of unverifiable local token burn
**Source:** `agents/token-budget-resource-model.md` (Section 5, Token burn economic model; Section 8, Scalability)

**Summary:** AGX rewards and reputation are tied to self-reported LLM token consumption (`ptok`).

**Why it doesn't fit:** Token burn happens inside an agent's local LLM inference runtime, which is cryptographically unverifiable by the rest of the network. The protocol proposes to economically reward "quality-per-token-burn" and penalise "excessive token burn," but these metrics are purely self-reported. In a trustless context this creates an immediate incentive to under-report burn, and the proposed cross-checks (output length heuristics) cannot verify actual input plus overhead tokens. Basing protocol economics on locally unobservable computation is architecturally unsound.

**Proposed fix:** Downgrade token burn from a protocol-enforced economic resource to a *local best-practice recommendation*. Do not tie AGX rewards or reputation to self-reported token metrics. Instead, reward only observable protocol outputs: accepted task deliverables, validated review records, and governance votes that survive challenge windows. Token budgets can remain as local runtime tuning knobs (governed by the operator), but the network should remain agnostic to how much compute an agent consumed internally.

---

## 5. Committee randomness sourced from Cloudflare drand
**Source:** `consensus-governance/agx-committee-bft-and-governance.md` (Section 5, Committee BFT from day 1)

**Summary:** Epoch committee selection relies primarily on `drand.cloudflare.com` as its randomness beacon.

**Why it doesn't fit:** Validator committee selection is a safety-critical function. Relying on a single external randomness service introduces a trusted third party and a single point of compromise into core consensus. Even with a hash-chain fallback, the canonical source is centralised infrastructure. A decentralised network should not outsource its committee entropy to a third-party service. (Note: the League of Entropy runs the drand network; Cloudflare is one node operator and endpoint host.)

**Proposed fix:** Replace the external drand dependency with an on-chain commitment-reveal scheme between validators as a first step: validators submit a hash commitment in block `N`, reveal the preimage in block `N+k`, and the XOR/hash of all valid reveals seeds the next epoch's committee. This is trust-minimised and requires no external infrastructure, but it is *not fully trust-minimised* — a malicious last-revealer can grind for a favourable output by selectively withholding their reveal. The complete solution is an on-chain verifiable-delay function (VDF, e.g., Wesolowski or Pietrzak) that eliminates last-revealer bias entirely. Treat this as a phased approach: commit-reveal now, VDF later. Until the VDF is live, the protocol should treat committee randomness as a known limitation and keep overlap/anti-concentration caps strict to bound any grinding advantage.

---

## 6. Geographic reviewer-spread requirements
**Source:** `agents/proof-of-work-quality-and-review-markets.md` (Section 8, Reviewer assignment parameters)

**Summary:** Reviewer assignment requires "min 2 different regions (Americas, EMEA, APAC)."

**Why it doesn't fit:** Enforcing geographic diversity in a pseudonymous decentralised network requires a trusted geo-IP oracle or self-attested location data, both of which are either centralised or trivially spoofed. The requirement silently depends on an external, trusted location provider to label peers by region, adding complexity without providing a real decentralisation guarantee in a permissionless setting.

**Proposed fix:** Replace the geographic spread rule with a network-topology diversity constraint that is actually verifiable on-chain: require reviewers to be selected from distinct *operator identity clusters* detected via stake-graph analysis and key correlation heuristics. If the pool is too small to satisfy cluster diversity, fall back to requiring independent validator endorsements for the review set. Stake-graph clustering is the right proxy for "avoid reviewer collusion" because it is derived from on-chain data, not asserted geography.
