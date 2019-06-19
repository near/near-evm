use std::collections::HashMap;
use std::sync::Mutex;

use lazy_static::lazy_static;

use crate::near_native::DATA_TYPE_INPUT;
use crate::near_native::DATA_TYPE_ORIGINATOR_ACCOUNT_ID;

lazy_static! {
    static ref STORAGE: Mutex<HashMap<Vec<u8>, Vec<u8>>> = Default::default();
    static ref INPUT: Mutex<Vec<u8>> = Default::default();
    static ref SENDER: Mutex<Vec<u8>> = Default::default();
    static ref RETURN_VALUE: Mutex<Vec<u8>> = Default::default();
}

pub fn set_input(input: Vec<u8>) {
    *INPUT.lock().unwrap() = input;
}

pub fn set_sender(sender: Vec<u8>) {
    *SENDER.lock().unwrap() = sender;
}

pub fn get_return_value() -> Vec<u8> {
    RETURN_VALUE.lock().unwrap().clone()
}

#[no_mangle]
pub unsafe fn storage_write(
    key_len: usize,
    key_ptr: *const u8,
    value_len: usize,
    value_ptr: *const u8,
) {
    let key = Vec::from(alloc::slice::from_raw_parts(key_ptr, key_len));
    let value = Vec::from(alloc::slice::from_raw_parts(value_ptr, value_len));
    STORAGE.lock().unwrap().insert(key, value);
}

#[no_mangle]
pub unsafe fn return_value(value_len: usize, value_ptr: *const u8) {
    *RETURN_VALUE.lock().unwrap() = Vec::from(alloc::slice::from_raw_parts(value_ptr, value_len));
}

#[no_mangle]
pub unsafe fn data_read(
    data_type_index: u32,
    key_len: usize,
    key_ptr: *const u8,
    max_buf_len: usize,
    buf_ptr: *mut u8,
) -> usize {
    let value = match data_type_index {
        DATA_TYPE_INPUT => INPUT.lock().unwrap().clone(),
        DATA_TYPE_ORIGINATOR_ACCOUNT_ID => SENDER.lock().unwrap().clone(),
        _ => {
            let key = Vec::from(alloc::slice::from_raw_parts(key_ptr, key_len));
            let value = STORAGE
                .lock()
                .unwrap()
                .get(&key)
                .cloned()
                .unwrap_or_else(|| vec![]);
            value
        }
    };
    if value.len() <= max_buf_len {
        std::ptr::copy_nonoverlapping(value.as_ptr(), buf_ptr, value.len());
    }
    value.len()
}

#[no_mangle]
pub fn assert(expr: bool) {
    if !expr {
        panic!()
    }
}

#[no_mangle]
pub fn debug(msg_len: usize, msg_ptr: *const u8) {
    unsafe {
        let s = String::from_utf8(Vec::from(alloc::slice::from_raw_parts(msg_ptr, msg_len)));
        println!("debug: {}", s.unwrap_or("unreadable".to_string()));
    }
}
