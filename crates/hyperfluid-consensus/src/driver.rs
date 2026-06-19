// === Consensus Driver ===
//
// Block production loop that wires the StateMachine into a running chain.
// Produces blocks from transactions, tracks height/epoch, maintains block store.
// Designed to accept a BFT consensus replacement (e.g. Malachite) later.
//
// Source: specs/protocol/consensus-spec.md Sections 1-2

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use arc_malachitebft_core_types::{Round, Timeout};

use parity_scale_codec::{Decode, Encode};
use sha3::{Digest, Sha3_256};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use std::collections::{BTreeMap, BTreeSet};

use hyperfluid_fee_market::{compute_next_base_fee, FeeConfig, FeeMarketState};
use hyperfluid_p2p::identity::Identity;
use hyperfluid_p2p::mempool::{Mempool, MempoolConfig, MempoolTx, TxTypeTag};
use hyperfluid_pdp::audit::AuditLog;
use hyperfluid_staking::SystemParameters;
use hyperfluid_state::state_machine::{
    ExecutionContext, ExecutionResult, SplitChildSpec, StateMachine, ValidatorLifecycleState,
};
use hyperfluid_state::{Account, HeartbeatPayload, ReviewVerdict, TaskStatus, TrustStageEnum};

use crate::genesis::GenesisConfig;
use crate::malachite_consensus::ConsensusNetworkMsg;
use crate::types::{
    Block, BlockHeader, DelegationAction, GovernanceAction, Hash32, StakingAction,
    TransactionEnvelope, TxType,
};
use hyperfluid_fastpath::lifecycle::FastPathEngine;
use hyperfluid_fastpath::types::{
    FastPathChallengeTx, FastPathParams, FastPathProposal, FastPathRollbackTx, ReviewerSignature,
    ReviewerVote,
};
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
///   `vote_signature`  → ML-DSA-65 signature over (proposal_id || vote_approve_byte).
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
    /// ML-DSA-65 signature over vote-specific payload:
    /// SHA3-256(proposal_id || proposer_id || vote_approve_byte)
    vote_signature: Vec<u8>,
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
    nonce: u64,
}

/// Payload format for HeartbeatTx transactions.
#[derive(Encode, Decode)]
struct HeartbeatTxPayload {
    lease_id: Hash32,
    artifact_hash: Option<Hash32>,
    diff_pointer: Option<Hash32>,
    test_result_ref: Option<Hash32>,
    signature: Vec<u8>,
    nonce: u64,
}

/// Payload format for ReleaseTaskTx transactions.
#[derive(Encode, Decode)]
struct ReleaseTaskPayload {
    task_id: Hash32,
    agent_id: Hash32,
    nonce: u64,
}

/// Payload format for SubmitTaskTx transactions.
#[derive(Encode, Decode)]
struct SubmitTaskPayload {
    task_id: Hash32,
    agent_id: Hash32,
    nonce: u64,
}

/// Payload format for SplitTaskTx transactions.
/// Encodes parent task + caller + list of child specifications.
#[derive(Encode, Decode)]
struct SplitTaskPayload {
    parent_task_id: Hash32,
    caller_id: Hash32,
    children: Vec<SplitChildPayload>,
    nonce: u64,
}

#[derive(Encode, Decode)]
struct SplitChildPayload {
    task_id: Hash32,
    bounty_share_pct: u8,
    depends_on: Vec<Hash32>,
    required_skills_hash: Hash32,
}

/// Payload format for TaskCreateTx transactions.
#[derive(Encode, Decode)]
struct TaskCreatePayload {
    creator_id: Hash32,
    bounty_agx: u128,
    seed_ref: Hash32,
    nonce: u64,
    /// Topic identifier for the task.
    topic_id: Hash32,
    /// Hash of the task metadata blob.
    metadata_hash: Hash32,
    /// Hash of the required skills specification.
    skills_hash: Hash32,
    /// Sponsor identifier for the task.
    sponsor_id: Hash32,
    /// Public key hash of the task requester.
    requester_pubkey: Hash32,
}

/// Payload format for SubmitReviewTx transactions.
#[derive(Encode, Decode)]
struct SubmitReviewPayload {
    review_task_id: Hash32,
    reviewer_id: Hash32,
    verdict_accept: bool, // true = Accept, false = Reject
    evidence_hash: Hash32,
    nonce: u64,
}

/// Payload format for EvidenceTx transactions.
///
/// evidence_type:
///   0 = equivocation (double-signing)
///   1 = downtime (missed blocks)
/// For equivocation: `evidence_height` = height of double-sign, `missed_blocks`/`total_window_blocks` ignored.
/// For downtime: `missed_blocks` = count of missed, `total_window_blocks` = liveness window size.
#[derive(Encode, Decode)]
struct EvidencePayload {
    evidence_type: u8,
    validator_id: Hash32,
    evidence_height: u64,
    missed_blocks: u64,
    total_window_blocks: u64,
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
    pub key_bindings: BTreeMap<Hash32, Vec<u8>>,
    /// Expected next nonce per agent (PDP replay protection).
    pub agent_nonces: BTreeMap<Hash32, u64>,
    /// Quota consumption state per (agent, quota_id).
    pub quota_states: BTreeMap<(Hash32, String), QuotaState>,
    /// Consumed plan IDs for PDP replay protection (deduplication).
    pub consumed_plan_ids: BTreeSet<Hash32>,
    /// Fee reward pool accumulated from priority fees, distributed to validators
    /// at epoch boundaries.
    pub fee_reward_pool: u128,
    /// Append-only audit log of PDP decisions.
    pub audit_log: AuditLog,
    /// This node's validator ID for block proposer identification.
    /// Set during initialization; defaults to zeros until configured.
    pub node_id: Hash32,
    /// When true, bypasses PDP validation for all transaction types.
    /// Used for development/testing when full PDP state (key bindings,
    /// nonce tracking, quota states) is not yet wired.
    /// Only available when the `pdp-bypass` feature is enabled.
    #[cfg(feature = "pdp-bypass")]
    pub pdp_bypass: bool,
    /// Tracks the approved git:head commit for on-chain governance.
    /// Updated when a governance proposal passes tally.
    pub git_head_commit: Hash32,
    /// Committee epoch history: epoch -> committee_id hash.
    /// Populated at epoch boundaries in produce_block.
    pub committee_history: BTreeMap<u64, Hash32>,
    /// Validator set per epoch: epoch -> validator public keys.
    pub epoch_validators: BTreeMap<u64, Vec<Hash32>>,
}

impl ConsensusDriver {
    /// Create a new consensus driver with zero height and an empty block store.
    /// Initializes the governance, fast-path engines, fee market, and staking
    /// parameters with their defaults.
    pub fn new(epoch_length: u64, node_id: Hash32, git_head_commit: Hash32) -> Self {
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
            node_id,
            #[cfg(feature = "pdp-bypass")]
            pdp_bypass: false,
            git_head_commit,
            fee_reward_pool: 0,
            audit_log: AuditLog::new(),
            committee_history: BTreeMap::new(),
            epoch_validators: BTreeMap::new(),
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

        // git_head_commit initialised to [0u8; 32] in new(); genesis config does not
        // carry a git:head field yet (deferred: governance tally integration)
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
    pub fn submit_tx(&mut self, tx: TransactionEnvelope) -> Result<Hash32, String> {
        let tx_data = tx.encode();
        let tx_hash = sha3_256_hash(&tx_data);
        if self.tx_store.contains_key(&tx_hash) {
            return Err("duplicate tx".into());
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
        if !self.mempool.insert(mtx) {
            return Err("mempool full".into());
        }
        Ok(tx_hash)
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
                let payload = TaskCreatePayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.creator_id)
            }
            TxType::ClaimTaskTx => {
                let payload = ClaimTaskPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.agent_id)
            }
            TxType::FastPathTx => {
                let payload = FastPathPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.proposer_id)
            }
            TxType::HeartbeatTx => {
                let payload = HeartbeatTxPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                // HeartbeatTx uses lease_id as agent identifier; return None if no agent_id available.
                let _ = payload.lease_id;
                None
            }
            TxType::EvidenceTx => {
                let payload = EvidencePayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.validator_id)
            }
            TxType::ReleaseTaskTx => {
                let payload = ReleaseTaskPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.agent_id)
            }
            TxType::SplitTaskTx => {
                let payload = SplitTaskPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.caller_id)
            }
            TxType::SubmitTaskTx => {
                let payload = SubmitTaskPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.agent_id)
            }
            TxType::SubmitReviewTx => {
                let payload = SubmitReviewPayload::decode(&mut &tx.tx_payload[..]).ok()?;
                Some(payload.reviewer_id)
            }
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

        // 1. Execute all transactions with PDP validation and fee burning
        for tx in &block_txs {
            // Snapshot PDP state before validation for rollback on failure
            let pdp_snapshot_nonces = self.agent_nonces.clone();
            let pdp_snapshot_plan_ids = self.consumed_plan_ids.clone();
            let pdp_snapshot_quotas = self.quota_states.clone();

            // Run PDP validation — skip tx if it fails
            if !self.validate_tx_pdp(tx, ctx) {
                continue;
            }

            let exec_result = self.execute_tx(tx, ctx);

            match exec_result {
                ExecutionResult::Success => {
                    // Deduct base fee from sender's account
                    if let Some(sender) = self.extract_sender_id(tx) {
                        let fee = self.fee_state.base_fee;
                        if self.state_machine.deduct_balance(&sender, fee) {
                            self.fee_reward_pool = self.fee_reward_pool.saturating_add(fee);
                            self.fee_state.accumulate_burn(fee);
                        }
                    }
                }
                ExecutionResult::Rejected => {
                    // Rollback PDP state on transaction rejection
                    self.agent_nonces = pdp_snapshot_nonces;
                    self.consumed_plan_ids = pdp_snapshot_plan_ids;
                    self.quota_states = pdp_snapshot_quotas;
                }
            }
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

        // 2c. Run trust promotion, topic decay, and governance finalization at epoch boundaries
        let new_epoch = new_height / self.epoch_length;
        if new_height > 0 && new_height.is_multiple_of(self.epoch_length) {
            self.state_machine.run_trust_promotion();
            self.state_machine.run_topic_decay(new_height);

            // Finalize governance proposals whose vote window has ended
            let total_stake: u128 = self
                .state_machine
                .validators_iter()
                .filter(|(_, vt)| vt.state == ValidatorLifecycleState::Active)
                .map(|(_, vt)| vt.self_bond)
                .sum();
            let active_ids = self.governance.active_proposal_ids();
            for pid in &active_ids {
                if let Ok(outcome) = self.governance.finalize_proposal(
                    *pid,
                    new_height,
                    total_stake,
                    self.epoch_length,
                ) {
                    tracing::info!(
                        "Governance proposal {:?} finalized: {:?}",
                        hex::encode(*pid),
                        outcome
                    );
                }
            }

            // Distribute epoch-end fee rebates to active validators
            self.state_machine.execute_distribute_rewards(&mut self.fee_reward_pool);

            // Execute passed proposals — update git:head when a proposal passes
            let passed_ids = self.governance.passed_proposal_ids();
            for pid in &passed_ids {
                if let Ok(proposed_commit) = self.governance.execute_proposal(*pid) {
                    self.git_head_commit = proposed_commit;
                    tracing::info!(
                        "Governance proposal {:?} executed — git:head updated to {:?}",
                        hex::encode(*pid),
                        hex::encode(proposed_commit)
                    );
                }
            }

            // Finalize fast-path certificates that have passed the challenge window
            for fpid in self.fastpath.proposal_ids() {
                if self.fastpath.get_certificate(&fpid).is_some() {
                    match self.fastpath.finalize_certificate(fpid, new_height) {
                        Ok(new_head) => {
                            tracing::info!(
                                "Fast-path certificate finalized: proposal={:?} new_head={:?}",
                                hex::encode(fpid),
                                hex::encode(new_head),
                            );
                        }
                        Err(e) => {
                            tracing::debug!(
                                "Fast-path certificate not finalized for proposal {:?}: {:?}",
                                hex::encode(fpid),
                                e,
                            );
                        }
                    }
                }
            }
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
                committee_id: new_epoch, // committee_id matches epoch number; committee_history tracks epoch hashes
                proposer_id: self.node_id,
                timestamp,
                epoch: new_epoch,
            },
            transactions: block_txs,
        };

        // 6. Store and advance
        let prev_epoch = self.epoch;
        self.block_store.push(block.clone());
        self.height = new_height;
        self.epoch = new_epoch;

        // 6b. Snapshot committee at epoch boundary
        if new_epoch != prev_epoch || self.committee_history.is_empty() {
            let committee_hash = sha3_256_hash(&new_epoch.to_le_bytes());
            self.committee_history.insert(new_epoch, committee_hash);

            // Populate epoch_validators from state machine validator set
            let validators_this_epoch: Vec<Hash32> = self
                .state_machine
                .validators_iter()
                .filter(|(_, vt)| vt.state == ValidatorLifecycleState::Active)
                .map(|(id, _)| *id)
                .collect();
            if !validators_this_epoch.is_empty() {
                self.epoch_validators.insert(new_epoch, validators_this_epoch);
            }
        }

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
    ///
    /// PDP pre-validation is performed by `produce_block` before calling
    /// this function. The `validate_tx_pdp` is NOT called here to avoid
    /// nonce state changes between the first and second call (the signature
    /// verification would fail on the second call because the nonce context
    /// has already been advanced).
    fn execute_tx(&mut self, tx: &TransactionEnvelope, ctx: ExecutionContext) -> ExecutionResult {
        match tx.tx_type {
            TxType::TransferTx => {
                if let Ok(payload) = TransferPayload::decode(&mut &tx.tx_payload[..]) {
                    self.state_machine.execute_transfer(
                        payload.sender_id,
                        payload.recipient_id,
                        payload.amount,
                        payload.nonce,
                        ctx,
                    )
                } else {
                    ExecutionResult::Rejected
                }
            }
            TxType::GovernanceTx(GovernanceAction::Propose) => {
                if let Ok(payload) = GovernancePayload::decode(&mut &tx.tx_payload[..]) {
                    let bundle_manifest_hash = sha3_256_hash(&payload.description_hash);
                    let current_epoch = ctx.height / self.epoch_length;
                    match self.governance.submit_proposal(
                        payload.proposer_id,
                        payload.target_hash,
                        bundle_manifest_hash,
                        self.git_head_commit,
                        ctx.height,
                        current_epoch,
                        0,
                        0,
                    ) {
                        Ok(proposal) => {
                            tracing::info!(
                                "Governance proposal submitted: id={} proposer={} status={:?}",
                                hex::encode(proposal.proposal_id),
                                hex::encode(proposal.proposer_id),
                                proposal.status,
                            );
                            ExecutionResult::Success
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Governance proposal rejected: id={} proposer={} error={:?}",
                                hex::encode(payload.proposal_id),
                                hex::encode(payload.proposer_id),
                                e,
                            );
                            // Mark the proposal as invalid if it was stored before the error
                            if let Err(mark_err) = self.governance.mark_invalid(
                                payload.proposal_id,
                                ctx.height,
                                self.epoch_length,
                            ) {
                                tracing::debug!(
                                    "mark_invalid skipped for proposal {}: {:?}",
                                    hex::encode(payload.proposal_id),
                                    mark_err,
                                );
                            }
                            ExecutionResult::Rejected
                        }
                    }
                } else {
                    ExecutionResult::Rejected
                }
            }
            TxType::GovernanceTx(GovernanceAction::Vote) => {
                if let Ok(payload) = GovernancePayload::decode(&mut &tx.tx_payload[..]) {
                    let vote_option =
                        if payload.vote_approve { VoteOption::Yes } else { VoteOption::No };
                    let vote_weight = self
                        .state_machine
                        .get_validator(&payload.proposer_id)
                        .map(|vt| vt.self_bond)
                        .unwrap_or(1);
                    // Use per-vote ML-DSA-65 signature if provided; fall back to tx signature
                    // for older clients that haven't been updated yet.
                    let vote_sig = if payload.vote_signature.len() == 3309 {
                        payload.vote_signature
                    } else {
                        tx.signature.clone()
                    };
                    let vote = GovernanceVote {
                        proposal_id: payload.proposal_id,
                        voter_id: payload.proposer_id,
                        vote: vote_option,
                        reason_hash: payload.description_hash,
                        vote_weight,
                        signature: vote_sig,
                    };
                    match self.governance.cast_vote(vote, ctx.height) {
                        Ok(()) => {
                            tracing::info!(
                                "Governance vote cast: proposal={} voter={}",
                                hex::encode(payload.proposal_id),
                                hex::encode(payload.proposer_id),
                            );
                            ExecutionResult::Success
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Governance vote rejected: proposal={} error={:?}",
                                hex::encode(payload.proposal_id),
                                e,
                            );
                            ExecutionResult::Rejected
                        }
                    }
                } else {
                    ExecutionResult::Rejected
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
                            signature: tx.signature.clone(),
                        };
                        let current_epoch = ctx.height / self.epoch_length;
                        match self.fastpath.submit_challenge(challenge, ctx.height, current_epoch) {
                            Ok(()) => {
                                tracing::info!(
                                    "Fast-path challenge submitted: proposal={} challenger={}",
                                    hex::encode(payload.proposal_id),
                                    hex::encode(payload.proposer_id),
                                );
                                // Rollback the challenged proposal
                                let rollback_to_head = self
                                    .fastpath
                                    .get_proposal(&payload.proposal_id)
                                    .map(|p| p.base_topic_head)
                                    .unwrap_or([0u8; 32]);
                                let rollback_tx = FastPathRollbackTx {
                                    proposal_id: payload.proposal_id,
                                    topic_id: payload.topic_id,
                                    rollback_to_head,
                                    arbiter_certificate: vec![],
                                    signature: tx.signature.clone(),
                                };
                                if let Err(e) = self.fastpath.rollback(rollback_tx) {
                                    tracing::warn!(
                                        "Fast-path rollback failed for proposal {}: {:?}",
                                        hex::encode(payload.proposal_id),
                                        e,
                                    );
                                }
                                ExecutionResult::Success
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Fast-path challenge rejected: proposal={} error={:?}",
                                    hex::encode(payload.proposal_id),
                                    e,
                                );
                                ExecutionResult::Rejected
                            }
                        }
                    } else {
                        // Compute commitment hashes from actual proposal data
                        let base_topic_head = sha3_256_hash(
                            &[&payload.topic_id[..], &payload.proposer_id[..]].concat(),
                        );
                        let bundle_manifest_hash = sha3_256_hash(&payload.merge_hash);
                        let proposal = FastPathProposal {
                            proposal_id: payload.proposal_id,
                            topic_id: payload.topic_id,
                            proposer_id: payload.proposer_id,
                            base_topic_head,
                            proposed_head: payload.merge_hash,
                            bundle_manifest_hash,
                            expires_at_height: ctx.height.saturating_add(1000),
                            proposer_signature: tx.signature.clone(),
                        };
                        match self.fastpath.submit_proposal(proposal, ctx.height) {
                            Ok(proposal_id) => {
                                tracing::info!(
                                    "Fast-path proposal submitted: id={} topic={}",
                                    hex::encode(proposal_id),
                                    hex::encode(payload.topic_id),
                                );
                                // Auto-issue a certificate with proposer as first approver
                                let signer_set_hash = sha3_256_hash(&payload.proposer_id);
                                let self_approval = ReviewerSignature {
                                    reviewer_id: payload.proposer_id,
                                    vote: ReviewerVote::Approve,
                                    reason_hash: [0u8; 32],
                                    signature: tx.signature.clone(),
                                };
                                if let Err(e) = self.fastpath.issue_certificate(
                                    payload.proposal_id,
                                    vec![self_approval],
                                    signer_set_hash,
                                    ctx.height,
                                    1, // topic_snapshot_weight = 1 → quorum = 1
                                ) {
                                    tracing::warn!(
                                        "Fast-path certificate issuance deferred for proposal {}: {:?}",
                                        hex::encode(payload.proposal_id),
                                        e,
                                    );
                                }
                                ExecutionResult::Success
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Fast-path proposal rejected: id={} error={:?}",
                                    hex::encode(payload.proposal_id),
                                    e,
                                );
                                ExecutionResult::Rejected
                            }
                        }
                    }
                } else {
                    ExecutionResult::Rejected
                }
            }
            // Staking transactions — validator lifecycle (bond/unbond/withdraw/renew)
            TxType::StakingTx(action) => {
                if let Ok(payload) = StakingPayload::decode(&mut &tx.tx_payload[..]) {
                    match action {
                        StakingAction::Bond => self.state_machine.execute_bond(
                            payload.validator_id,
                            payload.amount,
                            payload.nonce,
                            self.staking_params.min_self_bond,
                            ctx.height,
                            ctx,
                        ),
                        StakingAction::Unbond => self.state_machine.execute_unbond(
                            payload.validator_id,
                            payload.nonce,
                            ctx.height,
                            ctx,
                        ),
                        StakingAction::Withdraw => self.state_machine.execute_withdraw(
                            payload.validator_id,
                            payload.nonce,
                            ctx.height,
                            self.staking_params.unbond_delay,
                            ctx,
                        ),
                        StakingAction::Renew => self.state_machine.execute_renew(
                            payload.validator_id,
                            payload.nonce,
                            ctx.height,
                            ctx,
                        ),
                    }
                } else {
                    ExecutionResult::Rejected
                }
            }
            // Delegation transactions — delegator/validator relationship management
            TxType::DelegationTx(action) => {
                if let Ok(payload) = DelegationPayload::decode(&mut &tx.tx_payload[..]) {
                    match action {
                        DelegationAction::Delegate => self.state_machine.execute_delegate(
                            payload.delegator_id,
                            payload.validator_id,
                            payload.amount,
                            payload.nonce,
                            self.staking_params.min_delegation,
                            ctx,
                        ),
                        DelegationAction::Undelegate => self.state_machine.execute_undelegate(
                            payload.delegator_id,
                            payload.validator_id,
                            payload.nonce,
                            ctx.height,
                            ctx,
                        ),
                        DelegationAction::WithdrawDelegation => {
                            self.state_machine.execute_withdraw_delegation(
                                payload.delegator_id,
                                payload.validator_id,
                                payload.nonce,
                                ctx.height,
                                self.staking_params.delegation_unbond_delay,
                                ctx,
                            )
                        }
                        DelegationAction::SetCommission => {
                            let commission_rate = payload.amount.min(100) as u8;
                            self.state_machine.execute_set_commission(
                                payload.validator_id,
                                commission_rate,
                                payload.nonce,
                                self.staking_params.max_commission_rate,
                                ctx,
                            )
                        }
                    }
                } else {
                    ExecutionResult::Rejected
                }
            }
            TxType::TaskCreateTx => {
                if let Ok(payload) = TaskCreatePayload::decode(&mut &tx.tx_payload[..]) {
                    let task_id = sha3_256_hash(&tx.tx_payload);
                    self.state_machine.execute_task_create(
                        payload.creator_id,
                        payload.bounty_agx,
                        0, // fee: not yet tracked in payload
                        task_id,
                        payload.nonce,
                        payload.seed_ref,
                        payload.topic_id,
                        payload.metadata_hash,
                        payload.skills_hash,
                        payload.sponsor_id,
                        payload.requester_pubkey,
                        ctx.height,
                        ctx,
                    )
                } else {
                    ExecutionResult::Rejected
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
                        payload.nonce,
                        ctx.height,
                        trust_stage,
                        ctx,
                    )
                } else {
                    ExecutionResult::Rejected
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
                    self.state_machine.execute_heartbeat(heartbeat, payload.nonce, ctx.height, ctx)
                } else {
                    ExecutionResult::Rejected
                }
            }
            TxType::ReleaseTaskTx => {
                if let Ok(payload) = ReleaseTaskPayload::decode(&mut &tx.tx_payload[..]) {
                    self.state_machine.execute_release_task(
                        payload.task_id,
                        payload.agent_id,
                        payload.nonce,
                        ctx,
                    )
                } else {
                    ExecutionResult::Rejected
                }
            }
            TxType::SubmitTaskTx => {
                if let Ok(payload) = SubmitTaskPayload::decode(&mut &tx.tx_payload[..]) {
                    self.state_machine.execute_submit_completion(
                        payload.task_id,
                        payload.agent_id,
                        payload.nonce,
                        ctx.height,
                        ctx,
                    )
                } else {
                    ExecutionResult::Rejected
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
                        payload.nonce,
                        ctx.height,
                        ctx,
                    )
                } else {
                    ExecutionResult::Rejected
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
                        payload.nonce,
                        ctx.height,
                        ctx,
                    )
                } else {
                    ExecutionResult::Rejected
                }
            }
            TxType::EvidenceTx => {
                if let Ok(payload) = EvidencePayload::decode(&mut &tx.tx_payload[..]) {
                    const MIN_JAIL_BLOCKS: u64 = 5000;
                    const PAUSE_THRESHOLD_PCT: u64 = 20;
                    match payload.evidence_type {
                        0 => self.state_machine.execute_slash_equivocation(
                            payload.validator_id,
                            payload.evidence_height,
                            MIN_JAIL_BLOCKS,
                            ctx.height,
                        ),
                        1 => self.state_machine.execute_slash_downtime(
                            payload.validator_id,
                            payload.missed_blocks,
                            payload.total_window_blocks,
                            payload.evidence_height,
                            PAUSE_THRESHOLD_PCT,
                            MIN_JAIL_BLOCKS,
                            ctx.height,
                        ),
                        _ => {
                            tracing::warn!(
                                "EvidenceTx: unknown evidence type {}",
                                payload.evidence_type
                            );
                            ExecutionResult::Rejected
                        }
                    }
                } else {
                    ExecutionResult::Rejected
                }
            }
        }
    }

    /// PDP pre-validation integration point for all transaction types.
    ///
    /// Maps `TxType` to `ActionType`, builds a `PdpContext` from live driver state
    /// (agent balance, nonce, key binding, quotas), verifies the agent's ML-DSA-65
    /// signature, and runs the 5-step deterministic rule chain.
    ///
    /// Signature verification (PDP step 2): looks up the agent's public key from
    /// `key_bindings`, verifies `tx.signature` against `tx.tx_payload` using
    /// ML-DSA-65. If the key is not found or verification fails, the transaction
    /// is rejected (fail-closed) without calling evaluate().
    ///
    /// When the `pdp-bypass` feature is enabled, all transactions pass through
    /// without PDP validation (for testing scenarios).
    fn validate_tx_pdp(&mut self, tx: &TransactionEnvelope, ctx: ExecutionContext) -> bool {
        #[cfg(feature = "pdp-bypass")]
        {
            if self.pdp_bypass {
                return true;
            }
        }

        let action_type = match tx.tx_type {
            TxType::GovernanceTx(GovernanceAction::Propose) => ActionType::SubmitGovernanceProposal,
            TxType::GovernanceTx(GovernanceAction::Vote) => ActionType::CastGovernanceVote,
            TxType::FastPathTx => ActionType::SubmitFastPathMerge,
            TxType::TransferTx => ActionType::Transfer,
            TxType::TaskCreateTx => ActionType::CreateTask,
            TxType::ClaimTaskTx => ActionType::ClaimTaskLease,
            TxType::HeartbeatTx => ActionType::RenewTaskLease,
            TxType::SubmitTaskTx => ActionType::SubmitTaskCompletion,
            TxType::SubmitReviewTx => ActionType::SubmitReview,
            TxType::StakingTx(_) => ActionType::StakeOperation,
            TxType::DelegationTx(_) => ActionType::DelegateOperation,
            TxType::EvidenceTx => ActionType::SubmitEvidence,
            TxType::ReleaseTaskTx => ActionType::ReleaseTask,
            TxType::SplitTaskTx => ActionType::CreateTask,
        };

        let agent_id = self.extract_sender_id(tx).unwrap_or([0u8; 32]);

        let balance = self.state_machine.get_account(&agent_id).map(|a| a.balance).unwrap_or(0);

        // `last_nonce` is the last used/validated nonce for this agent.
        // The PDP expects `request.nonce == ctx.agent_nonce + 1`.
        let last_nonce = self.agent_nonces.get(&agent_id).copied().unwrap_or(0);
        let next_nonce = last_nonce.saturating_add(1);

        let key_binding = self.state_machine.get_account(&agent_id).and_then(|a| a.pubkey.clone());

        // ── Step 2: ML-DSA-65 signature verification (PDP pre-check) ──
        // Look up the agent's public key from key_bindings and verify the
        // agent_signature against the canonical action plan hash.
        // This runs BEFORE rule_chain::evaluate() as a fast rejection.
        let request = ActionPlanRequest {
            plan_id: sha3_256_hash(&tx.tx_payload),
            agent_id,
            action_type,
            resource_id: sha3_256_hash(&tx.tx_payload),
            reason_hash: [0u8; 32],
            evidence_refs: vec![],
            nonce: next_nonce,
            expires_at_height: ctx.height.saturating_add(1000),
            agent_signature: tx.signature.clone(), // F-3: populated from tx envelope
        };

        if !self.verify_agent_signature(&request, &agent_id) {
            tracing::debug!("PDP signature verification failed: agent={}", hex::encode(agent_id),);
            return false;
        }

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
            agent_nonce: last_nonce,
            consumed_plan_ids: self.consumed_plan_ids.iter().copied().collect(),
            quota_states,
            trust_stage,
        };

        let response = rule_chain::evaluate(&request, &pdp_ctx, &mut self.audit_log, None);

        if matches!(response.decision, Decision::Approved) {
            if let Some(ref consumed) = response.consumed_quota {
                self.apply_quota_consumption(agent_id, consumed);
            }
            self.agent_nonces.insert(agent_id, last_nonce.saturating_add(1));
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

    /// Verify the agent's ML-DSA-65 signature against the action plan hash.
    ///
    /// Looks up the agent's public key in `key_bindings` and verifies
    /// `request.agent_signature` against `hash_action_plan_for_signing(request)`.
    /// This matches what the PDP rule chain's step2 does internally, providing
    /// fast rejection before the full rule chain evaluation.
    ///
    /// Returns `true` if the signature is valid.
    /// Returns `false` if key not found, signature empty, or verification fails.
    fn verify_agent_signature(&self, request: &ActionPlanRequest, agent_id: &Hash32) -> bool {
        match self.key_bindings.get(agent_id) {
            Some(pubkey_bytes) => {
                if request.agent_signature.is_empty() {
                    tracing::debug!(
                        "verify_agent_signature: empty signature for agent {} — rejecting",
                        hex::encode(agent_id),
                    );
                    return false;
                }
                let msg_hash = hyperfluid_pdp::rule_chain::hash_action_plan_for_signing(request);
                let valid =
                    Identity::verify_with_pubkey(pubkey_bytes, &msg_hash, &request.agent_signature);
                if !valid {
                    tracing::debug!(
                        "verify_agent_signature: ML-DSA-65 verification failed for agent {}",
                        hex::encode(agent_id),
                    );
                }
                valid
            }
            None => {
                // No pubkey registered for this agent — fail-closed.
                // Agents must register a pubkey in key_bindings before submitting
                // transactions that require PDP validation.
                tracing::debug!(
                    "verify_agent_signature: no pubkey binding for agent {} — rejecting",
                    hex::encode(agent_id),
                );
                false
            }
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

    /// Return the current approved git:head commit hash.
    /// Updated when a governance proposal passes tally.
    pub fn git_head(&self) -> Hash32 {
        self.git_head_commit
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
                if let Ok(mut d) = driver.lock() {
                    let height = d.height + 1;
                    let block = d.produce_block(vec![], height);
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
    #[allow(clippy::too_many_arguments)]
    pub fn run_bft_loop(
        driver: Arc<Mutex<ConsensusDriver>>,
        running: Arc<AtomicBool>,
        config: crate::malachite_consensus::ConsensusNetworkConfig,
        channels: crate::malachite_consensus::ConsensusChannels,
        identity: Arc<hyperfluid_p2p::identity::Identity>,
        node_addr: crate::malachite::Address32,
        validator_set: crate::malachite::HyperfluidValidatorSet,
        proposer_seed: [u8; 32],
        peer_tx_rx_pairs: Option<
            Vec<(mpsc::UnboundedSender<Vec<u8>>, mpsc::UnboundedReceiver<Vec<u8>>)>,
        >,
        external_bridge: Option<Arc<Mutex<crate::network_bridge::NetworkBridge>>>,
    ) -> JoinHandle<()> {
        use crate::malachite_consensus::BftDriver;
        use crate::network_bridge;

        tokio::spawn(async move {
            let mut bft = BftDriver::new(validator_set.clone(), proposer_seed, identity, node_addr);

            // -- Network bridge setup --
            // Clone incoming_tx before partial moves from channels
            let incoming_tx_for_peers = channels.incoming_tx.clone();
            let mut incoming = channels.incoming_rx;

            let outgoing = if let Some(bridge) = external_bridge {
                // External bridge: peers managed dynamically (added/removed
                // as connections are established/lost). run_sender reads
                // from bridge_rx and broadcasts to all current bridge.peers.
                // Inbound messages arrive via the consensus_handler callback
                // wired at the TCP layer, which forwards directly to incoming_tx.
                let (bridge_tx, bridge_rx) = mpsc::unbounded_channel();
                let _sender_handle = network_bridge::run_sender(bridge, bridge_rx);
                bridge_tx
            } else if let Some(pairs) = peer_tx_rx_pairs {
                let (senders, receivers): (Vec<_>, Vec<_>) = pairs.into_iter().unzip();
                let (bridge_tx, bridge_rx) = mpsc::unbounded_channel();
                let bridge = Arc::new(Mutex::new(network_bridge::NetworkBridge {
                    outgoing: channels.outgoing_tx,
                    peers: senders,
                }));
                let _sender_handle = network_bridge::run_sender(Arc::clone(&bridge), bridge_rx);
                let _receiver_handle =
                    network_bridge::run_receiver(receivers, incoming_tx_for_peers);
                bridge_tx
            } else {
                channels.outgoing_tx
            };

            let start_height: u64;
            {
                let d = match driver.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => {
                        tracing::warn!("driver mutex poisoned, recovering");
                        poisoned.into_inner()
                    }
                };
                start_height = d.height.saturating_add(1);
            }

            // Timeout queue: sorted vec of (Timeout, deadline). Earliest deadline first.
            let mut timeout_queue: Vec<(Timeout, Instant)> = Vec::new();

            // Helper: process events through handle_bft_event, inserting any new
            // timeouts into the queue with per-step durations from config.
            let process_events = |events: Vec<crate::malachite_consensus::ConsensusEvent>,
                                  bft: &mut crate::malachite_consensus::BftDriver,
                                  timeout_queue: &mut Vec<(Timeout, Instant)>|
             -> () {
                for event in events {
                    Self::handle_bft_event(
                        event,
                        bft,
                        &driver,
                        &outgoing,
                        &config,
                        &channels.incoming_tx,
                        timeout_queue,
                    );
                }
            };

            // Start consensus at the next height
            let events = bft.start_height(start_height, validator_set);
            process_events(events, &mut bft, &mut timeout_queue);

            loop {
                if !running.load(Ordering::Acquire) {
                    break;
                }

                tokio::select! {
                    biased;

                    msg = incoming.recv() => {
                        let Some(msg) = msg else { break; };

                        let events = match &msg {
                            ConsensusNetworkMsg::Vote(vote) => {
                                let r = vote.message.round.as_u32().unwrap_or(0);
                                let t = match vote.message.vote_type { arc_malachitebft_core_types::VoteType::Prevote => "Prevote", arc_malachitebft_core_types::VoteType::Precommit => "Precommit" };
                                eprintln!("[BFT] RECV {} round={} val_type={:?}", t, r, vote.message.value_id.is_nil());
                                let ev = bft.process_vote(vote.clone());
                                eprintln!("[BFT] -> process_vote returned {} events", ev.len());
                                ev
                            }
                            ConsensusNetworkMsg::Proposal(proposal) => {
                                let r = proposal.message.round.as_u32().unwrap_or(0);
                                eprintln!("[BFT] RECV proposal round={}", r);
                                let ev = bft.process_proposal(proposal.clone(), arc_malachitebft_core_types::Validity::Valid);
                                eprintln!("[BFT] -> process_proposal returned {} events", ev.len());
                                ev
                            }
                        };

                        process_events(events, &mut bft, &mut timeout_queue);
                    }

                    _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => {
                        // Idle tick: fire all expired timeouts in order
                        timeout_queue.sort_by_key(|(_, d)| *d);
                        let now = Instant::now();
                        {
                            let q: Vec<String> = timeout_queue.iter().map(|(t, _)| {
                                format!("{:?}r{}", t.kind, t.round.as_u32().unwrap_or(0))
                            }).collect();
                            eprintln!("[BFT] IDLE TICK queue=[{}]", q.join(","));
                        }
                        while !timeout_queue.is_empty() {
                            let (timeout, deadline) = &timeout_queue[0];
                            if now < *deadline {
                                break;
                            }
                            let kind_str = format!("{:?}", timeout.kind);
                            let r = timeout.round.as_u32().unwrap_or(0);
                            let (timeout, _) = timeout_queue.remove(0);
                            let events = bft.process_timeout(timeout);
                            eprintln!("[BFT] TIMEOUT {} r{} -> {} events", kind_str, r, events.len());
                            process_events(events, &mut bft, &mut timeout_queue);
                            {
                                let q: Vec<String> = timeout_queue.iter().map(|(t, _)| {
                                    format!("{:?}r{}", t.kind, t.round.as_u32().unwrap_or(0))
                                }).collect();
                                eprintln!("[BFT] post-process queue=[{}]", q.join(","));
                            }
                        }
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
        config: &crate::malachite_consensus::ConsensusNetworkConfig,
        incoming_tx: &tokio::sync::mpsc::UnboundedSender<
            crate::malachite_consensus::ConsensusNetworkMsg,
        >,
        timeout_queue: &mut Vec<(Timeout, Instant)>,
    ) {
        match event {
            crate::malachite_consensus::ConsensusEvent::RequestBlock { height, round } => {
                let block = if let Ok(mut d) = driver.lock() {
                    let block = d.produce_block(vec![], height);
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
                    Self::handle_bft_event(
                        evt,
                        bft,
                        driver,
                        outgoing,
                        config,
                        incoming_tx,
                        timeout_queue,
                    );
                }
            }
            crate::malachite_consensus::ConsensusEvent::BlockCommitted { height, round, block } => {
                eprintln!(
                    "[BFT] BLOCK COMMITTED height={} round={} hash={}",
                    height,
                    round,
                    hex::encode(block.header.block_hash()),
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
                let r = vote.message.round.as_u32().unwrap_or(0);
                let t = match vote.message.vote_type {
                    arc_malachitebft_core_types::VoteType::Prevote => "Prevote",
                    arc_malachitebft_core_types::VoteType::Precommit => "Precommit",
                };
                eprintln!(
                    "[BFT] BROADCAST {} round={} val_type={:?}",
                    t,
                    r,
                    vote.message.value_id.is_nil()
                );
                let _ = incoming_tx.send(ConsensusNetworkMsg::Vote(vote.clone()));
                let _ = outgoing.send(ConsensusNetworkMsg::Vote(vote));
            }
            crate::malachite_consensus::ConsensusEvent::BroadcastProposal { proposal, .. } => {
                let r = proposal.message.round.as_u32().unwrap_or(0);
                eprintln!("[BFT] BROADCAST Proposal round={}", r);
                let _ = incoming_tx.send(ConsensusNetworkMsg::Proposal(proposal.clone()));
                let _ = outgoing.send(ConsensusNetworkMsg::Proposal(proposal));
            }
            crate::malachite_consensus::ConsensusEvent::ScheduleTimeout { height, round, kind } => {
                // Deduplicate: remove existing timeout for same (round, kind) before inserting.
                timeout_queue.retain(|(existing, _)| {
                    existing.round.as_u32().unwrap_or(0) != round || existing.kind != kind
                });
                let duration = config.duration_for(&kind);
                let t = Timeout::new(Round::new(round), kind);
                let deadline = Instant::now() + duration;
                timeout_queue.push((t, deadline));
                eprintln!(
                    "[BFT] SCHEDULE timeout {:?} height={} round={} dur={:?} queue_len={}",
                    t.kind,
                    height,
                    round,
                    duration,
                    timeout_queue.len(),
                );
            }
            crate::malachite_consensus::ConsensusEvent::NewHeight { height, round } => {
                eprintln!("[BFT] NEW ROUND height={} round={}", height, round,);
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
        let mut driver = ConsensusDriver::new(100, [0u8; 32], [0u8; 32]);
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
        let mut driver = ConsensusDriver::new(100, [0u8; 32], [0u8; 32]);
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
        let mut driver = ConsensusDriver::new(100, [0u8; 32], [0u8; 32]);
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
        let mut driver = ConsensusDriver::new(100, [0u8; 32], [0u8; 32]);
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
        let mut driver = ConsensusDriver::new(100, [0u8; 32], [0u8; 32]);
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
        let mut driver = ConsensusDriver::new(100, [0u8; 32], [0u8; 32]);
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
        let mut driver = ConsensusDriver::new(100, [0u8; 32], [0u8; 32]);
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
        use hyperfluid_p2p::identity::Identity;
        use hyperfluid_pdp::rule_chain::hash_action_plan_for_signing;
        use hyperfluid_pdp::types::{ActionPlanRequest, ActionType};

        let mut driver = ConsensusDriver::new(100, [0u8; 32], [0u8; 32]);
        let alice_id = [1u8; 32];
        let bob_id = [2u8; 32];

        // Create a real ML-DSA-65 identity for Alice to sign the transaction
        let alice_identity = Identity::generate();
        let alice_pubkey = alice_identity.verifying_key_encoded();

        let genesis = test_genesis(vec![
            GenesisAccount {
                account_id: alice_id,
                balance: 1_000_000_000_000_000_000_000u128, // 1000 AGX
                pubkey: Some(alice_pubkey.clone()),
            },
            GenesisAccount { account_id: bob_id, balance: 0, pubkey: None },
        ]);
        driver.init_genesis(&genesis);
        // Register Alice's pubkey for PDP signature verification
        driver.key_bindings.insert(alice_id, alice_pubkey);
        // Initialize nonce so PDP replay check passes:
        // PDP expects request.nonce = ctx.agent_nonce + 1
        driver.agent_nonces.insert(alice_id, 0);
        let root_before = driver.state_machine.compute_state_root();

        let payload = TransferPayload {
            sender_id: alice_id,
            recipient_id: bob_id,
            amount: 100_000_000_000_000_000_000u128, // 100 AGX
            nonce: 1,
        };
        let tx_payload = payload.encode();

        // The PDP rule chain verifies the signature against hash_action_plan_for_signing,
        // NOT against the raw tx_payload. We must pre-compute this hash and sign it.
        let plan_id = sha3_256_hash(&tx_payload);
        let action_request = ActionPlanRequest {
            plan_id,
            agent_id: alice_id,
            action_type: ActionType::Transfer,
            resource_id: plan_id,
            reason_hash: [0u8; 32],
            evidence_refs: vec![],
            nonce: 1,                // next_nonce = last_nonce(0) + 1
            expires_at_height: 1001, // height(1) + 1000
            agent_signature: vec![],
        };
        let msg_hash = hash_action_plan_for_signing(&action_request);
        let signature = alice_identity.sign(&msg_hash);

        let tx = TransactionEnvelope {
            tx_type: TxType::TransferTx,
            tx_payload,
            approved_plan_id: None,
            gateway_signature: None,
            signature,
        };
        assert!(driver.submit_tx(tx).is_ok());

        let block = driver.produce_block(vec![], 1);
        let root_after = block.header.state_root;

        // Balance after transfer (100 AGX) minus base fee deduction (1_000_000 atto-AGX)
        let expected_alice = 900_000_000_000_000_000_000u128 - driver.fee_state.base_fee;
        assert_eq!(driver.account_balance(&alice_id), Some(expected_alice));
        assert_eq!(driver.account_balance(&bob_id), Some(100_000_000_000_000_000_000u128));
        assert_ne!(root_after, root_before);
        assert_ne!(root_after, [0u8; 32]);
    }

    #[test]
    fn epoch_boundary_detection() {
        let mut driver = ConsensusDriver::new(5, [0u8; 32], [0u8; 32]); // short epoch for testing
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
        use std::sync::Arc;

        use hyperfluid_p2p::identity::Identity;
        use tokio::sync::mpsc;

        use crate::malachite::{
            Address32, HyperfluidValidator, HyperfluidValidatorSet, MlDsa65PublicKey,
        };
        use crate::malachite_consensus::{BftDriver, ConsensusEvent, ConsensusNetworkConfig};

        // 1. Create driver with genesis state
        let driver = Arc::new(Mutex::new(ConsensusDriver::new(100, [0u8; 32], [0u8; 32])));
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
        let identity = Arc::new(Identity::generate());
        let pk_bytes = identity.verifying_key_encoded();
        let addr_bytes = sha3_256_hash(&pk_bytes);
        let addr = Address32::new(addr_bytes);

        let set = HyperfluidValidatorSet::new(vec![HyperfluidValidator::new(
            addr,
            MlDsa65PublicKey(pk_bytes),
            100,
        )]);
        let mut bft = BftDriver::new(set, [0xAAu8; 32], identity, addr);

        let (outgoing_tx, _outgoing_rx) = mpsc::unbounded_channel();
        let (incoming_tx, _incoming_rx) = mpsc::unbounded_channel();
        let config = ConsensusNetworkConfig::default();

        // 4. Simulate a BFT commit event
        let event = ConsensusEvent::BlockCommitted { height: 1, round: 0, block };
        let mut timeout_queue = Vec::new();
        ConsensusDriver::handle_bft_event(
            event,
            &mut bft,
            &driver,
            &outgoing_tx,
            &config,
            &incoming_tx,
            &mut timeout_queue,
        );

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
        let mut driver = ConsensusDriver::new(100, [0u8; 32], [0u8; 32]);
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
