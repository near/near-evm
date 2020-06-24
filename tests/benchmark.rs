use std::sync::Arc;
use ethabi_contract::use_contract;
use near_primitives::serialize::from_base64;
use near_crypto::{InMemorySigner, KeyType};
use near_primitives::views::FinalExecutionStatus;
use near_testlib::user::{rpc_user::RpcUser, User};
use near_evm::utils;
use ethereum_types::{Address, U256};

#[cfg(test)]
#[macro_use]
extern crate lazy_static_include;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;

use_contract!(cryptozombies, "src/tests/build/ZombieAttack.abi");
use_contract!(erc20, "src/tests/build/ERC20.abi");

lazy_static_include_bytes!(EVM, "res/near_evm.wasm");
lazy_static_include_str!(ZOMBIES, "src/tests/build/ZombieAttack.bin");
lazy_static_include_str!(ERC20, "src/tests/build/ERC20.bin");
// lazy_static_include_str!(PRECOMPILES, "src/tests/build/Precompiles.bin");

const CONTRACT_NAME: &str = "near_evm";
const SIGNER_NAME: &str = "test.near";
const SIGNER_NAME2: &str = "other.near";
const LOTS_OF_GAS: u64 = 500_000_000_000_000; // 100 Tgas
const ACCOUNT_DEPOSIT: u128 = 100_000_000_000_000_000_000_000_000; // 100 NEAR
const SOME_MONEY: u128 = 100_000_000;

////////////////////////////////////////////////
// NEAR-EVM
////////////////////////////////////////////////

fn create_account(client: &RpcUser, evm_account_signer: &InMemorySigner, signer_name: String) {
    let tx_result = client.create_account(
        signer_name.to_owned(),
        CONTRACT_NAME.to_owned(),
        evm_account_signer.public_key.clone(),
        ACCOUNT_DEPOSIT,
    );
    if let FinalExecutionStatus::SuccessValue(_) = tx_result.as_ref().unwrap().status {
        println!("Create account Success");
    }
}

fn deploy_evm(contract_user: &RpcUser) {
    let contract = EVM.to_vec();
    let tx_result = contract_user.deploy_contract(CONTRACT_NAME.to_owned(), contract);

    let gas_burnt = tx_result
                        .as_ref()
                        .unwrap()
                        .transaction_outcome
                        .outcome
                        .gas_burnt;

    println!("deployed evm:\t\t{}", gas_burnt);
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

fn execute_tx(client: &RpcUser, signer_name: String, method_name: String, input: String, output: String, return_addr: bool) -> String {
    let tx_result = client.function_call(
        signer_name,
        CONTRACT_NAME.to_owned(),
        &method_name,
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );
    // if output == "transfer:\t\t" {
    //     println!("tx_result: {:?}", tx_result)
    // }
    if let FinalExecutionStatus::SuccessValue(ref base64) = tx_result.as_ref().unwrap().status {
        let address;
        if return_addr {
            let bytes = from_base64(&base64.to_owned()).unwrap();
            let addr_bytes = bytes[1..bytes.len() - 1].to_vec();
            address = String::from_utf8(addr_bytes).unwrap();
        } else {
            address = "".to_string();
        }
        println!(
            "{}{}",
            output,
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
            "{} failed: {:?}",
            method_name,
            tx_result
        ))
    }
}

////////////////////////////////////////////////
// CRYPTOZOMBIES
////////////////////////////////////////////////

fn deploy_cryptozombies(client: &RpcUser) -> String {
    let input = format!("{{\"bytecode\":\"{}\"}}", ZOMBIES);
    execute_tx(
        client,
        SIGNER_NAME.to_owned(),
        "deploy_code".to_string(),
        input,
        "deployed cryptozombies:\t".to_owned(),
        true
    )
}

fn create_random_zombie(client: &RpcUser, zombies_address: &str, name: &str) {
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call(name.to_owned());
    let input = format!(
        "{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\"}}",
        zombies_address,
        hex::encode(input)
    );
    execute_tx(
        client,
        SIGNER_NAME.to_owned(),
        "call_contract".to_string(),
        input,
        "createRandomZombie:\t".to_owned(),
        false
    );
}

////////////////////////////////////////////////
// ERC20
////////////////////////////////////////////////

fn deploy_erc20(client: &RpcUser) -> String {
    let input = format!("{{\"bytecode\":\"{}\"}}", ERC20);
    execute_tx(
        client,
        SIGNER_NAME.to_owned(),
        "deploy_code".to_string(),
        input,
        "deploy erc20:\t\t".to_owned(),
        true
    )
}

fn transfer_erc20(client: &RpcUser, erc20_address: &str, amount: U256) {
    let recipient = (utils::near_account_id_to_evm_address("random")).0;
    let (input, _decoder) = erc20::functions::transfer::call(recipient, amount);
    let input = format!(
        "{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\"}}",
        erc20_address,
        hex::encode(input)
    );

    execute_tx(
        client,
        SIGNER_NAME.to_owned(),
        "call_contract".to_string(),
        input,
        "transfer:\t\t".to_owned(),
        false
    );
}

fn approve_erc20(client: &RpcUser, erc20_address: &str, spender: Address, amount: U256) {
    let (input, _decoder) = erc20::functions::approve::call(spender, amount);
    let input = format!(
        "{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\"}}",
        erc20_address,
        hex::encode(input)
    );

    execute_tx(
        client,
        SIGNER_NAME.to_owned(),
        "call_contract".to_string(),
        input,
        "approve:\t\t".to_owned(),
        false
    );
}

fn transfer_from_erc20(client: &RpcUser, erc20_address: &str, spender: Address, recipient: Address, amount: U256) {
    let (input, _decoder) = erc20::functions::transfer_from::call(spender, recipient, amount);
    let input = format!(
        "{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\"}}",
        erc20_address,
        hex::encode(input)
    );

    execute_tx(
        client,
        CONTRACT_NAME.to_owned(),
        "call_contract".to_string(),
        input,
        "transfer_from:\t\t".to_owned(),
        false
    );
}

fn increase_allowance_erc20(client: &RpcUser, erc20_address: &str, spender: Address, amount: U256) {
    let (input, _decoder) = erc20::functions::increase_allowance::call(spender, amount);
    let input = format!(
        "{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\"}}",
        erc20_address,
        hex::encode(input)
    );
    execute_tx(
        client,
        SIGNER_NAME.to_owned(),
        "call_contract".to_string(),
        input,
        "increase allowance:\t".to_owned(),
        false
    );
}



////////////////////////////////////////////////
// Precompiles
////////////////////////////////////////////////

fn deploy_precompiles(client: &RpcUser) -> String {
    let input = "".to_string();//format!("{{\"bytecode\":\"{}\"}}", PRECOMPILES);
    execute_tx(
        client,
        SIGNER_NAME.to_owned(),
        "deploy_code".to_string(),
        input,
        "deploy erc20:\t\t".to_owned(),
        true
    )
}


#[test]
fn bench_test_all_in_one() {
    // Begin shared test prefix
    let addr = "localhost:3030";
    let contract_signer = InMemorySigner::from_seed(CONTRACT_NAME, KeyType::ED25519, CONTRACT_NAME);
    let contract_signer = Arc::new(contract_signer);
    let contract_user = RpcUser::new(addr, "near_evm".to_owned(), contract_signer.clone());
    let contract_user_ethaddr = utils::near_account_id_to_evm_address(CONTRACT_NAME);

    let devnet_signer = InMemorySigner::from_seed(SIGNER_NAME, KeyType::ED25519, "alice.near");
    let devnet_signer = Arc::new(devnet_signer);
    let devnet_user = RpcUser::new(addr, SIGNER_NAME.to_owned(), devnet_signer.clone());
    let devnet_user_ethaddr = utils::near_account_id_to_evm_address(SIGNER_NAME);

    create_account(&devnet_user, &contract_signer, SIGNER_NAME.to_owned());
    deploy_evm(&contract_user);
    // End shared test prefix
    //
    // ///////// CRYPTOZOMBIES //////
    // let zombies_address = deploy_cryptozombies(&devnet_user);
    // create_random_zombie(&devnet_user, &zombies_address, "zomb1");
    //
    // ///////// ERC20 //////
    // let erc20_address = deploy_erc20(&devnet_user);
    // transfer_erc20(&devnet_user, &erc20_address, U256::from(20));
    // approve_erc20(&devnet_user, &erc20_address, contract_user_ethaddr, U256::from(10));
    // transfer_from_erc20(&contract_user, &erc20_address, devnet_user_ethaddr, contract_user_ethaddr, U256::from(5));
    // increase_allowance_erc20(&devnet_user, &erc20_address, contract_user_ethaddr, U256::from(30));

    ///////// Precompiles //////
    // let precompiles_address = deploy_precompiles(&devnet_user);
}
