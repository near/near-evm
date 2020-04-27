use std::sync::Arc;

use ethereum_types::{Address, U256};
use evm::Factory;
use vm::{ActionParams, ActionValue, CallType, Ext, GasLeft, ParamsType, ReturnData, Schedule};

use crate::evm_state::{EvmState, StateStore, SubState};
use crate::near_ext::NearExt;

pub fn deploy_code(
    state: &mut dyn EvmState,
    origin: &Address,
    sender: &Address,
    value: U256,
    call_stack_depth: usize,
    address: &Address,
    code: &[u8],
) {
    if state.code_at(address).is_some() {
        panic!(format!(
            "Contract exists at {:?}. How did this happen?",
            hex::encode(address)
        ));
    }

    let (result, state_updates) = _create(
        state,
        origin,
        sender,
        value,
        call_stack_depth,
        address,
        code,
    );

    // Apply known gas amount changes (all reverts are NeedsReturn)
    // Apply NeedsReturn changes if apply_state
    // Return the result unmodified
    let (return_data, apply) = match result {
        Some(GasLeft::Known(_)) => (ReturnData::empty(), true),
        Some(GasLeft::NeedsReturn {
            gas_left: _,
            data,
            apply_state,
        }) => (data, apply_state),
        _ => panic!("Unknown Error".to_string()),
    };

    if apply {
        state.commit_changes(&state_updates.unwrap());
        state.set_code(address, &return_data.to_vec());
    }
}

pub fn _create(
    state: &mut dyn EvmState,
    origin: &Address,
    sender: &Address,
    value: U256,
    call_stack_depth: usize,
    address: &Address,
    code: &[u8],
) -> (Option<GasLeft>, Option<StateStore>) {
    let mut store = StateStore::default();
    let mut sub_state = SubState::new(sender, &mut store, state);

    let params = ActionParams {
        code_address: *address,
        address: *address,
        sender: *sender,
        origin: *origin,
        gas: 1_000_000_000.into(),
        gas_price: 1.into(),
        value: ActionValue::Transfer(value),
        code: Some(Arc::new(code.to_vec())),
        code_hash: None,
        data: None,
        call_type: CallType::None,
        params_type: vm::ParamsType::Embedded,
    };

    sub_state.transfer_balance(sender, address, value);

    let mut ext = NearExt::new(
        *address,
        *origin,
        &mut sub_state,
        call_stack_depth + 1,
        false,
    );
    ext.info.gas_limit = U256::from(1_000_000_000);
    ext.schedule = Schedule::new_constantinople();

    let instance = Factory::default().create(params, ext.schedule(), ext.depth());

    // Run the code
    let result = instance.exec(&mut ext);

    (result.ok().unwrap().ok(), Some(store))
}

pub fn call(
    state: &mut dyn EvmState,
    origin: &Address,
    sender: &Address,
    value: Option<U256>,
    call_stack_depth: usize,
    contract_address: &Address,
    input: &[u8],
    should_commit: bool,
) -> Result<ReturnData, String> {
    run_and_commit_if_success(
        state,
        origin,
        sender,
        value,
        call_stack_depth,
        CallType::Call,
        contract_address,
        contract_address,
        input,
        false,
        should_commit,
    )
}

pub fn delegate_call(
    state: &mut dyn EvmState,
    origin: &Address,
    sender: &Address,
    call_stack_depth: usize,
    context: &Address,
    delegee: &Address,
    input: &[u8],
) -> Result<ReturnData, String> {
    run_and_commit_if_success(
        state,
        origin,
        sender,
        None,
        call_stack_depth,
        CallType::DelegateCall,
        context,
        delegee,
        input,
        false,
        true,
    )
}

pub fn static_call(
    state: &mut dyn EvmState,
    origin: &Address,
    sender: &Address,
    call_stack_depth: usize,
    contract_address: &Address,
    input: &[u8],
) -> Result<ReturnData, String> {
    run_and_commit_if_success(
        state,
        origin,
        sender,
        None,
        call_stack_depth,
        CallType::StaticCall,
        contract_address,
        contract_address,
        input,
        true,
        false,
    )
}

fn run_and_commit_if_success(
    state: &mut dyn EvmState,
    origin: &Address,
    sender: &Address,
    value: Option<U256>,
    call_stack_depth: usize,
    call_type: CallType,
    state_address: &Address,
    code_address: &Address,
    input: &[u8],
    is_static: bool,
    should_commit: bool,
) -> Result<ReturnData, String> {
    // run the interpreter and
    let (result, state_updates) = run_against_state(
        state,
        origin,
        sender,
        value,
        call_stack_depth,
        call_type,
        state_address,
        code_address,
        input,
        is_static,
    );

    // Apply known gas amount changes (all reverts are NeedsReturn)
    // Apply NeedsReturn changes if apply_state
    // Return the result unmodified
    let return_data = match result {
        Some(GasLeft::Known(_)) => Ok(ReturnData::empty()),
        Some(GasLeft::NeedsReturn {
            gas_left: _,
            data,
            apply_state: true,
        }) => Ok(data),
        Some(GasLeft::NeedsReturn {
            gas_left: _,
            data,
            apply_state: false,
        }) => Err(hex::encode(data.to_vec())),
        _ => Err("Unknown Error".to_string()),
    };

    // Don't apply changes from a static context (these _should_ error in the ext)
    if !is_static && return_data.is_ok() && should_commit {
        state.commit_changes(&state_updates.unwrap());
    }

    return_data
}

/// Runs the interpreter. Produces state diffs
fn run_against_state(
    state: &dyn EvmState,
    origin: &Address,
    sender: &Address,
    value: Option<U256>,
    call_stack_depth: usize,
    call_type: CallType,
    state_address: &Address,
    code_address: &Address,
    input: &[u8],
    is_static: bool,
) -> (Option<GasLeft>, Option<StateStore>) {
    let code = state.code_at(code_address).unwrap_or_else(Vec::new);

    let mut store = StateStore::default();
    let mut sub_state = SubState::new(sender, &mut store, state);

    let mut params = ActionParams {
        code_address: *code_address,
        code_hash: None,
        address: *state_address,
        sender: *sender,
        origin: *origin,
        gas: 1_000_000_000.into(),
        gas_price: 1.into(),
        value: ActionValue::Apparent(0.into()),
        code: Some(Arc::new(code)),
        data: Some(input.to_vec()),
        call_type,
        params_type: ParamsType::Separate,
    };

    if let Some(val) = value {
        params.value = ActionValue::Transfer(val);
        // substate transfer will get reverted if the call fails
        sub_state.transfer_balance(sender, state_address, val);
    }

    let mut ext = NearExt::new(
        *state_address,
        *origin,
        &mut sub_state,
        call_stack_depth + 1,
        is_static,
    );
    ext.info.gas_limit = U256::from(1_000_000_000);
    ext.schedule = Schedule::new_constantinople();

    let instance = Factory::default().create(params, ext.schedule(), ext.depth());

    // Run the code
    let result = instance.exec(&mut ext);
    (result.ok().unwrap().ok(), Some(store))
}
