// === C3 Staking & Validator Manager: Stake-Graph Analysis ===
//
// Source: specs/protocol/stake-graph-analysis-spec.md Section 1
// Detects correlated validator keys via on-chain funding trace.
// Clusters are treated as a single entity for committee weight computation.

use std::collections::{HashMap, HashSet};

use crate::{Hash32, ValidatorRecord};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClusterAncestorType {
    AirdropAgent,
    DirectFunding,
    IndirectFunding,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FundingEdge {
    pub from_account: Hash32,
    pub to_account: Hash32,
    pub amount: u128,
    pub height: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterRecord {
    pub cluster_id: Hash32,
    pub members: Vec<Hash32>,
    pub ancestor_root: Hash32,
    pub total_bonded_stake: u128,
    pub cluster_size_diversity_bonus: u8,
    pub detected_at_epoch: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterDetectionResult {
    pub epoch: u64,
    pub clusters: Vec<ClusterRecord>,
    pub unclustered_validators: Vec<Hash32>,
}

pub struct StakeGraph {
    edges: Vec<FundingEdge>,
    adj_out: HashMap<Hash32, Vec<usize>>,
    adj_in: HashMap<Hash32, Vec<usize>>,
}

impl StakeGraph {
    fn new() -> Self {
        Self { edges: Vec::new(), adj_out: HashMap::new(), adj_in: HashMap::new() }
    }

    fn add_edge(&mut self, edge: FundingEdge) {
        let idx = self.edges.len();
        self.adj_out.entry(edge.from_account).or_default().push(idx);
        self.adj_in.entry(edge.to_account).or_default().push(idx);
        self.edges.push(edge);
    }

    fn get_ancestors(&self, account: &Hash32, max_hops: usize) -> Vec<Hash32> {
        let mut visited = HashSet::new();
        let mut ancestors = Vec::new();
        self.trace_backwards(account, max_hops, &mut visited, &mut ancestors);
        ancestors
    }

    fn trace_backwards(
        &self,
        account: &Hash32,
        hops_remaining: usize,
        visited: &mut HashSet<Hash32>,
        ancestors: &mut Vec<Hash32>,
    ) {
        if hops_remaining == 0 || visited.contains(account) {
            return;
        }
        visited.insert(*account);
        if let Some(in_edges) = self.adj_in.get(account) {
            for &edge_idx in in_edges {
                let edge = &self.edges[edge_idx];
                if !visited.contains(&edge.from_account) {
                    ancestors.push(edge.from_account);
                }
                self.trace_backwards(&edge.from_account, hops_remaining - 1, visited, ancestors);
            }
        }
    }
}

pub fn build_stake_funding_graph(edges: Vec<FundingEdge>) -> StakeGraph {
    let mut graph = StakeGraph::new();
    for edge in edges {
        graph.add_edge(edge);
    }
    graph
}

pub fn detect_clusters(
    validators: &[ValidatorRecord],
    edges: &[FundingEdge],
    max_hops: usize,
    airdrop_agent_id: &Hash32,
    epoch: u64,
) -> ClusterDetectionResult {
    let graph = build_stake_funding_graph(edges.to_vec());

    let mut clusters: Vec<ClusterRecord> = Vec::new();
    let mut clustered = HashSet::new();

    let validator_ids: Vec<Hash32> = validators.iter().map(|v| v.validator_id).collect();

    for i in 0..validator_ids.len() {
        if clustered.contains(&validator_ids[i]) {
            continue;
        }

        let ancestors_i = graph.get_ancestors(&validator_ids[i], max_hops);
        let mut group = vec![validator_ids[i]];

        for (_j, vj_id) in validator_ids.iter().enumerate().skip(i + 1) {
            if clustered.contains(vj_id) {
                continue;
            }
            let ancestors_j = graph.get_ancestors(vj_id, max_hops);
            let common: Vec<&Hash32> =
                ancestors_i.iter().filter(|a| ancestors_j.contains(a)).collect();

            let has_non_airdrop_common = common.iter().any(|a| *a != airdrop_agent_id);
            if has_non_airdrop_common {
                group.push(*vj_id);
            }
        }

        if group.len() > 1 {
            let mut sorted_members = group.clone();
            sorted_members.sort();

            let concat: Vec<u8> = sorted_members.iter().flat_map(|m| m.to_vec()).collect();
            let cluster_id = crate::sha3_256(&concat);
            let ancestor_root = crate::sha3_256(&sorted_members[0]);

            let total_bonded_stake: u128 = validators
                .iter()
                .filter(|v| sorted_members.contains(&v.validator_id))
                .map(|v| v.bonded_stake)
                .sum();

            let diversity = usize::min(5, group.len()) as u8;

            clusters.push(ClusterRecord {
                cluster_id,
                members: sorted_members,
                ancestor_root,
                total_bonded_stake,
                cluster_size_diversity_bonus: diversity,
                detected_at_epoch: epoch,
            });

            for member in &group {
                clustered.insert(*member);
            }
        }
    }

    let unclustered_validators: Vec<Hash32> =
        validator_ids.iter().filter(|v| !clustered.contains(*v)).copied().collect();

    ClusterDetectionResult { epoch, clusters, unclustered_validators }
}

pub fn compute_committee_weights(
    validators: &[ValidatorRecord],
    clusters: &[ClusterRecord],
) -> HashMap<Hash32, u128> {
    let mut weights: HashMap<Hash32, u128> = HashMap::new();

    for c in clusters {
        let weight_per_member = if !c.members.is_empty() {
            c.total_bonded_stake / (c.members.len() as u128)
        } else {
            0
        };
        for member in &c.members {
            weights.insert(*member, weight_per_member);
        }
    }

    for v in validators {
        weights.entry(v.validator_id).or_insert(v.bonded_stake);
    }

    weights
}

pub fn prune_old_edges(edges: &mut Vec<FundingEdge>, threshold_height: u64) {
    edges.retain(|e| e.height >= threshold_height);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ValidatorRecord;
    use crate::ValidatorState;

    fn make_validator(id: u8, stake: u128) -> ValidatorRecord {
        ValidatorRecord {
            validator_id: [id; 32],
            state: ValidatorState::Active,
            bonded_stake: stake,
            self_bond: stake,
            total_delegated: 0,
            commission_rate: 0,
            bonding_height: 0,
            unbonding_height: 0,
            jail_until_height: 0,
            liveness_bitmap: Vec::new(),
            slash_count: 0,
            missed_blocks: 0,
            last_renew_height: 0,
        }
    }

    fn airdrop_id() -> Hash32 {
        [0xFFu8; 32]
    }

    #[test]
    fn cluster_detection_same_funder_within_hops() {
        let funder = [0xAAu8; 32];
        let v1 = make_validator(1, 1000);
        let v2 = make_validator(2, 1000);

        let edges = vec![
            FundingEdge { from_account: funder, to_account: [1u8; 32], amount: 1000, height: 0 },
            FundingEdge { from_account: funder, to_account: [2u8; 32], amount: 1000, height: 0 },
        ];

        let result = detect_clusters(&[v1, v2], &edges, 3, &airdrop_id(), 42);
        assert_eq!(result.clusters.len(), 1);
        assert_eq!(result.clusters[0].members.len(), 2);
    }

    #[test]
    fn no_cluster_for_independent_funding() {
        let funder_a = [0xAAu8; 32];
        let funder_b = [0xBBu8; 32];
        let v1 = make_validator(1, 1000);
        let v2 = make_validator(2, 1000);

        let edges = vec![
            FundingEdge { from_account: funder_a, to_account: [1u8; 32], amount: 1000, height: 0 },
            FundingEdge { from_account: funder_b, to_account: [2u8; 32], amount: 1000, height: 0 },
        ];

        let result = detect_clusters(&[v1, v2], &edges, 3, &airdrop_id(), 42);
        assert_eq!(result.clusters.len(), 0);
    }

    #[test]
    fn hop_limit_3_excludes_4_hop_chain() {
        let origin = [0xCCu8; 32];
        let a = [0xDDu8; 32];
        let b = [0xEEu8; 32];
        let c = [0x11u8; 32];
        let v1 = make_validator(1, 1000);
        let v2 = make_validator(2, 1000);

        let edges = vec![
            FundingEdge { from_account: origin, to_account: a, amount: 1000, height: 0 },
            FundingEdge { from_account: a, to_account: b, amount: 1000, height: 1 },
            FundingEdge { from_account: b, to_account: c, amount: 1000, height: 2 },
            FundingEdge { from_account: c, to_account: [1u8; 32], amount: 1000, height: 3 },
            // 4 hops: origin → a → b → c → v1
            FundingEdge { from_account: origin, to_account: [2u8; 32], amount: 1000, height: 0 },
            // 1 hop: origin → v2
        ];

        let result = detect_clusters(&[v1, v2], &edges, 3, &airdrop_id(), 42);
        assert_eq!(result.clusters.len(), 0, "4-hop chain should not cluster with origin");
    }

    #[test]
    fn airdrop_agent_is_not_clustering() {
        let v1 = make_validator(1, 1000);
        let v2 = make_validator(2, 1000);

        let edges = vec![
            FundingEdge {
                from_account: airdrop_id(),
                to_account: [1u8; 32],
                amount: 100,
                height: 0,
            },
            FundingEdge {
                from_account: airdrop_id(),
                to_account: [2u8; 32],
                amount: 100,
                height: 0,
            },
        ];

        let result = detect_clusters(&[v1, v2], &edges, 3, &airdrop_id(), 42);
        assert_eq!(result.clusters.len(), 0, "airdrop agent should not cause clustering");
        assert_eq!(result.unclustered_validators.len(), 2);
    }

    #[test]
    fn cluster_detection_deterministic() {
        let funder = [0xAAu8; 32];
        let v1 = make_validator(1, 1000);
        let v2 = make_validator(2, 1000);

        let edges = vec![
            FundingEdge { from_account: funder, to_account: [1u8; 32], amount: 1000, height: 0 },
            FundingEdge { from_account: funder, to_account: [2u8; 32], amount: 1000, height: 0 },
        ];

        let r1 = detect_clusters(&[v1.clone(), v2.clone()], &edges, 3, &airdrop_id(), 42);
        let r2 = detect_clusters(&[v1, v2], &edges, 3, &airdrop_id(), 42);
        assert_eq!(r1.clusters.len(), r2.clusters.len());
        assert_eq!(r1.clusters[0].cluster_id, r2.clusters[0].cluster_id);
    }

    #[test]
    fn compute_weights_splits_cluster_stake() {
        let funder = [0xAAu8; 32];
        let v1 = make_validator(1, 1000);
        let v2 = make_validator(2, 2000);

        let edges = vec![
            FundingEdge { from_account: funder, to_account: [1u8; 32], amount: 1000, height: 0 },
            FundingEdge { from_account: funder, to_account: [2u8; 32], amount: 2000, height: 0 },
        ];

        let result = detect_clusters(&[v1.clone(), v2.clone()], &edges, 3, &airdrop_id(), 42);
        let weights = compute_committee_weights(&[v1, v2], &result.clusters);

        assert!(weights.contains_key(&[1u8; 32]));
        let v1w = weights[&[1u8; 32]];
        let v2w = weights[&[2u8; 32]];
        assert_eq!(v1w, v2w, "cluster stake should be evenly split");
        assert_eq!(v1w + v2w, 3000);
    }

    #[test]
    fn prune_edges_removes_old() {
        let mut edges = vec![
            FundingEdge { from_account: [1u8; 32], to_account: [2u8; 32], amount: 100, height: 50 },
            FundingEdge {
                from_account: [2u8; 32],
                to_account: [3u8; 32],
                amount: 200,
                height: 150,
            },
            FundingEdge {
                from_account: [3u8; 32],
                to_account: [4u8; 32],
                amount: 300,
                height: 250,
            },
        ];
        prune_old_edges(&mut edges, 100);
        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|e| e.height >= 100));
    }
}
