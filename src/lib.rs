use std::sync::Arc;
use std::collections::HashMap;

use ethereum_types::{Address, U256};
use evm::Factory;
use vm::{ActionParams, CallType, Ext, GasLeft, Schedule};

use borsh::{BorshSerialize, BorshDeserialize};

use near_bindgen::collections::Map as NearMap;
use near_bindgen::{env, near_bindgen as near_bindgen_macro};

use crate::near_ext::NearExt;
use crate::evm_state::{EvmState, SubState, StateStore};
use crate::utils::{prefix_for_contract_storage};


#[cfg(test)]
mod tests;

mod near_ext;
mod evm_state;
mod utils;


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

    pub fn run_command(&mut self, contract_address: String, encoded_input: String) -> String {
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
            for (k2, v2) in v.iter() {
                println!("COMMIT {:?} is {:?}", hex::encode(k2), hex::encode(v2));
            }
            let mut storage = self.contract_storage(k);
            storage.extend(v.into_iter().map(|(k, v)| (k.clone(), v.clone())));
            self.storages.insert(k, &storage);
        }
        for (k, v) in self.storages.iter() {
            println!("DUMPING STATE");
            for (k2, v2) in v.iter() {
                println!("STATE {:?} {:?} is {:?}", hex::encode(&k), hex::encode(k2), hex::encode(v2));
            }
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

fn run_and_commit_if_success(state: &mut dyn EvmState,
                             state_address: Vec<u8>,
                             code_address: Vec<u8>,
                             input: Vec<u8>) -> Option<GasLeft> {
        let (result, state_updates) = run_against_state(
            state,
            state_address,
            code_address,
            input);
        match result {
            Some(GasLeft::Known(_)) => {
                state.commit_changes(&state_updates.unwrap());
                result
            },
            Some(GasLeft::NeedsReturn{
                gas_left: _,
                data: _,
                apply_state,
            }) => {
                if apply_state {
                    state.commit_changes(&state_updates.unwrap());
                }
                result
            },
            None => None
        }
}

fn run_against_state(state: &dyn EvmState,
                     state_address: Vec<u8>,
                     code_address: Vec<u8>,
                     input: Vec<u8>) -> (Option<GasLeft>, Option<StateStore>) {
    let startgas = 1_000_000_000;
    let code = state.code_at(&code_address).expect("code does not exist");

    let mut store = StateStore::default();
    let mut sub_state = SubState::new(&mut store, state);

    let mut params = ActionParams::default();

    params.call_type = CallType::None;
    params.code = Some(Arc::new(code));
    params.sender = sender_as_eth();
    params.origin = params.sender;
    params.gas = U256::from(startgas);
    params.data = Some(input.to_vec());

    let mut ext = NearExt::new(state_address.to_vec(), &mut sub_state, 0);
    ext.info.gas_limit = U256::from(startgas);
    ext.schedule = Schedule::new_constantinople();

    let instance = Factory::default().create(params, ext.schedule(), ext.depth());

    // Run the code
    let result = instance.exec(&mut ext);

    (result.ok().unwrap().ok(), Some(store))
}

fn sender_as_eth() -> Address {
    let mut sender =
        env::signer_account_id().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender)
}
