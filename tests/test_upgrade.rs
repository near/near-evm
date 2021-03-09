use near_evm::types::NewArgs;
use near_sdk::borsh::BorshSerialize;
use near_sdk::PendingContractTx;
use near_sdk_sim::types::Balance;
use near_sdk_sim::{UserAccount, STORAGE_AMOUNT};

near_sdk_sim::lazy_static! {
    static ref EVM_WASM_BYTES: &'static [u8] = include_bytes!("../res/near_evm.wasm").as_ref();
}

const EVM_CONTRACT: &str = "evm_contract";

fn init() -> (UserAccount, UserAccount) {
    let master_account = near_sdk_sim::init_simulator(None);
    let initial_supply: Balance = near_sdk_sim::to_yocto("1000000").into();
    let contract_account =
        master_account.deploy(*EVM_WASM_BYTES, "evm_contract".into(), initial_supply);
    let res = contract_account.call(
        PendingContractTx {
            receiver_id: "evm_contract".to_string(),
            method: "new".to_string(),
            args: NewArgs { owner_id: master_account.account_id.to_string(), bridge_prover_id: "evm_contract".to_string() }.try_to_vec().unwrap(),
            is_view: false,
        },
        STORAGE_AMOUNT,
        10u64.pow(12),
    );
    res.assert_success();
    (master_account, contract_account)
}

#[test]
fn test_successful_set_owner() {
    let (master_account, contract_account) = init();
    let new_owner = "alice";
    let res = master_account.call(
        PendingContractTx {
            receiver_id: EVM_CONTRACT.to_string(),
            method: "set_owner".to_string(),
            args: new_owner.try_to_vec().unwrap(),
            is_view: false,
        },
        STORAGE_AMOUNT,
        10u64.pow(12),
    );
    res.assert_success();

    let res = contract_account.call(
        PendingContractTx {
            receiver_id: EVM_CONTRACT.to_string(),
            method: "get_owner".to_string(),
            args: vec![],
            is_view: false,
        },
        STORAGE_AMOUNT,
        10u64.pow(12),
    );
    res.assert_success();
    let res_owner: String = res.unwrap_borsh();
    assert_eq!(res_owner, new_owner);
}

#[test]
fn test_failed_set_owner() {
    let (_master_account, contract_account) = init();
    let new_owner = "alice";
    let res = contract_account.call(
        PendingContractTx {
            receiver_id: EVM_CONTRACT.to_string(),
            method: "set_owner".to_string(),
            args: new_owner.try_to_vec().unwrap(),
            is_view: false,
        },
        STORAGE_AMOUNT,
        10u64.pow(12),
    );
    assert!(!res.is_ok());
}

#[test]
fn test_contract_upgrade() {
    let (master_account, _contract_account) = init();
    let res = master_account.call(
        PendingContractTx {
            receiver_id: EVM_CONTRACT.to_string(),
            method: "stage_upgrade".to_string(),
            args: EVM_WASM_BYTES.to_vec(),
            is_view: false,
        },
        STORAGE_AMOUNT,
        50_000_000_000_000,
    );
    res.assert_success();

    // requires methods from the `UserAccount` to get access to the runtime.
}
