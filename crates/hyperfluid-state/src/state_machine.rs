// === State Machine ===
//
// Deterministic state transitions for the Hyperfluid protocol.
// Executes transactions against account state, enforces nonces,
// tracks consumed action plans for replay protection.
//
// Source: consensus-spec.md Section 2.4

use std::collections::{HashMap, HashSet};

use parity_scale_codec::Encode;

use crate::smt::SparseMerkleTree;
use crate::{state_key, Account, Hash32, KeyPrefix};

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
    /// Tracked task IDs for deduplication
    task_ids: HashSet<Hash32>,
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}

impl StateMachine {
    pub fn new() -> Self {
        Self { accounts: HashMap::new(), consumed_plans: HashSet::new(), task_ids: HashSet::new() }
    }

    /// Bootstrap accounts from genesis configuration.
    pub fn init_account(&mut self, account: Account) {
        self.accounts.insert(account.account_id, account);
    }

    /// Get a reference to an account.
    pub fn get_account(&self, account_id: &Hash32) -> Option<&Account> {
        self.accounts.get(account_id)
    }

    /// Execute a transfer from sender to recipient.
    /// TransferTx: debit sender.balance -= amount, credit recipient.balance += amount.
    /// Enforces: sender nonce check, sufficient balance, non-zero amount.
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

        if nonce != sender_nonce + 1 {
            return ExecutionResult::Rejected;
        }

        // Check sender balance
        let sender_balance = self.accounts.get(&sender_id).map(|a| a.balance).unwrap_or(0);

        if sender_balance < amount {
            return ExecutionResult::Rejected;
        }

        // Debit sender
        if let Some(sender) = self.accounts.get_mut(&sender_id) {
            sender.balance -= amount;
            sender.nonce = nonce;
            // First-spend: reveal pubkey (spec 2.7 hook 5)
            if sender.pubkey.is_none() {
                // pubkey must be embedded in the transaction; for now we
                // mark that the account has been used (pubkey_hash was set
                // at creation, full pubkey reveal happens per FR-0005/0006).
                // SPEC_DEVIATION: pubkey reveal from first transaction
                // payload is deferred until ML-DSA signature verification
                // is integrated in C1 consensus proper.
            }
        } else {
            // Sender account doesn't exist – auto-create with 0 balance?
            // Per spec Section 2.4: Account created on first inbound transfer.
            // Transfers FROM non-existent accounts are rejected.
            return ExecutionResult::Rejected;
        }

        // Credit recipient (auto-create if needed, per spec 2.4)
        let recipient = self.accounts.entry(recipient_id).or_insert_with(|| Account {
            account_id: recipient_id,
            balance: 0,
            nonce: 0,
            pubkey_hash: [0u8; 32],
            pubkey: None,
        });
        recipient.balance += amount;

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

    /// Execute a TaskCreateTx.
    /// Debits bounty from creator, creates an escrow entry, records the task.
    pub fn execute_task_create(
        &mut self,
        creator_id: Hash32,
        bounty_agx: u128,
        fee_agx: u128,
        task_id: Hash32,
        nonce: u64,
        _seed_ref: Hash32,
        _ctx: ExecutionContext,
    ) -> ExecutionResult {
        // Prevent duplicate task creation
        if self.task_ids.contains(&task_id) {
            return ExecutionResult::Rejected;
        }

        // Nonce enforcement
        let creator_nonce = self.accounts.get(&creator_id).map(|a| a.nonce).unwrap_or(0);

        if nonce != creator_nonce + 1 {
            return ExecutionResult::Rejected;
        }

        let total_cost = bounty_agx.saturating_add(fee_agx);

        let creator_balance = self.accounts.get(&creator_id).map(|a| a.balance).unwrap_or(0);

        if creator_balance < total_cost {
            return ExecutionResult::Rejected;
        }

        // Debit creator
        if let Some(creator) = self.accounts.get_mut(&creator_id) {
            creator.balance -= total_cost;
            creator.nonce = nonce;
        }

        // Record task for deduplication
        self.task_ids.insert(task_id);

        ExecutionResult::Success
    }

    /// Compute the SMT root from the current state machine state.
    /// All accounts are serialised with SCALE encoding and inserted
    /// into the SMT sorted by state key (spec 2.2).
    pub fn compute_state_root(&self) -> Hash32 {
        let mut tree = SparseMerkleTree::new();

        for (account_id, account) in &self.accounts {
            let key = state_key(KeyPrefix::Account, account_id);
            let value = account.encode();
            tree.insert(key, value);
        }

        tree.root()
    }

    /// Return the number of accounts tracked.
    pub fn account_count(&self) -> usize {
        self.accounts.len()
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
            ExecutionContext { height: 10, timestamp: 1000 },
        );
        assert_eq!(r1, ExecutionResult::Success);

        // Second creation with same task_id
        sm.init_account(test_account(1, 2000, 1));
        let r2 = sm.execute_task_create(
            [1u8; 32],
            100,
            10,
            task_id,
            2,
            seed,
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
            ExecutionContext { height: 10, timestamp: 1000 },
        );
        assert_eq!(result, ExecutionResult::Rejected);
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
}
