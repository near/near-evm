#[cfg(test)]
extern crate alloc;
#[cfg(test)]
#[macro_use]
extern crate ethabi_derive;
extern crate ethereum_types;
extern crate evm;
extern crate parity_bytes;
extern crate vm;

use std::convert::TryInto;
use std::sync::Arc;

use ethereum_types::{Address, U256};
use evm::Factory;
use serde_derive::{Deserialize, Serialize};
use vm::{ActionParams, CallType, Ext, GasLeft, Schedule};

use fake_ext::FakeExt;

#[cfg(test)]
mod tests;

mod fake_ext;
mod near_native;

#[derive(Serialize, Deserialize)]
pub struct RunCommandInput {
    // address of the evm subcontract
    pub contract_address: String,
    // evm abi encoded input encoded as hex
    pub encoded_input: String,
}

#[derive(Serialize, Deserialize)]
pub struct DeployCodeInput {
    // address of the evm subcontract
    pub contract_address: String,
    // bytecode encoded as hex
    pub bytecode: String,
}

pub fn sender_name_to_eth_address(sender: &str) -> Address {
    let mut sender = sender.to_string().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender[0..20])
}

const CODE_PREFIX: &str = "_evm_code";
const ETH_BALANCE_PREFIX: &str = "_balance";

fn read(type_index: u32, key_len: usize, key: *const u8) -> Vec<u8> {
    let mut temp_buf: Vec<u8> = Vec::new();
    const MAX_BUF_SIZE: usize = 1 << 16;
    unsafe {
        if temp_buf.len() == 0 {
            temp_buf.resize(MAX_BUF_SIZE, 0);
        }
        let len = near_native::data_read(
            type_index,
            key_len,
            key,
            MAX_BUF_SIZE,
            temp_buf.as_mut_ptr(),
        );
        near_native::assert(len <= MAX_BUF_SIZE);
        temp_buf[..len as usize].to_vec()
    }
}

fn storage_read(key_len: usize, key: *const u8) -> Vec<u8> {
    read(near_native::DATA_TYPE_STORAGE, key_len, key)
}

fn input_read() -> Vec<u8> {
    read(near_native::DATA_TYPE_INPUT, 0, std::ptr::null())
}

fn sender() -> Vec<u8> {
    read(
        near_native::DATA_TYPE_ORIGINATOR_ACCOUNT_ID,
        0,
        std::ptr::null(),
    )
}

fn sender_as_eth() -> Address {
    let mut sender = sender();
    sender.resize(20, 0);
    Address::from_slice(&sender)
}

fn code_key(sender: String) -> Vec<u8> {
    let mut code_key = CODE_PREFIX.as_bytes().to_vec();
    code_key.append(&mut sender.into_bytes());
    code_key
}

fn code_for_contract(contract_address: String) -> Vec<u8> {
    let code_key = code_key(contract_address);
    storage_read(code_key.len(), code_key.as_ptr())
}

fn get_eth_balance(account: Address) -> u64 {
    let mut key = ETH_BALANCE_PREFIX.as_bytes().to_vec();
    key.append(&mut account.as_bytes().to_vec());
    let value = storage_read(key.len(), key.as_ptr());
    if value.len() != 8 {
        0
    } else {
        u64::from_le_bytes(value[0..8].try_into().unwrap())
    }
}

fn set_eth_balance(account: Address, balance: u64) {
    let mut key = ETH_BALANCE_PREFIX.as_bytes().to_vec();
    key.append(&mut account.as_bytes().to_vec());
    let value = u64::to_le_bytes(balance);
    unsafe {
        near_native::storage_write(key.len(), key.as_ptr(), 8, value.as_ptr());
    }
}

fn run_command_input() -> RunCommandInput {
    let input = input_read();
    let json_string = String::from_utf8(input).expect("invalid json");
    serde_json::from_str(&json_string).expect("invalid json")
}

fn deploy_code_input() -> DeployCodeInput {
    let input = input_read();
    let json_string = String::from_utf8(input).expect("invalid json");
    serde_json::from_str(&json_string).expect("invalid json")
}

#[no_mangle]
pub extern "C" fn deploy_code() {
    let deploy_code_input = deploy_code_input();
    let code = hex::decode(deploy_code_input.bytecode).expect("invalid hex");
    let code_address = deploy_code_input.contract_address.clone();
    let code_key = code_key(code_address);
    unsafe {
        near_native::storage_write(code_key.len(), code_key.as_ptr(), code.len(), code.as_ptr());
    }

    let run = RunCommandInput {
        contract_address: deploy_code_input.contract_address,
        encoded_input: "".to_string(),
    };

    if let Some(GasLeft::NeedsReturn { data, .. }) = run_command_internal(run) {
        unsafe {
            near_native::storage_write(
                code_key.len(),
                code_key.as_ptr(),
                data.len(),
                data.as_ptr(),
            );
        }
        debug(format!("ok deployed {} bytes of code", data.len()).as_str());
    } else {
        panic!("init failed");
    }
}

fn debug(debug_msg: &str) {
    unsafe {
        near_native::debug(debug_msg.len(), debug_msg.as_ptr());
    }
}

fn run_command_internal(run_command_input: RunCommandInput) -> Option<GasLeft> {
    let startgas = 1_000_000_000;
    let code = code_for_contract(run_command_input.contract_address);
    let input = run_command_input.encoded_input;
    let input = hex::decode(input).expect("invalid hex");

    let mut params = ActionParams::default();

    params.call_type = CallType::None;
    params.code = Some(Arc::new(code));
    params.sender = sender_as_eth();
    params.origin = params.sender;
    params.gas = U256::from(startgas);
    params.data = Some(input);

    let mut ext = FakeExt::default();
    ext.info.gas_limit = U256::from(startgas);

    ext.schedule = Schedule::new_constantinople();

    let instance = Factory::default().create(params, ext.schedule(), ext.depth());

    let result = instance.exec(&mut ext);
    result.ok().unwrap().ok()
}

#[no_mangle]
pub extern "C" fn run_command() {
    let run_command_input = run_command_input();
    let result = run_command_internal(run_command_input);
    match result.unwrap() {
        GasLeft::NeedsReturn {
                  gas_left: _,
                  data,
                  apply_state: _,
              } => {
            unsafe { near_native::return_value(data.len(), data.as_ptr()) };
        }
        GasLeft::Known(_gas_left) => {
            // no return value
        }
    }
}
