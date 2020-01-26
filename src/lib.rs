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

        if let Some(GasLeft::NeedsReturn { data, .. }) = self.run_command_internal(&contract_address, "".to_string()) {
            let data = data.to_vec();
            self.set_code(&contract_address, &data);
            env::log(format!("ok deployed {} bytes of code at address {}", data.len(), hex::encode(&contract_address)).as_bytes());
        } else {
            panic!("init failed");
        }
    }

    pub fn run_command(&mut self, contract_address: String, encoded_input: String) -> String {
        let contract_address = contract_address.into_bytes();
        println!("\n\nSTART");
        let result = self.run_command_internal(&contract_address, encoded_input);

        println!("NAXT");
        let unwrapped = result.unwrap();
        println!("THARD");
        match unwrapped {
            GasLeft::NeedsReturn {
                gas_left: _,
                data,
                apply_state: _,
            } => {
                println!("RET DATA {:?}", hex::encode(data.to_vec()));
                hex::encode(data.to_vec())
            },
            GasLeft::Known(_gas_left) => "".to_owned()
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

    fn run_command_internal(&mut self, contract_address: &Vec<u8>, encoded_input: String) -> Option<GasLeft> {
        // decode
        let input = encoded_input;
        let input = hex::decode(input).expect("invalid hex");

        // run
        let (result, state_updates) = self.run_against_state(
            &contract_address,
            &contract_address,
            &input);
        println!("RESULT IS {:?}", result);

        // TODO: commit only if result is good
        //       return properly
        self.commit_changes(&state_updates.unwrap());
        println!("new balance shoes {:?}", self.balance_of(contract_address));
        println!(
            "new storage {:?}",
            self.read_contract_storage(
                contract_address,
                &hex::decode("b9d38c3fd8a7d9c75ec9c730df6f3da77bd5dacce98f82cfebee48889fc7f80d").ok().unwrap()
            ));
        println!("RETURNING");
        result
    }

    fn run_against_state(&self,
                         state_address: &Vec<u8>,
                         code_address: &Vec<u8>,
                         input: &Vec<u8>,
    ) -> (Option<GasLeft>, Option<StateStore>) {

        let mut store = StateStore::default();
        let mut sub_state = SubState::new(&mut store, self);

        let code = sub_state.code_at(code_address).expect("code does not exist");

        println!("contract of {:?} bytes", code.len());

        let mut params = ActionParams::default();

        params.call_type = CallType::None;
        params.code = Some(Arc::new(code));
        params.sender = sender_as_eth();
        params.origin = params.sender;
        params.gas = U256::from(1_000_000_000);
        params.data = Some(input.to_vec());

        let mut ext = NearExt::new(state_address.to_vec(), &mut sub_state, self, 0, 0);
        ext.info.gas_limit = U256::from(1_000_000_000);
        ext.schedule = Schedule::new_constantinople();

        let instance = Factory::default().create(params, ext.schedule(), ext.depth());

        // Run the code
        let result = instance.exec(&mut ext);

        // println!(
        //     "sub new storage {:?}",
        //     sub_state.read_contract_storage(
        //         state_address,
        //         &hex::decode("b9d38c3fd8a7d9c75ec9c730df6f3da77bd5dacce98f82cfebee48889fc7f80d").ok().unwrap()
        //     ));

        print_storages(&"intermediate".to_string(), &store);

        let okayed = result.ok();
        println!("okayed {:?}", &okayed);
        (okayed.unwrap().ok(), Some(store))
        // (result.ok().unwrap().ok(), Some(store))
    }
}

fn print_storages(prefix: &String, store: &StateStore) {
    let storages = &store.storages;
    for (k, v) in storages.into_iter() {
        println!("{:?}", hex::encode(&k));
        print_storage(prefix, v)
    }
}

fn print_storage(prefix: &String, storage: &HashMap<Vec<u8>, Vec<u8>>) {
    for (k, v) in storage.into_iter() {
        println!("{} {:?} is {:?}", prefix, hex::encode(k), hex::encode(v));
    }
}

fn sender_as_eth() -> Address {
    let mut sender =
        env::signer_account_id().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender)
}
