use ethabi_contract::use_contract;
use ethereum_types::U256;

use crate::utils;

mod cryptozombies;
mod test_utils;

use_contract!(soltest, "src/tests/build/SolTests.abi");
use_contract!(subcontract, "src/tests/build/SubContract.abi");

lazy_static_include_str!(TEST, "src/tests/build/SolTests.bin");
lazy_static_include_str!(SUB_TEST, "src/tests/build/SubContract.bin");

#[test]
fn test_sends() {
    test_utils::run_test(100, |contract| {
        let evm_acc = hex::encode(utils::near_account_id_to_evm_address("evmGuy").0);

        assert_eq!(contract.balance_of_near_account("owner1".to_string()).0, 0);
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
    })
}

#[test]
fn test_deploy_with_nonce() {
    test_utils::run_test(0, |contract| {
        let evm_acc = hex::encode(utils::near_account_id_to_evm_address("owner1").0);
        assert_eq!(contract.nonce_of_near_account("owner1".to_string()).0, 0);
        assert_eq!(contract.nonce_of_evm_address(evm_acc.clone()).0, 0);

        contract.deploy_code(TEST.to_string());
        assert_eq!(contract.nonce_of_near_account("owner1".to_string()).0, 1);
        assert_eq!(contract.nonce_of_evm_address(evm_acc.clone()).0, 1);

        contract.deploy_code(TEST.to_string()); // at a different address
        assert_eq!(contract.nonce_of_near_account("owner1".to_string()).0, 2);
        assert_eq!(contract.nonce_of_evm_address(evm_acc.clone()).0, 2);
    })
}

#[test]
fn test_internal_create() {
    test_utils::run_test(0, |contract| {
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
    })
}

#[test]
fn test_deploy_and_transfer() {
    test_utils::run_test(100, |contract| {
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
    })
}

#[test]
fn test_deploy_with_value() {
    test_utils::run_test(100, |contract| {
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
    })
}

#[test]
fn test_contract_to_eoa_transfer() {
    test_utils::run_test(100, |contract| {
        // This test is identical to the previous one
        // As we expect behavior to be the same.
        let test_addr = contract.deploy_code(TEST.to_string());
        assert_eq!(contract.balance_of_evm_address(test_addr.clone()).0, 100);

        let (input, _) = soltest::functions::return_some_funds::call();
        let raw = contract.call_contract(test_addr.clone(), hex::encode(input));

        let sender_addr = raw[24..64].to_string();
        assert_eq!(contract.balance_of_evm_address(test_addr).0, 150);
        assert_eq!(contract.balance_of_evm_address(sender_addr).0, 50);
    })
}

#[test]
fn test_get_code() {
    test_utils::run_test(0, |contract| {
        let test_addr = contract.deploy_code(TEST.to_string());
        assert!(contract.get_code(test_addr).len() > 3002);

        let no_code_addr = "0000000000000000000000000000000000000000".to_owned();
        assert_eq!(contract.get_code(no_code_addr), "");
    })
}

#[test]
fn test_view_call() {
    test_utils::run_test(0, |contract| {
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
    })
}
