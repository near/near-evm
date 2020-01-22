use ethabi::{Address, Uint};
use ethabi_contract::use_contract;
use near_crypto::{InMemorySigner, KeyType};

use near_evm::{sender_name_to_eth_address};
use near_testlib::user::{rpc_user::RpcUser, User};
use std::sync::Arc;
use near_primitives::views::FinalExecutionStatus;
use near_primitives::serialize::from_base64;

use_contract!(cryptozombies, "src/tests/zombieAttack.abi");

const CONTRACT_NAME: &str = "near_evm";
const LOTS_OF_GAS:u64 = 1_000_000_000_000_000_000;

fn create_account(client: &mut RpcUser, account_signer: &InMemorySigner) {
    let devnet_signer = InMemorySigner::from_seed("test.near", KeyType::ED25519, "alice.near");
    let devnet_account_id = devnet_signer.account_id.clone();
    let old_signer = client.signer();
    client.set_signer(Arc::new(devnet_signer));
    let tx_result = client.create_account(devnet_account_id, account_signer.account_id.clone(), account_signer.public_key.clone(), 10_000_000_000);
    client.set_signer(old_signer);
    println!("Create account: {:?}", tx_result);
}

fn deploy_evm(client: &RpcUser, account_signer: &InMemorySigner) {
    println!("Deploying evm contract");
    let contract = include_bytes!("../res/near_evm.wasm").to_vec();
    let tx_result = client.deploy_contract(account_signer.account_id.clone(), contract);
    println!("Deploy evm contract: {:?}", tx_result);
}

fn deploy_cryptozombies(client: &RpcUser, account_signer: &InMemorySigner) {
    let zombie_code = include_bytes!("../src/tests/zombieAttack.bin").to_vec();
    let input = format!("{{\"contract_address\":\"cryptozombies\",\"bytecode\":\"{}\"}}", String::from_utf8(zombie_code).unwrap());
    let tx_result = client.function_call(account_signer.account_id.clone(), CONTRACT_NAME.to_string(), "deploy_code", input.into_bytes(), LOTS_OF_GAS, 0);
    println!("deploy_code(cryptozombies): {:?}", tx_result);
}

fn create_random_zombie(client: &RpcUser, account_signer: &InMemorySigner, name: &str) {
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call(name.to_string());
    let input = format!("{{\"contract_address\":\"cryptozombies\",\"encoded_input\":\"{}\"}}", hex::encode(input));
    let tx_result = client.function_call(account_signer.account_id.clone(), CONTRACT_NAME.to_string(), "run_command", input.into_bytes(), LOTS_OF_GAS, 0);
    println!("run_command(createRandomZombie): {:?}", tx_result);
}

fn get_zombies_by_owner(
    client: &RpcUser,
    account_signer: &InMemorySigner,
    owner: Address,
) -> Vec<Uint> {
    let (input, _decoder) = cryptozombies::functions::get_zombies_by_owner::call(owner);
    let input = format!("{{\"contract_address\":\"cryptozombies\",\"encoded_input\":\"{}\"}}", hex::encode(input));
    let tx_result = client.function_call(account_signer.account_id.clone(), CONTRACT_NAME.to_string(), "run_command", input.into_bytes(), LOTS_OF_GAS, 0);
    println!("run_command(getZombiesByOwner): {:?}", tx_result);
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
    let mut user = RpcUser::new(addr, "alice.near".to_owned(),signer.clone());
    create_account(&mut user, &signer);
    deploy_evm(&user, &signer);
    deploy_cryptozombies(&user, &signer);
    create_random_zombie(&user, &signer, "zomb1");
    let zombies = get_zombies_by_owner(
        &user,
        &signer,
        sender_name_to_eth_address(&signer.account_id),
    );
    assert_eq!(zombies, vec![Uint::from(0)]);
}
