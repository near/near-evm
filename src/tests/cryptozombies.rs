use ethabi::{Address, Uint};
use ethabi_contract::use_contract;

use crate::{deploy_code, run_command};
use crate::{DeployCodeInput, RunCommandInput, sender_name_to_eth_address};

use super::near_stubs::{get_return_value, set_input, set_sender};

use_contract!(cryptozombies, "src/tests/zombieAttack.abi");

fn set_sender_name(sender: &Address) {
    set_sender(sender.as_bytes().to_vec());
}

fn deploy_cryptozombies() {
    let zombie_code = include_bytes!("zombieAttack.bin").to_vec();
    let code = DeployCodeInput {
        contract_address: "zombies".to_string(),
        bytecode: String::from_utf8(zombie_code).unwrap(),
    };
    set_input(serde_json::to_string(&code).unwrap().into_bytes());
    deploy_code();
}

fn create_random_zombie(name: &str) {
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call(name.to_string());
    let run = RunCommandInput {
        contract_address: "zombies".to_string(),
        encoded_input: hex::encode(input),
    };
    set_input(serde_json::to_string(&run).unwrap().into_bytes());
    run_command();
}

fn get_zombies_by_owner(owner: Address) -> Vec<Uint> {
    let (input, _decoder) = cryptozombies::functions::get_zombies_by_owner::call(owner);
    let run = RunCommandInput {
        contract_address: "zombies".to_string(),
        encoded_input: hex::encode(input),
    };
    set_input(serde_json::to_string(&run).unwrap().into_bytes());
    run_command();
    let data = get_return_value();
    cryptozombies::functions::get_zombies_by_owner::decode_output(&data).unwrap()
}

#[test]
// CryptoZombies
fn test_zombies() {
    let sender = sender_name_to_eth_address("owner1");
    set_sender_name(&sender);
    deploy_cryptozombies();

    create_random_zombie("zomb1");
    create_random_zombie("zomb2");
    create_random_zombie("zomb3");

    let zombies = get_zombies_by_owner(sender_name_to_eth_address("owner1"));
    println!("getZombiesByOwner: {:?}", zombies);

    let zombies = get_zombies_by_owner(sender_name_to_eth_address("owner2"));
    println!("getZombiesByOwner: {:?}", zombies);
}
