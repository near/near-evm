use ethabi_contract::use_contract;

use near_bindgen::MockedBlockchain;
use near_bindgen::{testing_env, VMContext};
use crate::EvmContract;
use crate::sender_name_to_eth_address;

use_contract!(cryptokitties, "src/tests/kittyCore.abi");

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
        account_locked_balance: 0,
        storage_usage: 0,
        attached_deposit: 0,
        prepaid_gas: 10u64.pow(9),
        random_seed: vec![0, 1, 2],
        is_view: false,
        output_data_receivers: vec![],
    }
}

fn deploy_cryptokitties(contract: &mut EvmContract) {
    let kitty_code = include_bytes!("kittyCore.bin").to_vec();
    contract.deploy_code("kitties".to_owned(), String::from_utf8(kitty_code).unwrap());
}

fn create_promo_kitty(contract: &mut EvmContract) {
    let (input, _decoder) =
        cryptokitties::functions::create_promo_kitty::call(0, sender_name_to_eth_address("cat"));
    contract.run_command("kitties".to_owned(), hex::encode(input));
}

#[test]
fn test_kitties() {
    let mut context = get_context(vec![]);
    context.signer_account_id = "owner1".to_owned();
    testing_env!(context);
    let mut contract = EvmContract::default();
    deploy_cryptokitties(&mut contract);
    create_promo_kitty(&mut contract);
}
