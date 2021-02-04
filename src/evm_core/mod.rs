//! Core layer for EVM.

// #![deny(warnings)]
// #![forbid(unsafe_code, missing_docs, unused_variables, unused_imports)]
// #![cfg_attr(not(feature = "std"), no_std)]

// extern crate alloc;
// extern crate core;

mod error;
mod eval;
mod memory;
mod opcode;
mod stack;
mod utils;
mod valids;

pub use crate::evm_core::error::{
    Capture, ExitError, ExitFatal, ExitReason, ExitRevert, ExitSucceed, Trap,
};
pub use crate::evm_core::memory::Memory;
pub use crate::evm_core::opcode::{ExternalOpcode, Opcode};
pub use crate::evm_core::stack::Stack;
pub use crate::evm_core::valids::Valids;

use crate::evm_core::eval::{eval, Control};
#[cfg(not(feature = "std"))]
use alloc::{rc::Rc, vec::Vec};

#[cfg(feature = "std")]
use std::{rc::Rc, vec::Vec};

use core::ops::Range;
use primitive_types::U256;

/// Core execution layer for EVM.
pub struct Machine {
    /// Program data.
    data: Rc<Vec<u8>>,
    /// Program code.
    code: Rc<Vec<u8>>,
    /// Program counter.
    position: Result<usize, ExitReason>,
    /// Return value.
    return_range: Range<U256>,
    /// Code validity maps.
    valids: Valids,
    /// Memory.
    memory: Memory,
    /// Stack.
    stack: Stack,
}

impl Machine {
    /// Reference of machine stack.
    #[allow(unused)]
    pub fn stack(&self) -> &Stack {
        &self.stack
    }
    /// Mutable reference of machine stack.
    pub fn stack_mut(&mut self) -> &mut Stack {
        &mut self.stack
    }
    /// Reference of machine memory.
    pub fn memory(&self) -> &Memory {
        &self.memory
    }
    /// Mutable reference of machine memory.
    pub fn memory_mut(&mut self) -> &mut Memory {
        &mut self.memory
    }

    /// Create a new machine with given code and data.
    pub fn new(
        code: Rc<Vec<u8>>,
        data: Rc<Vec<u8>>,
        stack_limit: usize,
        memory_limit: usize,
    ) -> Self {
        let valids = Valids::new(&code[..]);

        Self {
            data,
            code,
            position: Ok(0),
            return_range: U256::zero()..U256::zero(),
            valids,
            memory: Memory::new(memory_limit),
            stack: Stack::new(stack_limit),
        }
    }

    /// Explict exit of the machine. Further step will return error.
    pub fn exit(&mut self, reason: ExitReason) {
        self.position = Err(reason);
    }

    /// Inspect the machine's next opcode and current stack.
    #[allow(unused)]
    pub fn inspect(&self) -> Option<(Result<Opcode, ExternalOpcode>, &Stack)> {
        let position = match self.position {
            Ok(position) => position,
            Err(_) => return None,
        };
        self.code
            .get(position)
            .map(|v| (Opcode::parse(*v), &self.stack))
    }

    /// Copy and get the return value of the machine, if any.
    pub fn return_value(&self) -> Vec<u8> {
        println!(
            "Return value: {}-{}",
            self.return_range.start, self.return_range.end
        );
        if self.return_range.start > U256::from(usize::max_value()) {
            let mut ret = Vec::new();
            ret.resize(
                (self.return_range.end - self.return_range.start).as_usize(),
                0,
            );
            ret
        } else if self.return_range.end > U256::from(usize::max_value()) {
            let mut ret = self.memory.get(
                self.return_range.start.as_usize(),
                usize::max_value() - self.return_range.start.as_usize(),
            );
            while ret.len() < (self.return_range.end - self.return_range.start).as_usize() {
                ret.push(0);
            }
            ret
        } else {
            self.memory.get(
                self.return_range.start.as_usize(),
                (self.return_range.end - self.return_range.start).as_usize(),
            )
        }
    }

    /// Loop stepping the machine, until it stops.
    #[allow(unused)]
    pub fn run(&mut self) -> Capture<ExitReason, Trap> {
        loop {
            match self.step() {
                Ok(()) => (),
                Err(res) => return res,
            }
        }
    }

    /// Step the machine, executing one opcode. It then returns.
    pub fn step(&mut self) -> Result<(), Capture<ExitReason, Trap>> {
        let position = *self
            .position
            .as_ref()
            .map_err(|reason| Capture::Exit(reason.clone()))?;

        match self.code.get(position).map(|v| Opcode::parse(*v)) {
            Some(Ok(opcode)) => match eval(self, opcode, position) {
                Control::Continue(p) => {
                    self.position = Ok(position + p);
                    Ok(())
                }
                Control::Exit(e) => {
                    self.position = Err(e.clone());
                    Err(Capture::Exit(e))
                }
                Control::Jump(p) => {
                    self.position = Ok(p);
                    Ok(())
                }
            },
            Some(Err(external)) => {
                self.position = Ok(position + 1);
                Err(Capture::Trap(external))
            }
            None => {
                self.position = Err(ExitSucceed::Stopped.into());
                Err(Capture::Exit(ExitSucceed::Stopped.into()))
            }
        }
    }
}
