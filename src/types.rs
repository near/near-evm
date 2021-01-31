#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

use borsh::{BorshDeserialize, BorshSerialize};
use primitive_types::{H160, H256, U256};

pub type RawAddress = [u8; 20];
pub type RawU256 = [u8; 32];

#[derive(BorshSerialize, BorshDeserialize)]
pub struct FunctionCallArgs {
    pub contract: RawAddress,
    pub input: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct ViewCallArgs {
    pub sender: RawAddress,
    pub address: RawAddress,
    pub amount: RawU256,
    pub input: Vec<u8>,
}

pub enum KeyPrefix {
    Code = 0x0,
    Balance = 0x1,
    Nonce = 0x2,
    Storage = 0x3,
}

pub fn address_to_key(prefix: KeyPrefix, address: &H160) -> [u8; 21] {
    let mut result = [0u8; 21];
    result[0] = prefix as u8;
    result[1..].copy_from_slice(&address.0);
    result
}

pub fn storage_to_key(address: &H160, key: &H256) -> [u8; 53] {
    let mut result = [0u8; 53];
    result[0] = KeyPrefix::Storage as u8;
    result[1..21].copy_from_slice(&address.0);
    result[21..].copy_from_slice(&key.0);
    result
}

pub fn u256_to_arr(value: &U256) -> [u8; 32] {
    let mut result = [0u8; 32];
    value.to_big_endian(&mut result);
    result
}
