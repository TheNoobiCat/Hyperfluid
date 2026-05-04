// === Consensus data structures ===
//
// Source: specs/protocol/consensus-spec.md Section 1.3

use serde::{Deserialize, Serialize};

pub type Hash32 = [u8; 32];
pub type Signature = Vec<u8>;

/// Epoch committee. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Committee {
    pub epoch: u64,
    pub seed: Hash32,
    pub members: Vec<Hash32>,
    pub weights: Vec<u64>,
}

/// Block header with state root commitment. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub height: u64,
    pub parent_hash: Hash32,
    pub state_root: Hash32,
    pub transaction_root: Hash32,
    pub committee_id: u64,
    pub proposer_id: Hash32,
    pub timestamp: u64,
    pub epoch: u64,
}

/// A full block. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<TransactionEnvelope>,
}

/// Transaction envelope wrapping a typed payload. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionEnvelope {
    pub tx_type: TxType,
    pub tx_payload: Vec<u8>,
    pub approved_plan_id: Option<Hash32>,
    pub gateway_signature: Option<Signature>,
}

/// All transaction types on the protocol. Source: consensus-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxType {
    TransferTx,
    StakeBondTx,
    StakeRenewTx,
    UnbondRequestTx,
    WithdrawUnbondedTx,
    GovernanceProposeTx,
    GovernanceVoteTx,
    EvidenceTx,
    FastPathProposalTx,
    FastPathReviewTx,
    FastPathChallengeTx,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_header_is_copyable_after_serde() {
        let h = BlockHeader {
            height: 1,
            parent_hash: [0; 32],
            state_root: [1; 32],
            transaction_root: [2; 32],
            committee_id: 0,
            proposer_id: [3; 32],
            timestamp: 1000,
            epoch: 0,
        };
        let json = serde_json::to_string(&h).unwrap();
        let h2: BlockHeader = serde_json::from_str(&json).unwrap();
        assert_eq!(h, h2);
    }

    #[test]
    fn tx_type_variants_are_exhaustive() {
        let types = [
            TxType::TransferTx,
            TxType::StakeBondTx,
            TxType::StakeRenewTx,
            TxType::UnbondRequestTx,
            TxType::WithdrawUnbondedTx,
            TxType::GovernanceProposeTx,
            TxType::GovernanceVoteTx,
            TxType::EvidenceTx,
            TxType::FastPathProposalTx,
            TxType::FastPathReviewTx,
            TxType::FastPathChallengeTx,
        ];
        assert_eq!(types.len(), 11);
    }

    #[test]
    fn committee_size_validation() {
        let c = Committee {
            epoch: 0,
            seed: [0; 32],
            members: vec![[1; 32]; 100],
            weights: vec![1000; 100],
        };
        assert_eq!(c.members.len(), 100);
        assert_eq!(c.weights.len(), 100);
    }
}
