use ethabi::{Address, Uint};
use ethabi_contract::use_contract;

use crate::utils;
use crate::EvmContract;

use super::test_utils;

use_contract!(cryptozombies, "src/tests/zombieAttack.abi");

fn deploy_cryptozombies(contract: &mut EvmContract) -> String {
    let zombie_code = include_bytes!("zombieAttack.bin").to_vec();
    contract.deploy_code(
        String::from_utf8(zombie_code).unwrap(),
    )
}

fn create_random_zombie(contract: &mut EvmContract, addr: &String, name: &str) {
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call(name.to_string());
    contract.call_contract(addr.to_string(), hex::encode(input));
}

fn get_zombies_by_owner(contract: &mut EvmContract, addr: &String, owner: Address) -> Vec<Uint> {
    let (input, _decoder) = cryptozombies::functions::get_zombies_by_owner::call(owner);
    let output = contract.call_contract(addr.to_string(), hex::encode(input));
    let output = hex::decode(output);
    cryptozombies::functions::get_zombies_by_owner::decode_output(&output.unwrap()).unwrap()
}


#[test]
// CryptoZombies
fn test_create_random_zombie() {
    test_utils::run_test(0, |mut contract| {
        let addr = deploy_cryptozombies(&mut contract);
        assert_eq!(
            get_zombies_by_owner(
                &mut contract,
                &addr,
                utils::near_account_id_to_eth_address(&"owner1".to_string())
            ),
            []
        );

        create_random_zombie(&mut contract, &addr, "zomb1");
        assert_eq!(
            get_zombies_by_owner(
                &mut contract,
                &addr,
                utils::near_account_id_to_eth_address(&"owner1".to_string())
            ),
            [Uint::from(0)]
        );

        create_random_zombie(&mut contract, &addr, "zomb2");
        assert_eq!(
            get_zombies_by_owner(
                &mut contract,
                &addr,
                utils::near_account_id_to_eth_address(&"owner1".to_string())
            ),
            [Uint::from(0)]
        );
    });
}
