#[cfg(not(feature = "std"))]
use alloc::{rc::Rc, vec, vec::Vec};
#[cfg(feature = "std")]
use std::{rc::Rc, vec, vec::Vec};

use primitive_types::{H256, U256};

#[cfg(not(feature = "external_evm_machine"))]
pub use embedded::*;
#[cfg(feature = "external_evm_machine")]
pub use sdk::*;

use crate::evm_core::{Capture, ExitError, ExitFatal, ExitReason, Trap};

pub trait Machine {
    fn push_evm_machine(&self, code: Rc<Vec<u8>>, data: Rc<Vec<u8>>);
    fn pop_evm_machine(&self);
    fn step(&self) -> Result<(), Capture<ExitReason, Trap>>;
    fn exit(&self, exit: ExitReason);
    fn return_value(&self) -> Vec<u8>;
    fn stack_push(&self, value: H256) -> Result<(), ExitError>;
    fn stack_pop(&self) -> Result<H256, ExitError>;
    fn memory_copy(
        &self,
        memory_offset: U256,
        data_offset: U256,
        len: U256,
        data: &[u8],
    ) -> Result<(), ExitFatal>;
    fn memory_get(&self, offset: usize, size: usize) -> Vec<u8>;
    fn memory_resize(&self, offset: U256, len: U256) -> Result<(), ExitError>;
}

#[cfg(feature = "external_evm_machine")]
mod sdk {
    use super::*;

    extern "C" {}

    pub struct SdkMachine {}

    impl Machine for SdkMachine {
        fn push_evm_machine(&self, code: Rc<Vec<u8>>, data: Rc<Vec<u8>>) {}
        fn pop_evm_machine(&self) {}
        fn step(&self) -> Result<(), Capture<ExitReason, Trap>> {
            Ok(())
        }
        fn exit(&self, exit: ExitReason) {}
        fn return_value(&self) -> Vec<u8> {
            vec![]
        }
        fn stack_push(&self, value: H256) -> Result<(), ExitError> {
            Ok(())
        }
        fn stack_pop(&self) -> Result<H256, ExitError> {
            Ok(H256::default())
        }
        fn memory_copy(
            &self,
            memory_offset: U256,
            data_offset: U256,
            len: U256,
            data: &[u8],
        ) -> Result<(), ExitFatal> {
            Ok(())
        }
        fn memory_get(&self, offset: usize, size: usize) -> Vec<u8> {
            vec![]
        }
        fn memory_resize(&self, offset: U256, len: U256) -> Result<(), ExitError> {
            Ok(())
        }
    }
}

#[cfg(not(feature = "external_machine"))]
mod embedded {
    use super::*;

    #[cfg(not(feature = "std"))]
    use core::cell::RefCell;
    #[cfg(feature = "std")]
    use std::cell::RefCell;

    pub struct EmbeddedMachine {
        machines: RefCell<Vec<crate::evm_core::Machine>>,
    }

    impl EmbeddedMachine {
        pub fn new() -> Self {
            EmbeddedMachine {
                machines: RefCell::new(vec![]),
            }
        }
    }

    impl Machine for EmbeddedMachine {
        #[inline]
        fn push_evm_machine(&self, code: Rc<Vec<u8>>, data: Rc<Vec<u8>>) {
            self.machines
                .borrow_mut()
                .push(crate::evm_core::Machine::new(code, data, 1024, usize::MAX));
        }

        #[inline]
        fn pop_evm_machine(&self) {
            self.machines.borrow_mut().pop();
        }

        #[inline]
        fn step(&self) -> Result<(), Capture<ExitReason, Trap>> {
            match self.machines.borrow_mut().last_mut() {
                Some(ref mut x) => x.step(),
                None => panic!(),
            }
        }

        fn exit(&self, exit: ExitReason) {
            match self.machines.borrow_mut().last_mut() {
                Some(ref mut x) => x.exit(exit),
                None => panic!(),
            }
        }

        #[inline]
        fn return_value(&self) -> Vec<u8> {
            match self.machines.borrow_mut().last_mut() {
                Some(ref x) => x.return_value(),
                None => panic!(),
            }
        }

        #[inline]
        fn stack_push(&self, value: H256) -> Result<(), ExitError> {
            match self.machines.borrow_mut().last_mut() {
                Some(ref mut x) => x.stack_mut().push(value),
                None => panic!(),
            }
        }

        #[inline]
        fn stack_pop(&self) -> Result<H256, ExitError> {
            match self.machines.borrow_mut().last_mut() {
                Some(ref mut x) => x.stack_mut().pop(),
                None => panic!(),
            }
        }

        #[inline]
        fn memory_copy(
            &self,
            memory_offset: U256,
            data_offset: U256,
            len: U256,
            data: &[u8],
        ) -> Result<(), ExitFatal> {
            match self.machines.borrow_mut().last_mut() {
                Some(ref mut x) => x
                    .memory_mut()
                    .copy_large(memory_offset, data_offset, len, data),
                None => panic!(),
            }
        }

        #[inline]
        fn memory_get(&self, offset: usize, size: usize) -> Vec<u8> {
            match self.machines.borrow_mut().last_mut() {
                Some(ref x) => x.memory().get(offset, size),
                None => panic!(),
            }
        }

        #[inline]
        fn memory_resize(&self, offset: U256, len: U256) -> Result<(), ExitError> {
            match self.machines.borrow_mut().last_mut() {
                Some(ref mut x) => x.memory_mut().resize_offset(offset, len),
                None => panic!(),
            }
        }
    }
}
