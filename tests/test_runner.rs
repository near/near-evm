use borsh::BorshSerialize;
use ethabi_contract::use_contract;
use primitive_types::{H160, U256};

use near_evm::backend::Backend;
use near_evm::runner::Runner;
use near_evm::types::{FunctionCallArgs, ViewCallArgs};

mod test_backend;

use_contract!(cryptozombies, "tests/build/ZombieOwnership.abi");

struct TestRunner {
    backend: test_backend::TestBackend,
}

impl TestRunner {
    pub fn new() -> Self {
        Self {
            backend: test_backend::TestBackend::new(H160::zero()),
        }
    }

    pub fn deploy_code(&mut self, code: Vec<u8>) -> H160 {
        Runner::deploy_code(&mut self.backend, &code)
    }

    pub fn call(&mut self, address: H160, input: Vec<u8>) -> Vec<u8> {
        Runner::call(
            &mut self.backend,
            &FunctionCallArgs {
                contract: address.0,
                input,
            }
            .try_to_vec()
            .unwrap(),
        )
    }

    pub fn view(&mut self, sender: H160, address: H160, value: U256, input: Vec<u8>) -> Vec<u8> {
        let mut amount = [0u8; 32];
        value.to_big_endian(&mut amount);
        Runner::view(
            &mut self.backend,
            &ViewCallArgs {
                sender: sender.0,
                address: address.0,
                amount,
                input,
            }
            .try_to_vec()
            .unwrap(),
        )
    }
}

#[test]
fn test_runner_deploy() {
    let mut runner = TestRunner::new();
    let address = runner
        .deploy_code(hex::decode(&include_bytes!("build/ZombieOwnership.bin").to_vec()).unwrap());
    println!("{:?}", address);
    println!("{:?}", runner.backend.code(address));
    assert!(runner.backend.code(address).len() > 0);
    let (input, _decoder) = cryptozombies::functions::balance_of::call(address.0);
    let result = runner.view(H160::zero(), address, U256::zero(), input);
    assert_eq!(U256::from_big_endian(&result), U256::zero());
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call("test");
    let result = runner.call(address, input);
    println!("{:?}", result);
    let (input, _decoder) = cryptozombies::functions::balance_of::call(H160::zero().0);
    let result = runner.view(H160::zero(), address, U256::zero(), input);
    assert_eq!(U256::from_big_endian(&result), U256::from(1));
}
