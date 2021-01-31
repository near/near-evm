use near_evm::backend::Backend;
use near_evm::runner::Runner;
use near_evm::types::ViewCallArgs;
use primitive_types::H160;

mod test_backend;

use_contract!("../../near-evm/src/tests/build/zo");

#[test]
fn test_runner_deploy() {
    let mut backend = test_backend::TestBackend::new(H160::zero());
    let address = Runner::deploy_code(
        &mut backend,
        &hex::decode(&include_bytes!("../../near-evm/src/tests/zombieAttack.bin").to_vec())
            .unwrap(),
    );
    println!("{:?}", address);
    println!("{:?}", backend.code(address));
    assert_ne!(backend.code(address), vec![]);
    Runner::view(
        &backend,
        ViewCallArgs {
            sender: vec![0u8; 20],
            address: vec![0u8; 20],
            amount: vec![0u8; 20],
            ipu,
        },
    );
}
