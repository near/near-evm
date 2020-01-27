use ethereum_types::Address;

use near_bindgen::env;

pub fn sender_as_eth() -> Address {
    let mut sender = env::signer_account_id().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender)
}

pub fn prefix_for_contract_storage(contract_address: &[u8]) -> Vec<u8> {
    let mut prefix = Vec::new();
    prefix.extend_from_slice(b"_storage");
    prefix.extend_from_slice(contract_address);
    prefix
}

pub fn sender_name_to_eth_address(sender: &str) -> Address {
    let mut sender = sender.to_string().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender[0..20])
}
