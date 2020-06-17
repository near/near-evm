use near_sdk::MockedBlockchain;
use near_sdk::{testing_env, VMContext};

use crate::EvmContract;

fn get_context(input: Vec<u8>) -> VMContext {
    VMContext {
        current_account_id: "zombies".to_string(),
        signer_account_id: "owner1".to_string(),
        signer_account_pk: vec![0, 1, 2],
        predecessor_account_id: "owner1".to_string(),
        input,
        block_index: 0,
        block_timestamp: 0,
        epoch_height: 0,
        account_balance: 0,
        account_locked_balance: 0,
        storage_usage: 0,
        attached_deposit: 0,
        prepaid_gas: 2u64.pow(63),
        random_seed: vec![0, 1, 2],
        is_view: false,
        output_data_receivers: vec![],
    }
}

pub fn initialize() -> EvmContract {
    set_default_context();
    return EvmContract::default();
}

pub fn set_default_context() {
    let context = get_context(vec![]);
    testing_env!(context);
}

pub fn attach_deposit(attached_deposit: u128) {
    let mut context = get_context(vec![]);
    context.attached_deposit = attached_deposit;
    testing_env!(context);
}
