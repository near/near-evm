use std::sync::Arc;
use ethabi_contract::use_contract;
use near_primitives::serialize::from_base64;
use near_crypto::{InMemorySigner, KeyType};
use near_primitives::views::FinalExecutionStatus;
use near_testlib::user::{rpc_user::RpcUser, User};

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
        // } else {
        //     panic!(format!("Create account Failed {:?}", tx_result));
    }
}

fn deploy_evm(contract_user: &RpcUser) {
    println!("Deploying evm contract");
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
            "deployed cryptozombies:\t{}",
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

#[test]
fn bench_test_all_in_one() {
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

    let zombies_address = deploy_cryptozombies(&devnet_user);
}
