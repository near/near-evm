use ethereum_types::{Address, H256, U256};
use vm::CreateContractAddress;
use keccak_hash::keccak;

use near_bindgen::env;

pub fn sender_as_eth() -> Address {
    let mut sender = env::signer_account_id().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender)
}

pub fn prefix_for_contract_storage(contract_address: &[u8]) -> Vec<u8> {
    let mut prefix = Vec::new();
    prefix.extend_from_slice(b"_storage");
    prefix.extend_from_slice(contract_address);
    prefix
}

pub fn sender_name_to_eth_address(sender: &str) -> Address {
    let mut sender = sender.to_string().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender[0..20])
}

pub fn eth_account_to_internal_address(addr: Address) -> Vec<u8> {
    addr.0.to_vec()
}

pub fn internal_address_to_eth_account(addr: &Vec<u8>) -> Address {
    let mut addr = addr.clone();
    addr.resize(20, 0);
    Address::from_slice(&addr)
}

pub fn sender_name_to_internal_address(sender: &str) -> Vec<u8> {
    eth_account_to_internal_address(sender_name_to_eth_address(sender))
}

/// Returns new address created from address, nonce, and code hash
/// Copied directly from the parity codebase
pub fn evm_contract_address(address_scheme: CreateContractAddress, sender: &Address, nonce: &U256, code: &[u8]) -> (Address, Option<H256>) {
	use rlp::RlpStream;

	match address_scheme {
		CreateContractAddress::FromSenderAndNonce => {
			let mut stream = RlpStream::new_list(2);
			stream.append(sender);
			stream.append(nonce);
			(From::from(keccak(stream.as_raw())), None)
		},
		CreateContractAddress::FromSenderSaltAndCodeHash(salt) => {
			let code_hash = keccak(code);
			let mut buffer = [0u8; 1 + 20 + 32 + 32];
			buffer[0] = 0xff;
			&mut buffer[1..(1+20)].copy_from_slice(&sender[..]);
			&mut buffer[(1+20)..(1+20+32)].copy_from_slice(&salt[..]);
			&mut buffer[(1+20+32)..].copy_from_slice(&code_hash[..]);
			(From::from(keccak(&buffer[..])), Some(code_hash))
		},
		CreateContractAddress::FromSenderAndCodeHash => {
			let code_hash = keccak(code);
			let mut buffer = [0u8; 20 + 32];
			&mut buffer[..20].copy_from_slice(&sender[..]);
			&mut buffer[20..].copy_from_slice(&code_hash[..]);
			(From::from(keccak(&buffer[..])), Some(code_hash))
		},
	}
}
