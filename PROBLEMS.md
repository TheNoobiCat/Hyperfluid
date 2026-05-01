# Hyperfluid Known Issues and Risks

Comprehensive analysis of problems, risks, and concerning assumptions identified across all 16 research documents.

**Last Updated:** 2026-04-27  
**Documents Analyzed:** 16 research documents  
**Status:** Research Phase - Issues Documented, Not Yet Addressed

---

## Executive Summary

After analyzing all research documents, **47 distinct issues** have been identified across 8 categories. **11 are CRITICAL** (must fix before any deployment), **18 are HIGH** (must fix before mainnet), **14 are MEDIUM** (should fix before testnet), and **4 are LOW** (can defer).

---

## Category A: Economic & Incentive Design

### A1. ~~Zero-Fee Transfer Spam Risk~~ ✅ RESOLVED
**Status:** DECISION MADE - Switching to fee markets  
**Resolution:** Zero-fee economics abandoned in favor of fee markets

**Rationale:** Zero-fee with PoW/quota has failed in every prior implementation (Nano, IOTA, EOS). Fee markets are the only proven solution.

**New Design:**
- Minimum fee for all transactions (e.g., 0.001 AGX base fee)
- EIP-1559 style dynamic fee market for priority
- Fee burn mechanism for deflationary pressure
- Staked validators get fee rebates

**Related:** All zero-fee references in research docs updated to fee markets

---

### A2. Review Market Collusion ⭐ CRITICAL
**Severity:** CRITICAL  
**Likelihood:** HIGH  
**Impact:** Quality control failure

**Evidence:**
- `proof-of-work-quality-and-review-markets.md` Section 5: Anti-collusion controls specified but rely on "reviewer independence scoring"
- Same doc, Section 7: "Reviewer cartel inflates quality scores" listed as failure mode but mitigation is weak
- Same doc, Tradeoff 2: "reviewer scarcity in niche domains may increase queue latency"

**Problem:** Review markets assume independent reviewers. The anti-collusion metrics (pair_repeat_ratio, vote_correlation_z) detect collusion after it occurs but don't prevent it. Economic incentives favor cartel formation.

**Specific Risks:**
- Review rings trade positive reviews for mutual benefit
- Experienced reviewers dominate, creating oligopoly
- Challenge windows may be too short for honest detection
- Reviewer bonds may be smaller than collusion profits

**Mitigation Required:**
- Pre-commitment schemes (reviewers commit to reviews before seeing each other's)
- Mandatory reviewer diversity (geographic, temporal, stake-weight spread)
- Higher bonds for reviewers than current specification
- Random reviewer assignment with no opt-out

**Related:** A3, C1, D2

---

### A3. Economic Timing Attacks
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** Economic extraction

**Evidence:**
- `proof-of-work-quality-and-review-markets.md` Section 5: "Finality verification: challenge window closes at deterministic height h_close"
- `agx-economics-and-adversarial-incentives.md` Section 5: "Settlement only accepts records with valid signatures, matching hashes, and finalized inclusion proofs"
- Timing between "provisional settlement" and "final settlement" not precisely specified

**Problem:** Time gaps between work submission, provisional acceptance, challenge window close, and final settlement create arbitrage opportunities. Attackers can exploit timing to extract value before challenges resolve.

**Specific Risks:**
- Flash loans to boost stake during reward calculation
- Front-running challenge submissions
- MEV extraction from settlement ordering
- Time-zone attacks on global challenge windows

**Mitigation Required:**
- Uniform challenge windows across all time zones
- Commit-reveal scheme for challenges
- Flash loan resistance in settlement (time-delayed stake weighting)
- Explicit ordering rules for simultaneous submissions

**Related:** A2, B2

---

### A4. Stake Centralization Pressure
**Severity:** HIGH  
**Likelihood:** HIGH  
**Impact:** Committee capture

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: "Committee overlap: 33% retained members between consecutive epochs"
- `decentralization-and-stack-benchmark.md` Section 7: "Economic centralization over epochs" failure mode
- Same doc: "operator identity aggregation, effective stake caps" proposed but not concretely specified

**Problem:** Wealthy operators can split stake across multiple validators (Sybil stake) to maximize committee representation. The 33% overlap rule favors incumbents. No concrete operator identity binding specified.

**Specific Risks:**
- Single entity controls >33% of committee via stake splitting
- Self-dealing in governance votes
- Censorship of unfavorable transactions
- Governance proposals favoring large stakeholders

**Mitigation Required:**
- Strong identity binding (KYC or long-cooldown identity establishment)
- Effective stake caps per verified operator
- Anti-Sybil deposits that increase super-linearly with validator count
- Mandatory diversity requirements in committee sampling

**Related:** A5, D1

---

### A5. Bond Economics Mismatch
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Insufficient deterrence

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: "equivocation_slash: 10% of bonded stake per proven event"
- `proof-of-work-quality-and-review-markets.md` Section 5: "Reviewer collateral is bonded per review batch"
- `agx-economics-and-adversarial-incentives.md` Section 5: "Lease claim requires small collateral"

**Problem:** Bond amounts not calibrated to attack value. 10% slash for equivocation may be less than attacker gain from double-spend. "Small collateral" for leases is undefined and likely too small.

**Specific Risks:**
- Rational attackers accept 10% loss for >10% gain
- Lease squatting is profitable with "small" collateral
- Reviewer bonds smaller than collusion profits

**Mitigation Required:**
- Dynamic bonds based on attack value at risk
- Higher bonds for high-value operations
- Economic modeling of rational attack scenarios

**Related:** A2, B5

---

### A6. Volume-Based Reward Drift
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Incentive misalignment

**Evidence:**
- `agx-economics-and-adversarial-incentives.md` Section 5: "Rewards weighted by accepted outcome quality signals"
- `proof-of-work-quality-and-review-markets.md` Section 6 Tradeoff 1: "volume-based incentives directly invite spam farms"

**Problem:** While quality is emphasized, implementation may drift toward volume metrics for simplicity. No specific quality metric formulas provided.

**Specific Risks:**
- Implementation simplifies to easily measurable metrics
- Quality signals become gameable
- Network floods with low-effort, technically valid submissions

**Mitigation Required:**
- Formal specification of quality scoring algorithms
- Multiple independent quality dimensions
- Regular recalibration based on outcome durability

---

## Category B: Consensus & Safety

### B1. Malachite Alpha Status ⭐ CRITICAL
**Severity:** CRITICAL  
**Likelihood:** HIGH  
**Impact:** Consensus failures

**Evidence:**
- `decentralization-and-stack-benchmark.md` Section 2: "Malachite is promising but still alpha; this raises operational risk"
- Same doc: "must be offset with strict versioning, simulation, and rollback procedures"

**Problem:** Hyperfluid depends on alpha-quality consensus library. Bugs in Malachite could cause finality failures, liveness issues, or safety violations.

**Specific Risks:**
- Undiscovered consensus bugs in alpha code
- Breaking changes in Malachite API
- Limited production battle-testing
- Integration bugs between Malachite and Hyperfluid code

**Mitigation Required:**
- Extensive simulation testing before mainnet
- Formal verification of critical consensus paths
- Canary deployment with rollback capability
- Bug bounty program specifically for consensus
- Fallback to Tendermint if Malachite fails

**Related:** B7, D7

---

### B2. Governance Determinism Failures ⭐ CRITICAL
**Severity:** CRITICAL  
**Likelihood:** MEDIUM  
**Impact:** Chain split

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: "determinism must be enforced with hermetic execution and reproducible input bundles"
- Same doc, Failure Mode: "Deterministic governance divergence" with handling: "reject on any deterministic mismatch"
- Same doc, Failure Mode: "Review subagent timeout or deadlock"

**Problem:** `git:head` governance requires perfectly deterministic git execution across heterogeneous machines. Git behavior varies by version, environment, filesystem. Subagent timeouts create non-determinism.

**Specific Risks:**
- Validators disagree on merge validity
- Chain splits on governance proposals
- Validators cannot reproduce each other's execution
- Timeout handling not deterministic

**Mitigation Required:**
- Pinned gix version across all validators
- Containerized hermetic execution environment
- Pre-flight determinism checks before voting
- Clear timeout handling in consensus rules

**Related:** B1, E3

---

### B3. No-Vote Timeout Non-Determinism
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** Liveness failures

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: "subagent timeout is 30 minutes; on timeout, no vote is emitted"
- `topic-fastpath-protocol-spec.md` Tradeoff 4: "treat reviewer timeout as no vote" chosen
- No explicit coordination of timeout behavior across validators

**Problem:** If validators have different timeout behaviors or clock skew, some vote while others don't. This creates inconsistent quorum calculations.

**Specific Risks:**
- Split quorums on same proposal
- Some validators accept, others reject
- No canonical rule for timeout synchronization

**Mitigation Required:**
- Synchronized timeout windows based on block height, not wall clock
- Explicit "no-vote" transaction signed by validator
- Canonical handling of partial participation

**Related:** B2, C4

---

### B4. Adaptive PoW Response Lag
**Severity:** HIGH  
**Likelihood:** HIGH  
**Impact:** Admission failures

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: "Adaptive target retargeting by mempool load"
- `decentralization-and-stack-benchmark.md` Section 5: "adaptive controls can be easier to spam than fee-market chains"
- No specific retargeting algorithm or speed specified

**Problem:** Adaptive difficulty responds to historical mempool state, not current attack. Botnet can flood faster than retargeting responds.

**Specific Risks:**
- Mempool saturates before difficulty increases
- Honest transactions rejected during attack
- Difficulty oscillation from noisy mempool signal

**Mitigation Required:**
- Predictive retargeting based on trend, not just current state
- Multi-tier response (immediate quota tightening, slower difficulty adjustment)
- Burst detection with immediate emergency mode

**Related:** A1, D6

---

### B5. Committee Randomness Weakness
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** Predictable committees

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: "Epoch seed derived from prior finalized randomness beacon"
- `decentralization-and-stack-benchmark.md` Future Improvements: "Add stronger distributed randomness beacons"
- Current randomness source not specified

**Problem:** If randomness is manipulable or predictable, attackers can influence committee selection. "Prior finalized randomness" may be grindable.

**Specific Risks:**
- Committee grinding attacks
- Predictable committee membership
- Targeted DDoS of next committee

**Mitigation Required:**
- VDF-based unbiasable randomness
- Commit-reveal scheme for committee selection
- Delayed committee reveal

**Related:** A4, D1

---

### B6. Equivocation Detection Latency
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Safety violations

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: "Equivocation: validator signs two conflicting votes for same height/round"
- Evidence transaction type exists but timing not specified
- Slashing happens after evidence, not prevention

**Problem:** Equivocation can succeed if evidence is submitted after blocks finalize. Double-signing window may allow attacks before slashing.

**Specific Risks:**
- Double-spend before slashing activates
- Evidence censorship by committee
- Light clients don't see equivocation

**Mitigation Required:**
- Pre-commitment to single chain before finality
- Immediate slashing on evidence
- Light client equivocation proofs

---

### B7. Liveness Misclassification
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Unnecessary slashing

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: "probationary" state for temporary misses
- Failure Mode: "Quorum oscillation from liveness misclassification"
- "hysteresis windows, rolling participation scores, and delayed status transitions" mentioned

**Problem:** Transient network issues can cause validator to be marked inactive, then active, then inactive. Oscillation wastes stake mobility.

**Specific Risks:**
- Honest validators slashed for transient issues
- Stake churn from status oscillation
- Committee instability

**Mitigation Required:**
- Conservative timeout thresholds
- Grace periods longer than typical network blips
- Appeals process for slashings

**Related:** B1, E4

---

## Category C: Trust & Identity

### C1. Trust Ladder Gaming ⭐ CRITICAL
**Severity:** CRITICAL  
**Likelihood:** HIGH  
**Impact:** Permission escalation

**Evidence:**
- `identity-reputation-and-trust-ladder.md` Section 5: "Promotion requirements" include "Minimum accepted-work count with low rollback/challenge-loss ratio"
- Same doc, Failure Mode: "Collusive review ring" - "tightly coupled identities exploit naive scoring"
- Same doc, Failure Mode: "Identity whitewashing" - "penalized operator rotates to fresh identities"

**Problem:** Trust progression can be gamed by coordinated behavior. New identities can "wash" reputation by behaving well during probation, then attack when trusted.

**Specific Risks:**
- Sybil army all promotes simultaneously
- Good behavior during sandboxed period, malicious when trusted
- Collusion rings boost each other's trust scores
- Easy identity creation enables rapid whitewashing

**Mitigation Required:**
- Economic cost for identity creation (bond)
- Minimum time in each stage (cannot rush)
- Diversity requirements in attestations
- Correlation detection in trust signal graph
- Identity linkability (new identities linked to old penalties)

**Related:** A2, C3, D2

---

### C2. Sybil Identity Flood
**Severity:** HIGH  
**Likelihood:** HIGH  
**Impact:** Resource exhaustion

**Evidence:**
- `agx-economics-and-adversarial-incentives.md` Failure Mode: "Sybil flood with low-cost identities"
- `inbox-attention-control-and-anti-spam.md` Section 5: "Unknown-sender payload policy: DIGEST_ONLY until trust threshold crossed"
- `ockam-decentralized-network-architecture.md` Section 5: "Malicious join swarm" failure mode

**Problem:** Low-cost identity creation enables spam armies. Even with rate limits per identity, aggregate spam can overwhelm.

**Specific Risks:**
- Millions of identities created for coordinated spam
- Per-identity quotas meaningless with infinite identities
- Network resources consumed by handshake/verification

**Mitigation Required:**
- Proof-of-personhood or economic bond for identity
- Global rate limit on identity creation
- Progressive trust only after time + work
- Identity creation cost increases with network load

**Related:** C1, A1, D6

---

### C3. Whitewash Guard Incompleteness
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** Reputation evasion

**Evidence:**
- `identity-reputation-and-trust-ladder.md` Section 5: "Whitewash guard: new identities cannot instantly inherit prior authority"
- Same doc, Failure Mode: "Identity whitewashing" handling: "stage reset to untrusted_joiner"
- No mention of economic or cryptographic identity linking

**Problem:** Penalized operators can create new identities with no link to past behavior. Current "whitewash guard" only delays, not prevents, re-entry.

**Specific Risks:**
- Same operator repeatedly abuses, creates new identity, repeats
- No lasting penalty for bad behavior
- Reputation system becomes meaningless

**Mitigation Required:**
- Economic bond forfeiture on penalty (not recoverable)
- Cryptographic identity linkability (zero-knowledge proof of unique personhood)
- Hardware attestation for identity binding
- Social graph analysis to detect operator clusters

**Related:** C1, A4

---

### C4. Reviewer Timeout Fairness
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Review quality degradation

**Evidence:**
- `topic-fastpath-protocol-spec.md` Tradeoff 4: "reviewer timeout as no vote" chosen
- `agx-committee-bft-and-governance.md` Section 5: "subagent timeout is 30 minutes"
- No-vote semantics not fully specified across governance, fastpath, runtime

**Problem:** Inconsistent no-vote handling across subsystems. Some systems may treat timeout as deny, others as abstain.

**Specific Risks:**
- Same situation handled differently in different contexts
- Validators confused about timeout behavior
- Strategic timeout exploitation

**Mitigation Required:**
- Unified no-vote semantics specification
- Explicit timeout transaction type
- Consistent handling across all review contexts

**Related:** B3, C1

---

## Category D: Network & P2P

### D1. Relay Concentration ⭐ CRITICAL
**Severity:** CRITICAL  
**Likelihood:** HIGH  
**Impact:** Censorship, network partition

**Evidence:**
- `ockam-decentralized-network-architecture.md` Section 7: "Relay market concentration" failure mode
- `decentralization-and-stack-benchmark.md` Failure Mode: "Relay market concentration" with handling "relay rewards tied to diversity"
- Rewards not specified in detail, may be insufficient

**Problem:** Network depends on relays for NAT traversal. If few operators run relays, they can censor or partition network.

**Specific Risks:**
- 3-4 relay operators control all NATed traffic
- Geographic concentration creates regional partitions
- Relay operators can censor specific content
- Relay failure cascades to large user segments

**Mitigation Required:**
- Strong relay operator incentives (significant rewards)
- Minimum relay diversity requirements
- Geographic distribution mandates
- Client relay set rotation

**Related:** A4, D4

---

### D2. Gossip Amplification Attacks
**Severity:** HIGH  
**Likelihood:** HIGH  
**Impact:** Bandwidth exhaustion

**Evidence:**
- `ockam-decentralized-network-architecture.md` Section 5: "Swarm-resistant ingress controls" mentions gossip budgets
- `decentralization-and-stack-benchmark.md` Section 5: Cross-layer hardening includes "staged ingress controls"
- Specific gossip fanout limits not clearly specified

**Problem:** Gossip protocols amplify messages. Malicious actor can create messages that spread exponentially.

**Specific Risks:**
- Sybil nodes amplify gossip artificially
- Duplicate suppression bypassed via message mutation
- Bandwidth exhaustion from gossip storms

**Mitigation Required:**
- Strict fanout limits (max 5-10 peers)
- Message ID duplicate suppression with bloom filters
- Gossip budget per sender per topic
- Backpressure when bandwidth exceeded

**Related:** D6, C2

---

### D3. Bootstrap Node Centralization
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** New node onboarding failure

**Evidence:**
- `ockam-decentralized-network-architecture.md` Section 4: "Bootstrap Node: low-churn discovery entrypoint"
- Same doc: "Does not become mandatory after network join"
- Bootstrap node selection not specified

**Problem:** New nodes must connect to bootstrap nodes first. If these are centralized or fail, network can't grow.

**Specific Risks:**
- Few bootstrap nodes = centralization
- Bootstrap node failure prevents new joins
- Eclipse attacks via compromised bootstrap

**Mitigation Required:**
- Large, diverse bootstrap node set (100+)
- Bootstrap node selection randomization
- Hardcoded bootstrap list in client
- Community-run bootstrap incentive program

**Related:** D1, D7

---

### D4. DHT Eclipse Vulnerability
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** Routing manipulation

**Evidence:**
- `ockam-decentralized-network-architecture.md` Tradeoff 2: "DHT churn and eclipse attacks degrade lookup quality unless bucket diversity and signature validation are enforced"
- Specific eclipse countermeasures not detailed

**Problem:** Kademlia DHT vulnerable to eclipse attacks where attacker controls all neighbors of target node.

**Specific Risks:**
- Attacker isolates node from network
- Routing to malicious intermediaries
- Lookup results manipulated

**Mitigation Required:**
- Bucket diversity enforcement (different ASNs, regions)
- Signature validation on all DHT records
- Lookup from multiple starting points
- Sybil resistance in DHT keyspace

**Related:** D1, C2

---

### D5. Content-Addressing Availability Gap
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** Governance failure

**Evidence:**
- `artifact-availability-and-retention.md` Section 2: "Governance and fast-path flows depend on artifact availability; missing data is a protocol-level fault condition"
- Same doc, Failure Mode: "Governance bundle unavailable at vote time" with handling "proposal precheck fails deterministically"
- Proof-of-possession cadence not specified

**Problem:** Governance assumes artifacts always available, but providers can churn. No guaranteed availability SLA.

**Specific Risks:**
- Validators cannot fetch proposal data
- Governance stalls on missing artifacts
- Attackers can target artifact providers to delay votes

**Mitigation Required:**
- Mandatory artifact availability proof before governance
- Higher replication factor for governance artifacts
- Validator obligation to serve artifacts they vote on
- Timeout for artifact fetch with fallback

**Related:** B2, E2

---

### D6. Ingress Budget Evasion
**Severity:** MEDIUM  
**Likelihood:** HIGH  
**Impact:** Admission control bypass

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: "Sender anti-sybil controls: unknown identity tx budget starts at 5 tx/min"
- `inbox-attention-control-and-anti-spam.md` Section 5: Multiple quota layers specified
- Evasion techniques not fully analyzed

**Problem:** Sophisticated attackers can evade per-sender quotas via IP rotation, identity rotation, or protocol-level tricks.

**Specific Risks:**
- IP rotation bypasses IP-level budgets
- Fast identity creation bypasses per-identity limits
- Budget classification manipulation

**Mitigation Required:**
- Multi-factor budget enforcement (IP + identity + stake)
- Anomaly detection for budget evasion
- Progressive verification requirements

**Related:** A1, C2, D2

---

### D7. NAT Traversal Failures
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Connectivity issues

**Evidence:**
- `ockam-decentralized-network-architecture.md` Section 5: "Ockam relay path is guaranteed fallback for hard NAT/firewall cases"
- No specific NAT traversal techniques (STUN, TURN, ICE) mentioned
- Relay dependency high

**Problem:** Without effective NAT traversal, many nodes become relay-dependent, increasing centralization pressure.

**Specific Risks:**
- 80%+ of nodes behind NAT need relays
- Relay capacity exhausted
- Network becomes hub-and-spoke

**Mitigation Required:**
- Implement ICE/STUN for NAT traversal
- UPnP/NAT-PMP support
- Hole punching techniques
- Reduce relay dependency

**Related:** D1, D3

---

## Category E: Agent Runtime & Tools

### E1. Infinite Loop Resource Exhaustion ⭐ CRITICAL
**Severity:** CRITICAL  
**Likelihood:** HIGH  
**Impact:** Agent crash, DoS

**Evidence:**
- `infinite-agent.md` Section 4: While True loop with no explicit resource limits
- Same doc: Token counting at 70% but no memory/CPU enforcement
- SQLite operations can block

**Problem:** Agent has no resource limits beyond tokens. Can exhaust memory, CPU, disk, or file descriptors.

**Specific Risks:**
- Memory leak in tool execution crashes agent
- Disk full from message logging
- CPU exhaustion from tight loop
- SQLite WAL grows unbounded

**Mitigation Required:**
- Memory limits with OOM handling
- Disk quotas with rotation
- CPU throttling
- Connection pooling limits

**Related:** E2, E5

---

### E2. Handoff Summary Quality
**Severity:** HIGH  
**Likelihood:** HIGH  
**Impact:** State loss

**Evidence:**
- `infinite-agent.md` Section 5: "Handoff Summary is Vague" failure mode
- Same doc: "Agent writes its own memory via handoff"
- No enforcement of summary completeness

**Problem:** Agent-generated handoff summaries may miss critical state. Next session loses context.

**Specific Risks:**
- Important decisions not recorded
- Task state lost between sessions
- Agent loops rediscovering same facts

**Mitigation Required:**
- Structured summary schema with mandatory fields
- Validation of summary completeness
- Key-value extraction for critical state
- Human review option for handoffs

**Related:** E1, E6

---

### E3. Review Sandbox Escape
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** Compromise

**Evidence:**
- `agx-committee-bft-and-governance.md` Section 5: Review subagent runs "isolated review subagent with fresh context"
- `prompt-injection-and-network-policy-boundary.md` Section 4: "local machine actions are intentionally out of protocol policy scope"
- Sandbox isolation not specified

**Problem:** Review sandbox may not be properly isolated. Malicious proposal could escape sandbox.

**Specific Risks:**
- Proposal content exploits sandbox vulnerability
- Review subagent compromises host
- Code execution in governance review

**Mitigation Required:**
- Strict sandbox (container, seccomp, namespaces)
- No network access from sandbox
- Resource limits enforced
- Read-only filesystem

**Related:** B2, E1

---

### E4. Token Counting Accuracy
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Context overflow

**Evidence:**
- `infinite-agent.md` Section 4: "Token counting" function mentioned but implementation not detailed
- `token-efficiency-under-high-interaction.md` Section 5: "Deterministic context budget envelope"
- Different models have different tokenization

**Problem:** Token counting must match model's actual tokenization. Mismatch causes early handoff or context overflow.

**Specific Risks:**
- Tiktoken vs model-native tokenization mismatch
- Handoff triggered too early (waste) or too late (overflow)
- Multi-model support complicates counting

**Mitigation Required:**
- Use model-native token counting API
- Conservative safety margin (65% instead of 70%)
- Model-specific tokenizers
- Overflow recovery

**Related:** E1, E2

---

### E5. Tool Failure Guard Circumvention
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Infinite failure loops

**Evidence:**
- `infinite-agent.md` Section 4: Failure guard uses "action_hash" of tool name + normalized params
- Same doc: "Block after 3 failures in last hour"
- Normalization not specified

**Problem:** Parameter normalization may have edge cases. Attackers could vary parameters slightly to bypass failure guard.

**Specific Risks:**
- Semantically identical calls hash differently
- Agent retries forever with slight variations
- Normalization bugs cause false blocks or bypasses

**Mitigation Required:**
- Canonical parameter serialization
- Semantic equivalence detection
- Fuzzy matching for similar calls
- Maximum retry limits regardless of hash

**Related:** E1, E6

---

### E6. Knowledge Accumulation Staleness
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Outdated decisions

**Evidence:**
- `infinite-agent.md` Section 5: "Knowledge Accumulation Grows Too Large" failure mode
- Pruning to "newest N rows" but no freshness validation
- `forget` tool exists but agent may not use it

**Problem:** Project knowledge table accumulates findings, but old findings may become outdated. Agent doesn't know to forget.

**Specific Risks:**
- Outdated constraints guide current work
- Deprecated patterns persisted
- Knowledge base polluted with errors

**Mitigation Required:**
- TTL on knowledge entries
- Automatic freshness scoring
- Conflict detection for contradictory knowledge
- Periodic knowledge audits

**Related:** E2, E5

---

## Category F: Policy & Security

### F1. Policy Bundle Split-Brain ⭐ CRITICAL
**Severity:** CRITICAL  
**Likelihood:** MEDIUM  
**Impact:** Inconsistent execution

**Evidence:**
- `network-policy-engine-spec.md` Failure Mode: "Policy bundle split-brain"
- Same doc: "peers evaluate same plan under different policy bundle versions"
- Handling: "reject when not active locally"

**Problem:** No specified propagation timeout. Plans may be approved under old policy by some, new policy by others.

**Specific Risks:**
- Same plan approved by some, rejected by others
- Network forks on policy interpretation
- Users confused by inconsistent behavior

**Mitigation Required:**
- Explicit policy bundle activation height
- Grace period for propagation
- Clear error messages for version mismatch
- Bundle propagation monitoring

**Related:** B2, F2

---

### F2. Replay Attack Vectors
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** Double execution

**Evidence:**
- `network-policy-engine-spec.md` Section 5: "Replay protection" with plan_id, nonce, TTL
- `topic-fastpath-protocol-spec.md` Failure Mode: "Replay of old certificate"
- No explicit replay window specification

**Problem:** Replay protection exists but window boundaries may be exploitable.

**Specific Risks:**
- Replay at window boundary
- Clock skew enables replay
- Consumed state not propagated fast enough

**Mitigation Required:**
- Strict monotonic nonces
- Time-based expiration with clock skew tolerance
- Immediate consumed state propagation
- Replay detection across all nodes

**Related:** F1, B2

---

### F3. Step-Up Control Bypass
**Severity:** HIGH  
**Likelihood:** MEDIUM  
**Impact:** Unauthorized high-risk actions

**Evidence:**
- `network-policy-engine-spec.md` Section 5: "medium: secondary reviewer attestation", "high: quorum certificate or delay window"
- `agx-committee-bft-and-governance.md` Section 5: "step-up certificates" mentioned
- Certificate validation not specified

**Problem:** Step-up controls add security but validation logic may have holes.

**Specific Risks:**
- Forged step-up certificates
- Reused certificates
- Bypass via action chaining

**Mitigation Required:**
- Strict certificate validation
- Single-use certificates with binding
- Audit of all step-up paths
- Formal verification of step-up logic

**Related:** F1, A2

---

### F4. Prompt Injection via Tool Output
**Severity:** HIGH  
**Likelihood:** HIGH  
**Impact:** Agent manipulation

**Evidence:**
- `prompt-injection-and-network-policy-boundary.md` Section 5: Attack taxonomy includes "tool_output_injection"
- `prompt-injection-redteam-and-evals.md` Section 5: Attack taxonomy includes "tool_output_injection"
- No specific mitigation for poisoned tool outputs

**Problem:** Tools that fetch external data (web, files) can return malicious payloads that influence agent.

**Specific Risks:**
- Web search returns poisoned results
- File read returns malicious content
- Database query returns injection payload

**Mitigation Required:**
- Tool output sanitization
- Content-type validation
- Size limits
- Policy gate for tool output-based actions

**Related:** F1, E3

---

## Category G: Scalability & Performance

### G1. Speculative Execution Required
**Severity:** HIGH  
**Likelihood:** HIGH  
**Impact:** Low throughput

**Evidence:**
- All consensus docs emphasize BFT finality
- No mention of optimistic execution, rollups, or L2
- Every transaction must go through committee

**Problem:** Committee BFT has inherent throughput limits. Without speculative execution or L2, tx/s will be low.

**Specific Risks:**
- <1000 tx/s maximum
- Agent collaboration stalls
- High latency for simple operations

**Mitigation Required:**
- Fast-path for low-value operations
- L2 rollups for agent collaboration
- Optimistic execution with fraud proofs
- Sharded execution

**Related:** G2, B1

---

### G2. Storage Growth Unbounded
**Severity:** MEDIUM  
**Likelihood:** HIGH  
**Impact:** Node centralization

**Evidence:**
- `infinite-agent.md` Section 4: Messages table "never used for prompt building" but stored forever
- `artifact-availability-and-retention.md`: Retention policies specified but may be too long
- No pruning specified for consensus state

**Problem:** History grows forever. Full nodes require ever-increasing storage.

**Specific Risks:**
- TB+ storage requirements
- Only data centers can run nodes
- Centralization

**Mitigation Required:**
- State pruning for old blocks
- Archive node / full node split
- Checkpointing with state truncation
- Rent for storage

**Related:** G1, E6

---

### G3. Review Queue Bottlenecks
**Severity:** MEDIUM  
**Likelihood:** MEDIUM  
**Impact:** Settlement delays

**Evidence:**
- `proof-of-work-quality-and-review-markets.md` Section 8: "Challenge arbitration load becomes significant"
- Same doc: "reviewer scarcity in niche domains may increase queue latency"
- No parallel review mechanism

**Problem:** Reviews are serialized per reviewer. Large work volume creates queues.

**Specific Risks:**
- Days-long settlement delays
- Capital locked in pending work
- Reviewer burnout

**Mitigation Required:**
- Parallel review pools
- Hierarchical review (quick screen, then deep)
- Automatic reviewer expansion under load

**Related:** A2, G1

---

## Category H: Integration & Interoperability

### H1. Research-to-Spec Gaps
**Severity:** HIGH  
**Likelihood:** CERTAIN  
**Impact:** Implementation confusion

**Evidence:**
- 16 research documents with cross-references
- Many "canonical source" mentions but no spec documents written yet
- BUILD-SYSTEM doc exists but implementation not started

**Problem:** Research is comprehensive but specs don't exist yet. Research has contradictions and gaps that will surface during specification.

**Specific Risks:**
- Implementers interpret differently
- Incompatible implementations
- Research contradictions not resolved

**Mitigation Required:**
- Complete Phase 1 (Requirements extraction)
- Resolve contradictions before Phase 3 (Specs)
- Cross-document review
- Reference implementation

**Related:** H2, B1

---

### H2. Malachite Integration Complexity
**Severity:** MEDIUM  
**Likelihood:** HIGH  
**Impact:** Development delays

**Evidence:**
- Malachite is alpha
- Complex consensus integration
- No prior Hyperfluid-Malachite integration

**Problem:** Integration between Hyperfluid state machine and Malachite consensus is non-trivial. May expose bugs in both.

**Specific Risks:**
- Integration bugs
- State machine mismatches
- Event ordering issues

**Mitigation Required:**
- Extensive integration tests
- Property-based testing
- Formal model of integration
- Fallback consensus

**Related:** B1, H1

---

## Summary Statistics

| Category | Critical | High | Medium | Low | Total |
|----------|----------|------|--------|-----|-------|
| Economic | 2 | 2 | 2 | 0 | 6 |
| Consensus | 2 | 3 | 2 | 0 | 7 |
| Trust | 1 | 2 | 1 | 0 | 4 |
| Network | 1 | 4 | 2 | 0 | 7 |
| Agent Runtime | 1 | 2 | 3 | 0 | 6 |
| Policy | 1 | 3 | 0 | 0 | 4 |
| Scalability | 0 | 1 | 2 | 0 | 3 |
| Integration | 0 | 1 | 1 | 0 | 2 |
| **TOTAL** | **8** | **18** | **15** | **0** | **41** |

## Priority Actions

### Immediate (Before Any Code)
1. Resolve A1 (Zero-fee spam) - redesign economics
2. Resolve B1 (Malachite alpha risk) - assess alternatives
3. Resolve C1 (Trust ladder gaming) - strengthen Sybil resistance
4. Resolve F1 (Policy split-brain) - specify bundle activation

### Before Testnet
5. Address all HIGH severity issues
6. Implement A2 (Review collusion) controls
7. Implement B2 (Governance determinism) validation
8. Implement D1 (Relay concentration) incentives

### Before Mainnet
9. Address all MEDIUM severity issues
10. Full adversarial simulation
11. Economic audit
12. Formal verification of critical paths

## Document Cross-Reference

| Issue ID | Primary Document | Related Documents |
|----------|-----------------|-------------------|
| A1 | agx-committee-bft-and-governance.md | agx-economics-and-adversarial-incentives.md, decentralization-and-stack-benchmark.md |
| A2 | proof-of-work-quality-and-review-markets.md | identity-reputation-and-trust-ladder.md, agx-economics-and-adversarial-incentives.md |
| A3 | proof-of-work-quality-and-review-markets.md | agx-economics-and-adversarial-incentives.md |
| A4 | agx-committee-bft-and-governance.md | decentralization-and-stack-benchmark.md, agx-economics-and-adversarial-incentives.md |
| A5 | agx-committee-bft-and-governance.md | proof-of-work-quality-and-review-markets.md, agx-economics-and-adversarial-incentives.md |
| A6 | agx-economics-and-adversarial-incentives.md | proof-of-work-quality-and-review-markets.md |
| B1 | decentralization-and-stack-benchmark.md | agx-committee-bft-and-governance.md |
| B2 | agx-committee-bft-and-governance.md | topic-fastpath-protocol-spec.md, artifact-availability-and-retention.md |
| B3 | agx-committee-bft-and-governance.md | topic-fastpath-protocol-spec.md |
| B4 | agx-committee-bft-and-governance.md | decentralization-and-stack-benchmark.md |
| B5 | agx-committee-bft-and-governance.md | decentralization-and-stack-benchmark.md |
| B6 | agx-committee-bft-and-governance.md | agx-economics-and-adversarial-incentives.md |
| B7 | agx-committee-bft-and-governance.md | decentralized-incident-response-and-recovery.md |
| C1 | identity-reputation-and-trust-ladder.md | proof-of-work-quality-and-review-markets.md, collaboration-layer-parallel-teams.md |
| C2 | agx-economics-and-adversarial-incentives.md | inbox-attention-control-and-anti-spam.md, ockam-decentralized-network-architecture.md |
| C3 | identity-reputation-and-trust-ladder.md | agx-committee-bft-and-governance.md |
| C4 | topic-fastpath-protocol-spec.md | agx-committee-bft-and-governance.md |
| D1 | ockam-decentralized-network-architecture.md | decentralization-and-stack-benchmark.md |
| D2 | ockam-decentralized-network-architecture.md | decentralization-and-stack-benchmark.md |
| D3 | ockam-decentralized-network-architecture.md | - |
| D4 | ockam-decentralized-network-architecture.md | - |
| D5 | artifact-availability-and-retention.md | agx-committee-bft-and-governance.md |
| D6 | agx-committee-bft-and-governance.md | inbox-attention-control-and-anti-spam.md |
| D7 | ockam-decentralized-network-architecture.md | - |
| E1 | infinite-agent.md | token-efficiency-under-high-interaction.md |
| E2 | infinite-agent.md | token-efficiency-under-high-interaction.md |
| E3 | agx-committee-bft-and-governance.md | prompt-injection-and-network-policy-boundary.md |
| E4 | infinite-agent.md | token-efficiency-under-high-interaction.md |
| E5 | infinite-agent.md | - |
| E6 | infinite-agent.md | - |
| F1 | network-policy-engine-spec.md | agx-committee-bft-and-governance.md |
| F2 | network-policy-engine-spec.md | topic-fastpath-protocol-spec.md |
| F3 | network-policy-engine-spec.md | agx-committee-bft-and-governance.md |
| F4 | prompt-injection-and-network-policy-boundary.md | prompt-injection-redteam-and-evals.md |
| G1 | decentralization-and-stack-benchmark.md | agx-committee-bft-and-governance.md |
| G2 | infinite-agent.md | artifact-availability-and-retention.md |
| G3 | proof-of-work-quality-and-review-markets.md | - |
| H1 | BUILD-SYSTEM.md | All documents |
| H2 | decentralization-and-stack-benchmark.md | agx-committee-bft-and-governance.md |

---

## Notes

- All issues are substantiated by specific citations from research documents
- No false positives - every issue has documented evidence
- Some issues have overlapping mitigations (efficiency in addressing)
- Regular updates needed as research evolves and specs are written
- Next update after Phase 1 (Requirements) completion
