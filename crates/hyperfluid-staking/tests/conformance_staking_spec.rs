// Conformance tests for staking-spec.md Section 1.7 (Validator Lifecycle)
// and delegation (FR-0020a)
//
// Source: docs/04-specifications/protocol/staking-spec.md Section 1.7

use hyperfluid_consensus::types::{DelegationAction, GovernanceAction, StakingAction, TxType};
use hyperfluid_state::state_machine::{ExecutionContext, ExecutionResult, StateMachine};
use hyperfluid_state::Account;

fn test_account(id: u8, balance: u128, nonce: u64) -> Account {
    let account_id = [id; 32];
    Account { account_id, balance, nonce, pubkey_hash: [0u8; 32], pubkey: Some(vec![id]) }
}

fn ctx(height: u64) -> ExecutionContext {
    ExecutionContext { height, timestamp: height * 10 }
}

#[test]
fn conforms_to_staking_spec_1_7_delegate_credits_validator() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 10000, 0)); // delegator
    sm.init_account(test_account(2, 0, 0)); // validator account
    sm.init_validator([2u8; 32], 1_000_000_000_000_000_000_000, 0);

    let min_delegation = 1u128;
    let result = sm.execute_delegate([1u8; 32], [2u8; 32], 500, 1, min_delegation, ctx(10));
    assert_eq!(result, ExecutionResult::Success);
    assert_eq!(sm.get_account(&[1u8; 32]).unwrap().balance, 9500);
}

#[test]
fn conforms_to_staking_spec_1_7_delegate_below_minimum_rejected() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 10000, 0));
    sm.init_account(test_account(2, 0, 0));

    let min_delegation = 1000u128;
    let result = sm.execute_delegate([1u8; 32], [2u8; 32], 500, 1, min_delegation, ctx(10));
    assert_eq!(result, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_staking_spec_1_7_delegate_self_rejected() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 10000, 0));

    let min_delegation = 1u128;
    let result = sm.execute_delegate([1u8; 32], [1u8; 32], 500, 1, min_delegation, ctx(10));
    assert_eq!(result, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_staking_spec_1_7_delegate_insufficient_balance() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 100, 0));
    sm.init_account(test_account(2, 0, 0));

    let min_delegation = 1u128;
    let result = sm.execute_delegate([1u8; 32], [2u8; 32], 500, 1, min_delegation, ctx(10));
    assert_eq!(result, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_staking_spec_1_7_undelegate_initiates_unbonding() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 10000, 0));
    sm.init_account(test_account(2, 0, 0));
    sm.init_validator([2u8; 32], 1_000_000_000_000_000_000_000, 0);

    let min_delegation = 1u128;
    sm.execute_delegate([1u8; 32], [2u8; 32], 500, 1, min_delegation, ctx(10));

    let result = sm.execute_undelegate([1u8; 32], [2u8; 32], 2, 20, ctx(20));
    assert_eq!(result, ExecutionResult::Success);
}

#[test]
fn conforms_to_staking_spec_1_7_undelegate_nonexistent_rejected() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 10000, 0));

    let result = sm.execute_undelegate([1u8; 32], [2u8; 32], 1, 20, ctx(20));
    assert_eq!(result, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_staking_spec_1_7_withdraw_delegation_after_delay() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 10000, 0));
    sm.init_account(test_account(2, 0, 0));
    sm.init_validator([2u8; 32], 1_000_000_000_000_000_000_000, 0);

    let min_delegation = 1u128;
    let delegation_unbond_delay = 60_480u64;

    sm.execute_delegate([1u8; 32], [2u8; 32], 500, 1, min_delegation, ctx(10));
    sm.execute_undelegate([1u8; 32], [2u8; 32], 2, 20, ctx(20));

    let result = sm.execute_withdraw_delegation(
        [1u8; 32],
        [2u8; 32],
        3,
        20 + delegation_unbond_delay + 1,
        delegation_unbond_delay,
        ctx(20 + delegation_unbond_delay + 1),
    );
    assert_eq!(result, ExecutionResult::Success);
    assert_eq!(sm.get_account(&[1u8; 32]).unwrap().balance, 10000);
}

#[test]
fn conforms_to_staking_spec_1_7_withdraw_delegation_before_delay_rejected() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 10000, 0));
    sm.init_account(test_account(2, 0, 0));

    let min_delegation = 1u128;
    let delegation_unbond_delay = 60_480u64;

    sm.execute_delegate([1u8; 32], [2u8; 32], 500, 1, min_delegation, ctx(10));
    sm.execute_undelegate([1u8; 32], [2u8; 32], 2, 20, ctx(20));

    let result = sm.execute_withdraw_delegation(
        [1u8; 32],
        [2u8; 32],
        3,
        21,
        delegation_unbond_delay,
        ctx(21),
    );
    assert_eq!(result, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_staking_spec_1_7_withdraw_delegation_nonexistent_rejected() {
    let mut sm = StateMachine::new();
    sm.init_account(test_account(1, 10000, 0));

    let result = sm.execute_withdraw_delegation([1u8; 32], [2u8; 32], 1, 100, 60_480, ctx(100));
    assert_eq!(result, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_staking_spec_1_7_set_commission_within_range() {
    let mut sm = StateMachine::new();
    let v = [1u8; 32];
    sm.init_account(test_account(1, 100_000_000_000_000_000_000, 0));
    sm.init_validator(v, 100_000_000_000_000_000_000, 1);

    let max_commission = 20u8;
    let result = sm.execute_set_commission(v, 10, 1, max_commission, ctx(10));
    assert_eq!(result, ExecutionResult::Success);
}

#[test]
fn conforms_to_staking_spec_1_7_set_commission_exceeds_max() {
    let mut sm = StateMachine::new();
    let v = [1u8; 32];
    sm.init_account(test_account(1, 100_000_000_000_000_000_000, 0));
    sm.init_validator(v, 100_000_000_000_000_000_000, 1);

    let max_commission = 20u8;
    let result = sm.execute_set_commission(v, 50, 2, max_commission, ctx(10));
    assert_eq!(result, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_staking_spec_1_7_set_commission_nonexistent_validator() {
    let mut sm = StateMachine::new();

    let max_commission = 20u8;
    let result = sm.execute_set_commission([99u8; 32], 10, 1, max_commission, ctx(10));
    assert_eq!(result, ExecutionResult::Rejected);
}

#[test]
fn conforms_to_staking_spec_1_7_delegation_tx_types_exist() {
    let types = [
        TxType::DelegationTx(DelegationAction::Delegate),
        TxType::DelegationTx(DelegationAction::Undelegate),
        TxType::DelegationTx(DelegationAction::WithdrawDelegation),
        TxType::DelegationTx(DelegationAction::SetCommission),
    ];
    assert_eq!(types.len(), 4);
}

#[test]
fn conforms_to_staking_spec_1_7_staking_tx_types_exist() {
    let types = [
        TxType::StakingTx(StakingAction::Bond),
        TxType::StakingTx(StakingAction::Renew),
        TxType::StakingTx(StakingAction::Unbond),
        TxType::StakingTx(StakingAction::Withdraw),
    ];
    assert_eq!(types.len(), 4);
}

#[test]
fn conforms_to_staking_spec_1_7_governance_tx_types_exist() {
    let types = [
        TxType::GovernanceTx(GovernanceAction::Propose),
        TxType::GovernanceTx(GovernanceAction::Vote),
    ];
    assert_eq!(types.len(), 2);
}
