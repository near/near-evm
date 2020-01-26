use std::collections::HashMap;

use vm::{GasLeft};

use borsh::{BorshSerialize, BorshDeserialize};

use near_bindgen::collections::Map as NearMap;
use near_bindgen::{env, near_bindgen as near_bindgen_macro};

use crate::evm_state::{EvmState, StateStore};
use crate::utils::{prefix_for_contract_storage};
use crate::interpreter::{run_and_commit_if_success};

#[cfg(test)]
mod tests;

mod near_ext;
mod evm_state;
mod interpreter;
pub mod utils;


#[near_bindgen_macro]
#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct EvmContract {
    pub code: NearMap<Vec<u8>, Vec<u8>>,
    pub balances: NearMap<Vec<u8>, u64>,
    pub storages: NearMap<Vec<u8>, NearMap<Vec<u8>, Vec<u8>>>,
}

impl EvmState for EvmContract {
    fn code_at(&self, address: &Vec<u8>) -> Option<Vec<u8>> {
        self.code.get(address)
    }

    fn set_code(&mut self, address: &Vec<u8>, bytecode: &Vec<u8>) {
        self.code.insert(address, bytecode);
    }

    fn set_balance(&mut self, address: &Vec<u8>, balance: u64) -> Option<u64> {
        self.balances.insert(address, &balance)
    }

    fn balance_of(&self, address: &Vec<u8>) -> u64 {
        self.balances.get(address).unwrap_or(0)
    }

    fn read_contract_storage(&self, address: &Vec<u8>, key: &Vec<u8>) -> Option<Vec<u8>> {
        self.contract_storage(address).get(key)
    }

    fn set_contract_storage(&mut self, address: &Vec<u8>, key: &Vec<u8>, value: &Vec<u8>)  -> Option<Vec<u8>> {
        self.contract_storage(address).insert(key, value)
    }

    fn commit_changes(&mut self, other: &StateStore) {
        self.commit_code(&other.code);
        self.commit_balances(&other.balances);
        self.commit_storages(&other.storages);
    }


}

#[near_bindgen_macro]
impl EvmContract {
    pub fn deploy_code(&mut self, contract_address: String, bytecode: String) {
        let code = hex::decode(bytecode).expect("invalid hex");
        let contract_address = contract_address.into_bytes();

        if self.code_at(&contract_address).is_some() {
            panic!(format!("Contract exists at {}", hex::encode(contract_address)));
        }

        self.set_code(&contract_address, &code);

        let opt = self.run_command_internal(&contract_address, "".to_string());

        match opt {
            Some(data) => {
                self.set_code(&contract_address, &data);
                env::log(format!("ok deployed {} bytes of code at address {}", data.len(), hex::encode(&contract_address)).as_bytes());
            }
            None => panic!("init failed")
        }
    }

    pub fn call_contract(&mut self, contract_address: String, encoded_input: String) -> String {
        let contract_address = contract_address.into_bytes();

        let result = self.run_command_internal(&contract_address, encoded_input);

        match result {
            Some(v) => hex::encode(v),
            None => panic!("internal command returned None")
        }
    }
}

impl EvmContract {
    pub fn commit_code(&mut self, other: &HashMap<Vec<u8>, Vec<u8>>) {
        self.code.extend(other.into_iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    pub fn commit_balances(&mut self, other: &HashMap<Vec<u8>, u64>) {
        self.balances.extend(other.into_iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    pub fn commit_storages(&mut self, other: &HashMap<Vec<u8>, HashMap<Vec<u8>, Vec<u8>>>) {
        for (k, v) in other.iter() {
            let mut storage = self.contract_storage(k);
            storage.extend(v.into_iter().map(|(k, v)| (k.clone(), v.clone())));
            self.storages.insert(k, &storage);
        }
    }

    fn contract_storage(&self, address: &Vec<u8>) -> NearMap<Vec<u8>, Vec<u8>>  {
         self.storages.get(address).unwrap_or_else(|| {
            self.get_new_contract_storage(address)
        })
    }

    fn get_new_contract_storage(&self, address: &Vec<u8>) -> NearMap<Vec<u8>, Vec<u8>>{
        let storage_prefix = prefix_for_contract_storage(&address);
        let storage = NearMap::<Vec<u8>, Vec<u8>>::new(storage_prefix);
        storage
    }

    fn run_command_internal(&mut self, contract_address: &Vec<u8>, encoded_input: String) -> Option<Vec<u8>> {
        // decode
        let input = encoded_input;
        let input = hex::decode(input).expect("invalid hex");

        // run
        let result = run_and_commit_if_success(
            self,
            contract_address.to_vec(),
            contract_address.to_vec(),
            input);

        match result {
            Some(GasLeft::Known(_)) => {  // No returndata
                Some(vec![])
            },
            Some(GasLeft::NeedsReturn{   // NB: EVM handles this separately because returning data costs variable gas
                gas_left: _,
                data,
                apply_state: _,
            }) => {
                Some(data.to_vec())
            },
            None => None
        }
    }
}
