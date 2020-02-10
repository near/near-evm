use std::collections::HashMap;

use ethereum_types::{Address, U256};
use vm::CreateContractAddress;

use borsh::{BorshDeserialize, BorshSerialize};

use near_bindgen::collections::Map as NearMap;
use near_bindgen::{env, ext_contract, near_bindgen as near_bindgen_macro, Promise};
use near_vm_logic::types::{AccountId, Balance};

use crate::evm_state::{EvmState, StateStore};
use crate::utils::prefix_for_contract_storage;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[macro_use]
extern crate lazy_static_include;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;

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
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self.code.get(&internal_addr.to_vec())
    }

    fn set_code(&mut self, address: &Address, bytecode: &Vec<u8>) {
        let internal_addr = utils::evm_account_to_internal_address(*address);
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
        self.commit_nonces(&other.nonces);
        self.commit_storages(&other.storages);
    }
}

#[near_bindgen_macro]
impl EvmContract {
    // for Eth call of similar name
    pub fn get_storage_at(&self, address: String, key: String) -> String {
        let mut key_buf = [0u8; 32];
        key_buf.copy_from_slice(&hex::decode(key).expect("invalid storage key"));
        let val = self
            .read_contract_storage(&utils::hex_to_evm_address(&address), key_buf)
            .unwrap_or([0u8; 32]);
        hex::encode(val)
    }

    pub fn deploy_code(&mut self, bytecode: String) -> String {
        let code = hex::decode(bytecode).expect("invalid hex");
        let sender = utils::predecessor_as_evm();

        // TODO: move into create
        let nonce = self.next_nonce(&sender);
        let (contract_address, _) = utils::evm_contract_address(
            CreateContractAddress::FromSenderAndNonce,
            &sender,
            &nonce,
            &code,
        );

        let val = attached_deposit_as_u256_opt().unwrap_or(U256::from(0));
        self.add_balance(&utils::predecessor_as_evm(), val);

        interpreter::deploy_code(self, &sender, val, 0, &contract_address, &code);
        hex::encode(&contract_address)
    }

    pub fn get_code(&self, address: &Address) -> Vec<u8> {
        self.code_at(address).expect("Contract not found")
    }

    pub fn call_contract(&mut self, contract_address: String, encoded_input: String) -> String {
        let contract_address =
            hex::decode(&contract_address).expect("contract_address must be hex");
        let contract_address = Address::from_slice(&contract_address);

        let value = attached_deposit_as_u256_opt();
        if let Some(val) = value {
            self.add_balance(&utils::predecessor_as_evm(), val);
        }
        let result = self.call_contract_internal(value, &contract_address, encoded_input);

        match result {
            Ok(v) => hex::encode(v),
            Err(s) => format!("internal call failed: {}", s),
        }
    }

    pub fn move_funds_to_near_account(&mut self, address: AccountId, amount: Balance) {
        let recipient = utils::near_account_id_to_evm_address(&address);
        let sender = utils::predecessor_as_evm();
        let amount = balance_to_u256(&amount);
        self.transfer_balance(&sender, &recipient, amount);
    }

    pub fn move_funds_to_evm_address(&mut self, address: String, amount: Balance) {
        let recipient = utils::hex_to_evm_address(&address);
        let sender = utils::predecessor_as_evm();
        let amount = balance_to_u256(&amount);
        self.sub_balance(&sender, amount);
        self.add_balance(&recipient, amount);
    }

    pub fn balance_of_near_account(&self, address: AccountId) -> Balance {
        let addr = utils::near_account_id_to_evm_address(&address);
        u256_to_balance(&self.balance_of(&addr))
    }

    pub fn balance_of_evm_address(&self, address: String) -> Balance {
        let addr = utils::hex_to_evm_address(&address);
        u256_to_balance(&self.balance_of(&addr))
    }

    pub fn add_near(&mut self) -> Balance {
        let val = attached_deposit_as_u256_opt().expect("Did not attach value");
        let addr = &utils::predecessor_as_evm();

        self.add_balance(&addr, val);
        u256_to_balance(&self.balance_of(&addr))
    }

    pub fn retrieve_near(&mut self, recipient: AccountId, amount: Balance) {
        let addr = utils::near_account_id_to_evm_address(&env::predecessor_account_id());

        if u256_to_balance(&self.balance_of(&addr)) < amount {
            panic!("insufficient funds");
        }

        Promise::new(recipient)
            .transfer(amount)
            .then(callback::finalize_retrieve_near(
                addr,
                amount.to_be_bytes().to_vec(),
                &env::current_account_id(),
                0,
                (env::prepaid_gas() - env::used_gas()) / 2,
            ));
    }

    pub fn finalize_retrieve_near(&mut self, addr: Address, amount: Vec<u8>) {
        let mut bin = [0u8; 16];
        bin.copy_from_slice(&amount[..]);
        // panics if called externally
        assert_eq!(
            env::current_account_id(),
            env::predecessor_account_id(),
            "caller is not self"
        );
        // panics if insufficient balance
        self.sub_balance(&addr, balance_to_u256(&Balance::from_be_bytes(bin)));
    }

    pub fn nonce_of_near_account(&self, address: AccountId) -> u128 {
        let addr = utils::near_account_id_to_evm_address(&address);
        u256_to_balance(&self.nonce_of(&addr))
    }

    pub fn nonce_of_evm_address(&self, address: String) -> u128 {
        let addr = utils::hex_to_evm_address(&address);
        u256_to_balance(&self.nonce_of(&addr))
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

    fn commit_nonces(&mut self, other: &HashMap<[u8; 20], [u8; 32]>) {
        self.nonces
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
        let internal_addr = utils::evm_account_to_internal_address(*address);
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
    ) -> Result<Vec<u8>, String> {
        // decode
        let input = encoded_input;
        let input = hex::decode(input).expect("invalid hex");

        // run
        let result = interpreter::call(
            self,
            &utils::near_account_id_to_evm_address(&env::predecessor_account_id()),
            value,
            0, // call-stack depth
            &contract_address,
            &input,
        );

        result.map(|v| v.to_vec())
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
