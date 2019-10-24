#[cfg(test)]
extern crate alloc;
#[cfg(test)]
#[macro_use]
extern crate ethabi_derive;
extern crate ethereum_types;
extern crate evm;
extern crate parity_bytes;
extern crate vm;

use std::sync::Arc;

use ethereum_types::{Address, U256};
use evm::Factory;
use near_bindgen::collections::Map as NearMap;
use vm::{ActionParams, CallType, Ext, GasLeft, Schedule};
use borsh::{BorshSerialize, BorshDeserialize};
use near_bindgen::{env, near_bindgen as near_bindgen_macro};

use fake_ext::FakeExt;

#[cfg(test)]
#[cfg(env_test)]
mod tests;

mod fake_ext;

#[near_bindgen_macro]
#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct EvmContract {
    code: NearMap<Vec<u8>, Vec<u8>>,
    balances: NearMap<Vec<u8>, u64>,
    storages: NearMap<Vec<u8>, NearMap<Vec<u8>, Vec<u8>>>,
}

#[near_bindgen_macro]
impl EvmContract {
    pub fn deploy_code(&mut self, contract_address: String, bytecode: String) {
        let code = hex::decode(bytecode).expect("invalid hex");
        let contract_address = contract_address.into_bytes();
        self.code.insert(&contract_address, &code);

        if let Some(GasLeft::NeedsReturn { data, .. }) = self.run_command_internal(&contract_address, "".to_string()) {
            let data = data.to_vec();
            self.code.insert(&contract_address, &data);
            env::log(format!("ok deployed {} bytes of code", data.len()).as_bytes());
        } else {
            panic!("init failed");
        }
    }

    pub fn run_command(&mut self, contract_address: String, encoded_input: String) -> String {
        let contract_address = contract_address.into_bytes();
        let result = self.run_command_internal(&contract_address, encoded_input);
        match result.unwrap() {
            GasLeft::NeedsReturn {
                gas_left: _,
                data,
                apply_state: _,
            } => {
                hex::encode(data.to_vec())
            }
            GasLeft::Known(_gas_left) => {
                "".to_owned()
            }
        }
    }
}

impl EvmContract {
    fn prefix_for_contract_storage(contract_address: &[u8]) -> Vec<u8> {
        let mut prefix = Vec::new();
        prefix.extend_from_slice(b"_storage");
        prefix.extend_from_slice(contract_address);
        prefix
    }

    fn run_command_internal(&mut self,
                            contract_address: &Vec<u8>,
                            encoded_input: String,
    ) -> Option<GasLeft> {
        let startgas = 1_000_000_000;
        let storage = self.storages.get(contract_address);
        let storage = if let Some(storage) = storage {
            storage
        } else {
            let storage_prefix = Self::prefix_for_contract_storage(&contract_address);
            let storage = NearMap::<Vec<u8>, Vec<u8>>::new(storage_prefix);
            self.storages.insert(&contract_address, &storage);
            storage
        };
        let code = self.code.get(contract_address).expect("code does not exist");
        let input = encoded_input;
        let input = hex::decode(input).expect("invalid hex");

        let mut params = ActionParams::default();

        params.call_type = CallType::None;
        params.code = Some(Arc::new(code));
        params.sender = sender_as_eth();
        params.origin = params.sender;
        params.gas = U256::from(startgas);
        params.data = Some(input);

        let mut ext = FakeExt::new(storage, self);
        ext.info.gas_limit = U256::from(startgas);

        ext.schedule = Schedule::new_constantinople();

        let instance = Factory::default().create(params, ext.schedule(), ext.depth());

        let result = instance.exec(&mut ext);
        result.ok().unwrap().ok()
    }
}

pub fn sender_name_to_eth_address(sender: &str) -> Address {
    let mut sender = sender.to_string().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender[0..20])
}

fn sender_as_eth() -> Address {
    let mut sender =
        env::signer_account_id().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender)
}
