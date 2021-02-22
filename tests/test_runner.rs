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
use_contract!(bpool, "tests/build/BPool.abi");
use_contract!(ttoken, "tests/build/TToken.abi");
use_contract!(tmath, "tests/build/TMath.abi");

fn alice_addr() -> H160 {
    near_account_to_evm_address(b"alice")
}

struct TestRunner {
    backend: test_backend::TestBackend,
}

impl TestRunner {
    pub fn new() -> Self {
        Self {
            backend: test_backend::TestBackend::new(alice_addr()),
        }
    }

    pub fn set_origin(&mut self, origin: H160) {
        self.backend.origin = origin;
    }

    pub fn deploy_code(&mut self, code: Vec<u8>) -> H160 {
        Runner::deploy_code(&mut self.backend, &code).1
    }

    pub fn call(&mut self, address: H160, input: Vec<u8>) -> Vec<u8> {
        let result = Runner::call(
            &mut self.backend,
            &FunctionCallArgs {
                contract: address.0,
                input,
            }
            .try_to_vec()
            .unwrap(),
        );
        assert!(result.0.is_succeed(), format!("{:?}", result.1));
        result.1
    }

    pub fn view(&mut self, sender: H160, address: H160, value: U256, input: Vec<u8>) -> Vec<u8> {
        let mut amount = [0u8; 32];
        value.to_big_endian(&mut amount);
        Runner::view(
            &mut self.backend,
            ViewCallArgs {
                sender: sender.0,
                address: address.0,
                amount,
                input,
            },
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
    let (input, _decoder) = cryptozombies::functions::balance_of::call(alice_addr().0);
    let result = runner.view(H160::zero(), address, U256::zero(), input);
    assert_eq!(U256::from_big_endian(&result), U256::from(1));
}

#[test]
fn test_tmath() {
    let mut runner = TestRunner::new();
    let address =
        runner.deploy_code(hex::decode(&include_bytes!("build/TMath.bin").to_vec()).unwrap());
    let (input, _decoder) = tmath::functions::calc_bsub::call(1, 2);
    let result = runner.view(alice_addr(), address, U256::zero(), input);
    assert!(String::from_utf8_lossy(&result).contains("ERR_SUB_UNDERFLOW"));
}

/// Creates and mints 5m of token for alice.
fn create_ttoken(runner: &mut TestRunner) -> H160 {
    runner.set_origin(alice_addr());
    let input = ttoken::constructor(
        hex::decode(&include_bytes!("build/TToken.bin").to_vec()).unwrap(),
        "XYZ",
        "XYZ",
        18,
    );
    let address = runner.deploy_code(input);
    let (input, _) = ttoken::functions::mint::call(&alice_addr().0, 10 * 10u128.pow(18));
    let _ = runner.call(address, input);
    address
}

#[test]
fn test_ttoken() {
    let mut runner = TestRunner::new();
    let address = create_ttoken(&mut runner);
    let (input, _) = ttoken::functions::transfer::call(&address.0, 1 * 10u128.pow(18));
    let _ = runner.call(address, input);
    let (input, _) = ttoken::functions::balance_of::call(&alice_addr().0);
    let result = runner.view(address, address, U256::zero(), input);
    assert_eq!(
        U256::from_big_endian(&result),
        U256::from(9 * 10u128.pow(18))
    );
}

#[test]
fn test_balancer() {
    let mut runner = TestRunner::new();
    let address =
        runner.deploy_code(hex::decode(&include_bytes!("build/BFactory.bin").to_vec()).unwrap());
    let (input, _) = bfactory::functions::new_b_pool::call();
    let pool_address =
        bfactory::functions::new_b_pool::decode_output(&runner.call(address, input)).unwrap();
    assert_eq!(
        hex::encode(pool_address),
        "f55df5ec5c8c64582378dce8eee51ec4af77ccd6"
    );
    let (input, _) = bpool::functions::get_controller::call();
    let result =
        bpool::functions::get_controller::decode_output(&runner.call(pool_address, input)).unwrap();
    assert_eq!(result, near_account_to_evm_address(b"alice"));

    let token_address = create_ttoken(&mut runner);
    let _ = runner.call(
        token_address,
        ttoken::functions::approve::call(&pool_address.0, 10 * 10u128.pow(18)).0,
    );

    let _ = runner.call(
        pool_address,
        bpool::functions::bind::call(token_address, 10 * 10u128.pow(18), 5 * 10u128.pow(18)).0,
    );

    let result = ttoken::functions::balance_of::decode_output(&runner.view(
        alice_addr(),
        token_address,
        U256::zero(),
        ttoken::functions::balance_of::call(alice_addr().0).0,
    ))
    .unwrap();
    assert_eq!(result, U256::zero());
}
