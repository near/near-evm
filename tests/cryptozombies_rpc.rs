#[macro_use]
extern crate ethabi_derive;

extern crate near_evm;
mod rpc_user;

use ethabi::{Address, Uint};
use ethabi_contract::use_contract;
use near_evm::{sender_name_to_eth_address, DeployCodeInput, RunCommandInput};
use near_primitives::crypto::signer::InMemorySigner;
use near_primitives::transaction::{
    CreateAccountTransaction, DeployContractTransaction, FunctionCallTransaction, TransactionBody,
};
use rpc_user::{RpcUser, User};

use_contract!(cryptozombies, "src/tests/zombieAttack.abi");

fn create_account(client: &RpcUser, account_signer: &InMemorySigner) {
    let devnet_signer = InMemorySigner::from_seed("test.near", "seed0");
    let create_account = CreateAccountTransaction {
        nonce: client
            .get_account_nonce(&devnet_signer.account_id)
            .unwrap_or_default()
            + 1,
        originator: devnet_signer.account_id.clone(),
        new_account_id: account_signer.account_id.clone(),
        amount: 1_000_000_000,
        public_key: account_signer.public_key.as_ref().to_vec(),
    };
    let transaction = TransactionBody::CreateAccount(create_account).sign(&devnet_signer);
    let tx_result = client.commit_transaction(transaction).unwrap();
    println!("Create account: {:?}", tx_result);
}

fn deploy_evm(client: &RpcUser, account_signer: &InMemorySigner) {
    let deploy_contract = DeployContractTransaction {
        nonce: client
            .get_account_nonce(&account_signer.account_id)
            .unwrap_or_default()
            + 1,
        contract_id: "near_evm".to_string(),
        wasm_byte_array: include_bytes!("../pkg/near_evm_bg.wasm").to_vec(),
    };
    let transaction = TransactionBody::DeployContract(deploy_contract).sign(account_signer);
    println!("Deploying evm contract");
    let tx_result = client.commit_transaction(transaction).unwrap();
    println!("Deploy evm contract: {:?}", tx_result);
}

fn deploy_cryptozombies(client: &RpcUser, account_signer: &InMemorySigner) {
    let zombie_code = include_bytes!("../src/tests/zombieAttack.bin").to_vec();
    let run = DeployCodeInput {
        contract_address: "zombies".to_string(),
        bytecode: String::from_utf8(zombie_code).unwrap(),
    };
    let call = FunctionCallTransaction {
        nonce: client
            .get_account_nonce(&account_signer.account_id)
            .unwrap_or_default()
            + 1,
        originator: account_signer.account_id.clone(),
        contract_id: "near_evm".to_string(),
        method_name: "deploy_code".to_string().into_bytes(),
        args: serde_json::to_string(&run).unwrap().into_bytes(),
        amount: 1_000_000_000,
    };
    let transaction = TransactionBody::FunctionCall(call).sign(account_signer);
    let tx_result = client.commit_transaction(transaction).unwrap();
    println!("deploy_code(cryptozombies): {:?}", tx_result);
}

fn create_random_zombie(client: &RpcUser, account_signer: &InMemorySigner, name: &str) {
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call(name.to_string());
    let run = RunCommandInput {
        contract_address: "cryptozombies".to_string(),
        encoded_input: hex::encode(input),
    };
    let call = FunctionCallTransaction {
        nonce: client
            .get_account_nonce(&account_signer.account_id)
            .unwrap_or_default()
            + 1,
        originator: account_signer.account_id.clone(),
        contract_id: "near_evm".to_string(),
        method_name: "run_command".to_string().into_bytes(),
        args: serde_json::to_string(&run).unwrap().into_bytes(),
        amount: 1_000_000_000,
    };
    let transaction = TransactionBody::FunctionCall(call).sign(account_signer);
    let tx_result = client.commit_transaction(transaction).unwrap();
    println!("run_command(createRandomZombie): {:?}", tx_result);
}

fn get_zombies_by_owner(
    client: &RpcUser,
    account_signer: &InMemorySigner,
    owner: Address,
) -> Vec<Uint> {
    let (input, _decoder) = cryptozombies::functions::get_zombies_by_owner::call(owner);
    let run = RunCommandInput {
        contract_address: "cryptozombies".to_string(),
        encoded_input: hex::encode(input),
    };
    let call = FunctionCallTransaction {
        nonce: client
            .get_account_nonce(&account_signer.account_id)
            .unwrap_or_default()
            + 1,
        originator: account_signer.account_id.clone(),
        contract_id: "near_evm".to_string(),
        method_name: "run_command".to_string().into_bytes(),
        args: serde_json::to_string(&run).unwrap().into_bytes(),
        amount: 1_000_000_000,
    };
    let transaction = TransactionBody::FunctionCall(call).sign(account_signer);
    let tx_result = client.commit_transaction(transaction).unwrap();
    println!("run_command(getZombiesByOwner): {:?}", tx_result);
    cryptozombies::functions::get_zombies_by_owner::decode_output(&tx_result.last_result())
        .unwrap()
}

#[test]
fn test_zombie() {
    //    System::new("actix").block_on(futures::lazy(|| {
    let addr = "localhost:3030";
    let user = RpcUser::new(addr);
    let signer = InMemorySigner::from_seed("near_evm", "near_evm");
    println!("OK HERE WE GO");
    create_account(&user, &signer);
    deploy_evm(&user, &signer);
    deploy_cryptozombies(&user, &signer);
    create_random_zombie(&user, &signer, "zomb1");
    let zombies = get_zombies_by_owner(
        &user,
        &signer,
        sender_name_to_eth_address(&signer.account_id),
    );
    assert_eq!(zombies, vec![Uint::from(0)]);
    //        future::ok::<(), ()>(())
    //    })).unwrap();
}
