#[cfg(not(feature = "std"))]
use alloc::{collections::BTreeMap, vec::Vec};
#[cfg(feature = "std")]
use std::{collections::BTreeMap, vec::Vec};

use crate::backend::{Apply, Basic};
use primitive_types::{H160, H256, U256};

use crate::sdk;
use crate::types::{address_to_key, storage_to_key, u256_to_arr, KeyPrefix};

pub struct Backend {
    origin: H160,
}

impl Backend {
    pub fn new(origin: H160) -> Self {
        Self { origin }
    }

    pub fn set_code(address: &H160, code: &[u8]) {
        sdk::write_storage(&address_to_key(KeyPrefix::Code, address), code);
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

    pub fn get_balance(address: &H160) -> U256 {
        sdk::read_storage(&address_to_key(KeyPrefix::Balance, address))
            .map(|value| U256::from_big_endian(&value))
            .unwrap_or_else(U256::zero)
    }

    pub fn set_storage(address: &H160, key: &H256, value: &H256) {
        sdk::write_storage(&storage_to_key(address, key), &value.0);
    }

    pub fn get_storage(address: &H160, key: &H256) -> H256 {
        sdk::read_storage(&storage_to_key(address, key))
            .map(|value| H256::from_slice(&value))
            .unwrap_or_else(H256::default)
    }

    pub fn remove_account(address: &H160) {}
}

impl crate::backend::Backend for Backend {
    fn gas_left(&self) -> U256 {
        unimplemented!()
    }

    fn gas_price(&self) -> U256 {
        unimplemented!()
    }

    fn origin(&self) -> H160 {
        self.origin
    }

    fn block_hash(&self, number: U256) -> H256 {
        unimplemented!()
    }

    fn block_number(&self) -> U256 {
        unimplemented!()
    }

    fn block_coinbase(&self) -> H160 {
        unimplemented!()
    }

    fn block_timestamp(&self) -> U256 {
        unimplemented!()
    }

    fn block_difficulty(&self) -> U256 {
        unimplemented!()
    }

    fn block_gas_limit(&self) -> U256 {
        unimplemented!()
    }

    fn chain_id(&self) -> U256 {
        // TODO:!!
        U256::zero()
    }

    fn exists(&self, address: H160) -> bool {
        unimplemented!()
    }

    fn basic(&self, address: H160) -> Basic {
        Basic {
            nonce: Backend::get_nonce(&address),
            balance: Backend::get_balance(&address),
        }
    }

    fn code_hash(&self, address: H160) -> H256 {
        unimplemented!()
    }

    fn code_size(&self, address: H160) -> usize {
        unimplemented!()
    }

    fn code(&self, address: H160) -> Vec<u8> {
        Backend::get_code(&address)
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        Backend::get_storage(&address, &index)
    }
}

impl crate::runner::BackendApply for Backend {
    fn apply(
        &mut self,
        values: Vec<Apply<BTreeMap<H256, H256>>>,
        logs: Vec<crate::backend::Log>,
        delete_empty: bool,
    ) {
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
                        // TODO: remove storage prefix.
                    }

                    for (index, value) in storage {
                        if value == H256::default() {
                            // TODO: remove
                        } else {
                            Backend::set_storage(&address, &index, &value);
                        }
                    }

                    if delete_empty {
                        // TODO: remove account if empty
                    }
                }
                Apply::Delete { address } => Backend::remove_account(&address),
            }
        }

        for log in logs {
            // TODO: deal with logs
        }
    }
}
