#[cfg(not(feature = "std"))]
use alloc::{string::String, vec, vec::Vec};
#[cfg(feature = "std")]
use std::{string::String, vec, vec::Vec};

use borsh::{BorshDeserialize, BorshSerialize};
use primitive_types::{H160, H256, U256};

use crate::backend::Log;

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

pub fn log_to_bytes(log: Log) -> Vec<u8> {
    let mut result = vec![0u8; 1 + log.topics.len() * 32 + log.data.len()];
    result[0] = log.topics.len() as u8;
    let index = 1;
    for topic in log.topics.iter() {
        result[index..index + 32].copy_from_slice(&topic.0);
    }
    result[index..].copy_from_slice(&log.data);
    result
}

const HEX_ALPHABET: &[u8; 16] = b"0123456789abcdef";

pub fn bytes_to_hex(v: &[u8]) -> String {
    let mut result = String::new();
    for x in v {
        result.push(HEX_ALPHABET[(x % 16) as usize] as char);
        result.push(HEX_ALPHABET[(x / 16) as usize] as char);
    }
    result
}
