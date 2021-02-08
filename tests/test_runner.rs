use borsh::BorshSerialize;
use ethabi_contract::use_contract;
use primitive_types::{H160, U256};

use near_evm::backend::Backend;
use near_evm::runner::Runner;
use near_evm::types::{near_account_to_evm_address, FunctionCallArgs, ViewCallArgs};

mod test_backend;

use_contract!(soltest, "tests/build/SolTests.abi");
use_contract!(cryptozombies, "tests/build/ZombieOwnership.abi");
use_contract!(bfactory, "tests/build/BFactory.abi");
use_contract!(ttoken, "tests/build/TToken.abi");
use_contract!(tmath, "tests/build/TMath.abi");

struct TestRunner {
    backend: test_backend::TestBackend,
}

impl TestRunner {
    pub fn new() -> Self {
        Self {
            backend: test_backend::TestBackend::new(H160::zero()),
        }
    }

    pub fn set_origin(&mut self, origin: H160) {
        self.backend.origin = origin;
    }

    pub fn deploy_code(&mut self, code: Vec<u8>) -> H160 {
        Runner::deploy_code(&mut self.backend, &code).1
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
        .1
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
        .1
    }
}

#[test]
fn test_runner_deploy() {
    let mut runner = TestRunner::new();
    let address = runner
        .deploy_code(hex::decode(&include_bytes!("build/ZombieOwnership.bin").to_vec()).unwrap());
    assert!(runner.backend.code(address).len() > 0);
    let (input, _decoder) = cryptozombies::functions::balance_of::call(address.0);
    let result = runner.view(H160::zero(), address, U256::zero(), input);
    assert_eq!(U256::from_big_endian(&result), U256::zero());
    let (input, _decoder) = cryptozombies::functions::create_random_zombie::call("test");
    let _ = runner.call(address, input);
    let (input, _decoder) = cryptozombies::functions::balance_of::call(H160::zero().0);
    let result = runner.view(H160::zero(), address, U256::zero(), input);
    assert_eq!(U256::from_big_endian(&result), U256::from(1));
}

#[test]
fn test_tmath() {
    let mut runner = TestRunner::new();
    let address =
        runner.deploy_code(hex::decode(&include_bytes!("build/TMath.bin").to_vec()).unwrap());
    let (input, _decoder) = tmath::functions::calc_bsub::call(1, 2);
    let result = runner.call(address, input);
    assert!(String::from_utf8_lossy(&result).contains("ERR_SUB_UNDERFLOW"));
}

#[test]
fn test_ttoken() {
    let mut runner = TestRunner::new();
    let alice_addr = near_account_to_evm_address(b"alice");
    runner.set_origin(alice_addr);
    let input = ttoken::constructor(
        hex::decode(&include_bytes!("build/TToken.bin").to_vec()).unwrap(),
        "XYZ",
        "XYZ",
        18,
    );
    let address = runner.deploy_code(input);
    let (input, _) = ttoken::functions::mint::call(&alice_addr.0, 5_000_000);
    let _ = runner.call(address, input);
    let (input, _) = ttoken::functions::transfer::call(&address.0, 1_000);
    let _ = runner.call(address, input);
    let (input, _) = ttoken::functions::balance_of::call(&alice_addr.0);
    let result = runner.view(address, address, U256::zero(), input);
    assert_eq!(U256::from_big_endian(&result), U256::from(4_999_000));
}

#[test]
fn test_balancer() {
    let mut runner = TestRunner::new();
    runner.set_origin(near_account_to_evm_address(b"alice"));
    let address =
        runner.deploy_code(hex::decode(&include_bytes!("build/BFactory.bin").to_vec()).unwrap());
    let (input, _) = bfactory::functions::new_b_pool::call();
    let pool_address = bfactory::functions::new_b_pool::decode_output(&runner.view(
        H160::zero(),
        address,
        U256::zero(),
        input,
    ))
    .unwrap();
    assert_eq!(
        hex::encode(pool_address),
        "f55df5ec5c8c64582378dce8eee51ec4af77ccd6"
    );
}
