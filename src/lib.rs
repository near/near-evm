#![feature(num_as_ne_bytes)]
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

use evm_core::ToStr;

#[cfg(feature = "contract")]
mod contract {
    use borsh::BorshDeserialize;

    use crate::near_backend::Backend;

    use super::*;
    use crate::evm_core::ExitReason;
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

    fn process_exit_reason(reason: ExitReason, return_value: &[u8]) {
        match reason {
            ExitReason::Succeed(_) => sdk::return_output(return_value),
            ExitReason::Revert(_) => sdk::panic_hex(&return_value),
            ExitReason::Error(error) => sdk::panic_utf8(error.to_str().as_bytes()),
            ExitReason::Fatal(error) => sdk::panic_utf8(error.to_str().as_bytes()),
        }
    }

    fn handle_storage<F>(f: F)
    where
        F: FnOnce() -> (),
    {
        let initial_storage = sdk::storage_usage();
        f();
        let final_storage = sdk::storage_usage();
        if final_storage > initial_storage {
            if sdk::attached_deposit()
                < ((final_storage - initial_storage) as u128) * sdk::storage_byte_cost()
            {
                // TODO: add how much storage needed to error?
                sdk::panic_utf8(b"LackBalanceForStorage");
            }
        } else {
            // TODO: return funds to caller?
        }
    }

    /// Sets the parameters for the EVM contract.
    /// Should be called on deployment.  
    #[no_mangle]
    pub extern "C" fn new() {
        let input = sdk::read_input();
        Backend::set_owner(&input);
    }

    #[no_mangle]
    pub extern "C" fn set_owner() {
        assert_eq!(
            Backend::get_owner(),
            sdk::predecessor_account_id(),
            "Must be owner"
        );
        let input = sdk::read_input();
        Backend::set_owner(&input);
    }

    const CODE_KEY: &[u8; 4] = b"CODE";
    const CODE_TIMEFRAME_KEY: &[u8; 4] = b"STGE";
    const UPGRADE_DURATION: u64 = 24 * 60 * 60 * 1_000_000_000;

    #[no_mangle]
    pub extern "C" fn get_owner() {
        sdk::return_output(&Backend::get_owner())
    }

    /// Stage new code for deployment.
    #[no_mangle]
    pub extern "C" fn stage_upgrade() {
        assert_eq!(
            Backend::get_owner(),
            sdk::predecessor_account_id(),
            "Must be owner"
        );
        let input = sdk::read_input();
        sdk::write_storage(CODE_KEY, &input);
        sdk::write_storage(CODE_TIMEFRAME_KEY, &sdk::block_timestamp().to_le_bytes());
    }

    /// Deploy staged upgrade.
    #[no_mangle]
    pub extern "C" fn deploy_upgrade() {
        let timestamp = sdk::read_u64(CODE_TIMEFRAME_KEY).unwrap();
        if sdk::block_timestamp() <= timestamp + UPGRADE_DURATION {
            sdk::panic_utf8(b"Too early for upgrade");
        }
        let code = sdk::read_storage(CODE_KEY).unwrap();
        if code.is_empty() {
            sdk::panic_utf8(b"No staged code");
        }
        sdk::write_storage(CODE_KEY, &[]);
        sdk::self_deploy(&code);
    }

    /// Deploy new EVM bytecode and returns the address.
    #[no_mangle]
    pub extern "C" fn deploy_code() {
        handle_storage(|| {
            let input = sdk::read_input();
            let mut backend = Backend::new(CHAIN_ID, predecessor_address());
            let (reason, return_value) = runner::Runner::deploy_code(&mut backend, &input);
            process_exit_reason(reason, &return_value.0);
        });
    }

    #[no_mangle]
    pub extern "C" fn call() {
        handle_storage(|| {
            let input = sdk::read_input();
            let mut backend = Backend::new(CHAIN_ID, predecessor_address());
            let (reason, return_value) = runner::Runner::call(&mut backend, &input);
            process_exit_reason(reason, &return_value);
        });
    }

    // TODO: raw_call

    // TODO: meta_call

    #[no_mangle]
    pub extern "C" fn view() {
        let input = sdk::read_input();
        let args = crate::types::ViewCallArgs::try_from_slice(&input).unwrap();
        let mut backend = Backend::new(CHAIN_ID, H160::from_slice(&args.sender));
        let (reason, return_value) = runner::Runner::view(&mut backend, args);
        process_exit_reason(reason, &return_value);
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
