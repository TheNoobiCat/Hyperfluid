use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

/// Mempool configuration. Source: p2p-wire-spec.md Section 2.3
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MempoolConfig {
    pub max_total_tx: u64,
    pub per_sender_tx_limit: u32,
    pub evidence_fee_discount_pct: u8,
    pub governance_fee_discount_pct: u8,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_total_tx: 10_000,
            per_sender_tx_limit: 100,
            evidence_fee_discount_pct: 50,
            governance_fee_discount_pct: 50,
        }
    }
}

/// Transaction type tag for fee discount determination.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TxTypeTag {
    Standard,
    Evidence,
    Governance,
}

/// A pending transaction in the mempool, ordered by fee.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MempoolTx {
    /// SHA3-256 of transaction bytes.
    pub tx_hash: [u8; 32],
    /// Sender account identifier.
    pub sender_id: [u8; 32],
    /// Transaction type for fee discount determination.
    pub tx_type: TxTypeTag,
    /// Priority fee offered by sender.
    pub priority_fee: u128,
    /// Base fee at time of admission.
    pub base_fee: u128,
    /// Maximum total fee the sender is willing to pay.
    pub max_fee_per_tx: u128,
    /// Serialized transaction payload (opaque bytes for the driver to decode).
    pub tx_data: Vec<u8>,
}

impl MempoolTx {
    /// Effective fee considering type-based discounts.
    /// Source: p2p-wire-spec.md Section 2.4.
    pub fn effective_fee(&self, config: &MempoolConfig) -> u128 {
        let discount = match self.tx_type {
            TxTypeTag::Evidence => config.evidence_fee_discount_pct,
            TxTypeTag::Governance => config.governance_fee_discount_pct,
            TxTypeTag::Standard => 0,
        };
        let effective_base = if discount > 0 {
            self.base_fee.saturating_mul((100 - discount) as u128) / 100
        } else {
            self.base_fee
        };
        effective_base.saturating_add(self.priority_fee)
    }

    /// Total cost: base_fee + priority_fee (undiscounted).
    pub fn total_cost(&self) -> u128 {
        self.base_fee.saturating_add(self.priority_fee)
    }
}

/// Ordering wrapper for BinaryHeap: highest effective fee first.
#[derive(Debug, Clone)]
struct FeeOrdered(MempoolTx, u128);

impl PartialEq for FeeOrdered {
    fn eq(&self, other: &Self) -> bool {
        self.1 == other.1
    }
}

impl Eq for FeeOrdered {}

impl PartialOrd for FeeOrdered {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FeeOrdered {
    fn cmp(&self, other: &Self) -> Ordering {
        self.1.cmp(&other.1)
    }
}

/// Single fee-ordered mempool. Source: p2p-wire-spec.md Section 2.
pub struct Mempool {
    config: MempoolConfig,
    /// Heap ordered by effective fee (lowest at top for fast eviction, then reversed for selection).
    txs: BinaryHeap<FeeOrdered>,
    /// Per-sender transaction count.
    per_sender: BTreeMap<[u8; 32], u32>,
    /// Set of tx hashes for duplicate detection.
    tx_hashes: BTreeSet<[u8; 32]>,
}

impl Mempool {
    /// Create a new mempool with the given configuration.
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            config,
            txs: BinaryHeap::new(),
            per_sender: BTreeMap::new(),
            tx_hashes: BTreeSet::new(),
        }
    }

    /// Number of transactions currently in the mempool.
    pub fn len(&self) -> usize {
        self.txs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }

    /// Attempt to insert a transaction into the mempool.
    /// Returns true if the transaction was inserted.
    ///
    /// Source: p2p-wire-spec.md Section 2.4 — Mempool admission flow.
    pub fn insert(&mut self, tx: MempoolTx) -> bool {
        if self.tx_hashes.contains(&tx.tx_hash) {
            return false;
        }

        let sender_count = self.per_sender.get(&tx.sender_id).copied().unwrap_or(0);
        if sender_count >= self.config.per_sender_tx_limit {
            return false;
        }

        let effective = tx.effective_fee(&self.config);

        self.tx_hashes.insert(tx.tx_hash);
        *self.per_sender.entry(tx.sender_id).or_insert(0) += 1;
        self.txs.push(FeeOrdered(tx, effective));

        if self.len() as u64 > self.config.max_total_tx {
            self.evict_lowest();
        }
        true
    }

    /// Select the highest-fee transactions up to `max_count`.
    /// Returns transactions ordered by effective fee descending.
    ///
    /// Source: p2p-wire-spec.md Section 2.4 step 5.
    pub fn select_for_block(&mut self, max_count: usize) -> Vec<MempoolTx> {
        let mut temp: Vec<FeeOrdered> = self.txs.drain().collect();
        temp.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.tx_hash.cmp(&b.0.tx_hash)));

        let selected: Vec<FeeOrdered> = temp.drain(0..max_count.min(temp.len())).collect();
        let remaining: Vec<FeeOrdered> = temp;

        let mut result: Vec<MempoolTx> = Vec::with_capacity(selected.len());
        for sel in selected {
            self.tx_hashes.remove(&sel.0.tx_hash);
            *self.per_sender.entry(sel.0.sender_id).or_insert(1) =
                self.per_sender.get(&sel.0.sender_id).unwrap_or(&1).saturating_sub(1);
            result.push(sel.0);
        }
        for rem in remaining {
            self.txs.push(rem);
        }
        result
    }

    /// Evict the globally lowest-fee transaction regardless of type.
    fn evict_lowest(&mut self) {
        let mut temp: Vec<FeeOrdered> = self.txs.drain().collect();
        temp.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.tx_hash.cmp(&b.0.tx_hash)));
        if let Some(evicted) = temp.pop() {
            self.tx_hashes.remove(&evicted.0.tx_hash);
            let sender = evicted.0.sender_id;
            let count = self.per_sender.get(&sender).copied().unwrap_or(1);
            if count <= 1 {
                self.per_sender.remove(&sender);
            } else {
                self.per_sender.insert(sender, count - 1);
            }
        }
        for rem in temp {
            self.txs.push(rem);
        }
    }

    /// Return the current base fee for reference.
    pub fn base_fee_for_test(&self) -> u128 {
        0
    }

    /// Get the current config (for testing).
    pub fn config(&self) -> &MempoolConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(
        hash: u8,
        sender: u8,
        tx_type: TxTypeTag,
        prio: u128,
        base: u128,
        max_fee: u128,
    ) -> MempoolTx {
        MempoolTx {
            tx_hash: [hash; 32],
            sender_id: [sender; 32],
            tx_type,
            priority_fee: prio,
            base_fee: base,
            max_fee_per_tx: max_fee,
            tx_data: vec![hash],
        }
    }

    #[test]
    fn insert_and_select_highest_fee_first() {
        let config = MempoolConfig::default();
        let mut pool = Mempool::new(config);

        pool.insert(tx(1, 1, TxTypeTag::Standard, 10, 100, 200));
        pool.insert(tx(2, 2, TxTypeTag::Standard, 50, 100, 200));
        pool.insert(tx(3, 3, TxTypeTag::Standard, 5, 100, 200));

        let selected = pool.select_for_block(2);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].tx_hash, [2; 32]);
        assert_eq!(selected[1].tx_hash, [1; 32]);
    }

    #[test]
    fn duplicate_tx_rejected() {
        let config = MempoolConfig::default();
        let mut pool = Mempool::new(config);

        assert!(pool.insert(tx(1, 1, TxTypeTag::Standard, 10, 100, 200)));
        assert!(!pool.insert(tx(1, 1, TxTypeTag::Standard, 10, 100, 200)));
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn per_sender_limit_enforced() {
        let config = MempoolConfig { per_sender_tx_limit: 2, ..Default::default() };
        let mut pool = Mempool::new(config);

        assert!(pool.insert(tx(1, 1, TxTypeTag::Standard, 10, 100, 200)));
        assert!(pool.insert(tx(2, 1, TxTypeTag::Standard, 20, 100, 200)));
        assert!(!pool.insert(tx(3, 1, TxTypeTag::Standard, 30, 100, 200)));
    }

    #[test]
    fn evidence_fee_discount_applied() {
        let config = MempoolConfig::default();
        let evidence = tx(1, 1, TxTypeTag::Evidence, 10, 100, 200);
        let standard = tx(2, 2, TxTypeTag::Standard, 40, 100, 200);

        let eff_evidence = evidence.effective_fee(&config);
        let eff_standard = standard.effective_fee(&config);
        assert_eq!(eff_evidence, 60);
        assert_eq!(eff_standard, 140);
    }

    #[test]
    fn evidence_tx_clears_cheaper_standard() {
        let config = MempoolConfig::default();
        let mut pool = Mempool::new(config);

        pool.insert(tx(1, 1, TxTypeTag::Evidence, 10, 100, 200));
        pool.insert(tx(2, 2, TxTypeTag::Standard, 40, 100, 200));
        pool.insert(tx(3, 3, TxTypeTag::Standard, 50, 100, 200));

        let selected = pool.select_for_block(2);
        assert_eq!(selected[0].tx_hash, [3; 32]);
        assert_eq!(selected[1].tx_hash, [2; 32]);
    }

    #[test]
    fn governance_fee_discount_applied() {
        let config = MempoolConfig::default();
        let governance = tx(1, 1, TxTypeTag::Governance, 10, 100, 200);
        let eff = governance.effective_fee(&config);
        assert_eq!(eff, 60);
    }

    #[test]
    fn no_lane_reservation_all_tx_types_share_pool() {
        let config = MempoolConfig::default();
        let mut pool = Mempool::new(config);

        pool.insert(tx(1, 1, TxTypeTag::Evidence, 10, 100, 200));
        pool.insert(tx(2, 2, TxTypeTag::Governance, 20, 100, 200));
        pool.insert(tx(3, 3, TxTypeTag::Standard, 60, 100, 200));

        let selected = pool.select_for_block(3);
        assert_eq!(selected[0].tx_hash, [3; 32]);
        assert_eq!(selected[1].tx_hash, [2; 32]);
        assert_eq!(selected[2].tx_hash, [1; 32]);
    }

    #[test]
    fn max_total_tx_eviction() {
        let config = MempoolConfig { max_total_tx: 3, ..Default::default() };
        let mut pool = Mempool::new(config);

        pool.insert(tx(1, 1, TxTypeTag::Standard, 10, 100, 200));
        pool.insert(tx(2, 2, TxTypeTag::Standard, 20, 100, 200));
        pool.insert(tx(3, 3, TxTypeTag::Standard, 5, 100, 200));
        pool.insert(tx(4, 4, TxTypeTag::Standard, 50, 100, 200));

        assert_eq!(pool.len(), 3);
        let selected = pool.select_for_block(3);
        assert_eq!(selected[0].tx_hash, [4; 32]);
    }

    #[test]
    fn zero_discount_for_standard() {
        let config = MempoolConfig::default();
        let standard = tx(1, 1, TxTypeTag::Standard, 10, 100, 200);
        assert_eq!(standard.effective_fee(&config), 110);
    }

    #[test]
    fn effective_fee_uses_effective_base_not_raw_base() {
        let config = MempoolConfig::default();
        let evidence = tx(1, 1, TxTypeTag::Evidence, 0, 100, 200);
        let standard_better = tx(2, 2, TxTypeTag::Standard, 45, 100, 200);

        let eff_evidence = evidence.effective_fee(&config);
        let eff_standard = standard_better.effective_fee(&config);
        assert_eq!(eff_evidence, 50);
        assert_eq!(eff_standard, 145);
    }

    #[test]
    fn negative_discount_boundary() {
        let config = MempoolConfig { evidence_fee_discount_pct: 100, ..Default::default() };
        let evidence = tx(1, 1, TxTypeTag::Evidence, 5, 100, 200);
        assert_eq!(evidence.effective_fee(&config), 5);
    }
}
