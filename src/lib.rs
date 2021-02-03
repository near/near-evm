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

const CHAIN_ID: u64 = 1;

#[cfg(feature = "contract")]
mod contract {
    use crate::near_backend::Backend;

    use super::*;
    use primitive_types::H160;

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

    #[no_mangle]
    pub extern "C" fn deploy_code() {
        let input = sdk::read_input();
        let mut backend = Backend::new(CHAIN_ID, H160::zero());
        let result = runner::Runner::deploy_code(&mut backend, &input);
        sdk::return_output(&result.0);
    }

    #[no_mangle]
    pub extern "C" fn get_code() {
        let address = sdk::read_input_arr20();
        let code = Backend::get_code(&H160(address));
        sdk::return_output(&code)
    }

    #[no_mangle]
    pub extern "C" fn call() {
        let input = sdk::read_input();
        let mut backend = Backend::new(CHAIN_ID, H160::zero());
        let result = runner::Runner::call(&mut backend, &input);
        sdk::return_output(&result);
    }

    #[no_mangle]
    pub extern "C" fn view() {
        let input = sdk::read_input();
        let mut backend = Backend::new(CHAIN_ID, H160::zero());
        let result = runner::Runner::view(&mut backend, &input);
        sdk::return_output(&result);
    }
}
