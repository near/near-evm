use std::sync::Arc;

use ethabi::{Address, Uint};
use ethabi_contract::use_contract;

use near_crypto::{InMemorySigner, KeyType};
use near_primitives::serialize::from_base64;
use near_primitives::views::FinalExecutionStatus;
use near_testlib::user::{rpc_user::RpcUser, User};

use near_evm::utils::near_account_id_to_evm_address;

#[cfg(test)]
#[macro_use]
extern crate lazy_static_include;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;

use_contract!(cryptozombies, "src/tests/zombieAttack.abi");

lazy_static_include_bytes!(EVM, "res/near_evm.wasm");
lazy_static_include_str!(ZOMBIES, "src/tests/zombieAttack.bin");

const CONTRACT_NAME: &str = "near_evm";
const SIGNER_NAME: &str = "test.near";
const LOTS_OF_GAS: u64 = 10_000_000_000_000_000;

fn create_account(client: &RpcUser, evm_account_signer: &InMemorySigner) {
    let tx_result = client.create_account(
        SIGNER_NAME.to_owned(),
        CONTRACT_NAME.to_owned(),
        evm_account_signer.public_key.clone(),
        10_000_000_000,
    );
    println!("Create account: {:?}\n", tx_result);
}

fn deploy_evm(contract_user: &RpcUser) {
    println!("Deploying evm contract");
    // let contract = include_bytes!("../res/near_evm.wasm").to_vec();
    println!("{:?}", EVM.to_vec().len());
    let contract = EVM.to_vec();
    let tx_result = contract_user.deploy_contract(CONTRACT_NAME.to_owned(), contract);
    println!("Deploy evm contract: {:?}\n", tx_result);
}

fn deploy_cryptozombies(client: &RpcUser) -> String {
    println!("Deploying zombies contract");
    let input = format!("{{\"bytecode\":\"{}\"}}", ZOMBIES);

    let tx_result = client.function_call(
        SIGNER_NAME.to_owned(),
        CONTRACT_NAME.to_owned(),
        "deploy_code",
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );

    if let FinalExecutionStatus::SuccessValue(ref base64) = tx_result.as_ref().unwrap().status {
        let bytes = from_base64(base64).unwrap();
        let addr_bytes = bytes[1..bytes.len() - 1].to_vec();
        let address = String::from_utf8(addr_bytes).unwrap();
        println!("deploy_code(cryptozombies): {}\n", address);
        address
    } else {
        panic!(tx_result)
    }
}

fn create_random_zombie(client: &RpcUser, zombies_address: &str, name: &str) {
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call(name.to_owned());
    let input = format!(
        "{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\"}}",
        zombies_address,
        hex::encode(input)
    );
    let tx_result = client.function_call(
        SIGNER_NAME.to_owned(),
        CONTRACT_NAME.to_owned(),
        "call_contract",
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );
    println!("call_contract(createRandomZombie): {:?}\n", tx_result);
}

fn get_zombies_by_owner(client: &RpcUser, zombies_address: &str, owner: Address) -> Vec<Uint> {
    let (input, _decoder) = cryptozombies::functions::get_zombies_by_owner::call(owner);
    let input = format!(
        "{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\"}}",
        zombies_address,
        hex::encode(input)
    );
    let tx_result = client.function_call(
        SIGNER_NAME.to_owned(),
        CONTRACT_NAME.to_owned(),
        "call_contract",
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );
    println!("call_contract(getZombiesByOwner): {:?}\n", tx_result);
    if let FinalExecutionStatus::SuccessValue(ref base64) = tx_result.as_ref().unwrap().status {
        let bytes = from_base64(base64).unwrap();
        let bytes = hex::decode(&bytes[1..bytes.len() - 1]).unwrap();
        cryptozombies::functions::get_zombies_by_owner::decode_output(&bytes).unwrap()
    } else {
        panic!(tx_result)
    }
}

fn evm_balance_of_near_account(client: &RpcUser, account: String) -> u128 {
    let input = format!("{{\"address\":\"{}\"}}", account);
    let tx_result = client.function_call(
        SIGNER_NAME.to_owned(),
        CONTRACT_NAME.to_owned(),
        "balance_of_near_account",
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );
    println!("evm_balance_of_near_account {} {:?}", account, tx_result);
    if let FinalExecutionStatus::SuccessValue(ref base64) = tx_result.as_ref().unwrap().status {
        let str_rep = from_base64(base64)
            .map(|v| String::from_utf8(v).ok())
            .ok()
            .flatten()
            .unwrap();
        u128::from_str_radix(&str_rep, 10).unwrap()
    } else {
        panic!(tx_result)
    }
}

#[test]
fn test_all_in_one() {
    let addr = "localhost:3030";
    let contract_signer = InMemorySigner::from_seed(CONTRACT_NAME, KeyType::ED25519, CONTRACT_NAME);
    let contract_signer = Arc::new(contract_signer);
    let contract_user = RpcUser::new(addr, "near_evm".to_owned(), contract_signer.clone());

    let devnet_signer = InMemorySigner::from_seed(SIGNER_NAME, KeyType::ED25519, "alice.near");
    let devnet_signer = Arc::new(devnet_signer);
    let devnet_user = RpcUser::new(addr, SIGNER_NAME.to_owned(), devnet_signer.clone());

    create_account(&devnet_user, &contract_signer);
    deploy_evm(&contract_user);
    // End shared test prefix

    let zombies_address = deploy_cryptozombies(&devnet_user);
    create_random_zombie(&devnet_user, &zombies_address, "zomb1");
    let zombies = get_zombies_by_owner(
        &devnet_user,
        &zombies_address,
        near_account_id_to_evm_address(&devnet_signer.account_id),
    );
    assert_eq!(zombies, vec![Uint::from(0)]);

    assert_eq!(
        evm_balance_of_near_account(&devnet_user, SIGNER_NAME.to_owned()),
        0
    );

    println!(
        "{:?}",
        devnet_user.view_balance(&SIGNER_NAME.to_owned()).unwrap()
    );
}
