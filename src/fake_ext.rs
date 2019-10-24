use std::sync::Arc;

use ethereum_types::{Address, H256, U256};
use parity_bytes::Bytes;
use vm::{
    CallType, ContractCreateResult, CreateContractAddress, EnvInfo, MessageCallResult, Result,
    ReturnData, Schedule, TrapKind,
};
use near_bindgen::collections::Map as NearMap;

use super::{ sender_as_eth};
use crate::EvmContract;
use near_bindgen::env;

pub struct FakeExt<'a> {
    pub info: EnvInfo,
    pub schedule: Schedule,
    pub selfdestruct_address: Option<Address>,
    pub contract: &'a EvmContract,
    pub storage: NearMap<Vec<u8>, Vec<u8>>,
}

impl<'a> FakeExt<'a> {
    pub fn new(storage: NearMap<Vec<u8>, Vec<u8>>, contract: &'a EvmContract) -> Self {
        Self {
            storage,
            contract,
            info: Default::default(),
            schedule: Default::default(),
            selfdestruct_address: Default::default(),
        }
    }
}

fn not_implemented(name: &str) {
    env::log(format!("not implemented: {}", name).as_bytes());
}

impl<'a> vm::Ext for FakeExt<'a> {
    /// Returns the storage value for a given key if reversion happens on the current transaction.
    fn initial_storage_at(&self, _key: &H256) -> Result<H256> {
        not_implemented("initial_storage_at");
        unimplemented!()
    }

    /// Returns a value for given key.
    fn storage_at(&self, key: &H256) -> Result<H256> {
        Ok(self.storage.get(&key.0.to_vec()).map(|raw_val| H256::from_slice(&raw_val)).unwrap_or_default())
    }

    /// Stores a value for given key.
    fn set_storage(&mut self, key: H256, value: H256) -> Result<()> {
        self.storage.insert(&key.0.to_vec(), &value.0.to_vec());
        Ok(())
    }

    fn exists(&self, _address: &Address) -> Result<bool> {
        not_implemented("exists");
        unimplemented!()
    }

    fn exists_and_not_null(&self, _address: &Address) -> Result<bool> {
        not_implemented("exists_and_not_null");
        unimplemented!()
    }

    // TODO: sender vs origin
    fn origin_balance(&self) -> Result<U256> {
        self.balance(&sender_as_eth())
    }

    fn balance(&self, address: &Address) -> Result<U256> {
        Ok(self.contract.balances.get(&address.0.to_vec()).unwrap().into())
    }

    fn blockhash(&mut self, _number: &U256) -> H256 {
        not_implemented("blockhash");
        unimplemented!()
    }

    fn create(
        &mut self,
        _gas: &U256,
        _value: &U256,
        _code: &[u8],
        _address: CreateContractAddress,
        _trap: bool,
    ) -> ::std::result::Result<ContractCreateResult, TrapKind> {
        not_implemented("create");
        unimplemented!()
    }

    /// Message call.
    ///
    /// Returns Err, if we run out of gas.
    /// Otherwise returns call_result which contains gas left
    /// and true if subcall was successfull.
    fn call(
        &mut self,
        _gas: &U256,
        _sender_address: &Address,
        _receive_address: &Address,
        _value: Option<U256>,
        _data: &[u8],
        _code_address: &Address,
        call_type: CallType,
        _trap: bool,
    ) -> ::std::result::Result<MessageCallResult, TrapKind> {
        match call_type {
            CallType::None => {
                not_implemented("CallType=None");
                unimplemented!()
            }
            CallType::Call => {
                not_implemented("Call");
                unimplemented!()
            }
            CallType::StaticCall => {
                // identical to call but do not allow state modifications
                not_implemented("StaticCall");
                unimplemented!()
            }
            CallType::CallCode => {
                // Call another contract using storage of the current contract
                not_implemented("CallCode");
                unimplemented!()
            }
            CallType::DelegateCall => {
                // identical to callcode but also keep caller and callvalue
                not_implemented("DelegateCall");
                unimplemented!()
            }
        }
    }

    /// Returns code at given address
    fn extcode(&self, _address: &Address) -> Result<Option<Arc<Bytes>>> {
        not_implemented("extcode");
        unimplemented!()
    }

    /// Returns code hash at given address
    fn extcodehash(&self, _address: &Address) -> Result<Option<H256>> {
        not_implemented("extcodehash");
        // NOTE: only used by constantinople's EXTCODEHASH
        // FIXME: implement
        unimplemented!()
    }

    /// Returns code size at given address
    fn extcodesize(&self, _address: &Address) -> Result<Option<usize>> {
        not_implemented("extcodesize");
        unimplemented!()
    }

    /// Creates log entry with given topics and data
    fn log(&mut self, _topics: Vec<H256>, data: &[u8]) -> Result<()> {
        near_bindgen::env::log(format!("evm log: {}",hex::encode(data)).as_bytes());
        Ok(())
    }

    /// Should be called when transaction calls `RETURN` opcode.
    /// Returns gas_left if cost of returning the data is not too high.
    fn ret(self, _gas: &U256, _data: &ReturnData, _apply_state: bool) -> Result<U256> {
        not_implemented("ret");
        // NOTE: this is only called through finalize(), but we are not using it
        // so it should be safe to ignore it here
        unimplemented!()
    }

    /// Should be called when contract commits suicide.
    /// Address to which funds should be refunded.
    fn suicide(&mut self, _refund_address: &Address) -> Result<()> {
        not_implemented("suicide");
        unimplemented!()
    }

    /// Returns schedule.
    fn schedule(&self) -> &Schedule {
        &self.schedule
    }

    /// Returns environment info.
    fn env_info(&self) -> &EnvInfo {
        &self.info
    }

    /// Returns current depth of execution.
    ///
    /// If contract A calls contract B, and contract B calls C,
    /// then A depth is 0, B is 1, C is 2 and so on.
    fn depth(&self) -> usize {
        // TODO
        0
    }

    /// Increments sstore refunds counter.
    fn add_sstore_refund(&mut self, _value: usize) {
        not_implemented("add_sstore_refund");
        unimplemented!()
    }

    /// Decrements sstore refunds counter.
    fn sub_sstore_refund(&mut self, _value: usize) {
        not_implemented("sub_sstore_refund");
        unimplemented!()
    }

    /// Decide if any more operations should be traced. Passthrough for the VM trace.
    fn trace_next_instruction(&mut self, _pc: usize, _instruction: u8, _current_gas: U256) -> bool {
        false
    }

    /// Prepare to trace an operation. Passthrough for the VM trace.
    fn trace_prepare_execute(
        &mut self,
        _pc: usize,
        _instruction: u8,
        _gas_cost: U256,
        _mem_written: Option<(usize, usize)>,
        _store_written: Option<(U256, U256)>,
    ) {
    }

    /// Trace the finalised execution of a single instruction.
    fn trace_executed(&mut self, _gas_used: U256, _stack_push: &[U256], _mem: &[u8]) {}

    /// Check if running in static context.
    fn is_static(&self) -> bool {
        not_implemented("is_static");
        unimplemented!();
    }
}
