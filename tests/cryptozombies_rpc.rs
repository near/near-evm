use std::sync::Arc;

use ethabi::{Address, Uint};
use ethabi_contract::use_contract;

use near_crypto::{InMemorySigner, KeyType};
use near_primitives::serialize::from_base64;
use near_primitives::views::{FinalExecutionStatus, ExecutionStatusView};
use near_testlib::user::{rpc_user::RpcUser, User};

use near_evm::utils::near_account_id_to_evm_address;

#[cfg(test)] #[macro_use] extern crate lazy_static_include;
#[cfg(test)] #[macro_use] extern crate lazy_static;

use_contract!(cryptozombies, "src/tests/zombieAttack.abi");

lazy_static_include_bytes!(EVM, "res/near_evm.wasm");
lazy_static_include_str!(ZOMBIES, "src/tests/zombieAttack.bin");

const CONTRACT_NAME: &str = "near_evm";
const LOTS_OF_GAS: u64 = 10_000_000_000_000_000;

fn create_account(client: &mut RpcUser, account_signer: &InMemorySigner) {
    let devnet_signer = InMemorySigner::from_seed("test.near", KeyType::ED25519, "alice.near");
    let devnet_account_id = devnet_signer.account_id.clone();
    let old_signer = client.signer();
    client.set_signer(Arc::new(devnet_signer));
    let tx_result = client.create_account(
        devnet_account_id,
        account_signer.account_id.clone(),
        account_signer.public_key.clone(),
        10_000_000_000,
    );
    client.set_signer(old_signer);
    println!("Create account: {:?}\n", tx_result);
}

fn deploy_evm(client: &RpcUser, account_signer: &InMemorySigner) {
    println!("Deploying evm contract");
    // let contract = include_bytes!("../res/near_evm.wasm").to_vec();
    let contract = EVM.to_vec();
    let tx_result = client.deploy_contract(account_signer.account_id.clone(), contract);
    println!("Deploy evm contract: {:?}\n", tx_result);
}

fn deploy_cryptozombies(client: &RpcUser, account_signer: &InMemorySigner) -> String {
    println!("Deploying zombies contract");
    let input = format!(
        "{{\"bytecode\":\"{}\"}}",
        ZOMBIES
    );

    let tx_result = client.function_call(
        account_signer.account_id.clone(),
        CONTRACT_NAME.to_owned(),
        "deploy_code",
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );
    let status = &tx_result.unwrap().receipts_outcome[0].outcome.status;

    let addr_b64 = match status {
        ExecutionStatusView::SuccessValue(v) => v.clone(),
        _ => panic!("failed cryptozombies deployment"),
    };
    let addr_bytes = from_base64(&addr_b64).unwrap();
    let addr_bytes = addr_bytes[1..addr_bytes.len() - 1].to_vec();
    let address = String::from_utf8(addr_bytes).unwrap();
    println!("deploy_code(cryptozombies): {}\n", address);
    address
}

fn create_random_zombie(client: &RpcUser, account_signer: &InMemorySigner, zombies_address: &str, name: &str) {
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call(name.to_owned());
    let input = format!(
        "{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\"}}",
        zombies_address,
        hex::encode(input)
    );
    let tx_result = client.function_call(
        account_signer.account_id.clone(),
        CONTRACT_NAME.to_owned(),
        "call_contract",
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );
    println!("call_contract(createRandomZombie): {:?}\n", tx_result);
}

fn get_zombies_by_owner(
    client: &RpcUser,
    account_signer: &InMemorySigner,
    zombies_address: &str,
    owner: Address,
) -> Vec<Uint> {
    let (input, _decoder) = cryptozombies::functions::get_zombies_by_owner::call(owner);
    let input = format!("{{\"contract_address\":\"{}\",\"encoded_input\":\"{}\"}}",
        zombies_address,
        hex::encode(input)
    );
    let tx_result = client.function_call(
        account_signer.account_id.clone(),
        CONTRACT_NAME.to_owned(),
        "call_contract",
        input.into_bytes(),
        LOTS_OF_GAS,
        0,
    );
    println!("call_contract(getZombiesByOwner): {:?}\n", tx_result);
    if let FinalExecutionStatus::SuccessValue(ref base64) = tx_result.as_ref().unwrap().status {
        let bytes = from_base64(base64).unwrap();
        assert!(bytes.len() >= 2);
        let bytes = hex::decode(&bytes[1..bytes.len() - 1]).unwrap();
        cryptozombies::functions::get_zombies_by_owner::decode_output(&bytes).unwrap()
    } else {
        panic!(tx_result)
    }
}

#[test]
fn test_zombie() {
    let addr = "localhost:3030";
    let signer = InMemorySigner::from_seed(CONTRACT_NAME, KeyType::ED25519, CONTRACT_NAME);
    let signer = Arc::new(signer);
    let mut user = RpcUser::new(addr, "alice.near".to_owned(), signer.clone());
    create_account(&mut user, &signer);
    deploy_evm(&user, &signer);
    let zombies_address = deploy_cryptozombies(&user, &signer);
    // assert_eq!(3,4);
    create_random_zombie(&user, &signer, &zombies_address, "zomb1");
    let zombies = get_zombies_by_owner(
        &user,
        &signer,
        &zombies_address,
        near_account_id_to_evm_address(&signer.account_id),
    );
    assert_eq!(zombies, vec![Uint::from(0)]);
}
