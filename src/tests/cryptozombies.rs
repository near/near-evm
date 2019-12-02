use ethabi::{Address, Uint};
use ethabi_contract::use_contract;

use near_bindgen::MockedBlockchain;
use near_bindgen::{testing_env, Config, VMContext};
use crate::EvmContract;
use crate::sender_name_to_eth_address;

use_contract!(cryptozombies, "src/tests/zombieAttack.abi");

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

fn deploy_cryptozombies(contract: &mut EvmContract) -> String {
    let zombie_code = include_bytes!("zombieAttack.bin").to_vec();
    contract.deploy_code(String::from_utf8(zombie_code).unwrap())
}

fn create_random_zombie(contract: &mut EvmContract, address: String, name: &str) {
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call(name.to_string());
    contract.run_command("zombies".to_owned(), hex::encode(input));
}

fn get_zombies_by_owner(contract: &mut EvmContract, address: String, owner: Address) -> Vec<Uint> {
    let (input, _decoder) = cryptozombies::functions::get_zombies_by_owner::call(owner);
    let output = contract.run_command("zombies".to_owned(), hex::encode(input));
    let output = hex::decode(output);
    cryptozombies::functions::get_zombies_by_owner::decode_output(&output.unwrap()).unwrap()
}

#[test]
// CryptoZombies
fn test_zombies() {
    let config = Config::default();
    let mut context = get_context(vec![]);
    context.signer_account_id = "owner1".to_owned();
    testing_env!(context, config);
    let mut contract = EvmContract::default();

    let address = deploy_cryptozombies(&mut contract);
    create_random_zombie(&mut contract, address.clone(), "zomb1");
    create_random_zombie(&mut contract, address.clone(), "zomb2");
    create_random_zombie(&mut contract, address.clone(), "zomb3");

    let zombies = get_zombies_by_owner(&mut contract, address.clone(), sender_name_to_eth_address("owner1"));
    println!("getZombiesByOwner: {:?}", zombies);

    let zombies = get_zombies_by_owner(&mut contract, address, sender_name_to_eth_address("owner2"));
    println!("getZombiesByOwner: {:?}", zombies);
}
