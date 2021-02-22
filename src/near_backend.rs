#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

use primitive_types::{H160, H256, U256};

use crate::backend::{Apply, ApplyBackend, Basic, Log};
use crate::sdk;
use crate::types::{
    address_to_key, bytes_to_hex, log_to_bytes, storage_to_key, u256_to_arr, KeyPrefix,
};

const OWNER_KEY: &[u8; 5] = b"OWNER";

pub struct Backend {
    chain_id: U256,
    origin: H160,
}

impl Backend {
    pub fn new(chain_id: u64, origin: H160) -> Self {
        Self {
            chain_id: U256::from(chain_id),
            origin,
        }
    }

    pub fn set_owner(account_id: &[u8]) {
        sdk::write_storage(OWNER_KEY, account_id)
    }

    pub fn get_owner() -> Vec<u8> {
        sdk::read_storage(OWNER_KEY).unwrap_or_else(Vec::new)
    }

    pub fn set_code(address: &H160, code: &[u8]) {
        sdk::write_storage(&address_to_key(KeyPrefix::Code, address), code);
    }

    pub fn remove_code(address: &H160) {
        sdk::remove_storage(&address_to_key(KeyPrefix::Code, address))
    }

    pub fn get_code(address: &H160) -> Vec<u8> {
        sdk::read_storage(&address_to_key(KeyPrefix::Code, address)).unwrap_or_else(Vec::new)
    }

    pub fn set_nonce(address: &H160, nonce: &U256) {
        sdk::write_storage(
            &address_to_key(KeyPrefix::Nonce, address),
            &u256_to_arr(nonce),
        );
    }

    pub fn remove_nonce(address: &H160) {
        sdk::remove_storage(&address_to_key(KeyPrefix::Nonce, address))
    }

    pub fn get_nonce(address: &H160) -> U256 {
        sdk::read_storage(&address_to_key(KeyPrefix::Nonce, address))
            .map(|value| U256::from_big_endian(&value))
            .unwrap_or_else(U256::zero)
    }

    pub fn set_balance(address: &H160, balance: &U256) {
        sdk::write_storage(
            &address_to_key(KeyPrefix::Balance, address),
            &u256_to_arr(balance),
        );
    }

    pub fn remove_balance(address: &H160) {
        sdk::remove_storage(&address_to_key(KeyPrefix::Balance, address))
    }

    pub fn get_balance(address: &H160) -> U256 {
        sdk::read_storage(&address_to_key(KeyPrefix::Balance, address))
            .map(|value| U256::from_big_endian(&value))
            .unwrap_or_else(U256::zero)
    }

    pub fn remove_storage(address: &H160, key: &H256) {
        sdk::remove_storage(&storage_to_key(address, key));
    }

    pub fn set_storage(address: &H160, key: &H256, value: &H256) {
        sdk::write_storage(&storage_to_key(address, key), &value.0);
    }

    pub fn get_storage(address: &H160, key: &H256) -> H256 {
        sdk::read_storage(&storage_to_key(address, key))
            .map(|value| H256::from_slice(&value))
            .unwrap_or_else(H256::default)
    }

    pub fn is_account_empty(address: &H160) -> bool {
        let balance = Self::get_balance(address);
        let nonce = Self::get_nonce(address);
        let code_len = Self::get_code(address).len();
        balance == U256::zero() && nonce == U256::zero() && code_len == 0
    }

    /// Removes all storage for given address.
    pub fn remove_all_storage(_address: &H160) {
        // TODO: remove storage prefix.
        // Currently there is no way to prefix delete from trie state.
    }

    // Remove an account if its empty.
    pub fn remove_account_if_empty(address: &H160) {
        if Self::is_account_empty(address) {
            Self::remove_account(address);
        }
    }

    /// Removes all the account information.
    pub fn remove_account(address: &H160) {
        Self::remove_nonce(address);
        Self::remove_balance(address);
        Self::remove_code(address);
        Self::remove_all_storage(address);
    }
}

impl crate::backend::Backend for Backend {
    fn gas_left(&self) -> U256 {
        U256::max_value()
    }

    fn gas_price(&self) -> U256 {
        U256::zero()
    }

    fn origin(&self) -> H160 {
        self.origin
    }

    fn block_hash(&self, _number: U256) -> H256 {
        // There is no access to block hashes from runtime.
        H256::zero()
    }

    fn block_number(&self) -> U256 {
        U256::from(sdk::block_index())
    }

    fn block_coinbase(&self) -> H160 {
        H160::zero()
    }

    fn block_timestamp(&self) -> U256 {
        U256::from(sdk::block_timestamp())
    }

    fn block_difficulty(&self) -> U256 {
        U256::zero()
    }

    fn block_gas_limit(&self) -> U256 {
        U256::zero()
    }

    fn chain_id(&self) -> U256 {
        self.chain_id
    }

    fn exists(&self, address: H160) -> bool {
        Backend::get_balance(&address) > U256::zero() || Backend::get_code(&address).len() > 0
    }

    fn basic(&self, address: H160) -> Basic {
        Basic {
            nonce: Backend::get_nonce(&address),
            balance: Backend::get_balance(&address),
        }
    }

    fn code_hash(&self, address: H160) -> H256 {
        crate::types::keccak(&Backend::get_code(&address))
    }

    fn code_size(&self, address: H160) -> usize {
        Backend::get_code(&address).len()
    }

    fn code(&self, address: H160) -> Vec<u8> {
        Backend::get_code(&address)
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        Backend::get_storage(&address, &index)
    }
}

impl ApplyBackend for Backend {
    fn apply<A, I, L>(&mut self, values: A, logs: L, delete_empty: bool)
    where
        A: IntoIterator<Item = Apply<I>>,
        I: IntoIterator<Item = (H256, H256)>,
        L: IntoIterator<Item = Log>,
    {
        for apply in values {
            match apply {
                Apply::Modify {
                    address,
                    basic,
                    code,
                    storage,
                    reset_storage,
                } => {
                    Backend::set_nonce(&address, &basic.nonce);
                    Backend::set_balance(&address, &basic.balance);
                    if let Some(code) = code {
                        Backend::set_code(&address, &code);
                    }

                    if reset_storage {
                        Backend::remove_all_storage(&address);
                    }

                    for (index, value) in storage {
                        if value == H256::default() {
                            Backend::remove_storage(&address, &index);
                        } else {
                            Backend::set_storage(&address, &index, &value);
                        }
                    }

                    if delete_empty {
                        Backend::remove_account_if_empty(&address);
                    }
                }
                Apply::Delete { address } => Backend::remove_account(&address),
            }
        }

        for log in logs {
            sdk::log_utf8(&bytes_to_hex(&log_to_bytes(log)).into_bytes());
        }
    }
}
