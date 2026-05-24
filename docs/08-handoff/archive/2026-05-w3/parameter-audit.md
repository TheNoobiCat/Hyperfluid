# Parameter Audit — Stage 01 Week 7-8

**Date:** 2026-05-14
**Stage:** 01 (Protocol Core) — Week 7-8 Integration & Polish
**Purpose:** Record all [TUNE] and spec-defined parameters with their current default values. Calibration target for Stage 03 (Validation).

---

## Fee Market Parameters (C5 — `hyperfluid-fee-market`)

| Parameter | Default | Code Location | Spec | Status |
|-----------|---------|---------------|------|--------|
| `min_base_fee` | 1,000,000 atto-AGX | `FeeConfig::default()` | fee-market-spec.md 1.3 | [TUNE] |
| `target_utilization_pct` | 50% | `FeeConfig::default()` | fee-market-spec.md 1.3 | spec-default |
| `max_adjustment_per_mil` | 125 (12.5%) | `FeeConfig::default()` | fee-market-spec.md 1.3 | spec-default |
| `fee_adjustment_denominator` | 8 | `compute_next_base_fee()` arg | fee-market-spec.md 1.4 | [TUNE] |
| `max_per_sender_tx` | 100 | `FeeConfig::default()` | fee-market-spec.md 1.3 | spec-default |

## P2P Networking Parameters (C7 — `hyperfluid-p2p`)

| Parameter | Default | Code Location | Spec | Status |
|-----------|---------|---------------|------|--------|
| `dht_k` | 20 | `DiscoveryConfig::default()` | p2p-wire-spec.md 1.3 | spec-default |
| `dht_refresh_secs` | 1800 (30 min) | `DiscoveryConfig::default()` | p2p-wire-spec.md 1.4 | spec-default |
| `gossip_fanout` | 8 | `GossipMessage::MAX_FANOUT` | p2p-wire-spec.md 1.3 | spec-max |
| `gossip_ttl` | 16 | `GossipMessage::MAX_TTL` | p2p-wire-spec.md 1.3 | spec-max |
| `upgrade_probe_secs` | 60 | `DiscoveryConfig::default()` | p2p-wire-spec.md 1.2 | spec-default |
| `upgrade_probe_jitter_pct` | 20 | `DiscoveryConfig::default()` | p2p-wire-spec.md 1.2 | spec-default |
| `direct_retry_attempts` | 3 | `DiscoveryConfig::default()` | p2p-wire-spec.md 1.5 | spec-default |
| `direct_retry_timeout_secs` | 5 | `DiscoveryConfig::default()` | p2p-wire-spec.md 1.5 | spec-default |
| `min_bootstrap_cache` | 5 | `DiscoveryConfig::default()` | p2p-wire-spec.md 1.5 | spec-default |

## Mempool Parameters (C7 — `hyperfluid-p2p::mempool`)

| Parameter | Default | Code Location | Spec | Status |
|-----------|---------|---------------|------|--------|
| `max_total_tx` | 10,000 | `MempoolConfig::default()` | p2p-wire-spec.md 2.3 | spec-default |
| `per_sender_tx_limit` | 100 | `MempoolConfig::default()` | p2p-wire-spec.md 2.3 | spec-default |
| `evidence_fee_discount_pct` | 50% | `MempoolConfig::default()` | p2p-wire-spec.md 2.3 | governance-adjustable |
| `governance_fee_discount_pct` | 50% | `MempoolConfig::default()` | p2p-wire-spec.md 2.3 | governance-adjustable |

## Consensus Parameters (C1 — `hyperfluid-consensus`)

| Parameter | Default | Code Location | Spec | Status |
|-----------|---------|---------------|------|--------|
| `COMMITTEE_SIZE` | 100 | `Committee::COMMITTEE_SIZE` | consensus-spec.md 1.3 | spec-constant |
| `NORMAL_THRESHOLD` | 67 | `Committee::NORMAL_THRESHOLD` | consensus-spec.md 1.2 | spec-constant |
| `DEGRADED_THRESHOLD` | 50 | `Committee::DEGRADED_THRESHOLD` | consensus-spec.md 1.2 | spec-constant |
| `EMERGENCY_IDLE_BLOCKS` | 500 | `Committee::EMERGENCY_IDLE_BLOCKS` | consensus-spec.md 1.2 | spec-constant |
| `MAX_OVERLAP_PCT` | 20% | `Committee::sample_with_rotation()` | consensus-spec.md 1.4 | spec-constant |
| `TWO_EPOCH_RECENCY` | 2 epochs | `Committee::sample_with_rotation()` | consensus-spec.md 1.4 | spec-constant |
| VDF fallback scheme | SHA3-256 | `Committee::vdf_fallback_seed()` | consensus-spec.md 1.3 | spec-default |

## Staking Parameters (C3 — `hyperfluid-staking`)

| Parameter | Default | Code Location | Spec | Status |
|-----------|---------|---------------|------|--------|
| Validator unbonding period | 14 days (epochs) | `staking-spec.md` 1.2 | staking-spec.md | spec-constant |
| Delegation unbonding delay | 7 days (60480 blocks) | `StateMachine` execution | staking-spec.md 1.3 | spec-constant |
| Max commission rate | 20% | `max_commission_rate` arg | staking-spec.md 1.3 | spec-constant |
| Min delegation | 1 AGX (10^18 atto-AGX) | `min_delegation` arg | staking-spec.md 1.3 | spec-constant |
| Stake-graph hop limit | 3 hops | `stake-graph-analysis-spec.md` | stake-graph-analysis-spec.md | spec-constant |

## Artifact Storage Parameters (C8 — `hyperfluid-artifact`)

| Parameter | Default | Code Location | Spec | Status |
|-----------|---------|---------------|------|--------|
| GovernanceBundle min replicas | 5 | `ArtifactClass::default_min_replicas()` | artifact-availability-spec.md 1.4 | spec-constant |
| ReviewEvidence min replicas | 3 | `ArtifactClass::default_min_replicas()` | artifact-availability-spec.md 1.4 | spec-constant |
| ResearchOutput min replicas | 2 | `ArtifactClass::default_min_replicas()` | artifact-availability-spec.md 1.4 | spec-constant |
| TelemetryArchive min replicas | 2 | `ArtifactClass::default_min_replicas()` | artifact-availability-spec.md 1.4 | spec-constant |
| Pinned retention tier | Never expires | `ArtifactManifest::is_expired()` | artifact-availability-spec.md 1.4 | spec-constant |
| MediumTerm retention | 90 days | artifact-availability-spec.md 1.4 | artifact-availability-spec.md | spec-constant |
| ShortTerm retention | 30 days | artifact-availability-spec.md 1.4 | artifact-availability-spec.md | spec-constant |

## State Machine Parameters (C2 — `hyperfluid-state`)

| Parameter | Default | Code Location | Spec | Status |
|-----------|---------|---------------|------|--------|
| Block time (implied) | 10s | consensus-spec.md | consensus-spec.md | spec-default |
| Epoch length | Testnet config dependent | `GenesisConfig` | consensus-spec.md | configurable |
| AGX precision | 10^18 atto-AGX | consensus-spec.md | consensus-spec.md | spec-constant |
| Total supply | 10M AGX = 10^25 atto-AGX | `GenesisConfig` | consensus-spec.md | spec-constant |

---

## Summary

- **Total parameters:** 41
- **[TUNE] marked:** 2 (`min_base_fee`, `fee_adjustment_denominator`)
- **Spec-constants:** 23
- **Governance-adjustable:** 2 (evidence/governance fee discounts)
- **Configurable:** 2 (epoch length, block time)

All defaults match spec-defined values. No parameter divergence detected between spec and code.
