use crate::evm_core::{Capture, ExitError, ExitFatal, ExitReason, Trap};

#[cfg(feature = "std")]
use std::{rc::Rc, vec::Vec};

#[cfg(not(feature = "std"))]
use alloc::{rc::Rc, vec::Vec};
use primitive_types::{H256, U256};

#[cfg(feature = "external_evm_machine")]
mod sdk {
    use super::*;
    use alloc::vec;

    extern "C" {}

    pub fn init_evm_machine(code: Rc<Vec<u8>>, data: Rc<Vec<u8>>) {}
    pub fn evm_machine_step() -> Result<(), Capture<ExitReason, Trap>> {
        Ok(())
    }
    pub fn evm_machine_exit(exit: ExitReason) {}
    pub fn evm_machine_return_value() -> Vec<u8> {
        vec![]
    }
    pub fn evm_machine_stack_push(value: H256) -> Result<(), ExitError> {
        Ok(())
    }
    pub fn evm_machine_stack_pop() -> Result<H256, ExitError> {
        Ok(H256::default())
    }
    pub fn evm_machine_copy(
        memory_offset: U256,
        data_offset: U256,
        len: U256,
        data: &[u8],
    ) -> Result<(), ExitFatal> {
        Ok(())
    }
    pub fn evm_machine_get(offset: usize, size: usize) -> Vec<u8> {
        vec![]
    }
    pub fn evm_machine_resize(offset: U256, len: U256) -> Result<(), ExitError> {
        Ok(())
    }
}

#[cfg(feature = "external_evm_machine")]
pub use sdk::*;

#[cfg(not(feature = "external_machine"))]
mod embedded {
    use crate::evm_core::Machine;

    use super::*;

    static mut MACHINE: Option<Machine> = None;

    #[inline]
    pub fn init_evm_machine(code: Rc<Vec<u8>>, data: Rc<Vec<u8>>) {
        unsafe {
            MACHINE = Some(Machine::new(code, data, 1024, usize::MAX));
        }
    }

    #[inline]
    pub fn evm_machine_step() -> Result<(), Capture<ExitReason, Trap>> {
        unsafe {
            match MACHINE {
                Some(ref mut x) => x.step(),
                None => panic!(),
            }
        }
    }

    pub fn evm_machine_exit(exit: ExitReason) {
        unsafe {
            match MACHINE {
                Some(ref mut x) => x.exit(exit),
                None => panic!(),
            }
        }
    }

    #[inline]
    pub fn evm_machine_return_value() -> Vec<u8> {
        unsafe {
            match MACHINE {
                Some(ref x) => x.return_value(),
                None => panic!(),
            }
        }
    }

    #[inline]
    pub fn evm_machine_stack_push(value: H256) -> Result<(), ExitError> {
        unsafe {
            match MACHINE {
                Some(ref mut x) => x.stack_mut().push(value),
                None => panic!(),
            }
        }
    }

    #[inline]
    pub fn evm_machine_stack_pop() -> Result<H256, ExitError> {
        unsafe {
            match MACHINE {
                Some(ref mut x) => x.stack_mut().pop(),
                None => panic!(),
            }
        }
    }

    #[inline]
    pub fn evm_machine_copy(
        memory_offset: U256,
        data_offset: U256,
        len: U256,
        data: &[u8],
    ) -> Result<(), ExitFatal> {
        unsafe {
            match MACHINE {
                Some(ref mut x) => x
                    .memory_mut()
                    .copy_large(memory_offset, data_offset, len, data),
                None => panic!(),
            }
        }
    }

    #[inline]
    pub fn evm_machine_get(offset: usize, size: usize) -> Vec<u8> {
        unsafe {
            match MACHINE {
                Some(ref x) => x.memory().get(offset, size),
                None => panic!(),
            }
        }
    }

    #[inline]
    pub fn evm_machine_resize(offset: U256, len: U256) -> Result<(), ExitError> {
        unsafe {
            match MACHINE {
                Some(ref mut x) => x.memory_mut().resize_offset(offset, len),
                None => panic!(),
            }
        }
    }

}

#[cfg(not(feature = "external_evm_machine"))]
pub use embedded::*;
