#[macro_use]
extern crate bencher;

use bencher::Bencher;
use ethabi::{Address, Uint};
use ethabi_contract::use_contract;

use near_sdk::MockedBlockchain;
use near_sdk::{testing_env, VMContext};

use near_evm::EvmContract;

use_contract!(cryptozombies, "src/tests/zombieAttack.abi");

fn get_context(input: Vec<u8>) -> VMContext {
    VMContext {
        current_account_id: "zombies".to_string(),
        signer_account_id: "owner1".to_string(),
        signer_account_pk: vec![0, 1, 2],
        predecessor_account_id: "owner1".to_string(),
        input,
        block_index: 0,
        block_timestamp: 0,
        epoch_height: 0,
        account_balance: 0,
        account_locked_balance: 0,
        storage_usage: 100000, // arbitrarily high number to avoid InconsistentStateError(IntegerOverflow) from resetting context params
        attached_deposit: 0,
        prepaid_gas: 2u64.pow(63),
        random_seed: vec![0, 1, 2],
        is_view: false,
        output_data_receivers: vec![],
    }
}


pub fn initialize() -> EvmContract {
    set_default_context();
    return EvmContract::default();
}

pub fn set_default_context() {
    let context = get_context(vec![]);
    testing_env!(context);
}

fn deploy_cryptozombies(contract: &mut EvmContract) -> String {
    let zombie_code = include_bytes!("../src/tests/zombieAttack.bin").to_vec();
    contract.deploy_code(String::from_utf8(zombie_code).unwrap())
}

fn crypto_zombie(bench: &mut Bencher) {
    let mut contract = initialize();
    let addr = deploy_cryptozombies(&mut contract);
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call("test".to_string());
    contract.call_contract(addr.clone(), hex::encode(input));
    bench.iter(|| {
        let (input, _decoder) = cryptozombies::functions::create_random_zombie::call("test".to_string());
        contract.call_contract(addr.clone(), hex::encode(input));
    });
}

benchmark_group!(
    benches,
    crypto_zombie,
);
benchmark_main!(benches);
