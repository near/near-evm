use ethereum_types::{Address, H256, U256};
use keccak_hash::keccak;
use vm::CreateContractAddress;

use near_vm_logic::types::{Balance};

use near_bindgen::env;

pub fn predecessor_as_evm() -> Address {
    near_account_id_to_evm_address(&env::predecessor_account_id())
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

pub fn evm_account_to_internal_address(addr: Address) -> [u8; 20] {
    addr.0
}

pub fn near_account_bytes_to_evm_address(addr: &Vec<u8>) -> Address {
    Address::from_slice(&keccak(addr)[12..])
}

pub fn near_account_id_to_evm_address(account_id: &str) -> Address {
    near_account_bytes_to_evm_address(&account_id.to_string().into_bytes())
}

pub fn near_account_id_to_internal_address(account_id: &str) -> [u8; 20] {
    evm_account_to_internal_address(near_account_id_to_evm_address(account_id))
}

pub fn hex_to_evm_address(address: &str) -> Address {
    let addr = hex::decode(&address).unwrap();
    Address::from_slice(&addr)
}


pub fn attached_deposit_as_u256_opt() -> Option<U256> {
    let attached = env::attached_deposit();
    if attached == 0 {
        None
    } else {
        Some(balance_to_u256(&attached))
    }
}

pub fn balance_to_u256(val: &Balance) -> U256 {
    let mut bin = [0u8; 32];
    bin[16..].copy_from_slice(&val.to_be_bytes());
    bin.into()
}

pub fn u256_to_balance(val: &U256) -> Balance {
    let mut scratch = [0u8; 32];
    let mut bin = [0u8; 16];
    val.to_big_endian(&mut scratch);
    bin.copy_from_slice(&scratch[16..]);
    Balance::from_be_bytes(bin)
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
