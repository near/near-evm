#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

use borsh::BorshDeserialize;
use primitive_types::{H160, U256};

use crate::backend::{ApplyBackend, Backend};
use crate::precompiles::precompiles;
use crate::runtime::{Config, CreateScheme, ExitReason};
use crate::stack::StackExecutor;
use crate::types::{FunctionCallArgs, ViewCallArgs};

pub struct Runner {}

impl Runner {
    pub fn execute<B, F, R>(
        backend: &mut B,
        _value: U256,
        should_commit: bool,
        f: F,
    ) -> (ExitReason, R)
    where
        B: ApplyBackend + Backend,
        F: FnOnce(&mut StackExecutor<B>) -> (ExitReason, R),
    {
        let config = Config::istanbul();
        #[cfg(feature = "external_machine")]
        let machine = crate::runtime::evm_machine::SdkMachine {};
        #[cfg(not(feature = "external_machine"))]
        let machine = crate::runtime::evm_machine::EmbeddedMachine::new();
        let mut executor =
            StackExecutor::new_with_precompile(backend, &machine, &config, precompiles);
        let (reason, return_value) = f(&mut executor);
        let (values, logs) = executor.deconstruct();
        if should_commit {
            backend.apply(values, logs, true);
        }
        (reason, return_value)
    }

    pub fn deploy_code<B>(backend: &mut B, input: &[u8]) -> (ExitReason, H160)
    where
        B: ApplyBackend + Backend,
    {
        let origin = backend.origin();
        let value = U256::zero();
        Self::execute(backend, value, true, |executor| {
            let address = executor.create_address(CreateScheme::Legacy { caller: origin });
            (
                executor.transact_create(origin, value, Vec::from(input)),
                address,
            )
        })
    }

    pub fn call<B>(backend: &mut B, input: &[u8]) -> (ExitReason, Vec<u8>)
    where
        B: ApplyBackend + Backend,
    {
        let args = FunctionCallArgs::try_from_slice(&input).unwrap();
        let origin = backend.origin();
        let value = U256::zero();
        Self::execute(backend, value, true, |executor| {
            executor.transact_call(origin, H160(args.contract), value, args.input)
        })
    }

    pub fn view<B>(backend: &mut B, input: &[u8]) -> (ExitReason, Vec<u8>)
    where
        B: ApplyBackend + Backend,
    {
        let args = ViewCallArgs::try_from_slice(&input).unwrap();
        let value = U256::from_big_endian(&args.amount);
        Self::execute(backend, value, false, |executor| {
            executor.transact_call(
                H160::from_slice(&args.sender),
                H160::from_slice(&args.address),
                value,
                args.input,
            )
        })
    }
}
