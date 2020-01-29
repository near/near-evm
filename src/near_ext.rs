use std::sync::Arc;

use ethereum_types::{Address, H256, U256};
use parity_bytes::Bytes;
use vm::{
    CallType, ContractCreateResult, CreateContractAddress, EnvInfo, Error as VmError, GasLeft,
    MessageCallResult, Result as EvmResult, ReturnData, Schedule, TrapKind,
};

use crate::evm_state::{EvmState, SubState};
use crate::interpreter;
use crate::utils;
use near_bindgen;

// https://github.com/paritytech/parity-ethereum/blob/77643c13e80ca09d9a6b10631034f5a1568ba6d3/ethcore/machine/src/externalities.rs
pub struct NearExt<'a> {
    pub info: EnvInfo,
    pub schedule: Schedule,
    pub context_addr: Address,
    pub selfdestruct_address: Option<Address>,
    pub sub_state: &'a mut SubState<'a>,
    pub static_flag: bool,
    pub depth: usize,
}

impl<'a> NearExt<'a> {
    pub fn new(
        context_addr: Address,
        sub_state: &'a mut SubState<'a>,
        depth: usize,
        static_flag: bool,
    ) -> Self {
        Self {
            info: Default::default(),
            schedule: Default::default(),
            context_addr,
            selfdestruct_address: Default::default(),
            sub_state,
            static_flag,
            depth,
        }
    }
}

fn not_implemented(name: &str) {
    near_bindgen::env::log(format!("not implemented: {}", name).as_bytes());
}

impl<'a> vm::Ext for NearExt<'a> {
    /// Returns the storage value for a given key if reversion happens on the current transaction.
    fn initial_storage_at(&self, _key: &H256) -> EvmResult<H256> {
        not_implemented("initial_storage_at");
        unimplemented!()
    }

    /// Returns a value for given key.
    fn storage_at(&self, key: &H256) -> EvmResult<H256> {
        let raw_val = self
            .sub_state
            .read_contract_storage(&self.context_addr, &key.0.to_vec())
            .map(|v| v.clone())
            .unwrap_or(vec![0; 32]); // default to an empty vec of correct length
        Ok(H256::from_slice(&raw_val))
    }

    /// Stores a value for given key.
    fn set_storage(&mut self, key: H256, value: H256) -> EvmResult<()> {
        if self.is_static() {
            return Err(VmError::MutableCallInStaticContext);
        }
        self.sub_state
            .set_contract_storage(&self.context_addr, &key.0.to_vec(), &value.0.to_vec());
        Ok(())
    }

    fn exists(&self, _address: &Address) -> EvmResult<bool> {
        not_implemented("exists");
        unimplemented!()
    }

    fn exists_and_not_null(&self, _address: &Address) -> EvmResult<bool> {
        not_implemented("exists_and_not_null");
        unimplemented!()
    }

    // TODO: sender vs origin
    fn origin_balance(&self) -> EvmResult<U256> {
        self.balance(&utils::predecessor_as_eth())
    }

    fn balance(&self, address: &Address) -> EvmResult<U256> {
        Ok(self.sub_state.balance_of(address))
    }

    fn blockhash(&mut self, _number: &U256) -> H256 {
        not_implemented("blockhash");
        unimplemented!()
    }

    fn create(
        &mut self,
        _gas: &U256,
        value: &U256,
        code: &[u8],
        address_type: CreateContractAddress,
        _trap: bool,
    ) -> Result<ContractCreateResult, TrapKind> {
        if self.is_static() {
            panic!("MutableCallInStaticContext")
        }

        let mut nonce = U256::default();
        if address_type == CreateContractAddress::FromSenderAndNonce {
            // TODO: we should create a new substate
            //       and commit to the increment in the substate AFTER success
            //       I think we have no failure cases right now, so that can wait
            nonce = self.sub_state.next_nonce(&self.context_addr);
        }

        let (addr, _) = utils::evm_contract_address(
            address_type,
            &self.context_addr,
            &nonce,
            code
        );

        interpreter::deploy_code(self.sub_state, &addr, &code.to_vec());
        self.sub_state.sub_balance(&addr, *value);
        self.sub_state.add_balance(&addr, *value);
        Ok(ContractCreateResult::Created(addr, 0.into()))
        //
        // // https://github.com/paritytech/parity-ethereum/blob/master/ethcore/vm/src/ext.rs#L57-L64
        // not_implemented("create");
        // unimplemented!()
    }

    /// Message call.
    ///
    /// Returns Err, if we run out of gas.
    /// Otherwise returns call_result which contains gas left
    /// and true if subcall was successfull.
    fn call(
        &mut self,
        _gas: &U256,
        sender_address: &Address,
        receive_address: &Address,
        value: Option<U256>,
        data: &[u8],
        code_address: &Address,
        call_type: CallType,
        _trap: bool,
    ) -> Result<MessageCallResult, TrapKind> {

        if self.is_static() && call_type != CallType::StaticCall {
            panic!("MutableCallInStaticContext")
        }

        let opt_gas_left = match call_type {
            CallType::None => {
                // Can stay unimplemented
                not_implemented("CallType=None");
                unimplemented!()
            }
            CallType::Call => interpreter::call(
                self.sub_state,
                sender_address,
                value,
                self.depth,
                receive_address,
                &data.to_vec(),
            ),
            CallType::StaticCall => interpreter::static_call(
                self.sub_state,
                sender_address,
                self.depth,
                receive_address,
                &data.to_vec(),
            ),
            CallType::CallCode => {
                // Call another contract using storage of the current contract
                // Can leave unimplemented, no longer used.
                not_implemented("CallCode");
                unimplemented!()
            }
            CallType::DelegateCall => interpreter::delegate_call(
                self.sub_state,
                sender_address,
                self.depth,
                receive_address,
                code_address,
                &data.to_vec(),
            ),
        };

        // GasLeft into MessageCallResult
        let res = match opt_gas_left {
            Some(GasLeft::Known(gas_left)) => {
                vm::MessageCallResult::Success(gas_left, ReturnData::empty())
            }
            Some(GasLeft::NeedsReturn {
                gas_left,
                data,
                apply_state: true,
            }) => vm::MessageCallResult::Success(gas_left, data),
            Some(GasLeft::NeedsReturn {
                gas_left,
                data,
                apply_state: false,
            }) => vm::MessageCallResult::Reverted(gas_left, data),
            _ => vm::MessageCallResult::Failed,
        };

        Ok(res) // Even failed is Ok. Err() is for resume traps
    }

    /// Returns code at given address
    fn extcode(&self, address: &Address) -> EvmResult<Option<Arc<Bytes>>> {
        let code = self
            .sub_state
            .code_at(address)
            .map(|c| Arc::new(c));
        Ok(code)
    }

    /// Returns code hash at given address
    fn extcodehash(&self, _address: &Address) -> EvmResult<Option<H256>> {
        not_implemented("extcodehash");
        // NOTE: only used by constantinople's EXTCODEHASH
        // FIXME: implement
        unimplemented!()
    }

    /// Returns code size at given address
    fn extcodesize(&self, address: &Address) -> EvmResult<Option<usize>> {
        Ok(self.sub_state.code_at(address).map(|c| c.len()))
    }

    /// Creates log entry with given topics and data
    fn log(&mut self, _topics: Vec<H256>, data: &[u8]) -> EvmResult<()> {
        if self.is_static() {
            return Err(VmError::MutableCallInStaticContext);
        }

        // TODO: Develop a NearCall logspec
        //       hijack NearCall logs here
        //       make a Vec<log> that accumulates committed logs
        //       return them after execution completes
        //       dispatch promises

        near_bindgen::env::log(format!("evm log: {}", hex::encode(data)).as_bytes());
        Ok(())
    }

    /// Should be called when transaction calls `RETURN` opcode.
    /// Returns gas_left if cost of returning the data is not too high.
    fn ret(self, _gas: &U256, _data: &ReturnData, _apply_state: bool) -> EvmResult<U256> {
        // NOTE: this is only called through finalize(), but we are not using it
        // so it should be safe to ignore it here
        not_implemented("ret");
        unimplemented!()
    }

    /// Should be called when contract commits suicide.
    /// Address to which funds should be refunded.
    fn suicide(&mut self, _refund_address: &Address) -> EvmResult<()> {
        // TODO: implement.
        //       Does suicide delete or preserve storage (delete I think?)
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
        self.depth
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
        self.static_flag
    }
}
