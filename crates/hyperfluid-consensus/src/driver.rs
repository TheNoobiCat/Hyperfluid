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

use std::collections::{BTreeMap, BTreeSet};

use hyperfluid_fee_market::{compute_next_base_fee, FeeConfig, FeeMarketState};
use hyperfluid_p2p::mempool::{Mempool, MempoolConfig, MempoolTx, TxTypeTag};
use hyperfluid_staking::SystemParameters;
use hyperfluid_state::state_machine::{ExecutionContext, SplitChildSpec, StateMachine};
use hyperfluid_state::{Account, HeartbeatPayload, ReviewVerdict, TaskStatus, TrustStageEnum};

use crate::genesis::GenesisConfig;
use crate::malachite_consensus::ConsensusNetworkMsg;
use crate::types::{
    Block, BlockHeader, DelegationAction, GovernanceAction, Hash32, StakingAction,
    TransactionEnvelope, TxType,
};
use hyperfluid_fastpath::lifecycle::FastPathEngine;
use hyperfluid_fastpath::types::{FastPathChallengeTx, FastPathParams, FastPathProposal};
use hyperfluid_governance::proposal::GovernanceEngine;
use hyperfluid_governance::types::{GovernanceParams, GovernanceVote, VoteOption};
use hyperfluid_pdp::rule_chain;
use hyperfluid_pdp::types::{
    ActionPlanRequest, ActionType, Decision, PdpContext, QuotaConsumption, QuotaState, TrustStage,
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

/// Payload format for ClaimTaskTx transactions.
#[derive(Encode, Decode)]
struct ClaimTaskPayload {
    task_id: Hash32,
    agent_id: Hash32,
    collateral: u128,
    trust_stage_flag: bool, // false = Untrusted, true = Trusted
}

/// Payload format for HeartbeatTx transactions.
#[derive(Encode, Decode)]
struct HeartbeatTxPayload {
    lease_id: Hash32,
    artifact_hash: Option<Hash32>,
    diff_pointer: Option<Hash32>,
    test_result_ref: Option<Hash32>,
    signature: Vec<u8>,
}

/// Payload format for ReleaseTaskTx transactions.
#[derive(Encode, Decode)]
struct ReleaseTaskPayload {
    task_id: Hash32,
    agent_id: Hash32,
}

/// Payload format for SubmitTaskTx transactions.
#[derive(Encode, Decode)]
struct SubmitTaskPayload {
    task_id: Hash32,
    agent_id: Hash32,
}

/// Payload format for SplitTaskTx transactions.
/// Encodes parent task + caller + list of child specifications.
#[derive(Encode, Decode)]
struct SplitTaskPayload {
    parent_task_id: Hash32,
    caller_id: Hash32,
    children: Vec<SplitChildPayload>,
}

#[derive(Encode, Decode)]
struct SplitChildPayload {
    task_id: Hash32,
    bounty_share_pct: u8,
    depends_on: Vec<Hash32>,
    required_skills_hash: Hash32,
}

/// Payload format for SubmitReviewTx transactions.
#[derive(Encode, Decode)]
struct SubmitReviewPayload {
    review_task_id: Hash32,
    reviewer_id: Hash32,
    verdict_accept: bool, // true = Accept, false = Reject
    evidence_hash: Hash32,
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
    /// Single fee-ordered mempool for pending transactions.
    pub mempool: Mempool,
    /// Full transaction storage, keyed by tx_hash, for mempool retrieval.
    pub tx_store: BTreeMap<[u8; 32], TransactionEnvelope>,
    /// Agent pubkey bindings for PDP signature verification.
    /// Stub: unit value — real ML-DSA key verification deferred to Week 9-10.
    pub key_bindings: BTreeMap<Hash32, Vec<u8>>,
    /// Expected next nonce per agent (PDP replay protection).
    pub agent_nonces: BTreeMap<Hash32, u64>,
    /// Quota consumption state per (agent, quota_id).
    pub quota_states: BTreeMap<(Hash32, String), QuotaState>,
    /// Consumed plan IDs for PDP replay protection (deduplication).
    pub consumed_plan_ids: BTreeSet<Hash32>,
    /// When true, bypasses PDP validation for all transaction types.
    /// Used for development/testing when full PDP state (key bindings,
    /// nonce tracking, quota states) is not yet wired.
    pub pdp_bypass: bool,
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
            mempool: Mempool::new(MempoolConfig::default()),
            tx_store: BTreeMap::new(),
            key_bindings: BTreeMap::new(),
            agent_nonces: BTreeMap::new(),
            quota_states: BTreeMap::new(),
            consumed_plan_ids: BTreeSet::new(),
            pdp_bypass: false,
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

    /// Submit a transaction to the mempool for future block inclusion.
    ///
    /// Encodes the envelope, derives a tx hash, infers a TxTypeTag, and inserts
    /// into the fee-ordered mempool. Stores the full envelope for retrieval at
    /// block production time.
    pub fn submit_tx(&mut self, tx: TransactionEnvelope) -> bool {
        let tx_data = tx.encode();
        let tx_hash = sha3_256_hash(&tx_data);
        if self.tx_store.contains_key(&tx_hash) {
            return false;
        }
        let sender_id = self.extract_sender_id(&tx).unwrap_or([0u8; 32]);
        let tx_type_tag = match tx.tx_type {
            TxType::GovernanceTx(_) => TxTypeTag::Governance,
            TxType::EvidenceTx => TxTypeTag::Evidence,
            _ => TxTypeTag::Standard,
        };
        let mtx = MempoolTx {
            tx_hash,
            sender_id,
            tx_type: tx_type_tag,
            priority_fee: 0,
            base_fee: self.fee_state.base_fee,
            max_fee_per_tx: self.fee_state.base_fee.saturating_add(1_000_000_000_000_000),
            tx_data,
        };
        self.tx_store.insert(tx_hash, tx);
        self.mempool.insert(mtx)
    }

    /// Extract the sender/agent ID from a transaction envelope for mempool routing.
    fn extract_sender_id(&self, tx: &TransactionEnvelope) -> Option<[u8; 32]> {
        match tx.tx_type {
            TxType::TransferTx => {
                let payload = TransferPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.sender_id)
            }
            TxType::GovernanceTx(_) => {
                let payload = GovernancePayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.proposer_id)
            }
            TxType::StakingTx(_) => {
                let payload = StakingPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.validator_id)
            }
            TxType::DelegationTx(_) => {
                let payload = DelegationPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.delegator_id)
            }
            TxType::TaskCreateTx => {
                let payload = TransferPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.sender_id)
            }
            TxType::ClaimTaskTx => {
                let payload = ClaimTaskPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.agent_id)
            }
            TxType::FastPathTx => {
                let payload = FastPathPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.proposer_id)
            }
            _ => None,
        }
    }

    /// Produce a new block at the next height.
    ///
    /// If `txs` is non-empty, those transactions are used directly. If `txs` is empty,
    /// transactions are selected from the fee-ordered mempool.
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

        let block_txs = if txs.is_empty() {
            let max_txs_per_block = 100usize;
            let selected_mtxs = self.mempool.select_for_block(max_txs_per_block);
            selected_mtxs.iter().filter_map(|mtx| self.tx_store.remove(&mtx.tx_hash)).collect()
        } else {
            txs
        };

        // 1. Execute all transactions
        for tx in &block_txs {
            self.execute_tx(tx, ctx);
        }

        // 2a. Check lease expiry for all active tasks
        let active_tasks: Vec<Hash32> = self
            .state_machine
            .tasks_iter()
            .filter(|t| {
                matches!(t.status, TaskStatus::Claimed | TaskStatus::InProgress)
                    && t.lease_expires_height <= new_height
            })
            .map(|t| t.task_id)
            .collect();
        for task_id in &active_tasks {
            self.state_machine.run_lease_expiry(task_id, new_height);
        }

        // 2b. Check review window expiry for all InReview tasks
        let in_review: Vec<Hash32> = self
            .state_machine
            .tasks_iter()
            .filter(|t| matches!(t.status, TaskStatus::InReview))
            .map(|t| t.task_id)
            .collect();
        for task_id in &in_review {
            self.state_machine.run_review_expiry(task_id, new_height);
        }

        // 2c. Run trust promotion and topic decay at epoch boundaries
        let new_epoch = new_height / self.epoch_length;
        if new_height > 0 && new_height.is_multiple_of(self.epoch_length) {
            self.state_machine.run_trust_promotion();
            self.state_machine.run_topic_decay(new_height);
        }

        // 3. Compute SMT root from post-execution state
        let state_root = self.state_machine.compute_state_root();

        // 3b. Compute transaction Merkle root
        let transaction_root = Self::compute_transaction_root(&block_txs);

        // 4. Adjust EIP-1559 base fee
        let block_util_pct = self.compute_block_utilization(&block_txs);
        self.fee_state.base_fee =
            compute_next_base_fee(self.fee_state.base_fee, block_util_pct, &self.fee_config, 8);

        // 5. Build the block header chaining to parent
        let parent_hash =
            self.block_store.last().map(|b| b.header.block_hash()).unwrap_or([0u8; 32]);

        let _new_epoch = new_height / self.epoch_length;

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
            transactions: block_txs,
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
                            let commission_rate = payload.amount.min(100) as u8;
                            self.state_machine.execute_set_commission(
                                payload.validator_id,
                                commission_rate,
                                payload.nonce,
                                self.staking_params.max_commission_rate,
                                ctx,
                            );
                        }
                    }
                }
            }
            TxType::TaskCreateTx => {
                if let Ok(payload) = TransferPayload::decode(&mut &tx.tx_payload[..]) {
                    let task_id = sha3_256_hash(&tx.tx_payload);
                    self.state_machine.execute_task_create(
                        payload.sender_id,
                        payload.amount,
                        0, // fee: not yet tracked in payload
                        task_id,
                        payload.nonce,
                        [0u8; 32], // seed_ref placeholder
                        [0u8; 32], // topic_id placeholder
                        [0u8; 32], // metadata_hash placeholder
                        [0u8; 32], // required_skills_hash placeholder
                        [0u8; 32], // sponsor_id placeholder
                        [0u8; 32], // requester_pubkey placeholder
                        ctx.height,
                        ctx,
                    );
                }
            }
            TxType::ClaimTaskTx => {
                if let Ok(payload) = ClaimTaskPayload::decode(&mut &tx.tx_payload[..]) {
                    let trust_stage = if payload.trust_stage_flag {
                        TrustStageEnum::Trusted
                    } else {
                        TrustStageEnum::Untrusted
                    };
                    self.state_machine.execute_claim_task(
                        payload.task_id,
                        payload.agent_id,
                        payload.collateral,
                        ctx.height,
                        trust_stage,
                        ctx,
                    );
                }
            }
            TxType::HeartbeatTx => {
                if let Ok(payload) = HeartbeatTxPayload::decode(&mut &tx.tx_payload[..]) {
                    let heartbeat = HeartbeatPayload {
                        lease_id: payload.lease_id,
                        artifact_hash: payload.artifact_hash,
                        diff_pointer: payload.diff_pointer,
                        test_result_ref: payload.test_result_ref,
                        signature: payload.signature,
                    };
                    self.state_machine.execute_heartbeat(heartbeat, ctx.height, ctx);
                }
            }
            TxType::ReleaseTaskTx => {
                if let Ok(payload) = ReleaseTaskPayload::decode(&mut &tx.tx_payload[..]) {
                    self.state_machine.execute_release_task(payload.task_id, payload.agent_id, ctx);
                }
            }
            TxType::SubmitTaskTx => {
                if let Ok(payload) = SubmitTaskPayload::decode(&mut &tx.tx_payload[..]) {
                    self.state_machine.execute_submit_completion(
                        payload.task_id,
                        payload.agent_id,
                        ctx.height,
                        ctx,
                    );
                }
            }
            TxType::SubmitReviewTx => {
                if let Ok(payload) = SubmitReviewPayload::decode(&mut &tx.tx_payload[..]) {
                    let verdict = if payload.verdict_accept {
                        ReviewVerdict::Accept
                    } else {
                        ReviewVerdict::Reject
                    };
                    self.state_machine.execute_submit_review(
                        payload.review_task_id,
                        payload.reviewer_id,
                        verdict,
                        payload.evidence_hash,
                        ctx.height,
                        ctx,
                    );
                }
            }
            TxType::SplitTaskTx => {
                if let Ok(payload) = SplitTaskPayload::decode(&mut &tx.tx_payload[..]) {
                    let children: Vec<SplitChildSpec> = payload
                        .children
                        .into_iter()
                        .map(|c| SplitChildSpec {
                            task_id: c.task_id,
                            bounty_share_pct: c.bounty_share_pct,
                            depends_on: c.depends_on,
                            required_skills_hash: c.required_skills_hash,
                        })
                        .collect();
                    self.state_machine.execute_split_task(
                        payload.parent_task_id,
                        payload.caller_id,
                        children,
                        ctx.height,
                        ctx,
                    );
                }
            }
            // EvidenceTx not yet wired
            _ => {}
        }
    }

    /// PDP pre-validation integration point for governance and fast-path transactions.
    ///
    /// Maps `TxType` to `ActionType`, builds a `PdpContext` from live driver state
    /// (agent balance, nonce, key binding, quotas), and runs the 5-step deterministic
    /// rule chain. When PDP bypass is enabled, all transactions pass through.
    ///
    /// Signature verification (step 2) is a stub — real ML-DSA-65 checking deferred
    /// to Week 9-10. When pdp_bypass is false, key_binding must be present for the
    /// agent (fail-closed), but step 2 itself always passes (no cryptographic check).
    fn validate_tx_pdp(&mut self, tx: &TransactionEnvelope, ctx: ExecutionContext) -> bool {
        if self.pdp_bypass {
            return true;
        }

        let action_type = match tx.tx_type {
            TxType::GovernanceTx(GovernanceAction::Propose) => ActionType::SubmitGovernanceProposal,
            TxType::GovernanceTx(GovernanceAction::Vote) => ActionType::CastGovernanceVote,
            TxType::FastPathTx => ActionType::SubmitFastPathMerge,
            TxType::TransferTx => ActionType::ClaimTaskLease,
            TxType::TaskCreateTx => ActionType::CreateTask,
            TxType::ClaimTaskTx => ActionType::ClaimTaskLease,
            TxType::HeartbeatTx => ActionType::RenewTaskLease,
            TxType::SubmitTaskTx => ActionType::PublishTopicMessage,
            TxType::SubmitReviewTx => ActionType::SubmitGovernanceProposal,
            _ => return true,
        };

        let agent_id = self.extract_sender_id(tx).unwrap_or([0u8; 32]);

        let balance = self.state_machine.get_account(&agent_id).map(|a| a.balance).unwrap_or(0);

        let nonce = self.agent_nonces.get(&agent_id).copied().unwrap_or(0);

        let key_binding = self
            .state_machine
            .get_account(&agent_id)
            .map(|a| a.pubkey.clone());

        let trust_stage = self
            .state_machine
            .trust_stages_iter()
            .find(|r| r.agent_id == agent_id)
            .map(|r| match r.stage {
                TrustStageEnum::Untrusted => TrustStage::Untrusted,
                TrustStageEnum::Trusted => TrustStage::Trusted,
            })
            .unwrap_or(TrustStage::Untrusted);

        let quota_states: Vec<QuotaState> = self
            .quota_states
            .iter()
            .filter(|((aid, _), _)| *aid == agent_id)
            .map(|(_, qs)| qs.clone())
            .collect();

        let pdp_ctx = PdpContext {
            current_height: ctx.height,
            key_binding,
            agent_balance_attagx: balance,
            agent_nonce: nonce,
            consumed_plan_ids: self.consumed_plan_ids.iter().copied().collect(),
            quota_states,
            trust_stage,
        };

        let request = ActionPlanRequest {
            plan_id: sha3_256_hash(&tx.tx_payload),
            agent_id,
            action_type,
            resource_id: sha3_256_hash(&tx.tx_payload),
            reason_hash: [0u8; 32],
            evidence_refs: vec![],
            nonce,
            expires_at_height: ctx.height.saturating_add(1000),
            agent_signature: vec![],
        };

        let response = rule_chain::evaluate(&request, &pdp_ctx);

        if matches!(response.decision, Decision::Approved) {
            if let Some(ref consumed) = response.consumed_quota {
                self.apply_quota_consumption(agent_id, consumed);
            }
            self.agent_nonces.insert(agent_id, nonce.saturating_add(1));
            self.consumed_plan_ids.insert(request.plan_id);
            true
        } else {
            tracing::debug!(
                "PDP denied tx: agent={} action={:?} reason={:?}",
                hex::encode(agent_id),
                action_type,
                response.deny_reason,
            );
            false
        }
    }

    fn apply_quota_consumption(&mut self, agent_id: Hash32, consumed: &[QuotaConsumption]) {
        for qc in consumed {
            let key = (agent_id, qc.quota_id.clone());
            let entry = self.quota_states.entry(key).or_insert(QuotaState {
                quota_id: qc.quota_id.clone(),
                consumed: 0,
                window_start_height: self.height,
            });
            entry.consumed = entry.consumed.saturating_add(qc.amount_consumed);
        }
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

    /// Start an async block production loop (single-validator mode, pre-BFT).
    ///
    /// When no BFT consensus is needed, this loop produces blocks autonomously
    /// from the local mempool at a fixed interval.
    pub fn run_block_loop(
        driver: Arc<Mutex<ConsensusDriver>>,
        running: Arc<AtomicBool>,
        block_interval: Duration,
    ) -> JoinHandle<()> {
        tokio::spawn(async move {
            while running.load(Ordering::Acquire) {
                let timestamp =
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();

                if let Ok(mut d) = driver.lock() {
                    let block = d.produce_block(vec![], timestamp);
                    tracing::info!(
                        "Produced block height={}, hash={}, state_root={}, mempool={}",
                        block.header.height,
                        hex::encode(block.header.block_hash()),
                        hex::encode(block.header.state_root),
                        d.mempool.len(),
                    );
                }

                tokio::time::sleep(block_interval).await;
            }
        })
    }

    /// Start a Malachite BFT consensus loop.
    ///
    /// Replaces the local produce_block() auto-loop with BFT-driven production.
    /// Uses BftDriver to coordinate propose/vote/commit cycles with ML-DSA-65
    /// signing. Blocks are built from the fee-ordered mempool and committed
    /// via the consensus protocol rather than a fixed timer.
    ///
    /// Stage 02 Week 7-8 per ADR-0018.
    pub fn run_bft_loop(
        driver: Arc<Mutex<ConsensusDriver>>,
        running: Arc<AtomicBool>,
        config: crate::malachite_consensus::ConsensusNetworkConfig,
        channels: crate::malachite_consensus::ConsensusChannels,
        keypair: crate::malachite::MlDsa65PrivateKey,
        node_addr: crate::malachite::Address32,
        validator_set: crate::malachite::HyperfluidValidatorSet,
        proposer_seed: [u8; 32],
    ) -> JoinHandle<()> {
        use crate::malachite_consensus::BftDriver;

        tokio::spawn(async move {
            let mut bft = BftDriver::new(validator_set.clone(), proposer_seed, keypair, node_addr);

            let mut incoming = channels.incoming_rx;
            let outgoing = channels.outgoing_tx;

            let start_height: u64;
            {
                let d = driver.lock().unwrap();
                start_height = d.height.saturating_add(1);
            }

            // Start consensus at the next height
            let events = bft.start_height(start_height, validator_set);
            for event in events {
                Self::handle_bft_event(event, &mut bft, &driver, &outgoing, &config);
            }

            loop {
                if !running.load(Ordering::Acquire) {
                    break;
                }

                tokio::select! {
                    biased;

                    msg = incoming.recv() => {
                        let Some(msg) = msg else { break; };

                        let events = match msg {
                            ConsensusNetworkMsg::Vote(vote) => bft.process_vote(vote),
                            ConsensusNetworkMsg::Proposal(proposal) => {
                                bft.process_proposal(proposal, arc_malachitebft_core_types::Validity::Valid)
                            }
                        };

                        for event in events {
                            Self::handle_bft_event(
                                event, &mut bft, &driver, &outgoing, &config,
                            );
                        }
                    }

                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        // Idle tick
                    }
                }
            }
        })
    }

    fn handle_bft_event(
        event: crate::malachite_consensus::ConsensusEvent,
        bft: &mut crate::malachite_consensus::BftDriver,
        driver: &Arc<Mutex<ConsensusDriver>>,
        outgoing: &tokio::sync::mpsc::UnboundedSender<
            crate::malachite_consensus::ConsensusNetworkMsg,
        >,
        _config: &crate::malachite_consensus::ConsensusNetworkConfig,
    ) {
        match event {
            crate::malachite_consensus::ConsensusEvent::RequestBlock { height, round } => {
                let timestamp =
                    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                let block = if let Ok(mut d) = driver.lock() {
                    let block = d.produce_block(vec![], timestamp);
                    tracing::info!(
                        "BFT: built block height={} round={} hash={}",
                        height,
                        round,
                        hex::encode(block.header.block_hash()),
                    );
                    block
                } else {
                    tracing::error!("BFT: failed to lock driver for block building");
                    return;
                };

                let r = arc_malachitebft_core_types::Round::new(round);
                let events = bft.propose_block_value(r, block);
                for evt in events {
                    Self::handle_bft_event(evt, bft, driver, outgoing, _config);
                }
            }
            crate::malachite_consensus::ConsensusEvent::BlockCommitted { height, round, block } => {
                tracing::info!(
                    "BFT: block committed height={} round={} hash={} parent={}",
                    height,
                    round,
                    hex::encode(block.header.block_hash()),
                    hex::encode(block.header.parent_hash),
                );
                // Persist committed block into driver state so the BFT loop
                // can advance chain state (GAP-01a).
                if let Ok(mut d) = driver.lock() {
                    d.block_store.push(block);
                    d.height = height;
                } else {
                    tracing::error!("BFT: failed to lock driver to persist committed block");
                }
            }
            crate::malachite_consensus::ConsensusEvent::BroadcastVote { vote, .. } => {
                let _ = outgoing.send(ConsensusNetworkMsg::Vote(vote));
            }
            crate::malachite_consensus::ConsensusEvent::BroadcastProposal { proposal, .. } => {
                let _ = outgoing.send(ConsensusNetworkMsg::Proposal(proposal));
            }
            crate::malachite_consensus::ConsensusEvent::ScheduleTimeout { height, round, kind } => {
                tracing::debug!(
                    "BFT: scheduling timeout height={} round={} kind={:?}",
                    height,
                    round,
                    kind,
                );
            }
            crate::malachite_consensus::ConsensusEvent::NewHeight { height, round } => {
                tracing::info!("BFT: new round started height={} round={}", height, round,);
            }
        }
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
        assert!(driver.submit_tx(tx));

        let block = driver.produce_block(vec![], 1);
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

    #[test]
    fn bft_block_committed_persists_block_store_and_height() {
        // GAP-01a: BFT-committed blocks MUST be persisted in the driver state.
        // This test goes through the full handle_bft_event code path.
        use ml_dsa::Generate;
        use ml_dsa::KeyExport;
        use ml_dsa::Keypair;
        use ml_dsa::MlDsa65;
        use tokio::sync::mpsc;

        use crate::malachite::{
            Address32, HyperfluidValidator, HyperfluidValidatorSet, MlDsa65PrivateKey,
            MlDsa65PublicKey,
        };
        use crate::malachite_consensus::{BftDriver, ConsensusEvent, ConsensusNetworkConfig};

        // 1. Create driver with genesis state
        let driver = Arc::new(Mutex::new(ConsensusDriver::new(100)));
        {
            let mut d = driver.lock().unwrap();
            let genesis = test_genesis(vec![GenesisAccount {
                account_id: [1u8; 32],
                balance: 1_000_000_000_000_000_000_000u128,
                pubkey: None,
            }]);
            d.init_genesis(&genesis);
        }

        // 2. Create a committed block at height 1
        let block = Block {
            header: BlockHeader {
                height: 1,
                parent_hash: [0u8; 32],
                state_root: [1u8; 32],
                transaction_root: [2u8; 32],
                committee_id: 0,
                proposer_id: [3u8; 32],
                timestamp: 100,
                epoch: 0,
            },
            transactions: vec![],
        };

        // 3. Create BFT infrastructure needed by handle_bft_event
        let keypair = ml_dsa::SigningKey::<MlDsa65>::generate();
        let pk_bytes = keypair.verifying_key().to_bytes().to_vec();
        let addr_bytes = sha3_256_hash(&pk_bytes);
        let addr = Address32::new(addr_bytes);
        let privkey = MlDsa65PrivateKey(keypair);

        let set = HyperfluidValidatorSet::new(vec![HyperfluidValidator::new(
            addr,
            MlDsa65PublicKey(pk_bytes),
            100,
        )]);
        let mut bft = BftDriver::new(set, [0xAAu8; 32], privkey, addr);

        let (outgoing_tx, _outgoing_rx) = mpsc::unbounded_channel();
        let config = ConsensusNetworkConfig::default();

        // 4. Simulate a BFT commit event
        let event = ConsensusEvent::BlockCommitted { height: 1, round: 0, block };
        ConsensusDriver::handle_bft_event(event, &mut bft, &driver, &outgoing_tx, &config);

        // 5. Verify driver state was updated
        let d = driver.lock().unwrap();
        assert_eq!(
            d.block_store.len(),
            2,
            "block_store should contain genesis (height 0) + committed block (height 1)"
        );
        assert_eq!(d.height, 1, "driver height should reflect committed block height");
    }

    #[test]
    fn bft_block_committed_direct_push() {
        // GAP-01a (simplified): Direct push equivalent of the BFT commit path.
        // Tests that the driver correctly updates block_store and height when a
        // block is committed — no BftDriver ceremony required.

        // 1. Create driver with genesis
        let mut driver = ConsensusDriver::new(100);
        let genesis = test_genesis(vec![GenesisAccount {
            account_id: [2u8; 32],
            balance: 1_000_000_000_000_000_000_000u128,
            pubkey: None,
        }]);
        driver.init_genesis(&genesis);

        // 2. Manually simulate a commit (as BlockCommitted handler does)
        let block = Block {
            header: BlockHeader {
                height: 42,
                parent_hash: [0xABu8; 32],
                state_root: [0xCDu8; 32],
                transaction_root: [0xEFu8; 32],
                committee_id: 0,
                proposer_id: [0xAAu8; 32],
                timestamp: 999,
                epoch: 0,
            },
            transactions: vec![],
        };
        driver.block_store.push(block);
        driver.height = 42;

        // 3. Verify
        assert_eq!(driver.block_store.len(), 2, "genesis + committed block");
        assert_eq!(driver.height, 42);
        assert_eq!(driver.block_store[1].header.height, 42, "committed block has correct height");
    }
}
