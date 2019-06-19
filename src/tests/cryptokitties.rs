use ethabi::Address;
use ethabi_contract::use_contract;

use crate::{deploy_code, run_command};
use crate::{DeployCodeInput, RunCommandInput, sender_name_to_eth_address};

use super::near_stubs::{set_input, set_sender};

use_contract!(cryptokitties, "src/tests/kittyCore.abi");

fn set_sender_name(sender: &Address) {
    set_sender(sender.as_bytes().to_vec());
}

fn deploy_cryptokitties() {
    let zombie_code = include_bytes!("kittyCore.bin").to_vec();
    let code = DeployCodeInput {
        contract_address: "kitties".to_string(),
        bytecode: String::from_utf8(zombie_code).unwrap(),
    };
    set_input(serde_json::to_string(&code).unwrap().into_bytes());
    deploy_code();
}

fn create_promo_kitty() {
    let (input, _decoder) =
        cryptokitties::functions::create_promo_kitty::call(0, sender_name_to_eth_address("cat"));
    let run = RunCommandInput {
        contract_address: "kitties".to_string(),
        encoded_input: hex::encode(input),
    };
    set_input(serde_json::to_string(&run).unwrap().into_bytes());
    run_command();
}

#[test]
fn test_kitties() {
    let sender = sender_name_to_eth_address("berry");
    set_sender_name(&sender);
    deploy_cryptokitties();
    create_promo_kitty();
}
