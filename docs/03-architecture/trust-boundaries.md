# Trust Boundaries

## 1. Executive Summary

Hyperfluid has a three-zone security model: the deterministic protocol core, the policy-gated security boundary, and the untrusted agent runtime. Every network-mutating action must cross from the untrusted zone through the deterministic policy gate before entering protocol state. This document defines each zone, the boundaries between them, and what any component can and cannot do.

## 2. Security Zone Diagram

```mermaid
flowchart TD
    subgraph ZONE3[Zone 3: Local-Only - Untrusted]
        AGENT_RT[Agent Runtime]
        LOCAL_TOOLS[Local Tools bash/todo/remember/forget]
        SANDBOX[Review Sandbox]
    end

    subgraph ZONE2[Zone 2: Policy-Gated - Semi-Trusted]
        PDP[Policy Decision Point]
        AUDIT_LOG[Audit Log]
        QUOTA_TRACK[Quota Tracker]
        TAINT_TRACK[Taint Tracker]
    end

    subgraph ZONE1[Zone 1: Protocol-Enforced - Trusted]
        CONSENSUS[Consensus Engine]
        STATE_MACHINE[State Machine / SMT]
        STAKING[Staking Manager]
        GOVERNANCE[Governance Engine]
        FEE_MARKET[Fee Market]
        FAST_PATH[Fast-Path Topics]
        P2P[P2P Networking]
        ARTIFACTS[Artifact Availability]
        ECONOMICS[Economics Controller]
    end

    AGENT_RT -->|action_plan + signature| PDP
    LOCAL_TOOLS -.->|local execution only| AGENT_RT
    SANDBOX -.->|isolated from main branch| AGENT_RT

    PDP -->|typed transaction| CONSENSUS
    PDP -->|policy decision| AUDIT_LOG
    PDP -.->|quota query| QUOTA_TRACK
    (taint tracking removed — runtime-local only)

    CONSENSUS --> STATE_MACHINE
    STATE_MACHINE --> STAKING
    STATE_MACHINE --> GOVERNANCE
    STATE_MACHINE --> FEE_MARKET
    STATE_MACHINE --> ARTIFACTS
    STATE_MACHINE --> ECONOMICS
```

## 3. Zone Definitions

### Zone 1: Protocol-Enforced (Trusted)

**Trust Assumption:** Components in Zone 1 are trusted to operate correctly because their behavior is fully deterministic and replicated across all honest nodes. Any deviation is detectable via state root mismatch.

**Components:** C1-C8, C12 + C2

**Access Rights:**
- Read/write to SMT protocol state
- Validate and include transactions
- Produce blocks and commit state transitions
- Manage economic parameters (via governance only)
- Sign blocks and consensus messages

**Constraints:**
- Cannot call external services (no oracles)
- Cannot contain probabilistic logic (no ML models in consensus path)
- Must converge to identical state given identical inputs
- All state writes flow through C2 State Machine

**Failure Mode:** A compromised Zone 1 component could fork consensus. Mitigation: BFT safety requires >33% Byzantine validators to break safety (NFR-0016). Individual node compromise cannot affect other nodes.

---

### Zone 2: Policy-Gated (Semi-Trusted)

**Trust Assumption:** The PDP runs locally on each node as part of the node process but is logically a gatekeeper. Its operation is deterministic and identical across all nodes when evaluating the same action plan against the same policy bundle and state.

**Components:** C9 (Policy Decision Point)

**Access Rights:**
- Read protocol state for policy evaluation (quota counters, trust stages, circuit-breaker mode)
- Validate and reject action plans
- Submit approved transactions to Zone 1
- Write to audit log (append-only, content-addressed)

**Constraints:**
- Cannot modify protocol state directly (only through Zone 1 submission)
- Cannot skip or reorder policy checks
- Must produce identical decisions for identical inputs on all nodes
- No probabilistic classifiers in root authorization path
- Classifier signals may only tighten quotas or trigger quarantine

**Failure Mode:** A buggy PDP could deny all actions (overblocking) or approve malicious actions (underblocking). Overblocking degrades collaboration but preserves safety. Underblocking is caught by challenge windows, slashing, and reviewer attestation. Split-brain prevention via policy bundle hash inclusion in every action plan (FR-0119).

---

### Zone 3: Local-Only (Untrusted)

**Trust Assumption:** Components in Zone 3 are untrusted by design. They execute arbitrary LLM output, run in sandboxed processes, and have no direct access to protocol state. All network mutations must pass through Zone 2.

**Components:** C10 (Agent Runtime), C11 + associated Review Sandbox instances

**Access Rights:**
- Local machine operations via `bash` tool (cgroup/resource limited)
- Read-only access to node API for queries
- Submit action plans to Zone 2 (PDP)
- Manage local SQLite state (todos, knowledge, handoffs, messages)
- Load skills from local disk

**Constraints:**
- Cannot write to protocol state directly
- Cannot submit raw consensus messages
- Cannot manage peer connections or networking
- Local `bash` commands are not subject to network policy gate (FR-0136)
- Network sockets from sandbox are mediated by node API
- Runtime process has no write access to node database (NFR-0028)

**Failure Mode:** A compromised agent runtime can submit malicious action plans, flood with spam, or attempt injection. All plans go through PDP (Zone 2). Quota limits, reputation decay, and abuse evidence prevent sustained attacks. Agent crash does not affect consensus.

---

## 4. Cross-Zone Interactions

### Zone 3 → Zone 2: Action Plan Submission

The only path from untrusted to policy-gated.

| Field | Verified By | Failure Mode |
|-------|-------------|--------------|
| plan_id uniqueness | PDP | Replay detected |
| agent_signature | PDP | Signature invalid |
| action_type validity | PDP | Schema violation |
| risk_class step-up | PDP | Step-up required |
| policy_bundle_hash match | PDP | Bundle mismatch |
| nonce monotonicity | PDP | Replay detected |
| TTL validity | PDP | TTL expired |
| quota availability | PDP | Quota exhausted |
| (removed — taint and plan binding are runtime-local concerns) | | |

### Zone 2 → Zone 1: Transaction Submission

Approved plans become transactions. Zone 1 validates:
- Gateway signature (PDP endorsement)
- Transaction type consistency with plan action_type
- Account nonce and balance
- Validator state (for staking/governance operations)

### Zone 1 → Zone 3: Query Responses

Agent runtime queries node API for state information. Responses are read-only and include `cache-control` headers. Query results are deterministic (same height, same input = same output).

## 5. In-Protocol vs Local-Only State

### Protocol-Enforced State (on-chain, in SMT)
- Account balances and nonces
- Validator states, stakes, slash records
- Governance proposals and votes
- Committee roster
- `git:head` pointer
- System parameters
- Circuit-breaker mode
- Task board (on-chain task registry)
- Artifact manifests and replication leases
- Action plan records (approved/denied/consumed)
- Trust stages and reputation vectors
- Airdrop pool state

### Local-Only State (not in SMT, per-node/per-agent)
- Agent SQLite database (todos, knowledge, handoffs, messages) — local to agent runtime
- System prompt — local to agent runtime
- Tool definitions and skill cache — local to agent runtime
- Peer routing table and DHT — local to node (informed by gossip, not consensus)
- Mempool contents — local to node (transactions not yet committed)
- Policy bundle binaries — local to node (hash is on-chain)
- Token burn telemetry — local diagnostics (not protocol economics)
- Network topology (connections, latencies) — local to node

## 6. Sandboxed vs Unsandboxed Execution

### Sandboxed Execution (Review Sandbox)
- **Scope:** Governance proposal review (C4), topic merge review (C6)
- **Isolation:** Fresh agent context, no access to main agent state
- **Tools:** Single `review(approve|deny, reason)` tool only
- **Timeout:** 30 minutes
- **Crash behavior:** No vote (not penalized)
- **Resources:** Process-level resource limits (CPU, RAM, disk)
- **Process:** Separate subprocess, seccomp/named-space isolation, restricted filesystem access (FR-0137)

### Unsandboxed Execution (Agent Runtime)
- **Scope:** Normal agent operation (C10)
- **Isolation:** Separate process from node (FR-0138)
- **Tools:** Full tool set (bash, todo, remember, forget) + network mutations via `hyperfluid` CLI
- **Resources:** cgroup limits (4GB RAM, 2 CPU cores, 10GB disk, 1024 FDs)
- **Network:** Local `bash` commands are network-scoped to operator machine only
- **Crash behavior:** WAL recovery from SQLite, resume from last handoff

## 7. Trust Assumptions Inventory

| Assumption | Component | Justification | Risk |
|-----------|-----------|---------------|------|
| Honest supermajority of committee | C1 | BFT safety property (NFR-0016) | Acceptable - committee rotation, slashing deter collusion |
| Deterministic state transitions | C2 | Required for convergence (NFR-0019) | Acceptable - governance execution hermeticity enforced |
| PDP operates correctly on all nodes | C9 | Policy decisions must be identical | Acceptable - deterministic rule chain, bundle hash binding |
| Agent keys not compromised en masse | C10 | Key rotation with grace window (FR-0118) | Acceptable - ML-DSA post-quantum, rotation supported |
| Economic parameters within bounds | C12 | Bounds enforced by protocol (FR-0155) | Acceptable - changes require governance proposal |
| Artifact replicas maintain minimum count | C8 | Proof-of-possession challenges (FR-0052) | Acceptable - SLA with repair coordinator |
| No oracle or external service required | C2 | All data is on-chain or content-addressed | Acceptable - no external trust dependency |

## 8. Decentralisation Audit (Checklist)

Per `BUILD-SYSTEM.md` decentralisation audit gate:

1. **External trust inventory:** PASS. Zero external oracles or mandatory centralized services. All data is on-chain or content-addressed.

2. **Centralized coordination:** PASS. No single dispatcher, scheduler, or admin role. All coordination is protocol-enforced (BFT consensus, task board soft leases, governance voting).

3. **Verifiable economic signals:** PASS. All rewards and penalties reference cryptographically verifiable on-chain records. Self-reported local metrics are excluded from reward calculations.

4. **Single points of failure:** PASS. No component whose failure stalls the entire system. Committee overlap and relay diversity provide redundancy. Circuit-breakers handle degradation gracefully.

5. **Sybil resistance:** PASS. Anti-Sybil relies on three-layered defense: SHA3-256 HashCash proof-of-agent with dynamic difficulty (FR-0176), progressive 20 AGX bond with work-gated release (FR-0157), and continuous behavioral correlation detection with automated adjudication (FR-0191). No IP-based limits.
