#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(core_intrinsics))]
#![cfg_attr(not(feature = "std"), feature(alloc_error_handler))]

#[cfg(not(feature = "std"))]
extern crate alloc;
#[cfg(not(feature = "std"))]
extern crate core;

pub mod backend;
mod evm_core;
mod precompiles;
pub mod runner;
mod runtime;
mod stack;
pub mod types;

#[cfg(feature = "contract")]
mod near_backend;
#[cfg(feature = "contract")]
mod sdk;

#[cfg(feature = "contract")]
mod contract {
    use borsh::BorshDeserialize;

    use crate::near_backend::Backend;

    use super::*;
    use crate::types::{near_account_to_evm_address, u256_to_arr, GetStorageAtArgs};
    use primitive_types::{H160, H256};

    // TODO: consider making a parameter, but migth cost extra.
    const CHAIN_ID: u64 = 1;

    #[global_allocator]
    static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

    #[panic_handler]
    #[no_mangle]
    pub unsafe fn on_panic(_info: &::core::panic::PanicInfo) -> ! {
        ::core::intrinsics::abort();
    }

    #[alloc_error_handler]
    #[no_mangle]
    pub unsafe fn on_alloc_error(_: core::alloc::Layout) -> ! {
        ::core::intrinsics::abort();
    }

    pub fn predecessor_address() -> H160 {
        near_account_to_evm_address(&sdk::predecessor_account_id())
    }

    #[no_mangle]
    pub extern "C" fn deploy_code() {
        let input = sdk::read_input();
        let mut backend = Backend::new(CHAIN_ID, predecessor_address());
        let result = runner::Runner::deploy_code(&mut backend, &input);
        sdk::return_output(&result.0);
    }

    #[no_mangle]
    pub extern "C" fn call() {
        let input = sdk::read_input();
        let mut backend = Backend::new(CHAIN_ID, predecessor_address());
        let result = runner::Runner::call(&mut backend, &input);
        sdk::return_output(&result);
    }

    // TODO: raw_call

    // TODO: meta_call

    #[no_mangle]
    pub extern "C" fn view() {
        let input = sdk::read_input();
        let mut backend = Backend::new(CHAIN_ID, H160::zero());
        let result = runner::Runner::view(&mut backend, &input);
        sdk::return_output(&result);
    }

    #[no_mangle]
    pub extern "C" fn get_code() {
        let address = sdk::read_input_arr20();
        let code = Backend::get_code(&H160(address));
        sdk::return_output(&code)
    }

    #[no_mangle]
    pub extern "C" fn get_balance() {
        let address = sdk::read_input_arr20();
        let balance = Backend::get_balance(&H160(address));
        sdk::return_output(&u256_to_arr(&balance))
    }

    #[no_mangle]
    pub extern "C" fn get_nonce() {
        let address = sdk::read_input_arr20();
        let nonce = Backend::get_nonce(&H160(address));
        sdk::return_output(&u256_to_arr(&nonce))
    }

    #[no_mangle]
    pub extern "C" fn get_storage_at() {
        let input = sdk::read_input();
        let args = GetStorageAtArgs::try_from_slice(&input).unwrap();
        let value = Backend::get_storage(&H160(args.address), &H256(args.key));
        sdk::return_output(&value.0)
    }
}
