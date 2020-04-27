use std::collections::HashMap;

use ethereum_types::{Address, U256};
use vm::CreateContractAddress;

use borsh::{BorshDeserialize, BorshSerialize};

use near_sdk::collections::Map as NearMap;
use near_sdk::{env, ext_contract, near_bindgen as near_bindgen_macro, Promise, AccountId};

use crate::evm_state::{EvmState, StateStore};
use crate::utils::{Balance, prefix_for_contract_storage};

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

/// Represents the state of the EVM. All NearMaps are persisted to Near chain storage
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
        for log in &other.logs {
            near_sdk::env::log(format!("evm log: {}", log).as_bytes());
        }
    }
}

#[near_bindgen_macro]
/// The EVM contract public interface. Generally, the EVM handles ethereum-style 20-byte
/// hex-encoded addresses. External Near accountIDs are converted to EVM addresses by hashing
/// them, and taking the final 20 bytes of the hash. Which is to say, they roughly correspond to
/// Ethereum's externally-owned-account public keys.
///
/// The EVM holds NEAR and keeps an internal balances mapping to all EVM accounts. Therefore EVM
/// contracts can hold NEAR and interact with it.
impl EvmContract {
    /// Returns the storage at a particular slot, as a hex-encoded. Slots are 32-bytes wide,
    /// empty slots will be all 0s.
    ///
    /// # Arguments
    ///
    /// * `address` - the hex-encoded account address to read from.
    /// * `key` - the hex-encoded 32-byte storage key to read
    ///
    /// # Panics
    ///
    /// * When `address` or `key` is not valid hex.
    pub fn get_storage_at(&self, address: String, key: String) -> String {
        let mut key_buf = [0u8; 32];
        key_buf.copy_from_slice(&hex::decode(key).expect("invalid storage key"));
        let val = self
            .read_contract_storage(&utils::hex_to_evm_address(&address), key_buf)
            .unwrap_or([0u8; 32]);
        hex::encode(val)
    }

    /// Deploy a new EVM contract. Returns the hex-encoded EVM address where the contract was
    /// deployed.
    ///
    /// # Arguments
    ///
    /// * `bytecode` - the hex-encoded contract initcode as produced by solc or another compiler.
    /// You can find this in truffle build json files as `bytecode`. Note that this is NOT the
    /// contract code. It is the initcode that runs when deploying the contract code.
    ///
    /// # Panics
    ///
    /// * When `bytecode` is not valid hex.
    /// * When the contract encounters a revert during initialization
    #[payable]
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

        let val = utils::attached_deposit_as_u256_opt().unwrap_or(U256::from(0));
        self.add_balance(&sender, val);

        interpreter::deploy_code(self, &sender, &sender, val, 0, &contract_address, &code);
        hex::encode(&contract_address)
    }

    /// Get the code deployed at an address. Returns the hex-encoded code at that address, or the
    /// empty string if no code is deployed.
    ///
    /// # Arguments
    ///
    /// * `address` - the hex-encoded account address to read from.
    ///
    /// # Panics
    ///
    /// * When `address` is not valid hex.
    pub fn get_code(&self, address: String) -> String {
        let address = utils::hex_to_evm_address(&address);
        hex::encode(self.code_at(&address).unwrap_or(vec![]))
    }

    /// Make an EVM transaction. Calls `contract_address` with `encoded_input`. Execution
    /// continues until all EVM messages have been processed. We expect this to behave identically
    /// to an Ethereum transaction, however there may be some edge cases.
    ///
    /// # Arguments
    ///
    /// * `contract_address` - the hex-encoded account address to call.
    /// * `encoded_input` - The hex-encoded data field of the Ethereum transaction. This typically
    /// contains the abi-encoded contract call arguments.
    ///
    /// # Panics
    ///
    /// * When `contract_address` or `encoded_input` is not valid hex.
    #[payable]
    pub fn call_contract(&mut self, contract_address: String, encoded_input: String) -> String {
        let contract_address = utils::hex_to_evm_address(&contract_address);
        let sender = utils::near_account_id_to_evm_address(&env::predecessor_account_id());

        let value = utils::attached_deposit_as_u256_opt();
        if let Some(val) = value {
            self.add_balance(&utils::predecessor_as_evm(), val);
        }
        let result = self.call_contract_internal(value, &contract_address, encoded_input, &sender, true);

        match result {
            Ok(v) => hex::encode(v),
            Err(s) => format!("internal call failed: {}", s),
        }
    }

    /// Make an EVM transaction. Calls `contract_address` with `encoded_input`. Execution
    /// continues until all EVM messages have been processed. We expect this to behave identically
    /// to an Ethereum transaction, however there may be some edge cases.
    ///
    /// This function serves the eth_call functionality, and will NOT apply state changes.
    ///
    /// # Arguments
    ///
    /// * `contract_address` - the hex-encoded account address to call.
    /// * `encoded_input` - the hex-encoded data field of the Ethereum transaction. This typically
    /// contains the abi-encoded contract call arguments.
    /// * `sender` - the hex-encoded sender of the view call. Used to pre-flight txns as if they
    /// were from the specified account.
    /// * `value` - the number of yoctoNEAR to simulate the call with. Sets msg.balance, but will
    /// NOT actually be transferred.
    ///
    /// # Panics
    ///
    /// * When `contract_address` or `encoded_input` or `sender` is not valid hex.
    pub fn view_call_contract(&mut self, contract_address: String, encoded_input: String, sender: String, value: Balance) -> String {
        let sender = utils::near_account_id_to_evm_address(&sender);
        let contract_address = utils::hex_to_evm_address(&contract_address);
        let val = match value {
            Balance(0) => None,
            Balance(v) => Some(U256::from(v))
        };

        let result = self.call_contract_internal(val, &contract_address, encoded_input, &sender, false);

        match result {
            Ok(v) => hex::encode(v),
            Err(s) => format!("internal call failed: {}", s),
        }
    }

    /// Move internal EVM balance to the EVM account corresponding to a specific Near account.
    /// This generally functions as an ethereum transfer, but will NOT trigger fallback functions.
    ///
    /// # Arguments
    ///
    /// * `address` - the NEAR account to credit EVM balance to.
    /// * `amount` - the number of yoctoNEAR to move
    ///
    /// # Panics
    ///
    /// * If the sender does not have sufficient NEAR balance in the EVM.
    pub fn move_funds_to_near_account(&mut self, address: AccountId, amount: Balance) {
        let sender = utils::predecessor_as_evm();
        let recipient = utils::near_account_id_to_evm_address(&address);
        let amount = utils::balance_to_u256(&amount);
        self.transfer_balance(&sender, &recipient, amount);
    }

    /// Move internal EVM balance to another EVM account.
    /// This generally functions as an ethereum transfer, but will NOT trigger fallback functions.
    ///
    /// # Arguments
    ///
    /// * `address` - the EVM account to credit EVM balance to.
    /// * `amount` - the number of yoctoNEAR to move
    ///
    /// # Panics
    ///
    /// * If `address` is not valid hex.
    /// * If the sender does not have sufficient NEAR balance in the EVM.
    pub fn move_funds_to_evm_address(&mut self, address: String, amount: Balance) {
        let recipient = utils::hex_to_evm_address(&address);
        let sender = utils::predecessor_as_evm();
        let amount = utils::balance_to_u256(&amount);
        self.sub_balance(&sender, amount);
        self.add_balance(&recipient, amount);
    }

    /// Returns the EVM balance of a Near AccountId.
    ///
    /// # Arguments
    ///
    /// * `address` - the Near account to check.
    pub fn balance_of_near_account(&self, address: AccountId) -> Balance {
        let addr = utils::near_account_id_to_evm_address(&address);
        utils::u256_to_balance(&self.balance_of(&addr))
    }

    /// Returns the EVM balance of an EVM address.
    ///
    /// # Arguments
    ///
    /// * `address` - the EVM account to check.
    ///
    /// # Panics
    ///
    /// * When `address` is not valid hex.
    pub fn balance_of_evm_address(&self, address: String) -> Balance {
        let addr = utils::hex_to_evm_address(&address);
        utils::u256_to_balance(&self.balance_of(&addr))
    }

    /// Credits near to the EVM account corresponding to the sending Near accountId. Used to fund
    /// EVM accounts so that the user can interact with contracts.
    ///
    /// # Panics
    ///
    /// * When no NEAR is attached to the call.
    #[payable]
    pub fn add_near(&mut self) -> Balance {
        let val = utils::attached_deposit_as_u256_opt().expect("Did not attach value");
        let addr = &utils::predecessor_as_evm();

        self.add_balance(&addr, val);
        utils::u256_to_balance(&self.balance_of(&addr))
    }

    /// Transfers NEAR out of the EVM to some Near account. Always transfers from the EVM account
    /// corresponding to the caller's near accountId.
    ///
    /// # Arguments
    ///
    /// * `recipient` - the Near accountId to which to transfer funds.
    /// * `amount` - the number of yoctoNEAR to transfer to `recipient`
    ///
    /// # Panics
    ///
    /// * If the sender does not have sufficient NEAR balance in the EVM.
    pub fn retrieve_near(&mut self, recipient: AccountId, amount: Balance) {
        let addr = utils::near_account_id_to_evm_address(&env::predecessor_account_id());

        if utils::u256_to_balance(&self.balance_of(&addr)) < amount {
            panic!("insufficient funds");
        }

        Promise::new(recipient)
            .transfer(amount.0)
            .then(callback::finalize_retrieve_near(
                addr,
                amount.to_be_bytes().to_vec(),
                &env::current_account_id(),
                0,
                (env::prepaid_gas() - env::used_gas()) / 2,
            ));
    }

    /// Internal method. Updates EVM accounting.
    /// TODO: check safety
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
        self.sub_balance(&addr, utils::balance_to_u256(&Balance::from_be_bytes(bin)));
    }

    /// Get the EVM nonce of a Near account. Unlike in the EVM, this is only incremented when
    /// deploying code. This function can be useful for predicting the address code will deploy
    /// at.
    ///
    /// # Arguments
    ///
    /// * `address` - the Near account to check.
    pub fn nonce_of_near_account(&self, address: AccountId) -> Balance {
        let addr = utils::near_account_id_to_evm_address(&address);
        utils::u256_to_balance(&self.nonce_of(&addr))
    }

    /// Get the EVM nonce of an EVM account. Unlike in the EVM, this is only incremented when
    /// deploying code. This function can be useful for predicting the address code will deploy
    /// at.
    ///
    /// # Arguments
    ///
    /// * `address` - the EVM account to check.
    ///
    /// # Panics
    ///
    /// * When `address` is not valid hex.
    pub fn nonce_of_evm_address(&self, address: String) -> Balance {
        let addr = utils::hex_to_evm_address(&address);
        utils::u256_to_balance(&self.nonce_of(&addr))
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
        sender: &Address,
        should_commit: bool,
    ) -> Result<Vec<u8>, String> {
        // decode
        let input = encoded_input;
        let input = hex::decode(input).expect("invalid hex");

        // run
        let result = interpreter::call(
            self,
            sender,
            sender,
            value,
            0, // call-stack depth
            &contract_address,
            &input,
            should_commit,
        );

        result.map(|v| v.to_vec())
    }
}
