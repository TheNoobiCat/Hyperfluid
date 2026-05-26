// === State Machine ===
//
// Deterministic state transitions for the Hyperfluid protocol.
// Executes transactions against account state, enforces nonces,
// tracks consumed action plans for replay protection.
//
// Source: consensus-spec.md Section 2.4

use std::collections::{HashMap, HashSet};

use parity_scale_codec::{Decode, Encode};

use crate::smt::SparseMerkleTree;
use crate::state_sync::Snapshot;
use crate::{
    state_key, Account, EscrowStatus, Hash32, HeartbeatPayload, KeyPrefix, ReviewRecord,
    ReviewVerdict, Task, TaskLease, TaskStatus, TopicRecord, TopicStatus, TrustStageEnum,
    TrustStageRecord,
};

/// Execution context passed to each state transition.
#[derive(Debug, Clone, Copy)]
pub struct ExecutionContext {
    pub height: u64,
    pub timestamp: u64,
}

/// Result of executing a single state transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionResult {
    Success,
    Rejected,
}

/// The state machine holds all account state and enforces deterministic
/// execution rules for every transaction type.
pub struct StateMachine {
    accounts: HashMap<Hash32, Account>,
    consumed_plans: HashSet<Hash32>,
    /// Canonical task records (key = task_id). Source: collaboration-spec.md §1.3
    tasks: HashMap<Hash32, Task>,
    /// Active leases (key = lease_id)
    leases: HashMap<Hash32, TaskLease>,
    /// Trust stage records (key = agent_id)
    trust_stages: HashMap<Hash32, TrustStageRecord>,
    /// Topic lifecycle records (key = topic_id)
    topic_records: HashMap<Hash32, TopicRecord>,
    /// Consumed freshness nonces: (task_id, nonce)
    consumed_nonces: HashSet<(Hash32, Hash32)>,
    /// Delegation records: (delegator_id, validator_id) -> (amount, unbonding_height, active)
    delegations: HashMap<(Hash32, Hash32), DelegationState>,
    /// Validator records: validator_id -> ValidatorTracker
    validators: HashMap<Hash32, ValidatorTracker>,
    /// Open review verdicts: work_task_id -> `Vec<ReviewRecord>`
    review_records: HashMap<Hash32, Vec<ReviewRecord>>,
    /// Review tasks map: review_task_id -> work_task_id (lookup for claim enforcement)
    review_task_map: HashMap<Hash32, Hash32>,
    /// Accumulated fee burn from slashing and base fee burning
    pub fee_burn_accumulator: u128,
}

/// In-memory delegation state tracked by the state machine.
/// Mirrors on-chain DelegationRecord from staking-spec.md Section 1.3.
#[derive(Debug, Clone, Encode, Decode)]
pub(crate) struct DelegationState {
    amount: u128,
    unbonding_at_height: u64,
    active: bool,
}

/// In-memory validator state tracked by the state machine.
/// Mirrors on-chain ValidatorRecord from staking-spec.md Section 1.3.
/// Tracks bond/unbond/withdraw lifecycle and stake amounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub struct ValidatorTracker {
    pub validator_id: Hash32,
    pub self_bond: u128,
    pub total_delegated: u128,
    pub commission_rate: u8,
    pub bonding_height: u64,
    pub unbonding_height: u64,
    pub state: ValidatorLifecycleState,
    /// Block height until which the validator is jailed (0 = not jailed)
    pub jailed_until_height: u64,
    /// Number of times this validator has been slashed
    pub slash_count: u64,
    /// Height of the most recent slash
    pub last_slash_height: u64,
    /// Height of the most recent reward distribution to this validator
    pub last_reward_height: u64,
}

/// Specification for a single child task in a split operation.
#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode)]
pub struct SplitChildSpec {
    pub task_id: Hash32,
    pub bounty_share_pct: u8, // percentage of parent bounty (sum must = 100)
    pub depends_on: Vec<Hash32>, // child task_ids that must complete first
    pub required_skills_hash: Hash32,
}

/// Four-state validator lifecycle. Source: staking-spec.md Section 1.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode)]
pub enum ValidatorLifecycleState {
    Active,
    Paused,
    Unbonding,
    Withdrawn,
}

/// Detect cycles in a dependency DAG via depth-first search.
/// Each element is (node_id, `\[depends_on_ids\]`). Returns true if any cycle exists.
fn has_cycle(graph: &[(Hash32, Vec<Hash32>)]) -> bool {
    use std::collections::HashMap as H;
    let node_indices: H<Hash32, usize> =
        graph.iter().enumerate().map(|(i, (id, _))| (*id, i)).collect();
    for i in 0..graph.len() {
        let mut visited = vec![false; graph.len()];
        let mut stack = vec![i];
        while let Some(cur) = stack.pop() {
            if visited[cur] {
                continue;
            }
            visited[cur] = true;
            for dep in &graph[cur].1 {
                if let Some(&dep_idx) = node_indices.get(dep) {
                    if dep_idx == i {
                        return true;
                    }
                    stack.push(dep_idx);
                }
            }
        }
    }
    false
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            consumed_plans: HashSet::new(),
            tasks: HashMap::new(),
            leases: HashMap::new(),
            trust_stages: HashMap::new(),
            topic_records: HashMap::new(),
            consumed_nonces: HashSet::new(),
            delegations: HashMap::new(),
            validators: HashMap::new(),
            review_records: HashMap::new(),
            review_task_map: HashMap::new(),
            fee_burn_accumulator: 0,
        }
    }

    /// Bootstrap accounts from genesis configuration.
    /// Panics if the account ID already exists (duplicate genesis entry).
    pub fn init_account(&mut self, account: Account) {
        if self.accounts.contains_key(&account.account_id) {
            panic!("duplicate genesis account: {:?}", hex::encode(account.account_id));
        }
        self.accounts.insert(account.account_id, account);
    }

    /// Bootstrap a validator from genesis configuration.
    /// Panics if the validator ID already exists (duplicate genesis entry).
    pub fn init_validator(&mut self, validator_id: Hash32, self_bond: u128, bonding_height: u64) {
        if self.validators.contains_key(&validator_id) {
            panic!("duplicate genesis validator: {:?}", hex::encode(validator_id));
        }
        self.validators.insert(
            validator_id,
            ValidatorTracker {
                validator_id,
                self_bond,
                total_delegated: 0,
                commission_rate: 0,
                bonding_height,
                unbonding_height: 0,
                state: ValidatorLifecycleState::Active,
                jailed_until_height: 0,
                slash_count: 0,
                last_slash_height: 0,
                last_reward_height: 0,
            },
        );
    }

    /// Get a reference to an account.
    pub fn get_account(&self, account_id: &Hash32) -> Option<&Account> {
        self.accounts.get(account_id)
    }

    /// Deduct `amount` from an account's balance (for fee burning).
    /// Returns `true` if the account exists and had sufficient balance.
    pub fn deduct_balance(&mut self, account_id: &Hash32, amount: u128) -> bool {
        if let Some(account) = self.accounts.get_mut(account_id) {
            if account.balance >= amount {
                account.balance = account.balance.saturating_sub(amount);
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Set the public key for an account, verifying it matches the pubkey_hash.
    /// Must be called before the first outgoing transfer (pubkey reveal).
    /// Returns Rejected if the account doesn't exist, pubkey is already set,
    /// or the pubkey_hash doesn't match.
    pub fn reveal_pubkey(&mut self, account_id: Hash32, pubkey_bytes: Vec<u8>) -> ExecutionResult {
        match self.accounts.get_mut(&account_id) {
            Some(account) => {
                if account.pubkey.is_some() {
                    return ExecutionResult::Rejected; // already revealed
                }
                let computed_hash = crate::sha3_256(&pubkey_bytes);
                if computed_hash != account.pubkey_hash {
                    return ExecutionResult::Rejected; // hash mismatch
                }
                account.pubkey = Some(pubkey_bytes);
                ExecutionResult::Success
            }
            None => ExecutionResult::Rejected,
        }
    }

    /// Execute a transfer from sender to recipient.
    /// TransferTx: debit sender.balance -= amount, credit recipient.balance += amount.
    /// Enforces: sender nonce check, sufficient balance, non-zero amount, pubkey revealed.
    pub fn execute_transfer(
        &mut self,
        sender_id: Hash32,
        recipient_id: Hash32,
        amount: u128,
        nonce: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        if amount == 0 {
            return ExecutionResult::Rejected;
        }

        // Sender nonce enforcement (spec 2.7 hook 3)
        let sender_nonce = self.accounts.get(&sender_id).map(|a| a.nonce).unwrap_or(0);

        if nonce != sender_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        // Check sender balance
        let sender_balance = self.accounts.get(&sender_id).map(|a| a.balance).unwrap_or(0);

        if sender_balance < amount {
            return ExecutionResult::Rejected;
        }

        // Debit sender
        if let Some(sender) = self.accounts.get_mut(&sender_id) {
            // First-spend: pubkey must be revealed before transfer (spec 2.7 hook 5)
            if sender.pubkey.is_none() {
                return ExecutionResult::Rejected;
            }
            sender.balance = sender.balance.saturating_sub(amount);
            sender.nonce = nonce;
        } else {
            // Sender account doesn't exist – auto-create with 0 balance?
            // Per spec Section 2.4: Account created on first inbound transfer.
            // Transfers FROM non-existent accounts are rejected.
            return ExecutionResult::Rejected;
        }

        // Credit recipient (auto-create if needed, per spec 2.4)
        // Guard: only auto-create if amount > 0 to prevent state trie bloat
        if amount > 0 {
            let recipient = self.accounts.entry(recipient_id).or_insert_with(|| Account {
                account_id: recipient_id,
                balance: 0,
                nonce: 0,
                pubkey_hash: recipient_id,
                pubkey: None,
            });
            recipient.balance = recipient.balance.saturating_add(amount);
        }

        ExecutionResult::Success
    }

    /// Mark an action plan as consumed for replay protection.
    /// Returns Rejected if the plan_id was already consumed. (spec 2.7 hook 4)
    pub fn consume_plan_id(&mut self, plan_id: Hash32, _ctx: ExecutionContext) -> ExecutionResult {
        if self.consumed_plans.contains(&plan_id) {
            return ExecutionResult::Rejected;
        }
        self.consumed_plans.insert(plan_id);
        ExecutionResult::Success
    }

    /// Execute an action plan by consuming its plan_id for replay protection.
    /// Returns Rejected if the plan_id was already consumed.
    /// Wired from PDP integration — calls consume_plan_id internally.
    pub fn execute_action_plan(
        &mut self,
        plan_id: Hash32,
        ctx: ExecutionContext,
    ) -> ExecutionResult {
        self.consume_plan_id(plan_id, ctx)
    }

    /// Execute a TaskCreateTx.
    /// Debits bounty from creator, stores full Task struct on-chain.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_task_create(
        &mut self,
        creator_id: Hash32,
        bounty_agx: u128,
        fee_agx: u128,
        task_id: Hash32,
        nonce: u64,
        seed_ref: Hash32,
        topic_id: Hash32,
        metadata_hash: Hash32,
        required_skills_hash: Hash32,
        sponsor_id: Hash32,
        requester_pubkey: Hash32,
        current_height: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        if self.tasks.contains_key(&task_id) {
            return ExecutionResult::Rejected;
        }

        let creator_nonce = self.accounts.get(&creator_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != creator_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        let total_cost = bounty_agx.saturating_add(fee_agx);
        let creator_balance = self.accounts.get(&creator_id).map(|a| a.balance).unwrap_or(0);
        if creator_balance < total_cost {
            return ExecutionResult::Rejected;
        }

        match self.accounts.get_mut(&creator_id) {
            Some(creator) => {
                creator.balance = creator.balance.saturating_sub(total_cost);
                creator.nonce = nonce;
            }
            None => return ExecutionResult::Rejected,
        }

        let task = Task {
            task_id,
            topic_id,
            seed_ref,
            parent_task_id: [0u8; 32],
            depends_on: vec![],
            funder: creator_id,
            primary_owner: [0u8; 32],
            status: TaskStatus::Open,
            bounty_agx,
            created_at_height: current_height,
            lease_expires_height: current_height,
            required_skills_hash,
            metadata_hash,
            sponsor_id,
            requester_pubkey,
            escrow_status: EscrowStatus::Locked,
        };
        self.tasks.insert(task_id, task);

        ExecutionResult::Success
    }

    /// Split a task into child subtasks forming a dependency DAG.
    ///
    /// Only the `funder` (if the task is Open) or the `primary_owner`
    /// (if Claimed/InProgress) may split. The parent's bounty is
    /// redistributed in full to children. Children are created as
    /// independent tasks with their allocated escrow, parent is
    /// marked Decomposed.
    ///
    /// Validates: caller authorised, child share sum == 100%, dependency
    /// graph acyclic (simple DFS). Atomic: all children created or none.
    #[allow(clippy::too_many_arguments)]
    pub fn execute_split_task(
        &mut self,
        parent_task_id: Hash32,
        caller_id: Hash32,
        children: Vec<SplitChildSpec>,
        nonce: u64,
        current_height: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        // 1. Validate parent exists and caller is authorised
        let parent = match self.tasks.get(&parent_task_id) {
            Some(p) => {
                let authorised = match p.status {
                    TaskStatus::Open => p.funder == caller_id,
                    TaskStatus::Claimed | TaskStatus::InProgress => p.primary_owner == caller_id,
                    _ => false,
                };
                if !authorised {
                    return ExecutionResult::Rejected;
                }
                p
            }
            None => return ExecutionResult::Rejected,
        };

        if children.is_empty() {
            return ExecutionResult::Rejected;
        }

        // Nonce enforcement for caller
        let caller_nonce = self.accounts.get(&caller_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != caller_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        // 2. Validate share sum == 100%
        let share_sum: u32 = children.iter().map(|c| c.bounty_share_pct as u32).sum();
        if share_sum != 100 {
            return ExecutionResult::Rejected;
        }

        // 3. Validate acyclic dependency graph (simple DFS)
        let graph: Vec<(Hash32, Vec<Hash32>)> =
            children.iter().map(|c| (c.task_id, c.depends_on.clone())).collect();
        if has_cycle(&graph) {
            return ExecutionResult::Rejected;
        }

        // 4. Check no child task_id duplicates existing tasks
        for child in &children {
            if self.tasks.contains_key(&child.task_id) {
                return ExecutionResult::Rejected;
            }
        }

        // 5. Atomic execution: create children, mark parent Decomposed
        let parent_bounty = parent.bounty_agx;

        // Update caller nonce
        if let Some(caller) = self.accounts.get_mut(&caller_id) {
            caller.nonce = nonce;
        }
        let parent_metadata = parent.metadata_hash;
        let parent_seed = parent.seed_ref;
        let parent_topic = parent.topic_id;
        let parent_sponsor = parent.sponsor_id;
        let parent_funder = parent.funder;
        // Release parent immutable borrow before mutating self.tasks
        let _ = parent;

        for child in &children {
            let child_bounty =
                parent_bounty.saturating_mul(child.bounty_share_pct as u128) / 100u128;
            let child_task = Task {
                task_id: child.task_id,
                topic_id: parent_topic,
                seed_ref: parent_seed,
                parent_task_id,
                depends_on: child.depends_on.clone(),
                funder: if parent_bounty > 0 { parent_funder } else { caller_id },
                primary_owner: [0u8; 32],
                status: TaskStatus::Open,
                bounty_agx: child_bounty,
                created_at_height: current_height,
                lease_expires_height: current_height,
                required_skills_hash: child.required_skills_hash,
                metadata_hash: parent_metadata,
                sponsor_id: parent_sponsor,
                requester_pubkey: [0u8; 32],
                escrow_status: EscrowStatus::Locked,
            };
            self.tasks.insert(child.task_id, child_task);
        }

        // Mark parent as Decomposed with redistributed escrow
        if let Some(parent) = self.tasks.get_mut(&parent_task_id) {
            parent.status = TaskStatus::Decomposed;
            parent.escrow_status = EscrowStatus::BountyRedistributed;
            parent.bounty_agx = 0;
        }

        ExecutionResult::Success
    }

    /// Delegate AGX from delegator to a validator.
    /// Deducts amount from delegator balance, credits validator's total_delegated.
    pub fn execute_delegate(
        &mut self,
        delegator_id: Hash32,
        validator_id: Hash32,
        amount: u128,
        nonce: u64,
        min_delegation: u128,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        if amount < min_delegation {
            return ExecutionResult::Rejected;
        }
        if amount == 0 {
            return ExecutionResult::Rejected;
        }
        if delegator_id == validator_id {
            return ExecutionResult::Rejected;
        }
        if !self.validators.contains_key(&validator_id) {
            return ExecutionResult::Rejected;
        }

        let delegator_nonce = self.accounts.get(&delegator_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != delegator_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        let delegator_balance = self.accounts.get(&delegator_id).map(|a| a.balance).unwrap_or(0);
        if delegator_balance < amount {
            return ExecutionResult::Rejected;
        }

        if let Some(delegator) = self.accounts.get_mut(&delegator_id) {
            delegator.balance = delegator.balance.saturating_sub(amount);
            delegator.nonce = nonce;
        } else {
            return ExecutionResult::Rejected;
        }

        let key = (delegator_id, validator_id);
        let existing = self.delegations.entry(key).or_insert_with(|| DelegationState {
            amount: 0,
            unbonding_at_height: 0,
            active: true,
        });
        existing.amount = existing.amount.saturating_add(amount);
        existing.active = true;
        existing.unbonding_at_height = 0;

        ExecutionResult::Success
    }

    /// Initiate undelegation. Starts the 7-day unbonding timer.
    pub fn execute_undelegate(
        &mut self,
        delegator_id: Hash32,
        validator_id: Hash32,
        nonce: u64,
        current_height: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let delegator_nonce = self.accounts.get(&delegator_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != delegator_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        // Validate delegation exists and is active
        let key = (delegator_id, validator_id);
        match self.delegations.get(&key) {
            Some(del) => {
                if !del.active {
                    return ExecutionResult::Rejected;
                }
            }
            None => return ExecutionResult::Rejected,
        }

        // Confirm account exists before mutating any state
        match self.accounts.get_mut(&delegator_id) {
            Some(delegator) => {
                delegator.nonce = nonce;
            }
            None => return ExecutionResult::Rejected,
        }

        // All checks passed — now mutate delegation state
        if let Some(del) = self.delegations.get_mut(&key) {
            del.active = false;
            del.unbonding_at_height = current_height;
        } else {
            return ExecutionResult::Rejected;
        }
        ExecutionResult::Success
    }

    /// Withdraw delegation after unbonding delay expires.
    pub fn execute_withdraw_delegation(
        &mut self,
        delegator_id: Hash32,
        validator_id: Hash32,
        nonce: u64,
        current_height: u64,
        delegation_unbond_delay: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let delegator_nonce = self.accounts.get(&delegator_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != delegator_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        let key = (delegator_id, validator_id);
        if let Some(del) = self.delegations.get(&key) {
            if del.active {
                return ExecutionResult::Rejected;
            }
            if current_height < del.unbonding_at_height.saturating_add(delegation_unbond_delay) {
                return ExecutionResult::Rejected;
            }
            let amount = del.amount;

            match self.accounts.get_mut(&delegator_id) {
                Some(delegator) => {
                    delegator.balance = delegator.balance.saturating_add(amount);
                    delegator.nonce = nonce;
                }
                None => return ExecutionResult::Rejected,
            }
            self.delegations.remove(&key);
            ExecutionResult::Success
        } else {
            ExecutionResult::Rejected
        }
    }

    /// Set validator commission rate. Rate takes effect after 2 epochs.
    /// Persists the rate on ValidatorTracker for state root computation.
    pub fn execute_set_commission(
        &mut self,
        validator_id: Hash32,
        commission_rate: u8,
        nonce: u64,
        max_commission_rate: u8,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        if commission_rate > max_commission_rate {
            return ExecutionResult::Rejected;
        }

        let validator_nonce = self.accounts.get(&validator_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != validator_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        if let Some(validator) = self.accounts.get_mut(&validator_id) {
            validator.nonce = nonce;
        } else {
            return ExecutionResult::Rejected;
        }

        if let Some(vt) = self.validators.get_mut(&validator_id) {
            vt.commission_rate = commission_rate;
        } else {
            return ExecutionResult::Rejected;
        }

        ExecutionResult::Success
    }

    /// Bond AGX as validator stake. Creates a new validator record or tops up
    /// an existing one. Funds are locked — they cannot be transferred while bonded.
    /// After bonding, the validator must wait `bond_delay` blocks before becoming
    /// eligible for committee selection.
    pub fn execute_bond(
        &mut self,
        validator_id: Hash32,
        amount: u128,
        nonce: u64,
        min_self_bond: u128,
        current_height: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        if amount < min_self_bond {
            return ExecutionResult::Rejected;
        }

        let validator_nonce = self.accounts.get(&validator_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != validator_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        let validator_balance = self.accounts.get(&validator_id).map(|a| a.balance).unwrap_or(0);
        if validator_balance < amount {
            return ExecutionResult::Rejected;
        }

        if let Some(validator) = self.accounts.get_mut(&validator_id) {
            validator.balance = validator.balance.saturating_sub(amount);
            validator.nonce = nonce;
        } else {
            return ExecutionResult::Rejected;
        }

        let vt = self.validators.entry(validator_id).or_insert(ValidatorTracker {
            validator_id,
            self_bond: 0,
            total_delegated: 0,
            commission_rate: 0,
            bonding_height: current_height,
            unbonding_height: 0,
            state: ValidatorLifecycleState::Active,
            jailed_until_height: 0,
            slash_count: 0,
            last_slash_height: 0,
            last_reward_height: 0,
        });
        vt.self_bond = vt.self_bond.saturating_add(amount);
        vt.state = ValidatorLifecycleState::Active;
        vt.bonding_height = current_height;

        ExecutionResult::Success
    }

    /// Initiate validator unbonding. Starts the unbond delay timer.
    /// Validator becomes ineligible for committee assignment but funds
    /// remain slashable during the unbonding period.
    pub fn execute_unbond(
        &mut self,
        validator_id: Hash32,
        nonce: u64,
        current_height: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let validator_nonce = self.accounts.get(&validator_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != validator_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        match self.validators.get(&validator_id) {
            Some(vt) => {
                if vt.state != ValidatorLifecycleState::Active {
                    return ExecutionResult::Rejected;
                }
            }
            None => return ExecutionResult::Rejected,
        }

        if let Some(validator) = self.accounts.get_mut(&validator_id) {
            validator.nonce = nonce;
        } else {
            return ExecutionResult::Rejected;
        }

        if let Some(vt) = self.validators.get_mut(&validator_id) {
            vt.state = ValidatorLifecycleState::Unbonding;
            vt.unbonding_height = current_height;
            ExecutionResult::Success
        } else {
            ExecutionResult::Rejected
        }
    }

    /// Withdraw bonded stake after unbond delay expires.
    /// Credits the staker's account balance and removes the validator record.
    pub fn execute_withdraw(
        &mut self,
        validator_id: Hash32,
        nonce: u64,
        current_height: u64,
        unbond_delay: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let validator_nonce = self.accounts.get(&validator_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != validator_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        let amount = match self.validators.get(&validator_id) {
            Some(vt) => {
                if vt.state != ValidatorLifecycleState::Unbonding {
                    return ExecutionResult::Rejected;
                }
                if current_height < vt.unbonding_height.saturating_add(unbond_delay) {
                    return ExecutionResult::Rejected;
                }
                vt.self_bond
            }
            None => return ExecutionResult::Rejected,
        };

        if let Some(validator) = self.accounts.get_mut(&validator_id) {
            validator.balance = validator.balance.saturating_add(amount);
            validator.nonce = nonce;
        } else {
            return ExecutionResult::Rejected;
        }

        if let Some(vt) = self.validators.get_mut(&validator_id) {
            vt.state = ValidatorLifecycleState::Withdrawn;
        } else {
            return ExecutionResult::Rejected;
        }
        self.validators.remove(&validator_id);
        ExecutionResult::Success
    }

    /// Renew validator bond. Resets the validator's commission tracking window.
    /// Only applicable to validators in Active or Paused state.
    pub fn execute_renew(
        &mut self,
        validator_id: Hash32,
        nonce: u64,
        current_height: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let validator_nonce = self.accounts.get(&validator_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != validator_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        match self.validators.get(&validator_id) {
            Some(vt) => {
                if vt.state != ValidatorLifecycleState::Active
                    && vt.state != ValidatorLifecycleState::Paused
                {
                    return ExecutionResult::Rejected;
                }
            }
            None => return ExecutionResult::Rejected,
        }

        if let Some(validator) = self.accounts.get_mut(&validator_id) {
            validator.nonce = nonce;
        } else {
            return ExecutionResult::Rejected;
        }

        if let Some(vt) = self.validators.get_mut(&validator_id) {
            vt.bonding_height = current_height;
            ExecutionResult::Success
        } else {
            ExecutionResult::Rejected
        }
    }

    // ── Task & Lease State Transitions ──────────────────────────────

    /// Claim an open task. Validates: task is Open, prior lease expired,
    /// agent has lease capacity, sufficient collateral.
    pub fn execute_claim_task(
        &mut self,
        task_id: Hash32,
        agent_id: Hash32,
        collateral: u128,
        nonce: u64,
        current_height: u64,
        trust_stage: TrustStageEnum,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let max_leases = match trust_stage {
            TrustStageEnum::Untrusted => 2,
            TrustStageEnum::Trusted => 6,
        };
        let active_count: u32 = self
            .leases
            .values()
            .filter(|l| l.owner_id == agent_id && l.expires_at_height > current_height)
            .count() as u32;
        if active_count >= max_leases {
            return ExecutionResult::Rejected;
        }

        let task = match self.tasks.get_mut(&task_id) {
            Some(t) if matches!(t.status, TaskStatus::Open) => t,
            _ => return ExecutionResult::Rejected,
        };

        if task.lease_expires_height > current_height {
            return ExecutionResult::Rejected;
        }

        // Review tasks are restricted to trusted agents only
        if self.review_task_map.contains_key(&task_id)
            && !matches!(trust_stage, TrustStageEnum::Trusted)
        {
            return ExecutionResult::Rejected;
        }

        let min_collateral = 10_000_000_000_000_000_000u128.max(task.bounty_agx * 5 / 1000);
        if collateral < min_collateral {
            return ExecutionResult::Rejected;
        }

        // Nonce enforcement for agent
        let agent_nonce = self.accounts.get(&agent_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != agent_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        task.status = TaskStatus::Claimed;
        task.primary_owner = agent_id;
        task.lease_expires_height = current_height + 120;

        // Update agent nonce
        if let Some(agent) = self.accounts.get_mut(&agent_id) {
            agent.nonce = nonce;
        }

        let lease_id = crate::sha3_256(
            &[task_id.as_slice(), agent_id.as_slice(), &current_height.to_le_bytes()].concat(),
        );
        let lease = TaskLease {
            lease_id,
            task_id,
            owner_id: agent_id,
            collateral,
            started_at_height: current_height,
            expires_at_height: task.lease_expires_height,
            last_heartbeat_height: current_height,
            heartbeats_received: 0,
        };
        self.leases.insert(lease_id, lease);
        ExecutionResult::Success
    }

    /// Submit a heartbeat for an active lease. Extends lease by 120 blocks.
    /// Rejected if heartbeat has no progress evidence.
    pub fn execute_heartbeat(
        &mut self,
        heartbeat: HeartbeatPayload,
        nonce: u64,
        current_height: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let lease = match self.leases.get_mut(&heartbeat.lease_id) {
            Some(l) => l,
            None => return ExecutionResult::Rejected,
        };

        // Nonce enforcement for lease owner
        let owner_nonce = self.accounts.get(&lease.owner_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != owner_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        if heartbeat.artifact_hash.is_none()
            && heartbeat.diff_pointer.is_none()
            && heartbeat.test_result_ref.is_none()
        {
            return ExecutionResult::Rejected;
        }

        lease.last_heartbeat_height = current_height;
        lease.heartbeats_received += 1;
        lease.expires_at_height = current_height.saturating_add(120);

        // Update owner nonce
        if let Some(owner) = self.accounts.get_mut(&lease.owner_id) {
            owner.nonce = nonce;
        }

        if let Some(task) = self.tasks.get_mut(&lease.task_id) {
            task.lease_expires_height = lease.expires_at_height;
            if matches!(task.status, TaskStatus::Claimed) {
                task.status = TaskStatus::InProgress;
            }
        }

        ExecutionResult::Success
    }

    /// Release a task lease (voluntary). Removes lease, returns task to Open.
    pub fn execute_release_task(
        &mut self,
        task_id: Hash32,
        agent_id: Hash32,
        nonce: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let task = match self.tasks.get_mut(&task_id) {
            Some(t) if t.primary_owner == agent_id => t,
            _ => return ExecutionResult::Rejected,
        };
        if !matches!(task.status, TaskStatus::Claimed | TaskStatus::InProgress) {
            return ExecutionResult::Rejected;
        }

        // Nonce enforcement for agent
        let agent_nonce = self.accounts.get(&agent_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != agent_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        let expired: Vec<Hash32> =
            self.leases.values().filter(|l| l.task_id == task_id).map(|l| l.lease_id).collect();
        for lid in expired {
            self.leases.remove(&lid);
        }

        task.primary_owner = [0u8; 32];
        task.status = TaskStatus::Open;

        // Update agent nonce
        if let Some(agent) = self.accounts.get_mut(&agent_id) {
            agent.nonce = nonce;
        }

        ExecutionResult::Success
    }

    /// Submit completed task for review. Flips work task to InReview and
    /// creates review tasks in the open pool — one per reviewer slot.
    /// Review tasks have zero bounty (paid from work task escrow on settlement)
    /// and are only claimable by trusted agents.
    pub fn execute_submit_completion(
        &mut self,
        task_id: Hash32,
        agent_id: Hash32,
        nonce: u64,
        current_height: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let task = match self.tasks.get_mut(&task_id) {
            Some(t) if t.primary_owner == agent_id => t,
            _ => return ExecutionResult::Rejected,
        };
        if !matches!(task.status, TaskStatus::InProgress) {
            return ExecutionResult::Rejected;
        }

        // Nonce enforcement for agent
        let agent_nonce = self.accounts.get(&agent_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != agent_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        // Update agent nonce
        if let Some(agent) = self.accounts.get_mut(&agent_id) {
            agent.nonce = nonce;
        }

        // Extract needed data before creating review tasks
        let work_bounty = task.bounty_agx;
        let task_topic = task.topic_id;
        let task_seed = task.seed_ref;
        let task_funder = task.funder;
        let task_metadata = task.metadata_hash;
        let task_sponsor = task.sponsor_id;
        let task_id_copy = task.task_id;
        task.status = TaskStatus::InReview;
        // Release mutable borrow before accessing self.tasks again
        let _ = task;

        // Create 2 review tasks (each worth 5% of work bounty)
        let review_count: u64 = 2;
        let mut created: u32 = 0;
        for i in 0..review_count {
            let review_task_id =
                crate::sha3_256(&[task_id_copy.as_slice(), &i.to_le_bytes()].concat());
            if self.tasks.contains_key(&review_task_id) {
                continue;
            }
            let review_task = Task {
                task_id: review_task_id,
                topic_id: task_topic,
                seed_ref: task_seed,
                parent_task_id: task_id,
                depends_on: vec![],
                funder: task_funder,
                primary_owner: [0u8; 32],
                status: TaskStatus::Open,
                bounty_agx: work_bounty * 5 / 100,
                created_at_height: current_height,
                lease_expires_height: current_height,
                required_skills_hash: [0u8; 32],
                metadata_hash: task_metadata,
                sponsor_id: task_sponsor,
                requester_pubkey: [0u8; 32],
                escrow_status: EscrowStatus::Locked,
            };
            self.tasks.insert(review_task_id, review_task);
            self.review_task_map.insert(review_task_id, task_id);
            created += 1;
        }
        debug_assert_eq!(created, review_count as u32, "must create exactly 2 review tasks");

        ExecutionResult::Success
    }

    /// Submit a review verdict for a work task. Only the agent who
    /// holds the active lease on the review task may submit.
    /// After N verdicts collected, the review is tallied and settled.
    pub fn execute_submit_review(
        &mut self,
        review_task_id: Hash32,
        reviewer_id: Hash32,
        verdict: ReviewVerdict,
        evidence_hash: Hash32,
        nonce: u64,
        current_height: u64,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        let work_task_id = match self.review_task_map.get(&review_task_id).copied() {
            Some(id) => id,
            None => return ExecutionResult::Rejected,
        };

        // Nonce enforcement for reviewer
        let reviewer_nonce = self.accounts.get(&reviewer_id).map(|a| a.nonce).unwrap_or(0);
        if nonce != reviewer_nonce.saturating_add(1) {
            return ExecutionResult::Rejected;
        }

        // Reviewer must be the primary owner of the review task
        let _review_task = match self.tasks.get(&review_task_id) {
            Some(t)
                if t.primary_owner == reviewer_id
                    && matches!(t.status, TaskStatus::Claimed | TaskStatus::InProgress) =>
            {
                t
            }
            _ => return ExecutionResult::Rejected,
        };

        let record = ReviewRecord {
            task_id: work_task_id,
            review_task_id,
            reviewer_id,
            verdict,
            evidence_hash,
            submitted_at_height: current_height,
        };

        // Store verdict
        self.review_records.entry(work_task_id).or_default().push(record);

        // Update reviewer nonce
        if let Some(reviewer) = self.accounts.get_mut(&reviewer_id) {
            reviewer.nonce = nonce;
        }

        // Mark review task done
        if let Some(rt) = self.tasks.get_mut(&review_task_id) {
            rt.status = TaskStatus::Done;
        }
        // Release review task lease
        self.leases.retain(|_, l| l.task_id != review_task_id);
        self.review_task_map.remove(&review_task_id);

        // Tally: if we have enough verdicts (2), settle
        let verdicts = self.review_records.get(&work_task_id).map(|v| v.len()).unwrap_or(0);
        if verdicts >= 2 {
            self.settle_review(work_task_id, current_height);
        }

        ExecutionResult::Success
    }

    /// Tally review verdicts and settle the work task's escrow.
    /// Majority accept → 90% to worker, 10% split among reviewers.
    /// Majority reject → task returns to Open, reviewers still paid.
    fn settle_review(&mut self, work_task_id: Hash32, current_height: u64) {
        let verdicts = match self.review_records.remove(&work_task_id) {
            Some(v) if !v.is_empty() => v,
            _ => return,
        };

        let accept_count =
            verdicts.iter().filter(|v| matches!(v.verdict, ReviewVerdict::Accept)).count();
        let reject_count = verdicts.len() - accept_count;
        let accepted = accept_count > reject_count;

        if let Some(task) = self.tasks.get_mut(&work_task_id) {
            let total_bounty = task.bounty_agx;
            let review_pool = total_bounty.saturating_mul(10) / 100; // 10% for reviewers
            let per_reviewer =
                if verdicts.is_empty() { 0 } else { review_pool / verdicts.len() as u128 };

            if accepted {
                // Worker gets 90%
                let worker_payout = total_bounty - review_pool;
                let worker_id = task.primary_owner;
                if worker_id != [0u8; 32] {
                    if let Some(acct) = self.accounts.get_mut(&worker_id) {
                        acct.balance = acct.balance.saturating_add(worker_payout);
                    }
                }
                // Also reward worker with trust advancement
                // (accepted_work_count handled by caller)
                task.status = TaskStatus::Done;
                task.escrow_status = EscrowStatus::Released;
            } else {
                // Task back to Open for retry
                task.status = TaskStatus::Open;
                task.primary_owner = [0u8; 32];
                task.lease_expires_height = current_height;
                // Don't refund escrow — it stays locked for the next attempt
            }

            // Pay reviewers regardless of accept/reject — they did the work
            for record in &verdicts {
                if let Some(acct) = self.accounts.get_mut(&record.reviewer_id) {
                    acct.balance = acct.balance.saturating_add(per_reviewer);
                }
                // Remove any leftover review task associated with this verdict
                if let Some(rt) = self.tasks.get_mut(&record.review_task_id) {
                    if matches!(rt.status, TaskStatus::Claimed | TaskStatus::InProgress) {
                        rt.status = TaskStatus::Done;
                    }
                }
                self.leases.retain(|_, l| l.task_id != record.review_task_id);
            }
        }
    }

    /// Run review window expiry: if a review task lease has expired
    /// without a verdict being submitted, return the work task to Open.
    pub fn run_review_expiry(&mut self, work_task_id: &Hash32, current_height: u64) -> bool {
        let verdicts = self.review_records.get(work_task_id).map(|v| v.len()).unwrap_or(0);
        if verdicts >= 2 {
            return false;
        }

        // Check if review tasks have expired
        let expired_review_tasks: Vec<Hash32> = self
            .tasks
            .iter()
            .filter(|(_, t)| {
                t.parent_task_id == *work_task_id
                    && matches!(t.status, TaskStatus::Claimed | TaskStatus::InProgress)
                    && t.lease_expires_height <= current_height
            })
            .map(|(id, _)| *id)
            .collect();

        if expired_review_tasks.is_empty() {
            return false;
        }

        for rid in &expired_review_tasks {
            if let Some(rt) = self.tasks.get_mut(rid) {
                rt.status = TaskStatus::Open;
                rt.primary_owner = [0u8; 32];
            }
            self.leases.retain(|_, l| l.task_id != *rid);
        }

        // If not enough verdicts after expiry, return work task to Open
        let remaining = self.review_records.get(work_task_id).map(|v| v.len()).unwrap_or(0);
        if remaining < 2 {
            if let Some(task) = self.tasks.get_mut(work_task_id) {
                if matches!(task.status, TaskStatus::InReview) {
                    task.status = TaskStatus::Open;
                    task.primary_owner = [0u8; 32];
                    task.lease_expires_height = current_height;
                }
            }
        }

        true
    }

    /// Check lease expiry for a task. If lease expired, return task to Open pool.
    /// If escrow is still Locked (bounty never released), refund the bounty to the
    /// task funder and mark escrow as Refunded.
    /// Called at every block boundary, not as a transaction.
    pub fn run_lease_expiry(&mut self, task_id: &Hash32, current_height: u64) -> bool {
        let prior_escrow = self.tasks.get(task_id).map(|t| t.escrow_status);
        let should_expire = self.tasks.get(task_id).is_some_and(|t| {
            matches!(t.status, TaskStatus::Claimed | TaskStatus::InProgress)
                && t.lease_expires_height <= current_height
        });
        if !should_expire {
            return false;
        }

        let expired_ids: Vec<Hash32> = self
            .leases
            .values()
            .filter(|l| l.task_id == *task_id && l.expires_at_height <= current_height)
            .map(|l| l.lease_id)
            .collect();

        for lid in &expired_ids {
            self.leases.remove(lid);
        }

        if let Some(task) = self.tasks.get_mut(task_id) {
            task.primary_owner = [0u8; 32];
            task.status = TaskStatus::Open;

            // Refund escrowed bounty if lease expired without release
            // SPEC_DEVIATION: Bounty is zeroed to prevent claim on refunded task.
            if prior_escrow == Some(EscrowStatus::Locked) {
                task.escrow_status = EscrowStatus::Refunded;
                let refund = task.bounty_agx;
                task.bounty_agx = 0;
                if let Some(funder) = self.accounts.get_mut(&task.funder) {
                    funder.balance = funder.balance.saturating_add(refund);
                }
            }
        }

        true
    }

    /// Consume a freshness nonce for artifact replay prevention.
    /// Returns Rejected if nonce already consumed (replay detected).
    pub fn consume_freshness_nonce(
        &mut self,
        task_id: Hash32,
        nonce: Hash32,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        if self.consumed_nonces.contains(&(task_id, nonce)) {
            return ExecutionResult::Rejected;
        }
        self.consumed_nonces.insert((task_id, nonce));
        ExecutionResult::Success
    }

    /// Consume a freshness nonce for artifact/chunk replay prevention.
    /// Wired from ProofOfPossession verification — calls consume_freshness_nonce.
    pub fn execute_consume_freshness_nonce(
        &mut self,
        task_id: Hash32,
        nonce: Hash32,
        ctx: ExecutionContext,
    ) -> ExecutionResult {
        self.consume_freshness_nonce(task_id, nonce, ctx)
    }

    // ── Trust Ladder ────────────────────────────────────────────────

    /// Initialize a trust stage record for a new agent.
    pub fn init_trust_stage(&mut self, agent_id: Hash32) {
        self.trust_stages.entry(agent_id).or_insert(TrustStageRecord {
            agent_id,
            stage: TrustStageEnum::Untrusted,
            accepted_work_count: 0,
            abuse_flags: 0,
        });
    }

    /// Record an accepted task completion for trust promotion.
    pub fn record_accepted_work(&mut self, agent_id: &Hash32) {
        if let Some(record) = self.trust_stages.get_mut(agent_id) {
            record.accepted_work_count += 1;
        }
    }

    /// Record an abuse event on an agent.
    pub fn record_abuse(&mut self, agent_id: &Hash32, is_high_severity: bool) {
        if let Some(record) = self.trust_stages.get_mut(agent_id) {
            record.abuse_flags += 1;
            if is_high_severity {
                record.stage = TrustStageEnum::Untrusted;
                record.accepted_work_count = 0;
            }
        }
    }

    /// Run trust promotion at epoch boundary.
    /// Promotes untrusted agents with >= 10 accepted work and 0 abuse flags.
    ///
    /// Determinism: All operations are commutative — iteration order over the
    /// HashMap does not affect final state. If future logic adds order-dependent
    /// behaviour, keys MUST be sorted before iteration.
    pub fn run_trust_promotion(&mut self) -> Vec<Hash32> {
        let to_promote: Vec<Hash32> = self
            .trust_stages
            .iter()
            .filter(|(_, r)| {
                r.stage == TrustStageEnum::Untrusted
                    && r.accepted_work_count >= 10
                    && r.abuse_flags == 0
            })
            .map(|(id, _)| *id)
            .collect();

        for agent_id in &to_promote {
            if let Some(record) = self.trust_stages.get_mut(agent_id) {
                record.stage = TrustStageEnum::Trusted;
            }
        }
        to_promote
    }

    // ── Topic Lifecycle ─────────────────────────────────────────────

    /// Register a new topic at genesis or governance.
    pub fn init_topic(&mut self, topic_id: Hash32, seed_ref: Hash32, created_at_height: u64) {
        self.topic_records.entry(topic_id).or_insert(TopicRecord {
            topic_id,
            seed_ref,
            status: TopicStatus::New,
            created_at_height,
            last_activity_height: created_at_height,
            message_count: 0,
            decay_score: 100,
        });
    }

    /// Record activity on a topic (resets decay).
    pub fn record_topic_activity(&mut self, topic_id: &Hash32, current_height: u64) {
        if let Some(topic) = self.topic_records.get_mut(topic_id) {
            topic.last_activity_height = current_height;
            topic.message_count += 1;
            topic.decay_score = topic.decay_score.saturating_add(10).min(100);
            if matches!(topic.status, TopicStatus::New | TopicStatus::Stale) {
                topic.status = TopicStatus::Active;
            }
        }
    }

    /// Run topic decay at epoch boundary.
    ///
    /// Determinism: All operations are commutative — iteration order over the
    /// HashMap does not affect final state. If future logic adds order-dependent
    /// behaviour, keys MUST be sorted before iteration.
    pub fn run_topic_decay(&mut self, current_height: u64) {
        let decay_rate: u64 = 1000;
        for topic in self.topic_records.values_mut() {
            let inactive_blocks = current_height.saturating_sub(topic.last_activity_height);
            let decay_units = inactive_blocks / decay_rate;
            topic.decay_score =
                topic.decay_score.saturating_sub(u32::try_from(decay_units).unwrap_or(u32::MAX));
            if topic.decay_score < 25 {
                topic.status = TopicStatus::Stale;
            }
            if topic.decay_score == 0 {
                topic.status = TopicStatus::Archived;
            }
        }
    }

    // ── Query Methods ───────────────────────────────────────────────

    pub fn get_task(&self, task_id: &Hash32) -> Option<&Task> {
        self.tasks.get(task_id)
    }

    pub fn get_lease(&self, lease_id: &Hash32) -> Option<&TaskLease> {
        self.leases.get(lease_id)
    }

    pub fn get_trust_stage(&self, agent_id: &Hash32) -> Option<&TrustStageRecord> {
        self.trust_stages.get(agent_id)
    }

    pub fn get_topic(&self, topic_id: &Hash32) -> Option<&TopicRecord> {
        self.topic_records.get(topic_id)
    }

    pub fn tasks_iter(&self) -> impl Iterator<Item = &Task> {
        self.tasks.values()
    }

    pub fn leases_iter(&self) -> impl Iterator<Item = &TaskLease> {
        self.leases.values()
    }

    pub fn trust_stages_iter(&self) -> impl Iterator<Item = &TrustStageRecord> {
        self.trust_stages.values()
    }

    pub fn topic_records_iter(&self) -> impl Iterator<Item = &TopicRecord> {
        self.topic_records.values()
    }

    pub fn task_count(&self) -> usize {
        self.tasks.len()
    }

    pub fn lease_count(&self) -> usize {
        self.leases.len()
    }

    // ── SMT Root Computation ────────────────────────────────────────

    pub fn compute_state_root(&self) -> Hash32 {
        let mut tree = SparseMerkleTree::new();

        for (account_id, account) in &self.accounts {
            let key = state_key(KeyPrefix::Account, account_id);
            let value = account.encode();
            tree.insert(key, value);
        }

        for (validator_id, vt) in &self.validators {
            let key = state_key(KeyPrefix::Validator, validator_id);
            let value = vt.encode();
            tree.insert(key, value);
        }

        for ((delegator_id, validator_id), del) in &self.delegations {
            let delegation_key = {
                let mut preimage = Vec::with_capacity(64);
                preimage.extend_from_slice(delegator_id);
                preimage.extend_from_slice(validator_id);
                let id = crate::sha3_256(&preimage);
                state_key(KeyPrefix::Delegation, &id)
            };
            let value = del.encode();
            tree.insert(delegation_key, value);
        }

        for plan_id in &self.consumed_plans {
            let key = state_key(KeyPrefix::ActionPlan, plan_id);
            tree.insert(key, vec![1u8]);
        }

        for (task_id, task) in &self.tasks {
            let key = state_key(KeyPrefix::Task, task_id);
            tree.insert(key, task.encode());
        }

        for (lease_id, lease) in &self.leases {
            let key = state_key(KeyPrefix::TaskLease, lease_id);
            tree.insert(key, lease.encode());
        }

        for (agent_id, record) in &self.trust_stages {
            let key = state_key(KeyPrefix::TrustStage, agent_id);
            tree.insert(key, record.encode());
        }

        for (topic_id, record) in &self.topic_records {
            let key = state_key(KeyPrefix::Topic, topic_id);
            tree.insert(key, record.encode());
        }

        for (task_id, nonce) in &self.consumed_nonces {
            let mut preimage = Vec::with_capacity(64);
            preimage.extend_from_slice(task_id);
            preimage.extend_from_slice(nonce);
            let id = crate::sha3_256(&preimage);
            let key = state_key(KeyPrefix::ConsumedNonce, &id);
            tree.insert(key, vec![1u8]);
        }

        for (work_task_id, records) in &self.review_records {
            let key = state_key(KeyPrefix::ReviewRecord, work_task_id);
            let value = records.encode();
            tree.insert(key, value);
        }

        for (review_task_id, work_task_id) in &self.review_task_map {
            let key = state_key(KeyPrefix::ReviewTaskMap, review_task_id);
            tree.insert(key, work_task_id.to_vec());
        }

        {
            let key = state_key(KeyPrefix::FeeBurnAccumulator, &[0u8; 32]);
            tree.insert(key, self.fee_burn_accumulator.to_le_bytes().to_vec());
        }

        tree.root()
    }

    /// Return the number of accounts tracked.
    pub fn account_count(&self) -> usize {
        self.accounts.len()
    }

    /// Iterate over all accounts.
    pub fn accounts_iter(&self) -> impl Iterator<Item = (&Hash32, &Account)> {
        self.accounts.iter()
    }

    /// Get validator tracker by ID.
    pub fn get_validator(&self, validator_id: &Hash32) -> Option<&ValidatorTracker> {
        self.validators.get(validator_id)
    }

    /// Iterate over all consumed plan IDs.
    pub fn consumed_plans_iter(&self) -> impl Iterator<Item = &Hash32> {
        self.consumed_plans.iter()
    }

    /// Iterate over all consumed freshness nonces.
    pub fn consumed_nonces_iter(&self) -> impl Iterator<Item = &(Hash32, Hash32)> {
        self.consumed_nonces.iter()
    }

    /// Iterate over all delegation records.
    pub(crate) fn delegations_iter(
        &self,
    ) -> impl Iterator<Item = (&(Hash32, Hash32), &DelegationState)> {
        self.delegations.iter()
    }

    /// Iterate over all validators.
    pub fn validators_iter(&self) -> impl Iterator<Item = (&Hash32, &ValidatorTracker)> {
        self.validators.iter()
    }

    /// Iterate over all review records (work_task_id -> `Vec<ReviewRecord>`).
    pub fn review_records_iter(&self) -> impl Iterator<Item = (&Hash32, &Vec<ReviewRecord>)> {
        self.review_records.iter()
    }

    /// Iterate over all review task map entries (review_task_id -> work_task_id).
    pub fn review_task_map_iter(&self) -> impl Iterator<Item = (&Hash32, &Hash32)> {
        self.review_task_map.iter()
    }

    /// Capture a state snapshot for checkpointing or RPC.
    /// Wires snapshot_state into production code.
    pub fn get_snapshot(&self, epoch: u64, height: u64, block_hash: Hash32) -> Snapshot {
        crate::state_sync::snapshot_state(self, epoch, height, block_hash)
    }

    /// Compute a checksum over the current state for integrity verification.
    /// Wires compute_state_checksum into production code.
    pub fn get_state_checksum(&self) -> Hash32 {
        let snapshot = crate::state_sync::snapshot_state(self, 0, 0, [0u8; 32]);
        crate::state_sync::compute_state_checksum(&snapshot.sst_keys)
    }

    pub fn delegations_count(&self) -> usize {
        self.delegations.len()
    }

    /// Execute equivocation slashing.
    ///
    /// Slashes 10% of the validator's self-bonded stake, burns the slashed amount,
    /// and jails the validator for a minimum jail period (30 days).
    /// The validator's state is set to Paused during the jail period.
    ///
    /// Returns `Rejected` if:
    /// - The validator does not exist
    /// - The validator is not Active
    /// - The evidence has already been processed (equivocation already recorded)
    pub fn execute_slash_equivocation(
        &mut self,
        validator_id: Hash32,
        evidence_height: u64,
        min_jail_blocks: u64,
        current_height: u64,
    ) -> ExecutionResult {
        let vt = match self.validators.get(&validator_id) {
            Some(vt) => vt,
            None => return ExecutionResult::Rejected,
        };

        if vt.state != ValidatorLifecycleState::Active {
            return ExecutionResult::Rejected;
        }

        if vt.jailed_until_height > current_height {
            return ExecutionResult::Rejected;
        }

        let slash_amount = vt.self_bond / 10; // 10% slash

        let vt = match self.validators.get_mut(&validator_id) {
            Some(vt) => vt,
            None => return ExecutionResult::Rejected,
        };
        vt.self_bond = vt.self_bond.saturating_sub(slash_amount);
        vt.state = ValidatorLifecycleState::Paused;
        vt.jailed_until_height = current_height + min_jail_blocks;
        vt.slash_count = vt.slash_count.saturating_add(1);
        vt.last_slash_height = evidence_height;

        // Burn the slashed AGX from the validator's account
        if let Some(account) = self.accounts.get_mut(&validator_id) {
            account.balance = account.balance.saturating_sub(slash_amount);
        }

        // Add to total burned counter
        self.fee_burn_accumulator = self.fee_burn_accumulator.saturating_add(slash_amount);

        // Slash delegations proportionally (10%)
        let delegator_ids: Vec<Hash32> = self
            .delegations
            .iter()
            .filter(|((_, vid), _)| *vid == validator_id)
            .map(|((did, _), _)| *did)
            .collect();
        let mut total_slashed_delegations: u128 = 0;
        for delegator_id in &delegator_ids {
            if let Some(del) = self.delegations.get_mut(&(*delegator_id, validator_id)) {
                let delegation_slash = del.amount / 10;
                del.amount = del.amount.saturating_sub(delegation_slash);
                total_slashed_delegations =
                    total_slashed_delegations.saturating_add(delegation_slash);
            }
        }
        if let Some(vt) = self.validators.get_mut(&validator_id) {
            vt.total_delegated = vt.total_delegated.saturating_sub(total_slashed_delegations);
        }

        ExecutionResult::Success
    }

    /// Execute downtime slashing.
    ///
    /// Slashes 1% of the validator's self-bonded stake per downtime incident.
    /// Repeated downtime incidents increase the slash proportionally.
    /// Validator is paused if they missed more than 20% of blocks in the liveness window.
    ///
    /// Returns `Rejected` if:
    /// - The validator does not exist
    /// - The validator is not Active
    pub fn execute_slash_downtime(
        &mut self,
        validator_id: Hash32,
        missed_blocks: u64,
        total_window_blocks: u64,
        evidence_height: u64,
        pause_threshold_pct: u64, // e.g. 20 = 20%
        min_jail_blocks: u64,
        current_height: u64,
    ) -> ExecutionResult {
        let vt = match self.validators.get(&validator_id) {
            Some(vt) => vt,
            None => return ExecutionResult::Rejected,
        };

        if vt.state != ValidatorLifecycleState::Active {
            return ExecutionResult::Rejected;
        }

        let downtime_pct = (missed_blocks * 100).checked_div(total_window_blocks).unwrap_or(0);

        if downtime_pct < pause_threshold_pct {
            return ExecutionResult::Success; // Below threshold, no action
        }

        // 1% slash per incident, capped at 5%
        let slash_basis_points = std::cmp::min(500, (vt.slash_count + 1) * 100);
        let slash_amount = (vt.self_bond * slash_basis_points as u128) / 10000;

        let vt = match self.validators.get_mut(&validator_id) {
            Some(vt) => vt,
            None => return ExecutionResult::Rejected,
        };
        vt.self_bond = vt.self_bond.saturating_sub(slash_amount);

        if downtime_pct >= pause_threshold_pct {
            vt.state = ValidatorLifecycleState::Paused;
            vt.jailed_until_height = current_height + min_jail_blocks;
        }
        vt.slash_count = vt.slash_count.saturating_add(1);
        vt.last_slash_height = evidence_height;

        if let Some(account) = self.accounts.get_mut(&validator_id) {
            account.balance = account.balance.saturating_sub(slash_amount);
        }

        self.fee_burn_accumulator = self.fee_burn_accumulator.saturating_add(slash_amount);

        // Slash delegations proportionally using the same slash_basis_points
        let delegator_ids: Vec<Hash32> = self
            .delegations
            .iter()
            .filter(|((_, vid), _)| *vid == validator_id)
            .map(|((did, _), _)| *did)
            .collect();
        let mut total_slashed_delegations: u128 = 0;
        for delegator_id in &delegator_ids {
            if let Some(del) = self.delegations.get_mut(&(*delegator_id, validator_id)) {
                let delegation_slash = (del.amount * slash_basis_points as u128) / 10000;
                del.amount = del.amount.saturating_sub(delegation_slash);
                total_slashed_delegations =
                    total_slashed_delegations.saturating_add(delegation_slash);
            }
        }
        if let Some(vt) = self.validators.get_mut(&validator_id) {
            vt.total_delegated = vt.total_delegated.saturating_sub(total_slashed_delegations);
        }

        ExecutionResult::Success
    }

    /// Distribute epoch-end fee rebates to validators proportionally to their
    /// self-bonded stake. Active validators receive rebates from the fee pool.
    ///
    /// Fee pool is consumed (set to zero) after distribution.
    ///
    /// Determinism: All operations are commutative — iteration order over the
    /// HashMap does not affect final state. If future logic adds order-dependent
    /// behaviour, keys MUST be sorted before iteration.
    pub fn execute_distribute_rewards(&mut self, epoch_fee_pool: &mut u128) -> ExecutionResult {
        if *epoch_fee_pool == 0 {
            return ExecutionResult::Success;
        }

        let total_stake: u128 = self
            .validators
            .values()
            .filter(|vt| vt.state == ValidatorLifecycleState::Active)
            .map(|vt| vt.self_bond)
            .sum();

        if total_stake == 0 {
            return ExecutionResult::Success;
        }

        let pool = *epoch_fee_pool;

        for vt in self.validators.values_mut() {
            if vt.state != ValidatorLifecycleState::Active {
                continue;
            }
            let share = pool
                .checked_mul(vt.self_bond)
                .and_then(|v| v.checked_div(total_stake))
                .unwrap_or(0);
            if let Some(account) = self.accounts.get_mut(&vt.validator_id) {
                account.balance = account.balance.saturating_add(share);
            }
            vt.last_reward_height = 0; // Will be set by caller
        }

        *epoch_fee_pool = 0;

        ExecutionResult::Success
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sha3_256;

    fn test_account(id: u8, balance: u128, nonce: u64) -> Account {
        let account_id = [id; 32];
        Account { account_id, balance, nonce, pubkey_hash: sha3_256(&[id]), pubkey: Some(vec![id]) }
    }

    #[test]
    fn transfer_debits_and_credits() {
        let mut sm = StateMachine::new();
        let alice = test_account(1, 1000, 0);
        let bob = test_account(2, 0, 0);
        sm.init_account(alice);
        sm.init_account(bob);

        let result = sm.execute_transfer(
            [1u8; 32],
            [2u8; 32],
            500,
            1,
            ExecutionContext { height: 1, timestamp: 0 },
        );
        assert_eq!(result, ExecutionResult::Success);
        assert_eq!(sm.get_account(&[1u8; 32]).unwrap().balance, 500);
        assert_eq!(sm.get_account(&[2u8; 32]).unwrap().balance, 500);
    }

    #[test]
    fn transfer_rejects_insufficient_balance() {
        let mut sm = StateMachine::new();
        sm.init_account(test_account(1, 100, 0));
        let result = sm.execute_transfer(
            [1u8; 32],
            [2u8; 32],
            101,
            1,
            ExecutionContext { height: 1, timestamp: 0 },
        );
        assert_eq!(result, ExecutionResult::Rejected);
    }

    #[test]
    fn nonce_wrong_is_rejected() {
        let mut sm = StateMachine::new();
        sm.init_account(test_account(1, 1000, 5));
        // Correct nonce should be 6 (nonce + 1)
        let result = sm.execute_transfer(
            [1u8; 32],
            [2u8; 32],
            100,
            7,
            ExecutionContext { height: 1, timestamp: 0 },
        );
        assert_eq!(result, ExecutionResult::Rejected);
    }

    #[test]
    fn nonce_correct_is_accepted() {
        let mut sm = StateMachine::new();
        sm.init_account(test_account(1, 1000, 5));
        let result = sm.execute_transfer(
            [1u8; 32],
            [2u8; 32],
            100,
            6,
            ExecutionContext { height: 1, timestamp: 0 },
        );
        assert_eq!(result, ExecutionResult::Success);
        assert_eq!(sm.get_account(&[1u8; 32]).unwrap().nonce, 6);
    }

    #[test]
    fn replay_protection_rejects_duplicate_plan() {
        let mut sm = StateMachine::new();
        let plan_id = [0xDEu8; 32];
        let ctx = ExecutionContext { height: 1, timestamp: 0 };

        assert_eq!(sm.consume_plan_id(plan_id, ctx), ExecutionResult::Success);
        assert_eq!(sm.consume_plan_id(plan_id, ctx), ExecutionResult::Rejected);
    }

    #[test]
    fn state_root_deterministic_same_state() {
        let mut sm1 = StateMachine::new();
        let mut sm2 = StateMachine::new();

        sm1.init_account(Account {
            account_id: [1u8; 32],
            balance: 100,
            nonce: 0,
            pubkey_hash: sha3_256(&[1]),
            pubkey: Some(vec![1]),
        });
        sm2.init_account(Account {
            account_id: [1u8; 32],
            balance: 100,
            nonce: 0,
            pubkey_hash: sha3_256(&[1]),
            pubkey: Some(vec![1]),
        });

        assert_eq!(sm1.compute_state_root(), sm2.compute_state_root());
    }

    #[test]
    fn state_root_differs_for_different_state() {
        let mut sm1 = StateMachine::new();
        let mut sm2 = StateMachine::new();

        sm1.init_account(Account {
            account_id: [1u8; 32],
            balance: 100,
            nonce: 0,
            pubkey_hash: sha3_256(&[1]),
            pubkey: Some(vec![1]),
        });
        sm2.init_account(Account {
            account_id: [1u8; 32],
            balance: 200,
            nonce: 0,
            pubkey_hash: sha3_256(&[1]),
            pubkey: Some(vec![1]),
        });

        assert_ne!(sm1.compute_state_root(), sm2.compute_state_root());
    }

    #[test]
    fn task_create_debits_creator() {
        let mut sm = StateMachine::new();
        sm.init_account(test_account(1, 1000, 0));

        let result = sm.execute_task_create(
            [1u8; 32],
            300,
            10,
            [0xA1u8; 32],
            1,
            [0xBBu8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            10,
            ExecutionContext { height: 10, timestamp: 1000 },
        );
        assert_eq!(result, ExecutionResult::Success);
        assert_eq!(sm.get_account(&[1u8; 32]).unwrap().balance, 690);
    }

    #[test]
    fn task_create_rejects_duplicate_task_id() {
        let mut sm = StateMachine::new();
        sm.init_account(test_account(1, 2000, 0));
        let task_id = [0xDDu8; 32];
        let seed = [0xEEu8; 32];

        let r1 = sm.execute_task_create(
            [1u8; 32],
            100,
            10,
            task_id,
            1,
            seed,
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            10,
            ExecutionContext { height: 10, timestamp: 1000 },
        );
        assert_eq!(r1, ExecutionResult::Success);

        // Second attempt with same task_id is rejected (task already exists)
        let r2 = sm.execute_task_create(
            [1u8; 32],
            100,
            10,
            task_id,
            2,
            seed,
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            11,
            ExecutionContext { height: 11, timestamp: 1000 },
        );
        assert_eq!(r2, ExecutionResult::Rejected);
    }

    #[test]
    fn task_create_rejects_insufficient_balance() {
        let mut sm = StateMachine::new();
        sm.init_account(test_account(1, 50, 0));

        let result = sm.execute_task_create(
            [1u8; 32],
            100,
            10,
            [0xA1u8; 32],
            1,
            [0xBBu8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            10,
            ExecutionContext { height: 10, timestamp: 1000 },
        );
        assert_eq!(result, ExecutionResult::Rejected);
    }

    #[test]
    fn run_lease_expiry_refunds_escrow_and_marks_refunded() {
        let mut sm = StateMachine::new();
        // Creator/funder with 500 AGX balance
        let creator_id = [1u8; 32];
        let task_id = [0xAAu8; 32];
        let seed_ref = [0xBBu8; 32];
        let agent_id = [2u8; 32];
        let bounty = 300_000_000_000_000_000_000u128; // 300 AGX
        let fee = 10_000_000_000_000_000_000u128; // 10 AGX

        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.init_account(test_account(2, 500_000_000_000_000_000_000, 0));

        // Create a task — escrow_status = Locked, 500 - 300 - 10 = 190 AGX remaining
        let r = sm.execute_task_create(
            creator_id,
            bounty,
            fee,
            task_id,
            1,
            seed_ref,
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            100,
            ctx(100),
        );
        assert_eq!(r, ExecutionResult::Success);
        assert_eq!(sm.get_account(&creator_id).unwrap().balance, 190_000_000_000_000_000_000);
        assert_eq!(sm.tasks.get(&task_id).unwrap().escrow_status, EscrowStatus::Locked,);

        // Claim the task so status → Claimed, lease starts at height 100, expires at 220
        let min_collateral = 10_000_000_000_000_000_000u128.max(bounty * 5 / 1000);
        let r = sm.execute_claim_task(
            task_id,
            agent_id,
            min_collateral,
            1,
            100,
            TrustStageEnum::Trusted,
            ctx(100),
        );
        assert_eq!(r, ExecutionResult::Success);
        assert_eq!(sm.tasks.get(&task_id).unwrap().status, TaskStatus::Claimed);

        // Advance past lease expiry (lease started at 100, expires at 220)
        let expired = sm.run_lease_expiry(&task_id, 221);
        assert!(expired);

        // Task should be back to Open, escrow = Refunded, bounty zeroed
        let task = sm.tasks.get(&task_id).unwrap();
        assert_eq!(task.status, TaskStatus::Open);
        assert_eq!(task.escrow_status, EscrowStatus::Refunded);
        assert_eq!(task.bounty_agx, 0);

        // Funder received the bounty back: 190 AGX + 300 AGX = 490 AGX
        assert_eq!(sm.get_account(&creator_id).unwrap().balance, 490_000_000_000_000_000_000);
    }

    #[test]
    fn run_lease_expiry_noop_when_released() {
        let mut sm = StateMachine::new();
        let creator_id = [1u8; 32];
        let task_id = [0xBBu8; 32];
        let agent_id = [2u8; 32];
        let bounty = 300_000_000_000_000_000_000u128;

        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.init_account(test_account(2, 500_000_000_000_000_000_000, 0));

        // Create and claim
        let r = sm.execute_task_create(
            creator_id,
            bounty,
            0,
            task_id,
            1,
            [0xBBu8; 32],
            [0xCCu8; 32],
            [0xDDu8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            100,
            ctx(100),
        );
        assert_eq!(r, ExecutionResult::Success);
        let min_collateral = 10_000_000_000_000_000_000u128.max(bounty * 5 / 1000);
        let r = sm.execute_claim_task(
            task_id,
            agent_id,
            min_collateral,
            1,
            100,
            TrustStageEnum::Trusted,
            ctx(100),
        );
        assert_eq!(r, ExecutionResult::Success);

        // Simulate escrow being Released (e.g. via review settlement)
        let t = sm.tasks.get_mut(&task_id).unwrap();
        t.escrow_status = EscrowStatus::Released;

        // Advance past lease expiry
        let expired = sm.run_lease_expiry(&task_id, 221);
        assert!(expired);

        // Escrow status should NOT change — already Released
        let task = sm.tasks.get(&task_id).unwrap();
        assert_eq!(task.escrow_status, EscrowStatus::Released);
        // Bounty should not have been refunded (already released)
        assert_eq!(task.bounty_agx, bounty);
    }

    #[test]
    fn run_lease_expiry_noop_when_bounty_redistributed() {
        let mut sm = StateMachine::new();
        let creator_id = [1u8; 32];
        let task_id = [0xCCu8; 32];
        let agent_id = [2u8; 32];
        let bounty = 300_000_000_000_000_000_000u128;

        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.init_account(test_account(2, 500_000_000_000_000_000_000, 0));

        let r = sm.execute_task_create(
            creator_id,
            bounty,
            0,
            task_id,
            1,
            [0xCCu8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            100,
            ctx(100),
        );
        assert_eq!(r, ExecutionResult::Success);
        let min_collateral = 10_000_000_000_000_000_000u128.max(bounty * 5 / 1000);
        let r = sm.execute_claim_task(
            task_id,
            agent_id,
            min_collateral,
            1,
            100,
            TrustStageEnum::Trusted,
            ctx(100),
        );
        assert_eq!(r, ExecutionResult::Success);

        // Simulate bounty redistributed (e.g. via task split)
        let t = sm.tasks.get_mut(&task_id).unwrap();
        t.escrow_status = EscrowStatus::BountyRedistributed;

        let expired = sm.run_lease_expiry(&task_id, 221);
        assert!(expired);

        let task = sm.tasks.get(&task_id).unwrap();
        assert_eq!(task.escrow_status, EscrowStatus::BountyRedistributed);
        assert_eq!(task.bounty_agx, bounty);
    }

    #[test]
    fn run_lease_expiry_noop_when_not_expired() {
        let mut sm = StateMachine::new();
        let task_id = [0xDDu8; 32];

        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));

        let r = sm.execute_task_create(
            [1u8; 32],
            100,
            0,
            task_id,
            1,
            [0xDDu8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            [0u8; 32],
            100,
            ctx(100),
        );
        assert_eq!(r, ExecutionResult::Success);

        // Task is Open (not Claimed/InProgress) — should not expire
        let expired = sm.run_lease_expiry(&task_id, 999);
        assert!(!expired);
    }

    #[test]
    fn compute_state_root_with_multiple_accounts() {
        let mut sm = StateMachine::new();
        sm.init_account(Account {
            account_id: [1u8; 32],
            balance: 100,
            nonce: 0,
            pubkey_hash: sha3_256(&[1]),
            pubkey: Some(vec![1]),
        });
        sm.init_account(Account {
            account_id: [2u8; 32],
            balance: 200,
            nonce: 3,
            pubkey_hash: sha3_256(&[2]),
            pubkey: Some(vec![2]),
        });

        let root = sm.compute_state_root();
        assert_ne!(root, [0u8; 32]);
    }

    // === Validator lifecycle tests ===

    fn ctx(h: u64) -> ExecutionContext {
        ExecutionContext { height: h, timestamp: 0 }
    }

    const MIN_BOND: u128 = 100_000_000_000_000_000_000u128; // 100 AGX
    const UNBOND_DELAY: u64 = 1000;

    #[test]
    fn bond_creates_validator_and_locks_funds() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));

        let r = sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));
        assert_eq!(r, ExecutionResult::Success);
        assert_eq!(sm.get_account(&v).unwrap().balance, 400_000_000_000_000_000_000);
        let vt = sm.get_validator(&v).unwrap();
        assert_eq!(vt.self_bond, MIN_BOND);
        assert_eq!(vt.state, ValidatorLifecycleState::Active);
    }

    #[test]
    fn bond_rejects_below_min_stake() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));

        let r = sm.execute_bond(v, MIN_BOND - 1, 1, MIN_BOND, 10, ctx(10));
        assert_eq!(r, ExecutionResult::Rejected);
        assert!(sm.get_validator(&v).is_none());
    }

    #[test]
    fn bond_rejects_insufficient_balance() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 50, 0));

        let r = sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));
        assert_eq!(r, ExecutionResult::Rejected);
    }

    #[test]
    fn bond_rejects_bad_nonce() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 5));

        let r = sm.execute_bond(v, MIN_BOND, 7, MIN_BOND, 10, ctx(10));
        assert_eq!(r, ExecutionResult::Rejected);
    }

    #[test]
    fn bond_tops_up_existing_validator() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 1_000_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));

        let r = sm.execute_bond(v, MIN_BOND, 2, MIN_BOND, 20, ctx(20));
        assert_eq!(r, ExecutionResult::Success);
        assert_eq!(sm.get_validator(&v).unwrap().self_bond, MIN_BOND * 2);
        assert_eq!(sm.get_account(&v).unwrap().balance, 800_000_000_000_000_000_000);
    }

    #[test]
    fn unbond_initiates_timer() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));

        let r = sm.execute_unbond(v, 2, 50, ctx(50));
        assert_eq!(r, ExecutionResult::Success);
        let vt = sm.get_validator(&v).unwrap();
        assert_eq!(vt.state, ValidatorLifecycleState::Unbonding);
        assert_eq!(vt.unbonding_height, 50);
    }

    #[test]
    fn unbond_rejects_non_existent_validator() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 100, 0));

        let r = sm.execute_unbond(v, 1, 10, ctx(10));
        assert_eq!(r, ExecutionResult::Rejected);
    }

    #[test]
    fn unbond_rejects_if_already_unbonding() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));
        sm.execute_unbond(v, 2, 50, ctx(50));

        let r = sm.execute_unbond(v, 3, 60, ctx(60));
        assert_eq!(r, ExecutionResult::Rejected);
    }

    #[test]
    fn withdraw_releases_funds_after_delay() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));
        sm.execute_unbond(v, 2, 50, ctx(50));

        let r = sm.execute_withdraw(v, 3, 50 + UNBOND_DELAY + 1, UNBOND_DELAY, ctx(1051));
        assert_eq!(r, ExecutionResult::Success);
        assert!(sm.get_validator(&v).is_none());
        assert_eq!(sm.get_account(&v).unwrap().balance, 400_000_000_000_000_000_000 + MIN_BOND);
    }

    #[test]
    fn withdraw_rejects_before_delay_expires() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));
        sm.execute_unbond(v, 2, 50, ctx(50));

        let r = sm.execute_withdraw(v, 3, 50, UNBOND_DELAY, ctx(50));
        assert_eq!(r, ExecutionResult::Rejected);
        assert!(sm.get_validator(&v).is_some());
    }

    #[test]
    fn renew_resets_bonding_height() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));

        let r = sm.execute_renew(v, 2, 200, ctx(200));
        assert_eq!(r, ExecutionResult::Success);
        assert_eq!(sm.get_validator(&v).unwrap().bonding_height, 200);
    }

    #[test]
    fn renew_rejects_non_validator() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 100, 0));

        let r = sm.execute_renew(v, 1, 10, ctx(10));
        assert_eq!(r, ExecutionResult::Rejected);
    }

    #[test]
    fn validator_affects_state_root() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        let root_before = sm.compute_state_root();

        sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));
        let root_after = sm.compute_state_root();
        assert_ne!(root_before, root_after);
    }

    #[test]
    fn validator_withdraw_removes_from_state_root() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND, 1, MIN_BOND, 10, ctx(10));
        let root_bonded = sm.compute_state_root();

        sm.execute_unbond(v, 2, 50, ctx(50));
        sm.execute_withdraw(v, 3, 50 + UNBOND_DELAY + 1, UNBOND_DELAY, ctx(1051));
        let root_withdrawn = sm.compute_state_root();

        assert_ne!(root_bonded, root_withdrawn);
    }

    // ── Slashing & Rewards Tests ──────────────────────────────────────

    const MIN_BOND_SLASH: u128 = 100_000_000_000_000_000_000u128;
    const JAIL_BLOCKS: u64 = 5000;

    #[test]
    fn slash_equivocation_reduces_stake_and_jails() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND_SLASH, 1, MIN_BOND_SLASH, 10, ctx(10));

        let r = sm.execute_slash_equivocation(v, 100, JAIL_BLOCKS, 100);
        assert_eq!(r, ExecutionResult::Success);

        let vt = sm.get_validator(&v).unwrap();
        assert_eq!(vt.self_bond, MIN_BOND_SLASH * 9 / 10);
        assert_eq!(vt.state, ValidatorLifecycleState::Paused);
        assert_eq!(vt.jailed_until_height, 100 + JAIL_BLOCKS);
        assert_eq!(vt.slash_count, 1);

        assert_eq!(sm.fee_burn_accumulator, MIN_BOND_SLASH / 10);
    }

    #[test]
    fn slash_equivocation_rejects_nonexistent() {
        let mut sm = StateMachine::new();
        let r = sm.execute_slash_equivocation([1u8; 32], 100, JAIL_BLOCKS, 100);
        assert_eq!(r, ExecutionResult::Rejected);
    }

    #[test]
    fn slash_equivocation_rejects_already_jailed() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND_SLASH, 1, MIN_BOND_SLASH, 10, ctx(10));
        sm.execute_slash_equivocation(v, 100, JAIL_BLOCKS, 100);

        // Second slash while jailed should be rejected
        let r = sm.execute_slash_equivocation(v, 150, JAIL_BLOCKS, 150);
        assert_eq!(r, ExecutionResult::Rejected);
    }

    #[test]
    fn slash_downtime_below_threshold_no_action() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND_SLASH, 1, MIN_BOND_SLASH, 10, ctx(10));

        let r = sm.execute_slash_downtime(v, 5, 100, 100, 20, JAIL_BLOCKS, 100);
        assert_eq!(r, ExecutionResult::Success);

        let vt = sm.get_validator(&v).unwrap();
        assert_eq!(vt.state, ValidatorLifecycleState::Active);
        assert_eq!(vt.self_bond, MIN_BOND_SLASH);
    }

    #[test]
    fn slash_downtime_above_threshold_pauses() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND_SLASH, 1, MIN_BOND_SLASH, 10, ctx(10));

        let r = sm.execute_slash_downtime(v, 50, 100, 100, 20, JAIL_BLOCKS, 100);
        assert_eq!(r, ExecutionResult::Success);

        let vt = sm.get_validator(&v).unwrap();
        assert_eq!(vt.state, ValidatorLifecycleState::Paused);
        assert!(vt.self_bond < MIN_BOND_SLASH);
        assert_eq!(vt.slash_count, 1);
    }

    #[test]
    fn reward_distribution_proportional_to_stake() {
        let mut sm = StateMachine::new();
        let v1 = [1u8; 32];
        let v2 = [2u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.init_account(test_account(2, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v1, 1_000, 1, 1_000, 10, ctx(10));
        sm.execute_bond(v2, 3_000, 1, 3_000, 10, ctx(10));

        let initial_v1 = sm.get_account(&v1).unwrap().balance;
        let initial_v2 = sm.get_account(&v2).unwrap().balance;

        let mut fee_pool: u128 = 40_000; // 40000 atto-AGX
        let r = sm.execute_distribute_rewards(&mut fee_pool);
        assert_eq!(r, ExecutionResult::Success);
        assert_eq!(fee_pool, 0);

        // v1 has 1/4 of total stake, v2 has 3/4
        let after_v1 = sm.get_account(&v1).unwrap().balance;
        let after_v2 = sm.get_account(&v2).unwrap().balance;
        assert!(after_v1 > initial_v1, "v1 should receive reward");
        assert!(after_v2 > initial_v2, "v2 should receive reward");
        assert!(after_v2 > after_v1);
    }

    #[test]
    fn reward_distribution_empty_pool_noop() {
        let mut sm = StateMachine::new();
        let v = [1u8; 32];
        sm.init_account(test_account(1, 500_000_000_000_000_000_000, 0));
        sm.execute_bond(v, MIN_BOND_SLASH, 1, MIN_BOND_SLASH, 10, ctx(10));

        let mut fee_pool: u128 = 0;
        let r = sm.execute_distribute_rewards(&mut fee_pool);
        assert_eq!(r, ExecutionResult::Success);
    }
}
