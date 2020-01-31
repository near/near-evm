mod cryptozombies;
mod test_utils;

use ethabi_contract::use_contract;
use ethereum_types::{U256};

use_contract!(soltest, "src/tests/build/soltest.abi");
use_contract!(subcontract, "src/tests/build/subcontract.abi");

lazy_static_include_str!(TEST, "src/tests/build/soltest.bin");
lazy_static_include_str!(SUB_TEST, "src/tests/build/subcontract.bin");

#[test]
fn test_send() {
    println!("{:?}", &TEST);
    test_utils::run_test(100, |contract| {
        assert_eq!(contract.balance("owner1".to_string()), 0);
        contract.add_near();
        assert_eq!(contract.balance("owner1".to_string()), 100);
        // TODO: assert contract NEAR balance
    })
}


#[test]
// #[should_panic]
fn test_double_deploy() {
    test_utils::run_test(0, |contract| {
        contract.deploy_code(TEST.to_string());
        contract.deploy_code(TEST.to_string());
    })
}

#[test]
fn test_internal_create() {
    test_utils::run_test(0, |contract| {
        let test_addr = contract.deploy_code(TEST.to_string());
        let (input, _) = soltest::functions::deploy_new_guy::call(8);
        let raw = contract.call_contract(test_addr, hex::encode(input));
        let sub_addr = raw[24..].to_string();

        let (new_input, _) = subcontract::functions::a_number::call();
        let new_raw = contract.call_contract(sub_addr, hex::encode(new_input));
        let output = subcontract::functions::a_number::decode_output(&hex::decode(&new_raw).unwrap()).unwrap();
        assert_eq!(output, U256::from(8));
    })
}
