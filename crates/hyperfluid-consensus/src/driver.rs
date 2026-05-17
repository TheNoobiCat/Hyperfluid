// === Consensus Driver ===
//
// Block production loop that wires the StateMachine into a running chain.
// Produces blocks from transactions, tracks height/epoch, maintains block store.
// Designed to accept a BFT consensus replacement (e.g. Malachite) later.
//
// Source: specs/protocol/consensus-spec.md Sections 1-2

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parity_scale_codec::{Decode, Encode};
use sha3::{Digest, Sha3_256};
use tokio::task::JoinHandle;

use hyperfluid_fee_market::{compute_next_base_fee, FeeConfig, FeeMarketState};
use hyperfluid_staking::SystemParameters;
use hyperfluid_state::state_machine::{ExecutionContext, StateMachine};
use hyperfluid_state::Account;

use hyperfluid_fastpath::lifecycle::FastPathEngine;
use hyperfluid_fastpath::types::{FastPathChallengeTx, FastPathParams, FastPathProposal};
use hyperfluid_governance::proposal::GovernanceEngine;
use hyperfluid_governance::types::{GovernanceParams, GovernanceVote, VoteOption};
use hyperfluid_pdp::rule_chain;
use hyperfluid_pdp::types::{ActionPlanRequest, ActionType, Decision, PdpContext};

use crate::genesis::GenesisConfig;
use crate::types::{
    Block, BlockHeader, DelegationAction, GovernanceAction, Hash32, StakingAction,
    TransactionEnvelope, TxType,
};

/// Payload format for TransferTx transactions.
/// SCALE-encoded into `TransactionEnvelope.tx_payload`.
#[derive(Encode, Decode)]
struct TransferPayload {
    sender_id: Hash32,
    recipient_id: Hash32,
    amount: u128,
    nonce: u64,
}

/// Payload format for GovernanceTx transactions.
/// Supports both Propose and Vote sub-actions in a single struct.
///
/// For `Propose`:
///   `target_hash`     → proposed git:head commit
///   `title_hash`      → hash of the proposal title
///   `description_hash`→ hash of the proposal description (used as bundle_manifest_hash)
///
/// For `Vote`:
///   `vote_approve`    → true = Yes, false = No
///   Other fields ignored.
#[derive(Encode, Decode)]
struct GovernancePayload {
    proposal_id: Hash32,
    proposer_id: Hash32,
    is_vote: bool,
    vote_approve: bool,
    target_hash: Hash32,
    title_hash: Hash32,
    description_hash: Hash32,
}

/// Payload format for FastPathTx transactions.
///
/// For merge proposals (`is_challenge = false`):
///   `merge_hash` → proposed topic head commit
///
/// For challenges (`is_challenge = true`):
///   `merge_hash` → evidence hash for the challenge
#[derive(Encode, Decode)]
struct FastPathPayload {
    proposal_id: Hash32,
    topic_id: Hash32,
    proposer_id: Hash32,
    merge_hash: Hash32,
    is_challenge: bool,
}

/// Payload format for StakingTx transactions.
/// Supports Bond, Unbond, Withdraw, and Renew sub-actions.
///
/// For Bond: `amount` = stake amount, `nonce` = validator nonce
/// For Unbond: `amount` ignored
/// For Withdraw: `amount` ignored
/// For Renew: `amount` ignored
#[derive(Encode, Decode)]
struct StakingPayload {
    validator_id: Hash32,
    amount: u128,
    nonce: u64,
}

/// Payload format for DelegationTx transactions.
/// Supports Delegate, Undelegate, WithdrawDelegation, SetCommission sub-actions.
///
/// For Delegate: `amount` = delegation amount, `nonce` = delegator nonce
/// For Undelegate: `amount` ignored
/// For WithdrawDelegation: `amount` ignored
/// For SetCommission: `amount` = commission_rate (0-20), `nonce` = validator nonce
#[derive(Encode, Decode)]
struct DelegationPayload {
    delegator_id: Hash32,
    validator_id: Hash32,
    amount: u128,
    nonce: u64,
}

/// Consensus driver that coordinates block production, transaction execution,
/// and state management. Wraps the deterministic StateMachine and maintains
/// the canonical block store.
pub struct ConsensusDriver {
    pub state_machine: StateMachine,
    pub block_store: Vec<Block>,
    pub height: u64,
    pub epoch: u64,
    pub epoch_length: u64,
    /// In-memory governance engine for proposal lifecycle and vote aggregation.
    pub governance: GovernanceEngine,
    /// In-memory fast-path engine for topic merge lifecycle.
    pub fastpath: FastPathEngine,
    /// EIP-1559 fee market state updated each block.
    pub fee_state: FeeMarketState,
    pub fee_config: FeeConfig,
    /// Staking system parameters for bond/unbond/delegation thresholds.
    pub staking_params: SystemParameters,
}

impl ConsensusDriver {
    /// Create a new consensus driver with zero height and an empty block store.
    /// Initializes the governance, fast-path engines, fee market, and staking
    /// parameters with their defaults.
    pub fn new(epoch_length: u64) -> Self {
        Self {
            state_machine: StateMachine::new(),
            block_store: Vec::new(),
            height: 0,
            epoch: 0,
            epoch_length,
            governance: GovernanceEngine::new(GovernanceParams::default()),
            fastpath: FastPathEngine::new(FastPathParams::default()),
            fee_state: FeeMarketState::default(),
            fee_config: FeeConfig::default(),
            staking_params: SystemParameters::default(),
        }
    }

    /// Bootstrap accounts from a genesis configuration into the state machine
    /// and create the genesis block (height 0) at the start of the chain.
    ///
    /// After this call, `self.height == 0` and the genesis block is in `block_store`.
    pub fn init_genesis(&mut self, genesis: &GenesisConfig) -> Block {
        for ga in &genesis.accounts {
            let account = Account {
                account_id: ga.account_id,
                balance: ga.balance,
                nonce: 0,
                pubkey_hash: sha3_256_hash(&ga.account_id),
                pubkey: ga.pubkey.clone(),
            };
            self.state_machine.init_account(account);
        }

        for gv in &genesis.validators {
            self.state_machine.init_validator(gv.validator_id, gv.bonded_stake, genesis.timestamp);
        }

        let state_root = self.state_machine.compute_state_root();
        let transaction_root = [0u8; 32];

        let genesis_block = Block {
            header: BlockHeader {
                height: 0,
                parent_hash: [0u8; 32],
                state_root,
                transaction_root,
                committee_id: 0,
                proposer_id: [0u8; 32],
                timestamp: genesis.timestamp,
                epoch: 0,
            },
            transactions: Vec::new(),
        };

        self.block_store.push(genesis_block.clone());
        self.height = 0;
        self.epoch = 0;

        genesis_block
    }

    /// Produce a new block at the next height.
    ///
    /// 1. Executes each transaction against the state machine
    /// 2. Computes the post-execution SMT state root
    /// 3. Computes the transaction Merkle root
    /// 4. Adjusts EIP-1559 base fee based on block utilization
    /// 5. Builds the Block with a proper header chaining to the parent
    /// 6. Stores the block and advances height/epoch
    pub fn produce_block(&mut self, txs: Vec<TransactionEnvelope>, timestamp: u64) -> Block {
        let new_height = self.height + 1;
        let ctx = ExecutionContext { height: new_height, timestamp };

        // 1. Execute all transactions
        for tx in &txs {
            self.execute_tx(tx, ctx);
        }

        // 2. Compute SMT root from post-execution state
        let state_root = self.state_machine.compute_state_root();

        // 3. Compute transaction Merkle root
        let transaction_root = Self::compute_transaction_root(&txs);

        // 4. Adjust EIP-1559 base fee
        let block_util_pct = self.compute_block_utilization(&txs);
        self.fee_state.base_fee =
            compute_next_base_fee(self.fee_state.base_fee, block_util_pct, &self.fee_config, 8);

        // 5. Build the block header chaining to parent
        let parent_hash =
            self.block_store.last().map(|b| b.header.block_hash()).unwrap_or([0u8; 32]);

        let new_epoch = new_height / self.epoch_length;

        let block = Block {
            header: BlockHeader {
                height: new_height,
                parent_hash,
                state_root,
                transaction_root,
                committee_id: 0,
                proposer_id: [0u8; 32],
                timestamp,
                epoch: new_epoch,
            },
            transactions: txs,
        };

        // 6. Store and advance
        self.block_store.push(block.clone());
        self.height = new_height;
        self.epoch = new_epoch;

        block
    }

    /// Compute block utilization as a percentage (0-100) based on the number
    /// of transactions relative to a target. With no capacity tracking yet,
    /// utilization is derived from transaction count vs a soft cap.
    fn compute_block_utilization(&self, txs: &[TransactionEnvelope]) -> u8 {
        let soft_cap = 100u64;
        if txs.is_empty() {
            return 0;
        }
        let tx_count = txs.len() as u64;
        if tx_count >= soft_cap {
            100
        } else {
            (tx_count * 100 / soft_cap) as u8
        }
    }

    /// Dispatch a single transaction to the appropriate state machine method
    /// or subsystem engine (governance, fast-path).
    fn execute_tx(&mut self, tx: &TransactionEnvelope, ctx: ExecutionContext) {
        // Run PDP pre-validation (integration point — currently pass-through)
        if !self.validate_tx_pdp(tx, ctx) {
            return;
        }

        match tx.tx_type {
            TxType::TransferTx => {
                if let Ok(payload) = TransferPayload::decode(&mut &tx.tx_payload[..]) {
                    self.state_machine.execute_transfer(
                        payload.sender_id,
                        payload.recipient_id,
                        payload.amount,
                        payload.nonce,
                        ctx,
                    );
                }
            }
            TxType::GovernanceTx(GovernanceAction::Propose) => {
                if let Ok(payload) = GovernancePayload::decode(&mut &tx.tx_payload[..]) {
                    // Hash description to produce bundle_manifest_hash
                    let bundle_manifest_hash = sha3_256_hash(&payload.description_hash);
                    let current_epoch = ctx.height / self.epoch_length;
                    match self.governance.submit_proposal(
                        payload.proposal_id,
                        payload.proposer_id,
                        payload.target_hash,
                        bundle_manifest_hash,
                        [0u8; 32], // current_commit: git:head tracking not yet wired
                        ctx.height,
                        current_epoch,
                        0, // total_snapshot_stake: not yet tracked
                    ) {
                        Ok(proposal) => {
                            tracing::info!(
                                "Governance proposal submitted: id={} proposer={} status={:?}",
                                hex::encode(proposal.proposal_id),
                                hex::encode(proposal.proposer_id),
                                proposal.status,
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Governance proposal rejected: id={} error={:?}",
                                hex::encode(payload.proposal_id),
                                e,
                            );
                        }
                    }
                }
            }
            TxType::GovernanceTx(GovernanceAction::Vote) => {
                if let Ok(payload) = GovernancePayload::decode(&mut &tx.tx_payload[..]) {
                    let vote_option =
                        if payload.vote_approve { VoteOption::Yes } else { VoteOption::No };
                    let vote = GovernanceVote {
                        proposal_id: payload.proposal_id,
                        voter_id: payload.proposer_id,
                        vote: vote_option,
                        reason_hash: payload.description_hash,
                        vote_weight: 1, // default weight: 1 unit (stake tracking not yet wired)
                        signature: vec![],
                    };
                    match self.governance.cast_vote(vote, ctx.height) {
                        Ok(()) => {
                            tracing::info!(
                                "Governance vote cast: proposal={} voter={}",
                                hex::encode(payload.proposal_id),
                                hex::encode(payload.proposer_id),
                            );
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Governance vote rejected: proposal={} error={:?}",
                                hex::encode(payload.proposal_id),
                                e,
                            );
                        }
                    }
                }
            }
            TxType::FastPathTx => {
                if let Ok(payload) = FastPathPayload::decode(&mut &tx.tx_payload[..]) {
                    if payload.is_challenge {
                        let challenge = FastPathChallengeTx {
                            proposal_id: payload.proposal_id,
                            topic_id: payload.topic_id,
                            challenger_id: payload.proposer_id,
                            evidence_hash: payload.merge_hash,
                            challenger_bond: 0,
                            signature: vec![],
                        };
                        let current_epoch = ctx.height / self.epoch_length;
                        match self.fastpath.submit_challenge(challenge, ctx.height, current_epoch) {
                            Ok(()) => {
                                tracing::info!(
                                    "Fast-path challenge submitted: proposal={} challenger={}",
                                    hex::encode(payload.proposal_id),
                                    hex::encode(payload.proposer_id),
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Fast-path challenge rejected: proposal={} error={:?}",
                                    hex::encode(payload.proposal_id),
                                    e,
                                );
                            }
                        }
                    } else {
                        let proposal = FastPathProposal {
                            proposal_id: payload.proposal_id,
                            topic_id: payload.topic_id,
                            proposer_id: payload.proposer_id,
                            base_topic_head: [0u8; 32],
                            proposed_head: payload.merge_hash,
                            bundle_manifest_hash: [0u8; 32],
                            expires_at_height: ctx.height.saturating_add(1000),
                            proposer_signature: vec![],
                        };
                        match self.fastpath.submit_proposal(proposal, ctx.height) {
                            Ok(()) => {
                                tracing::info!(
                                    "Fast-path proposal submitted: id={} topic={}",
                                    hex::encode(payload.proposal_id),
                                    hex::encode(payload.topic_id),
                                );
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Fast-path proposal rejected: id={} error={:?}",
                                    hex::encode(payload.proposal_id),
                                    e,
                                );
                            }
                        }
                    }
                }
            }
            // Staking transactions — validator lifecycle (bond/unbond/withdraw/renew)
            TxType::StakingTx(action) => {
                if let Ok(payload) = StakingPayload::decode(&mut &tx.tx_payload[..]) {
                    match action {
                        StakingAction::Bond => {
                            self.state_machine.execute_bond(
                                payload.validator_id,
                                payload.amount,
                                payload.nonce,
                                self.staking_params.min_self_bond,
                                ctx.height,
                                ctx,
                            );
                        }
                        StakingAction::Unbond => {
                            self.state_machine.execute_unbond(
                                payload.validator_id,
                                payload.nonce,
                                ctx.height,
                                ctx,
                            );
                        }
                        StakingAction::Withdraw => {
                            self.state_machine.execute_withdraw(
                                payload.validator_id,
                                payload.nonce,
                                ctx.height,
                                self.staking_params.unbond_delay,
                                ctx,
                            );
                        }
                        StakingAction::Renew => {
                            self.state_machine.execute_renew(
                                payload.validator_id,
                                payload.nonce,
                                ctx.height,
                                ctx,
                            );
                        }
                    }
                }
            }
            // Delegation transactions — delegator/validator relationship management
            TxType::DelegationTx(action) => {
                if let Ok(payload) = DelegationPayload::decode(&mut &tx.tx_payload[..]) {
                    match action {
                        DelegationAction::Delegate => {
                            self.state_machine.execute_delegate(
                                payload.delegator_id,
                                payload.validator_id,
                                payload.amount,
                                payload.nonce,
                                self.staking_params.min_delegation,
                                ctx,
                            );
                        }
                        DelegationAction::Undelegate => {
                            self.state_machine.execute_undelegate(
                                payload.delegator_id,
                                payload.validator_id,
                                payload.nonce,
                                ctx.height,
                                ctx,
                            );
                        }
                        DelegationAction::WithdrawDelegation => {
                            self.state_machine.execute_withdraw_delegation(
                                payload.delegator_id,
                                payload.validator_id,
                                payload.nonce,
                                ctx.height,
                                self.staking_params.delegation_unbond_delay,
                                ctx,
                            );
                        }
                        DelegationAction::SetCommission => {
                            self.state_machine.execute_set_commission(
                                payload.validator_id,
                                payload.amount as u8,
                                payload.nonce,
                                self.staking_params.max_commission_rate,
                                ctx,
                            );
                        }
                    }
                }
            }
            // Other transaction types (TaskCreateTx, EvidenceTx) are not yet wired.
            _ => {}
        }
    }

    /// PDP pre-validation integration point for governance and fast-path transactions.
    ///
    /// Maps `TxType` to `ActionType`, builds a minimal `PdpContext` from available
    /// driver state, and runs the 5-step deterministic rule chain. When key bindings,
    /// nonce tracking, and quota states are fully wired into ConsensusDriver, this
    /// method will enforce all PDP rules deterministically before execution.
    ///
    /// Currently allows all transactions through — PDP state (key bindings, nonces,
    /// quota tracking) is not yet maintained in ConsensusDriver, so the rule chain
    /// would deny everything. When that state is wired, the key_binding check below
    /// is removed and the `Decision::Approved` gate takes effect.
    fn validate_tx_pdp(&self, tx: &TransactionEnvelope, ctx: ExecutionContext) -> bool {
        let action_type = match tx.tx_type {
            TxType::GovernanceTx(GovernanceAction::Propose) => ActionType::SubmitGovernanceProposal,
            TxType::GovernanceTx(GovernanceAction::Vote) => ActionType::CastGovernanceVote,
            TxType::FastPathTx => ActionType::SubmitFastPathMerge,
            _ => return true,
        };

        let pdp_ctx = PdpContext {
            current_height: ctx.height,
            key_binding: None, // Not yet tracked in ConsensusDriver
            agent_balance_attagx: u128::MAX,
            agent_nonce: 0,
            consumed_plan_ids: vec![],
            quota_states: vec![],
        };

        let request = ActionPlanRequest {
            plan_id: [0u8; 32],
            agent_id: [0u8; 32],
            action_type,
            resource_id: [0u8; 32],
            reason_hash: [0u8; 32],
            evidence_refs: vec![],
            nonce: 0,
            expires_at_height: ctx.height.saturating_add(1000),
            agent_signature: vec![],
        };

        let response = rule_chain::evaluate(&request, &pdp_ctx);

        // When full state is wired, this gate enforces PDP rules.
        // Currently key_binding is always None, so execution proceeds.
        if pdp_ctx.key_binding.is_none() {
            return true;
        }

        matches!(response.decision, Decision::Approved)
    }

    /// Compute the transaction Merkle root from a list of transaction envelopes.
    ///
    /// - Empty list → [0u8; 32]
    /// - Single tx → SHA3-256(SCALE(tx))
    /// - Multiple txs → binary Merkle tree built bottom-up from SCALE-encoded leaf hashes
    fn compute_transaction_root(txs: &[TransactionEnvelope]) -> Hash32 {
        if txs.is_empty() {
            return [0u8; 32];
        }

        // Leaf hash for each tx: SHA3-256(SCALE-encoded tx)
        let mut level: Vec<Hash32> = txs.iter().map(|tx| sha3_256_hash(&tx.encode())).collect();

        if level.len() == 1 {
            return level[0];
        }

        // Bottom-up binary Merkle tree construction
        while level.len() > 1 {
            let mut next = Vec::new();
            for chunk in level.chunks(2) {
                let left = chunk[0];
                let right = chunk.get(1).copied().unwrap_or([0u8; 32]);
                let mut hasher = Sha3_256::new();
                hasher.update(left);
                hasher.update(right);
                let mut out = [0u8; 32];
                out.copy_from_slice(&hasher.finalize());
                next.push(out);
            }
            level = next;
        }
        level[0]
    }

    /// Query an account's balance. Returns None if the account does not exist.
    pub fn account_balance(&self, account_id: &[u8; 32]) -> Option<u128> {
        self.state_machine.get_account(account_id).map(|a| a.balance)
    }

    /// Query an account's nonce. Returns None if the account does not exist.
    pub fn account_nonce(&self, account_id: &[u8; 32]) -> Option<u64> {
        self.state_machine.get_account(account_id).map(|a| a.nonce)
    }

    /// Start an async block production loop.
    ///
    /// Produces one empty block per interval. Accepts an `Arc<Mutex<Self>>`
    /// so the loop can be spawned into a Tokio task with `'static` lifetime.
    /// Later this will pull transactions from a mempool instead of producing
    /// empty blocks.
    pub fn run_block_loop(
        driver: Arc<Mutex<ConsensusDriver>>,
        running: Arc<AtomicBool>,
        block_interval: Duration,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while running.load(Ordering::Relaxed) {
                let timestamp =
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

                if let Ok(mut d) = driver.lock() {
                    let block = d.produce_block(vec![], timestamp);
                    tracing::info!(
                        "Produced block height={}, hash={}, state_root={}",
                        block.header.height,
                        hex::encode(block.header.block_hash()),
                        hex::encode(block.header.state_root),
                    );
                }

                tokio::time::sleep(block_interval).await;
            }
        })
    }
}

/// Convenience SHA3-256 wrapper returning `Hash32`.
fn sha3_256_hash(data: &[u8]) -> Hash32 {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genesis::GenesisAccount;

    /// Helper: build a minimal GenesisConfig with the given accounts.
    fn test_genesis(accounts: Vec<GenesisAccount>) -> GenesisConfig {
        GenesisConfig {
            chain_id: "test".into(),
            timestamp: 0,
            epoch_length: 100,
            committee_size: 100,
            min_stake: 1_000_000_000_000_000_000_000u128,
            bond_delay: 100,
            unbond_delay: 1000,
            max_governance_proposals: 32,
            proposal_deposit: 500_000_000_000_000_000_000u128,
            liveness_window_blocks: 100,
            liveness_miss_threshold_pct: 20,
            total_agx_supply: 10_000_000_000_000_000_000_000_000u128,
            airdrop_amount_per_agent: 100_000_000_000_000_000_000u128,
            accounts,
            validators: Vec::new(),
        }
    }

    #[test]
    fn genesis_block_has_height_zero() {
        let mut driver = ConsensusDriver::new(100);
        let genesis = test_genesis(vec![GenesisAccount {
            account_id: [1u8; 32],
            balance: 1_000_000_000_000_000_000_000u128,
            pubkey: None,
        }]);
        let block = driver.init_genesis(&genesis);
        assert_eq!(block.header.height, 0);
        assert_eq!(block.header.parent_hash, [0u8; 32]);
        assert_eq!(driver.height, 0);
        assert_eq!(driver.block_store.len(), 1);
    }

    #[test]
    fn genesis_state_root_is_nonzero() {
        let mut driver = ConsensusDriver::new(100);
        let genesis = test_genesis(vec![GenesisAccount {
            account_id: [1u8; 32],
            balance: 1_000_000_000_000_000_000_000u128,
            pubkey: None,
        }]);
        let block = driver.init_genesis(&genesis);
        assert_ne!(block.header.state_root, [0u8; 32]);
    }

    #[test]
    fn produce_block_advances_height() {
        let mut driver = ConsensusDriver::new(100);
        let genesis = test_genesis(vec![GenesisAccount {
            account_id: [1u8; 32],
            balance: 1_000_000_000_000_000_000_000u128,
            pubkey: None,
        }]);
        driver.init_genesis(&genesis);

        let block = driver.produce_block(vec![], 1);
        assert_eq!(block.header.height, 1);
        assert_eq!(driver.height, 1);
        assert_eq!(driver.block_store.len(), 2);
    }

    #[test]
    fn blocks_chain_parent_hash() {
        let mut driver = ConsensusDriver::new(100);
        let genesis = test_genesis(vec![GenesisAccount {
            account_id: [1u8; 32],
            balance: 1_000_000_000_000_000_000_000u128,
            pubkey: None,
        }]);
        let genesis_block = driver.init_genesis(&genesis);

        let b1 = driver.produce_block(vec![], 1);
        let b2 = driver.produce_block(vec![], 2);

        assert_eq!(b1.header.parent_hash, genesis_block.header.block_hash());
        assert_eq!(b2.header.parent_hash, b1.header.block_hash());
        assert_ne!(b1.header.parent_hash, [0u8; 32]);
        assert_ne!(b2.header.parent_hash, [0u8; 32]);
    }

    #[test]
    fn empty_block_state_root_unchanged() {
        let mut driver = ConsensusDriver::new(100);
        let genesis = test_genesis(vec![GenesisAccount {
            account_id: [1u8; 32],
            balance: 1_000_000_000_000_000_000_000u128,
            pubkey: None,
        }]);
        driver.init_genesis(&genesis);
        let root_before = driver.state_machine.compute_state_root();

        let block = driver.produce_block(vec![], 1);
        assert_eq!(block.header.state_root, root_before);
    }

    #[test]
    fn account_balance_query() {
        let mut driver = ConsensusDriver::new(100);
        let genesis = test_genesis(vec![GenesisAccount {
            account_id: [0xAAu8; 32],
            balance: 5_000_000_000_000_000_000_000u128,
            pubkey: None,
        }]);
        driver.init_genesis(&genesis);

        assert_eq!(driver.account_balance(&[0xAAu8; 32]), Some(5_000_000_000_000_000_000_000u128));
        assert_eq!(driver.account_balance(&[0xBBu8; 32]), None);
    }

    #[test]
    fn account_nonce_query() {
        let mut driver = ConsensusDriver::new(100);
        let genesis = test_genesis(vec![GenesisAccount {
            account_id: [0xCCu8; 32],
            balance: 1_000u128,
            pubkey: None,
        }]);
        driver.init_genesis(&genesis);

        assert_eq!(driver.account_nonce(&[0xCCu8; 32]), Some(0));
    }

    #[test]
    fn transfer_tx_changes_state() {
        let mut driver = ConsensusDriver::new(100);
        let alice_id = [1u8; 32];
        let bob_id = [2u8; 32];

        let genesis = test_genesis(vec![
            GenesisAccount {
                account_id: alice_id,
                balance: 1_000_000_000_000_000_000_000u128, // 1000 AGX
                pubkey: None,
            },
            GenesisAccount { account_id: bob_id, balance: 0, pubkey: None },
        ]);
        driver.init_genesis(&genesis);
        let root_before = driver.state_machine.compute_state_root();

        let payload = TransferPayload {
            sender_id: alice_id,
            recipient_id: bob_id,
            amount: 100_000_000_000_000_000_000u128, // 100 AGX
            nonce: 1,
        };
        let tx = TransactionEnvelope {
            tx_type: TxType::TransferTx,
            tx_payload: payload.encode(),
            approved_plan_id: None,
            gateway_signature: None,
        };

        let block = driver.produce_block(vec![tx], 1);
        let root_after = block.header.state_root;

        assert_eq!(driver.account_balance(&alice_id), Some(900_000_000_000_000_000_000u128));
        assert_eq!(driver.account_balance(&bob_id), Some(100_000_000_000_000_000_000u128));
        assert_ne!(root_after, root_before);
        assert_ne!(root_after, [0u8; 32]);
    }

    #[test]
    fn epoch_boundary_detection() {
        let mut driver = ConsensusDriver::new(5); // short epoch for testing
        let genesis = test_genesis(vec![GenesisAccount {
            account_id: [1u8; 32],
            balance: 1_000u128,
            pubkey: None,
        }]);
        driver.init_genesis(&genesis);

        // Heights: 0 (genesis), 1, 2, 3, 4 → epoch 0
        // Heights: 5 → epoch 1
        for _ in 0..4 {
            driver.produce_block(vec![], 0);
        }
        assert_eq!(driver.epoch, 0);
        assert_eq!(driver.height, 4);

        let b5 = driver.produce_block(vec![], 0);
        assert_eq!(b5.header.height, 5);
        assert_eq!(b5.header.epoch, 1);
        assert_eq!(driver.epoch, 1);
    }
}
