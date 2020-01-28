use near_bindgen::MockedBlockchain;
use near_bindgen::{testing_env, VMContext};

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

pub fn run_test<T>(attached_deposit: u128, test: T) -> ()
where
    T: FnOnce(&mut EvmContract) -> (),
{
    let mut context = get_context(vec![]);
    context.attached_deposit = attached_deposit;
    context.account_balance = attached_deposit;
    testing_env!(context);
    let mut contract = EvmContract::default();
    test(&mut contract)
}
