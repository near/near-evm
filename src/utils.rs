use ethereum_types::{Address, H256, U256};
use keccak_hash::keccak;
use vm::CreateContractAddress;

use near_bindgen::env;

// TODO: clean up all these
// TODO: proper keccak[12..] for addresses

pub fn predecessor_as_eth() -> Address {
    near_account_id_to_eth_address(&env::predecessor_account_id())
}

pub fn predecessor_as_internal_address() -> [u8; 20] {
    near_account_id_to_internal_address(&env::predecessor_account_id())
}

pub fn prefix_for_contract_storage(contract_address: &[u8]) -> Vec<u8> {
    let mut prefix = Vec::new();
    prefix.extend_from_slice(b"_storage");
    prefix.extend_from_slice(contract_address);
    prefix
}

pub fn eth_account_to_internal_address(addr: Address) -> [u8; 20] {
    let mut bin = [0u8; 20];
    bin.copy_from_slice(&addr[..]);
    bin
}

pub fn near_account_bytes_to_eth_address(addr: &Vec<u8>) -> Address {
    Address::from_slice(&keccak(addr)[12..])
}

pub fn near_account_id_to_eth_address(account_id: &str) -> Address {
    near_account_bytes_to_eth_address(&account_id.to_string().into_bytes())
}

pub fn near_account_id_to_internal_address(account_id: &str) -> [u8; 20] {
    eth_account_to_internal_address(near_account_id_to_eth_address(account_id))
}

/// Returns new address created from address, nonce, and code hash
/// Copied directly from the parity codebase
pub fn evm_contract_address(
    address_scheme: CreateContractAddress,
    sender: &Address,
    nonce: &U256,
    code: &[u8],
) -> (Address, Option<H256>) {
    use rlp::RlpStream;

    match address_scheme {
        CreateContractAddress::FromSenderAndNonce => {
            let mut stream = RlpStream::new_list(2);
            stream.append(sender);
            stream.append(nonce);
            (From::from(keccak(stream.as_raw())), None)
        }
        CreateContractAddress::FromSenderSaltAndCodeHash(salt) => {
            let code_hash = keccak(code);
            let mut buffer = [0u8; 1 + 20 + 32 + 32];
            buffer[0] = 0xff;
            &mut buffer[1..(1 + 20)].copy_from_slice(&sender[..]);
            &mut buffer[(1 + 20)..(1 + 20 + 32)].copy_from_slice(&salt[..]);
            &mut buffer[(1 + 20 + 32)..].copy_from_slice(&code_hash[..]);
            (From::from(keccak(&buffer[..])), Some(code_hash))
        }
        CreateContractAddress::FromSenderAndCodeHash => {
            let code_hash = keccak(code);
            let mut buffer = [0u8; 20 + 32];
            &mut buffer[..20].copy_from_slice(&sender[..]);
            &mut buffer[20..].copy_from_slice(&code_hash[..]);
            (From::from(keccak(&buffer[..])), Some(code_hash))
        }
    }
}
