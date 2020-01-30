use std::collections::HashMap;

use ethereum_types::{Address, U256};
use vm::GasLeft;

use borsh::{BorshDeserialize, BorshSerialize};

use near_bindgen::collections::Map as NearMap;
use near_bindgen::{env, ext_contract, near_bindgen as near_bindgen_macro, Promise};
use near_vm_logic::types::{AccountId, Balance};

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
    nonces: NearMap<Vec<u8>, [u8; 32]>,
    storages: NearMap<Vec<u8>, NearMap<[u8; 32], [u8; 32]>>,
}

#[ext_contract]
pub trait Callback {
    fn finalize_retrieve_near(&mut self, addr: Address, amount: Vec<u8>);
}

impl EvmState for EvmContract {
    // Default code of None
    fn code_at(&self, address: &Address) -> Option<Vec<u8>> {
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self.code.get(&internal_addr.to_vec())
    }

    fn set_code(&mut self, address: &Address, bytecode: &Vec<u8>) {
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self.code.insert(&internal_addr.to_vec(), bytecode);
    }

    fn _set_balance(&mut self, address: [u8; 20], balance: [u8; 32]) -> Option<[u8; 32]> {
        self.balances.insert(&address.to_vec(), &balance)
    }

    // default balance of 0
    fn _balance_of(&self, address: [u8; 20]) -> [u8; 32] {
        self.balances.get(&address.to_vec()).unwrap_or([0u8; 32])
    }

    fn _set_nonce(&mut self, address: [u8; 20], nonce: [u8; 32]) -> Option<[u8; 32]> {
        self.nonces.insert(&address.to_vec(), &nonce)
    }

    // default nonce of 0
    fn _nonce_of(&self, address: [u8; 20]) -> [u8; 32] {
        self.nonces.get(&address.to_vec()).unwrap_or([0u8; 32])
    }

    // Default storage of None
    fn read_contract_storage(&self, address: &Address, key: [u8; 32]) -> Option<[u8; 32]> {
        self.contract_storage(address).get(&key)
    }

    fn set_contract_storage(
        &mut self,
        address: &Address,
        key: [u8; 32],
        value: [u8; 32],
    ) -> Option<[u8; 32]> {
        self.contract_storage(address).insert(&key, &value)
    }

    fn commit_changes(&mut self, other: &StateStore) {
        self.commit_code(&other.code);
        self.commit_balances(&other.balances);
        self.commit_storages(&other.storages);
    }
}

#[near_bindgen_macro]
impl EvmContract {
    // TODO: this is UNSAFE. need to calculate contract address rather than pass in
    pub fn deploy_code(&mut self, contract_address: AccountId, bytecode: String) -> String {
        let code = hex::decode(bytecode).expect("invalid hex");
        let contract_address = utils::near_account_id_to_eth_address(&contract_address);

        if self.code_at(&contract_address).is_some() {
            panic!(format!(
                "Contract exists at {}",
                hex::encode(contract_address)
            ));
        }

        self.set_code(&contract_address, &code);

        let val = attached_deposit_as_u256_opt();
        let opt = self.call_contract_internal(val, &contract_address, "".to_string());

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
                hex::encode(&contract_address)
            }
            None => panic!("init failed"),
        }
    }

    pub fn call_contract(&mut self, contract_address: AccountId, encoded_input: String) -> String {
        let contract_address = utils::near_account_id_to_eth_address(&contract_address);
        let val = attached_deposit_as_u256_opt();

        let result = self.call_contract_internal(val, &contract_address, encoded_input);

        match result {
            Some(v) => hex::encode(v),
            None => panic!("internal command returned None"),
        }
    }

    pub fn balance(&self, address: AccountId) -> Balance {
        let addr = utils::near_account_id_to_eth_address(&address);
        u256_to_balance(&self.balance_of(&addr))
    }

    pub fn add_near(&mut self) -> Balance {
        let val = attached_deposit_as_u256_opt().expect("Did not attach value");
        let addr = utils::near_account_id_to_eth_address(&env::predecessor_account_id());

        self.add_balance(&addr, val);
        u256_to_balance(&self.balance_of(&addr))
    }

    pub fn retrieve_near(&mut self, recipient: AccountId, amount: Balance) {
        let addr = utils::near_account_id_to_eth_address(&env::predecessor_account_id());

        if u256_to_balance(&self.balance_of(&addr)) < amount {
            panic!("insufficient funds");
        }

        /*
        panicked at 'called `Result::unwrap()` on an `Err` value: HostError(GasExceeded)',
        near-bindgen/src/environment/mocked_blockchain.rs:252:9
        */
        Promise::new(recipient)
            .transfer(amount)
            .then(callback::finalize_retrieve_near(
                addr,
                amount.to_be_bytes().to_vec(),
                &env::current_account_id(),
                0,
                2u64.pow(63),
            ));
    }

    pub fn finalize_retrieve_near(&mut self, addr: Address, amount: Vec<u8>) {
        let mut bin = [0u8; 16];
        bin.copy_from_slice(&amount[..]);
        // panics if called externally
        assert_eq!(env::current_account_id(), env::predecessor_account_id());
        // panics if insufficient balance
        self.sub_balance(&addr, balance_to_u256(&Balance::from_be_bytes(bin)));
    }
}

impl EvmContract {
    fn commit_code(&mut self, other: &HashMap<[u8; 20], Vec<u8>>) {
        self.code
            .extend(other.into_iter().map(|(k, v)| (k.to_vec(), v.clone())));
    }

    fn commit_balances(&mut self, other: &HashMap<[u8; 20], [u8; 32]>) {
        self.balances
            .extend(other.into_iter().map(|(k, v)| (k.to_vec(), v.clone())));
    }

    fn commit_storages(&mut self, other: &HashMap<[u8; 20], HashMap<[u8; 32], [u8; 32]>>) {
        for (k, v) in other.iter() {
            let mut storage = self._contract_storage(*k);
            storage.extend(v.into_iter().map(|(k, v)| (k.clone(), v.clone())));
            self.storages.insert(&k.to_vec(), &storage);
        }
    }

    fn _contract_storage(&self, address: [u8; 20]) -> NearMap<[u8; 32], [u8; 32]> {
        self.storages
            .get(&address.to_vec())
            .unwrap_or_else(|| self.get_new_contract_storage(address))
    }

    fn contract_storage(&self, address: &Address) -> NearMap<[u8; 32], [u8; 32]> {
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self._contract_storage(internal_addr)
    }

    fn get_new_contract_storage(&self, address: [u8; 20]) -> NearMap<[u8; 32], [u8; 32]> {
        let storage_prefix = prefix_for_contract_storage(&address);
        let storage = NearMap::<[u8; 32], [u8; 32]>::new(storage_prefix);
        storage
    }

    fn call_contract_internal(
        &mut self,
        value: Option<U256>,
        contract_address: &Address,
        encoded_input: String,
    ) -> Option<Vec<u8>> {
        // decode
        let input = encoded_input;
        let input = hex::decode(input).expect("invalid hex");

        // run
        let result = interpreter::call(
            self,
            &utils::near_account_id_to_eth_address(&env::predecessor_account_id()),
            value,
            0, // call-stack depth
            &contract_address,
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
