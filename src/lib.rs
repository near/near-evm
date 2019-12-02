use std::sync::Arc;

use borsh::{BorshDeserialize, BorshSerialize};
use ethereum_types::{Address, H160, U256};
use evm::Factory;
use near_bindgen::{env, near_bindgen as near_bindgen_macro};
use near_bindgen::collections::Map as NearMap;
use vm::{ActionParams, CallType, Ext, GasLeft, Schedule};

use crate::fake_ext::FakeExt;
use keccak_hash::keccak;

#[cfg(test)]
#[cfg(feature = "env_test")]
mod tests;

mod fake_ext;

#[near_bindgen_macro]
#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct EvmContract {
    code: NearMap<Vec<u8>, Vec<u8>>,
    balances: NearMap<Vec<u8>, u64>,
    storages: NearMap<Vec<u8>, NearMap<Vec<u8>, Vec<u8>>>,
}

fn hash_to_h160(hash: U256) -> H160 {
    let mut result = H160([0; 20]);
    result.0.copy_from_slice(&(hash.0).0[..20]);
    result
}

#[near_bindgen_macro]
impl EvmContract {
    pub fn deploy_code(&mut self, bytecode: String) -> String {
        let code = hex::decode(bytecode).expect("invalid hex");
        let contract_address = hash_to_h160(keccak(&code));
        self.code.insert(&contract_address.0.to_vec(), &code);

        if let Some(GasLeft::NeedsReturn { data, .. }) = self.run_command_internal(sender_as_eth(), &contract_address.0, "".to_string()) {
            let data = data.to_vec();
            self.code.insert(&contract_address.0.to_vec(), &data);
            env::log(format!("ok deployed {} bytes of code", data.len()).as_bytes());
            hex::encode(contract_address)
        } else {
            panic!("init failed");
        }
    }

    pub fn view_call(&mut self, contract_address: String, encoded_input: String) -> String {
        let contract_address = contract_address.into_bytes();
        let result = self.run_command_internal(H160([0; 20]), &contract_address, encoded_input);
        match result.unwrap() {
            GasLeft::NeedsReturn {data, ..} => hex::encode(data.to_vec()),
            GasLeft::Known(_) => "".to_owned(),
        }
    }

    pub fn run_command(&mut self, contract_address: String, encoded_input: String) -> String {
        let contract_address = contract_address.into_bytes();
        let result = self.run_command_internal(sender_as_eth(), &contract_address, encoded_input);
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
                            sender: Address,
                            contract_address: &[u8],
                            encoded_input: String,
    ) -> Option<GasLeft> {
        let startgas = 1_000_000_000;
        let key = contract_address.to_vec();
        let storage = self.storages.get(&key);
        let storage = if let Some(storage) = storage {
            storage
        } else {
            let storage_prefix = Self::prefix_for_contract_storage(&contract_address);
            let storage = NearMap::<Vec<u8>, Vec<u8>>::new(storage_prefix);
            self.storages.insert(&key, &storage);
            storage
        };
        let code = self.code.get(&key).expect("code does not exist");
        let input = encoded_input;
        let input = hex::decode(input).expect("invalid hex");

        let mut params = ActionParams::default();

        params.call_type = CallType::None;
        params.code = Some(Arc::new(code));
        params.sender = sender;
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
