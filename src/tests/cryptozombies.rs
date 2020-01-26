use ethabi::{Address, Uint};
use ethabi_contract::use_contract;

use crate::EvmContract;
use crate::utils::sender_name_to_eth_address;

use super::test_utils;

use_contract!(cryptozombies, "src/tests/zombieAttack.abi");

fn deploy_cryptozombies(contract: &mut EvmContract) {
    let zombie_code = include_bytes!("zombieAttack.bin").to_vec();
    contract.deploy_code("zombies".to_owned(), String::from_utf8(zombie_code).unwrap());
}

fn create_random_zombie(contract: &mut EvmContract, name: &str) {
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call(name.to_string());
    contract.call_contract("zombies".to_owned(), hex::encode(input));
}

fn get_zombies_by_owner(contract: &mut EvmContract, owner: Address) -> Vec<Uint> {
    let (input, _decoder) = cryptozombies::functions::get_zombies_by_owner::call(owner);
    let output = contract.call_contract("zombies".to_owned(), hex::encode(input));
    let output = hex::decode(output);
    cryptozombies::functions::get_zombies_by_owner::decode_output(&output.unwrap()).unwrap()
}


#[test]
#[should_panic]
fn test_double_deploy() {
    test_utils::run_test(|mut contract| {
        deploy_cryptozombies(&mut contract);
        deploy_cryptozombies(&mut contract);
    })
}

#[test]
// CryptoZombies
fn test_create_random_zombie() {
    test_utils::run_test(|mut contract| {
        deploy_cryptozombies(&mut contract);

        assert_eq!(
            get_zombies_by_owner(&mut contract, sender_name_to_eth_address("owner1")),
            []
        );

        create_random_zombie(&mut contract, "zomb1");
        assert_eq!(
            get_zombies_by_owner(&mut contract, sender_name_to_eth_address("owner1")),
            [Uint::from(0)]
        );

        create_random_zombie(&mut contract, "zomb2");
        assert_eq!(
            get_zombies_by_owner(&mut contract, sender_name_to_eth_address("owner1")),
            [Uint::from(0)]
        );
    });
}
