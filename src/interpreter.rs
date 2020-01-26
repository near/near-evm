use std::sync::Arc;

use vm::{ActionParams, CallType, Ext, GasLeft, Schedule};
use evm::Factory;
use ethereum_types::{Address, U256};

use near_bindgen::{env};

use crate::near_ext::NearExt;
use crate::evm_state::{EvmState, SubState, StateStore};

pub fn sender_as_eth() -> Address {
    let mut sender =
        env::signer_account_id().into_bytes();
    sender.resize(20, 0);
    Address::from_slice(&sender)
}

/// implements non-static calls, commits succesful updates to state
pub fn run_and_commit_if_success(state: &mut dyn EvmState,
                                 state_address: Vec<u8>,
                                 code_address: Vec<u8>,
                                 input: Vec<u8>) -> Option<GasLeft> {
        let (result, state_updates) = run_against_state(
            state,
            state_address,
            code_address,
            input);
        match result {
            Some(GasLeft::Known(_)) => {
                state.commit_changes(&state_updates.unwrap());
                result
            },
            Some(GasLeft::NeedsReturn{
                gas_left: _,
                data: _,
                apply_state,
            }) => {
                if apply_state {
                    state.commit_changes(&state_updates.unwrap());
                }
                result
            },
            None => None
        }
}

/// implements non-static calls. Produces state diffs
pub fn run_against_state(state: &dyn EvmState,
                         state_address: Vec<u8>,
                         code_address: Vec<u8>,
                         input: Vec<u8>) -> (Option<GasLeft>, Option<StateStore>) {
    let startgas = 1_000_000_000;
    let code = state.code_at(&code_address).expect("code does not exist");

    let mut store = StateStore::default();
    let mut sub_state = SubState::new(&mut store, state);

    let mut params = ActionParams::default();

    params.call_type = CallType::None;
    params.code = Some(Arc::new(code));
    params.sender = sender_as_eth();
    params.origin = params.sender;
    params.gas = U256::from(startgas);
    params.data = Some(input.to_vec());

    let mut ext = NearExt::new(state_address.to_vec(), &mut sub_state, 0);
    ext.info.gas_limit = U256::from(startgas);
    ext.schedule = Schedule::new_constantinople();

    let instance = Factory::default().create(params, ext.schedule(), ext.depth());

    // Run the code
    let result = instance.exec(&mut ext);

    (result.ok().unwrap().ok(), Some(store))
}
