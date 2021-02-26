use near_sdk_sim::{call, deploy, ContractAccount};

/// Load in contract bytes
near_sdk_sim::lazy_static! {
    static ref EVM_WASM_BYTES: &'static [u8] = include_bytes!("../res/evm.wasm").as_ref();
}

// Deploy
// init with owner
// change owner
// fail to change from another account
// stage contract
// upload contract
fn init() {
    let master_account = near_sdk_sim::init_simulator(None);
    let initial_balance = near_sdk_sim::to_yocto("100_000");
    let contract_user = deploy!(
        contract: todo!(), // Where is this from?
        contract_id: "evm_contract",
        bytes: &EVM_WASM_BYTES,
        signer_accounts: master_account,
        init_method: near_sdk_sim::new(master_account.account_id(), initial_balance.into())
    );
}

#[test]
fn test_upgrade() {
    init();
}
