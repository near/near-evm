use near_bindgen::{Config, testing_env, VMContext};
use near_bindgen::MockedBlockchain;

use crate::EvmContract;

fn get_context(input: Vec<u8>) -> VMContext {
    VMContext {
        current_account_id: "evm.near".to_string(),
        signer_account_id: "bob.near".to_string(),
        signer_account_pk: vec![0, 1, 2],
        predecessor_account_id: "carol.near".to_string(),
        input,
        block_index: 0,
        block_timestamp: 0,
        account_balance: 0,
        storage_usage: 0,
        attached_deposit: 0,
        prepaid_gas: 10u64.pow(9),
        random_seed: vec![0, 1, 2],
        is_view: false,
        output_data_receivers: vec![],
    }
}

fn deploy_migrations(contract: &mut EvmContract) -> String {
    let code = include_bytes!("migrations.bin").to_vec();
    contract.deploy_code(String::from_utf8(code).unwrap())
}

#[test]
fn test_truffle() {
    let config = Config::default();
    let mut context = get_context(vec![]);
    context.signer_account_id = "owner1".to_owned();
    testing_env!(context, config);
    let mut contract = EvmContract::default();
    let _ = deploy_migrations(&mut contract);
}
