use ethabi_contract::use_contract;
use ethereum_types::{Address, U256, H256};
use crate::{evm_state::*, utils};

mod cryptozombies;
mod test_utils;

use_contract!(soltest, "src/tests/build/SolTests.abi");
use_contract!(subcontract, "src/tests/build/SubContract.abi");
use_contract!(create2factory, "src/tests/build/Create2Factory.abi");
use_contract!(selfdestruct, "src/tests/build/SelfDestruct.abi");

lazy_static_include_str!(TEST, "src/tests/build/SolTests.bin");
lazy_static_include_str!(FACTORY_TEST, "src/tests/build/Create2Factory.bin");
lazy_static_include_str!(DESTRUCT_TEST, "src/tests/build/SelfDestruct.bin");
lazy_static_include_str!(CONSTRUCTOR_TEST, "src/tests/build/ConstructorRevert.bin");

#[test]
fn test_sends() {
    let mut contract = test_utils::initialize();
    let evm_acc = hex::encode(utils::near_account_id_to_evm_address("evmGuy").0);

    assert_eq!(contract.balance_of_near_account("owner1".to_string()).0, 0);
    test_utils::attach_deposit(100);
    contract.add_near();
    assert_eq!(
        contract.balance_of_near_account("owner1".to_string()).0,
        100
    );

    contract.move_funds_to_evm_address(evm_acc.clone(), utils::Balance(50));
    assert_eq!(contract.balance_of_near_account("owner1".to_string()).0, 50);
    assert_eq!(contract.balance_of_evm_address(evm_acc).0, 50);

    contract.move_funds_to_near_account("someGuy".to_string(), utils::Balance(25));
    assert_eq!(contract.balance_of_near_account("owner1".to_string()).0, 25);
    assert_eq!(
        contract.balance_of_near_account("someGuy".to_string()).0,
        25
    );
    // TODO: assert contract NEAR balance
}

#[test]
fn test_deploy_with_nonce() {
    let mut contract = test_utils::initialize();
    let evm_acc = hex::encode(utils::near_account_id_to_evm_address("owner1").0);
    assert_eq!(contract.nonce_of_near_account("owner1".to_string()).0, 0);
    assert_eq!(contract.nonce_of_evm_address(evm_acc.clone()).0, 0);

    contract.deploy_code(TEST.to_string());
    assert_eq!(contract.nonce_of_near_account("owner1".to_string()).0, 1);
    assert_eq!(contract.nonce_of_evm_address(evm_acc.clone()).0, 1);

    contract.deploy_code(TEST.to_string()); // at a different address
    assert_eq!(contract.nonce_of_near_account("owner1".to_string()).0, 2);
    assert_eq!(contract.nonce_of_evm_address(evm_acc.clone()).0, 2);
}

#[test]
#[should_panic]
fn test_failed_deploy_returns_error() {
    let mut contract = test_utils::initialize();
    contract.deploy_code(CONSTRUCTOR_TEST.to_string());

}

#[test]
fn test_internal_create() {
    let mut contract = test_utils::initialize();
    let test_addr = contract.deploy_code(TEST.to_string());
    assert_eq!(contract.nonce_of_evm_address(test_addr.clone()).0, 0);

    // This should increment the nonce of the deploying contract
    let (input, _) = soltest::functions::deploy_new_guy::call(8);
    let raw = contract.call_contract(test_addr.clone(), hex::encode(input));
    assert_eq!(contract.nonce_of_evm_address(test_addr.clone()).0, 1);

    let sub_addr = raw[24..64].to_string();
    let (new_input, _) = subcontract::functions::a_number::call();
    let new_raw = contract.call_contract(sub_addr, hex::encode(new_input));
    let output =
        subcontract::functions::a_number::decode_output(&hex::decode(&new_raw).unwrap())
            .unwrap();
    assert_eq!(output, U256::from(8));
}

#[test]
fn test_deploy_and_transfer() {
    let mut contract = test_utils::initialize();
    test_utils::attach_deposit(100);
    let test_addr = contract.deploy_code(TEST.to_string());
    assert_eq!(contract.balance_of_evm_address(test_addr.clone()).0, 100);

    // This should increment the nonce of the deploying contract
    // There is 100 attached to this that should be passed through
    let (input, _) = soltest::functions::deploy_new_guy::call(8);
    let raw = contract.call_contract(test_addr.clone(), hex::encode(input));

    // The sub_addr should have been transferred 100 monies
    let sub_addr = raw[24..64].to_string();
    assert_eq!(contract.balance_of_evm_address(test_addr).0, 100);
    assert_eq!(contract.balance_of_evm_address(sub_addr).0, 100);
}

#[test]
fn test_deploy_with_value() {
    let mut contract = test_utils::initialize();
    test_utils::attach_deposit(100);

    // This test is identical to the previous one
    // As we expect behavior to be the same.
    let test_addr = contract.deploy_code(TEST.to_string());
    assert_eq!(contract.balance_of_evm_address(test_addr.clone()).0, 100);

    // This should increment the nonce of the deploying contract
    // There is 100 attached to this that should be passed through
    let (input, _) = soltest::functions::pay_new_guy::call(8);
    let raw = contract.call_contract(test_addr.clone(), hex::encode(input));

    // The sub_addr should have been transferred 100 monies
    let sub_addr = raw[24..64].to_string();
    assert_eq!(contract.balance_of_evm_address(test_addr).0, 100);
    assert_eq!(contract.balance_of_evm_address(sub_addr).0, 100);
}

#[test]
fn test_contract_to_eoa_transfer() {
    let mut contract = test_utils::initialize();
    test_utils::attach_deposit(100);

    // This test is identical to the previous one
    // As we expect behavior to be the same.
    let test_addr = contract.deploy_code(TEST.to_string());
    assert_eq!(contract.balance_of_evm_address(test_addr.clone()).0, 100);

    let (input, _) = soltest::functions::return_some_funds::call();
    let raw = contract.call_contract(test_addr.clone(), hex::encode(input));

    let sender_addr = raw[24..64].to_string();
    assert_eq!(contract.balance_of_evm_address(test_addr).0, 150);
    assert_eq!(contract.balance_of_evm_address(sender_addr).0, 50);
}

#[test]
fn test_get_code() {
    let mut contract = test_utils::initialize();
    let test_addr = contract.deploy_code(TEST.to_string());
    assert!(contract.get_code(test_addr).len() > 3000); // contract code should roughly be over length 3000

    let no_code_addr = "0000000000000000000000000000000000000000".to_owned();
    assert_eq!(contract.get_code(no_code_addr), "");
}

#[test]
fn test_view_call() {
    let mut contract = test_utils::initialize();
    let test_addr = contract.deploy_code(TEST.to_string());

    // This should NOT increment the nonce of the deploying contract
    // And NO CODE should be deployed
    let (input, _) = soltest::functions::deploy_new_guy::call(8);
    let raw = contract.view_call_contract(
        test_addr.clone(),
        hex::encode(input),
        test_addr.clone(),
        utils::Balance(0),
    );
    assert_eq!(contract.nonce_of_evm_address(test_addr.clone()).0, 0);

    let sub_addr = raw[24..64].to_string();
    assert_eq!(contract.get_code(sub_addr), "");
}

#[test]
fn test_accurate_storage_on_selfdestruct() {
    let mut contract = test_utils::initialize();
    test_utils::attach_deposit(100);
    contract.add_near();
    test_utils::set_default_context(); // clear attached deposit

    let salt = H256([0u8; 32]);
    let destruct_code = hex::decode(DESTRUCT_TEST.to_string()).expect("invalid hex");

    // Deploy CREATE2 Factory
    let factory_addr = contract.deploy_code(FACTORY_TEST.to_string());

    // Deploy SelfDestruct contract using CREATE2 and confirm code at address
    let mut input = create2factory::functions::deploy::call(salt, destruct_code.clone()).0;
    let mut raw = contract.call_contract(factory_addr.clone(), hex::encode(input));
    let destruct_addr = raw[24..64].to_string();
    assert!(contract.get_code(destruct_addr.clone()).len() > 1000);

    // Write to and successfully read from storage
    let stored_uint = 5;
    input = selfdestruct::functions::store_uint::call(stored_uint).0;
    contract.call_contract(destruct_addr.clone(), hex::encode(input));
    input = selfdestruct::functions::stored_uint::call().0;
    raw = contract.call_contract(destruct_addr.clone(), hex::encode(input));
    assert_eq!(stored_uint, raw.parse::<i32>().unwrap());

    // Send money to SelfDestruct Contract
    contract.move_funds_to_evm_address(destruct_addr.clone(), utils::Balance(10));
    assert_eq!(contract.balance_of_evm_address(destruct_addr.clone()), utils::Balance(10));

    // Destroy contract and confirm 0 balance
    input = selfdestruct::functions::destruction::call().0;
    contract.call_contract(destruct_addr.clone(), hex::encode(input));
    assert_eq!(contract.balance_of_evm_address(destruct_addr.clone()), utils::Balance(0));
    assert_eq!(contract.get_code(destruct_addr.clone()).len(), 0);

    // Send money to SelfDestruct Contract and confirm balance
    contract.move_funds_to_evm_address(destruct_addr.clone(), utils::Balance(10));
    assert_eq!(contract.balance_of_evm_address(destruct_addr.clone()), utils::Balance(10));

    // Redeploy contract to same address with CREATE2 and confirm balance
    input = create2factory::functions::deploy::call(salt, destruct_code.clone()).0;
    contract.call_contract(factory_addr.clone(), hex::encode(input));
    assert_eq!(contract.balance_of_evm_address(destruct_addr.clone()), utils::Balance(10));

    // Confirm empty storage on fresh contract
    input = selfdestruct::functions::stored_uint::call().0;
    raw = contract.call_contract(destruct_addr.clone(), hex::encode(input));
    assert_eq!(0, raw.parse::<i32>().unwrap());

    input = selfdestruct::functions::stored_address::call().0;
    raw = contract.call_contract(destruct_addr.clone(), hex::encode(input));
    assert_eq!(0, raw.parse::<i32>().unwrap());
}

#[test]
fn state_management() {
    let mut contract = test_utils::initialize();
    let addr_0 = Address::repeat_byte(0);
    let addr_1 = Address::repeat_byte(1);
    let addr_2 = Address::repeat_byte(2);

    let zero = U256::zero();
    let code: [u8; 3] = [0, 1, 2];
    let nonce = U256::from_dec_str("103030303").unwrap();
    let balance = U256::from_dec_str("3838209").unwrap();
    let storage_key_0 = [4u8; 32];
    let storage_key_1 = [5u8; 32];
    let storage_value_0 = [6u8; 32];
    let storage_value_1 = [7u8; 32];

    contract.set_code(&addr_0, &code);
    assert_eq!(contract.code_at(&addr_0), Some(code.to_vec()));
    assert_eq!(contract.code_at(&addr_1), None);
    assert_eq!(contract.code_at(&addr_2), None);

    contract.set_nonce(&addr_0, nonce);
    assert_eq!(contract.nonce_of(&addr_0), nonce);
    assert_eq!(contract.nonce_of(&addr_1), zero);
    assert_eq!(contract.nonce_of(&addr_2), zero);

    contract.set_balance(&addr_0, balance);
    assert_eq!(contract.balance_of(&addr_0), balance);
    assert_eq!(contract.balance_of(&addr_1), zero);
    assert_eq!(contract.balance_of(&addr_2), zero);

    contract.set_contract_storage(&addr_0, storage_key_0, storage_value_0);
    // assert_eq!(contract.read_contract_storage(&addr_0, storage_key_0), Some(storage_value_0));
    assert_eq!(contract.read_contract_storage(&addr_1, storage_key_0), None);
    assert_eq!(contract.read_contract_storage(&addr_2, storage_key_0), None);

    let next = {
        // Open a new store
        let mut next = StateStore::default();
        let mut sub1 = SubState::new(&addr_0, &mut next, &contract);

        sub1.set_code(&addr_1, &code);
        assert_eq!(sub1.code_at(&addr_0), Some(code.to_vec()));
        assert_eq!(sub1.code_at(&addr_1), Some(code.to_vec()));
        assert_eq!(sub1.code_at(&addr_2), None);

        sub1.set_nonce(&addr_1, nonce);
        assert_eq!(sub1.nonce_of(&addr_0), nonce);
        assert_eq!(sub1.nonce_of(&addr_1), nonce);
        assert_eq!(sub1.nonce_of(&addr_2), zero);

        sub1.set_balance(&addr_1, balance);
        assert_eq!(sub1.balance_of(&addr_0), balance);
        assert_eq!(sub1.balance_of(&addr_1), balance);
        assert_eq!(sub1.balance_of(&addr_2), zero);

        sub1.set_contract_storage(&addr_1, storage_key_0, storage_value_0);
        // assert_eq!(sub1.read_contract_storage(&addr_0, storage_key_0), Some(storage_value_0));
        assert_eq!(
            sub1.read_contract_storage(&addr_1, storage_key_0),
            Some(storage_value_0)
        );
        assert_eq!(sub1.read_contract_storage(&addr_2, storage_key_0), None);

        sub1.set_contract_storage(&addr_1, storage_key_0, storage_value_1);
        // assert_eq!(sub1.read_contract_storage(&addr_0, storage_key_0), Some(storage_value_0));
        assert_eq!(
            sub1.read_contract_storage(&addr_1, storage_key_0),
            Some(storage_value_1)
        );
        assert_eq!(sub1.read_contract_storage(&addr_2, storage_key_0), None);

        sub1.set_contract_storage(&addr_1, storage_key_1, storage_value_1);
        assert_eq!(
            sub1.read_contract_storage(&addr_1, storage_key_0),
            Some(storage_value_1)
        );
        assert_eq!(
            sub1.read_contract_storage(&addr_1, storage_key_1),
            Some(storage_value_1)
        );

        sub1.set_contract_storage(&addr_1, storage_key_0, storage_value_0);
        assert_eq!(
            sub1.read_contract_storage(&addr_1, storage_key_0),
            Some(storage_value_0)
        );
        assert_eq!(
            sub1.read_contract_storage(&addr_1, storage_key_1),
            Some(storage_value_1)
        );

        next
    };

    contract.commit_changes(&next);
    assert_eq!(contract.code_at(&addr_0), Some(code.to_vec()));
    assert_eq!(contract.code_at(&addr_1), Some(code.to_vec()));
    assert_eq!(contract.code_at(&addr_2), None);
    assert_eq!(contract.nonce_of(&addr_0), nonce);
    assert_eq!(contract.nonce_of(&addr_1), nonce);
    assert_eq!(contract.nonce_of(&addr_2), zero);
    assert_eq!(contract.balance_of(&addr_0), balance);
    assert_eq!(contract.balance_of(&addr_1), balance);
    assert_eq!(contract.balance_of(&addr_2), zero);
    // assert_eq!(contract.read_contract_storage(&addr_0, storage_key_0), Some(storage_value_0));
    assert_eq!(
        contract.read_contract_storage(&addr_1, storage_key_0),
        Some(storage_value_0)
    );
    assert_eq!(
        contract.read_contract_storage(&addr_1, storage_key_1),
        Some(storage_value_1)
    );
    assert_eq!(contract.read_contract_storage(&addr_2, storage_key_0), None);
}
