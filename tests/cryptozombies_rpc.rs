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

use_contract!(cryptozombies, "src/tests/build/ZombieAttack.abi");

lazy_static_include_bytes!(EVM, "res/near_evm.wasm");
lazy_static_include_str!(ZOMBIES, "src/tests/build/ZombieAttack.bin");

const CONTRACT_NAME: &str = "near_evm";
const SIGNER_NAME: &str = "test.near";
const LOTS_OF_GAS: u64 = 500_000_000_000_000; // 100 Tgas
const ACCOUNT_DEPOSIT: u128 = 100_000_000_000_000_000_000_000_000; // 100 NEAR
const SOME_MONEY: u128 = 100_000_000;

fn create_account(client: &RpcUser, evm_account_signer: &InMemorySigner) {
    let tx_result = client.create_account(
        SIGNER_NAME.to_owned(),
        CONTRACT_NAME.to_owned(),
        evm_account_signer.public_key.clone(),
        ACCOUNT_DEPOSIT,
    );
    if let FinalExecutionStatus::SuccessValue(_) = tx_result.as_ref().unwrap().status {
        println!("Create account Success");
    }
}

fn deploy_evm(contract_user: &RpcUser) {
    println!("Deploying evm contract");
    // let contract = include_bytes!("../res/near_evm.wasm").to_vec();
    let contract = EVM.to_vec();
    let tx_result = contract_user.deploy_contract(CONTRACT_NAME.to_owned(), contract);
    if let FinalExecutionStatus::SuccessValue(_) = tx_result.as_ref().unwrap().status {
        println!(
            "Deploy Evm Success, gas = {}",
            tx_result
                .as_ref()
                .unwrap()
                .transaction_outcome
                .outcome
                .gas_burnt
        );
    } else {
        panic!(format!("Deploy Evm Failed {:?}", tx_result));
    }
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
        println!(
            "deploy_code(cryptozombies): {}, gas burnt: {}\n",
            address,
            tx_result
                .as_ref()
                .unwrap()
                .transaction_outcome
                .outcome
                .gas_burnt
        );
        address
    } else {
        panic!(format!(
            "deploy_code(cryptozombies) failed: {:?}",
            tx_result
        ))
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
    if let FinalExecutionStatus::SuccessValue(_) = tx_result.as_ref().unwrap().status {
        println!(
            "createRandomZombie Success, gas burnt: {}",
            tx_result
                .as_ref()
                .unwrap()
                .transaction_outcome
                .outcome
                .gas_burnt
        );
    } else {
        panic!(format!("createRandomZombie Failed {:?}", tx_result));
    }
}

fn get_zombies_by_owner(client: &RpcUser, zombies_address: &str, owner: Address) -> Vec<Uint> {
    let (input, _decoder) = cryptozombies::functions::get_zombies_by_owner::call(owner);
    let input = format!(
        "{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\",\"sender\":\"{}\", \"value\":\"0\"}}",
        zombies_address,
        hex::encode(input),
        owner
    );
    let tx_result = client.function_call(
        SIGNER_NAME.to_owned(),
        CONTRACT_NAME.to_owned(),
        "view_call_contract",
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );
    if let FinalExecutionStatus::SuccessValue(ref base64) = tx_result.as_ref().unwrap().status {
        let bytes = from_base64(base64).unwrap();
        let bytes = hex::decode(&bytes[1..bytes.len() - 1]).unwrap();
        let res = cryptozombies::functions::get_zombies_by_owner::decode_output(&bytes).unwrap();
        println!("view_call_contract(getZombiesByOwner): {:?}\n", res);
        res
    } else {
        panic!(tx_result)
    }
}

fn add_near(client: &RpcUser) {
    let tx_result = client.function_call(
        SIGNER_NAME.to_owned(),
        CONTRACT_NAME.to_owned(),
        "add_near",
        vec![],
        LOTS_OF_GAS,
        100_000_000,
    );
    if let FinalExecutionStatus::SuccessValue(_) = tx_result.as_ref().unwrap().status {
        println!(
            "Add Near Success, gas burnt: {}",
            tx_result
                .as_ref()
                .unwrap()
                .transaction_outcome
                .outcome
                .gas_burnt
        );
    } else {
        panic!(format!("Add Near Failed {:?}", tx_result));
    }
}

fn retrieve_near(client: &RpcUser) {
    let input = format!(
        "{{\"recipient\":\"{}\",\"amount\":\"{}\"}}",
        SIGNER_NAME.to_owned(),
        SOME_MONEY
    );
    let tx_result = client.function_call(
        SIGNER_NAME.to_owned(),
        CONTRACT_NAME.to_owned(),
        "retrieve_near",
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );
    if let FinalExecutionStatus::SuccessValue(_) = tx_result.as_ref().unwrap().status {
        println!(
            "Retrieve Near Success, gas burnt: {}",
            tx_result
                .as_ref()
                .unwrap()
                .transaction_outcome
                .outcome
                .gas_burnt
        );
    } else {
        panic!(format!("Retrieve Near Failed {:?}", tx_result));
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
    if let FinalExecutionStatus::SuccessValue(ref base64) = tx_result.as_ref().unwrap().status {
        let json_rep = from_base64(base64)
            .map(|v| String::from_utf8(v).ok())
            .ok()
            .flatten()
            .unwrap();
        let balance_str = serde_json::from_str(&json_rep).expect("Failed to parse JSON response");
        let balance = u128::from_str_radix(balance_str, 10).expect("Failed to parse integer");
        println!(
            "evm_balance_of_near_account {} {:?}, gas burnt: {}",
            account,
            balance,
            tx_result
                .as_ref()
                .unwrap()
                .transaction_outcome
                .outcome
                .gas_burnt
        );
        balance
    } else {
        panic!(tx_result)
    }
}

#[test]
fn test_all_in_one() {
    // Begin shared test prefix
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

    // Begin zombie call tests
    let zombies_address = deploy_cryptozombies(&devnet_user);
    create_random_zombie(&devnet_user, &zombies_address, "zomb1");
    let zombies = get_zombies_by_owner(
        &devnet_user,
        &zombies_address,
        near_account_id_to_evm_address(&devnet_signer.account_id),
    );
    assert_eq!(zombies, vec![Uint::from(0)]);

    // Begin add_near test
    let evm_start_bal = evm_balance_of_near_account(&devnet_user, SIGNER_NAME.to_owned());
    let near_start_bal = devnet_user.view_balance(&SIGNER_NAME.to_owned()).unwrap();
    add_near(&devnet_user);
    assert_eq!(
        devnet_user.view_balance(&SIGNER_NAME.to_owned()).unwrap(),
        near_start_bal - SOME_MONEY
    );
    assert_eq!(
        devnet_user.view_balance(&CONTRACT_NAME.to_owned()).unwrap(),
        ACCOUNT_DEPOSIT + evm_start_bal + SOME_MONEY
    );
    assert_eq!(
        evm_balance_of_near_account(&devnet_user, SIGNER_NAME.to_owned()),
        evm_start_bal + SOME_MONEY
    );

    // Begin retrieve_near test
    retrieve_near(&devnet_user);
    assert_eq!(
        evm_balance_of_near_account(&devnet_user, SIGNER_NAME.to_owned()),
        evm_start_bal
    );
    assert_eq!(
        devnet_user.view_balance(&SIGNER_NAME.to_owned()).unwrap(),
        near_start_bal
    );
    assert_eq!(
        devnet_user.view_balance(&CONTRACT_NAME.to_owned()).unwrap(),
        ACCOUNT_DEPOSIT + evm_start_bal
    );
}
