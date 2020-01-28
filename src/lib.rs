use std::collections::HashMap;

use vm::GasLeft;
use ethereum_types::U256;

use borsh::{BorshDeserialize, BorshSerialize};

use near_vm_logic::types::{AccountId, Balance};
use near_bindgen::collections::Map as NearMap;
use near_bindgen::{callback_args, env, ext_contract, near_bindgen as near_bindgen_macro, Promise};

use crate::evm_state::{EvmState, StateStore};
use crate::utils::prefix_for_contract_storage;

#[cfg(test)]
mod tests;

mod evm_state;
mod interpreter;
mod near_ext;
pub mod utils;

#[near_bindgen_macro]
#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct EvmContract {
    code: NearMap<Vec<u8>, Vec<u8>>,
    balances: NearMap<Vec<u8>, [u8; 32]>,
    storages: NearMap<Vec<u8>, NearMap<Vec<u8>, Vec<u8>>>,
}

#[ext_contract]
pub trait Callback {
    fn finalize_retrieve_near(&mut self, addr: &Vec<u8>, amount: &Vec<u8>);
}

impl EvmState for EvmContract {

    // Default code of None
    fn code_at(&self, address: &Vec<u8>) -> Option<Vec<u8>> {
        self.code.get(address)
    }

    fn set_code(&mut self, address: &Vec<u8>, bytecode: &Vec<u8>) {
        self.code.insert(address, bytecode);
    }

    fn _set_balance(&mut self, address: &Vec<u8>, balance: [u8; 32]) -> Option<[u8; 32]> {
        self.balances.insert(address, &balance)
    }

    fn set_balance(&mut self, address: &Vec<u8>, balance: U256) -> Option<U256> {
        let mut bin = [0u8; 32];
        balance.to_big_endian(&mut bin);
        self._set_balance(address, bin).map(|v| v.into())
    }

    // default balance of 0
    fn _balance_of(&self, address: &Vec<u8>) -> [u8; 32] {
        self.balances.get(address).unwrap_or([0u8; 32])
    }

    fn balance_of(&self, address: &Vec<u8>) -> U256 {
        self._balance_of(address).into()
    }

    // Default storage of None
    fn read_contract_storage(&self, address: &Vec<u8>, key: &Vec<u8>) -> Option<Vec<u8>> {
        self.contract_storage(address).get(key)
    }

    fn set_contract_storage(
        &mut self,
        address: &Vec<u8>,
        key: &Vec<u8>,
        value: &Vec<u8>,
    ) -> Option<Vec<u8>> {
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
            panic!(format!(
                "Contract exists at {}",
                hex::encode(contract_address)
            ));
        }

        self.set_code(&contract_address, &code);

        let val = attached_deposit_as_u256_opt();
        let opt = self.call_contract_internal(
            val,
            &contract_address,
            "".to_string());

        match opt {
            Some(data) => {
                self.set_code(&contract_address, &data);
                env::log(
                    format!(
                        "ok deployed {} bytes of code at address {}",
                        data.len(),
                        hex::encode(&contract_address)
                    )
                    .as_bytes(),
                );
            }
            None => panic!("init failed"),
        }
    }

    pub fn call_contract(&mut self, contract_address: String, encoded_input: String) -> String {
        let contract_address = contract_address.into_bytes();
        let val = attached_deposit_as_u256_opt();

        let result = self.call_contract_internal(val, &contract_address, encoded_input);

        match result {
            Some(v) => hex::encode(v),
            None => panic!("internal command returned None"),
        }
    }

    pub fn balance(&self, address: AccountId) -> Balance {
        let addr = utils::sender_name_to_internal_address(&address);
        u256_to_balance(&self.balance_of(&addr))
    }

    pub fn add_near(&mut self) -> Balance {
        let val = attached_deposit_as_u256_opt().expect("Did not attach value");
        let addr = utils::sender_name_to_internal_address(&env::predecessor_account_id());


        self.add_balance(&addr, val);
        u256_to_balance(&self.balance_of(&addr))
    }

    pub fn retrieve_near(&mut self, recipient: AccountId, amount: Balance) {
        let addr = utils::sender_name_to_internal_address(&env::predecessor_account_id());

        if u256_to_balance(&self.balance_of(&addr)) < amount {
            panic!("insufficient funds");
        }

        /*
        panicked at 'called `Result::unwrap()` on an `Err` value: HostError(GasExceeded)',
        near-bindgen/src/environment/mocked_blockchain.rs:252:9
        */
        Promise::new(recipient)
            .transfer(amount)
            .then(
                callback::finalize_retrieve_near(
                    &addr,
                    &amount.to_be_bytes().to_vec(),
                    &env::current_account_id(),
                    0,
                    2u64.pow(63))
            );
    }

    pub fn finalize_retrieve_near(&mut self, addr: &Vec<u8>, amount: &Vec<u8>) {
        let mut bin = [0u8; 16];
        bin.copy_from_slice(&amount[..]);
        // panics if called externally
        assert_eq!(
            env::current_account_id(),
            env::predecessor_account_id());
        // panics if insufficient balance
        self.sub_balance(addr, balance_to_u256(&Balance::from_be_bytes(bin)));
    }
}

impl EvmContract {

    fn commit_code(&mut self, other: &HashMap<Vec<u8>, Vec<u8>>) {
        self.code
            .extend(other.into_iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    fn commit_balances(&mut self, other: &HashMap<Vec<u8>, [u8; 32]>) {
        self.balances
            .extend(other
                       .into_iter()
                       .map(|(k, v)| (k.clone(), v.clone())));
    }

    fn commit_storages(&mut self, other: &HashMap<Vec<u8>, HashMap<Vec<u8>, Vec<u8>>>) {
        for (k, v) in other.iter() {
            let mut storage = self.contract_storage(k);
            storage.extend(v.into_iter().map(|(k, v)| (k.clone(), v.clone())));
            self.storages.insert(k, &storage);
        }
    }

    fn contract_storage(&self, address: &Vec<u8>) -> NearMap<Vec<u8>, Vec<u8>> {
        self.storages
            .get(address)
            .unwrap_or_else(|| self.get_new_contract_storage(address))
    }

    fn get_new_contract_storage(&self, address: &Vec<u8>) -> NearMap<Vec<u8>, Vec<u8>> {
        let storage_prefix = prefix_for_contract_storage(&address);
        let storage = NearMap::<Vec<u8>, Vec<u8>>::new(storage_prefix);
        storage
    }

    fn call_contract_internal(
        &mut self,
        value: Option<U256>,
        contract_address: &Vec<u8>,
        encoded_input: String,
    ) -> Option<Vec<u8>> {
        // decode
        let input = encoded_input;
        let input = hex::decode(input).expect("invalid hex");

        // run
        let result = interpreter::call(
            self,
            value,
            0, // call-stack depth
            contract_address,
            &input,
        );

        match result {
            Some(GasLeft::Known(_)) => {
                // No returndata
                Some(vec![])
            }
            Some(GasLeft::NeedsReturn {
                // NB: EVM handles this separately because returning data costs variable gas
                gas_left: _,
                data,
                apply_state: _,
            }) => Some(data.to_vec()),
            None => None,
        }
    }
}

fn attached_deposit_as_u256_opt() -> Option<U256> {
    let attached = env::attached_deposit();
    if attached == 0 {
        None
    } else {
        Some(balance_to_u256(&attached))
    }
}

fn balance_to_u256(val: &Balance) -> U256 {
    let mut bin = [0u8; 32];
    bin[16..].copy_from_slice(&val.to_be_bytes());
    bin.into()
}

fn u256_to_balance(val: &U256) -> Balance {
    let mut scratch = [0u8; 32];
    let mut bin = [0u8; 16];
    val.to_big_endian(&mut scratch);
    bin.copy_from_slice(&scratch[16..]);
    Balance::from_be_bytes(bin)
}
