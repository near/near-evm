use std::collections::HashMap;

use ethereum_types::{Address, U256};

use crate::utils;

pub trait EvmState {
    fn code_at(&self, address: &Address) -> Option<Vec<u8>>;
    fn set_code(&mut self, address: &Address, bytecode: &[u8]);

    fn _set_balance(&mut self, address: [u8; 20], balance: [u8; 32]) -> Option<[u8; 32]>;
    fn set_balance(&mut self, address: &Address, balance: U256) -> Option<U256> {
        let mut bin = [0u8; 32];
        balance.to_big_endian(&mut bin);
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self._set_balance(internal_addr, bin).map(|v| v.into())
    }

    fn _balance_of(&self, address: [u8; 20]) -> [u8; 32];
    fn balance_of(&self, address: &Address) -> U256 {
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self._balance_of(internal_addr).into()
    }

    fn _set_nonce(&mut self, address: [u8; 20], nonce: [u8; 32]) -> Option<[u8; 32]>;
    fn set_nonce(&mut self, address: &Address, nonce: U256) -> Option<U256> {
        let mut bin = [0u8; 32];
        nonce.to_big_endian(&mut bin);
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self._set_nonce(internal_addr, bin).map(|v| v.into())
    }

    fn _nonce_of(&self, address: [u8; 20]) -> [u8; 32];
    fn nonce_of(&self, address: &Address) -> U256 {
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self._nonce_of(internal_addr).into()
    }

    fn next_nonce(&mut self, address: &Address) -> U256 {
        let nonce = self.nonce_of(address);
        self.set_nonce(address, nonce + 1);
        nonce
    }

    fn read_contract_storage(&self, address: &Address, key: [u8; 32]) -> Option<[u8; 32]>;
    fn set_contract_storage(
        &mut self,
        address: &Address,
        key: [u8; 32],
        value: [u8; 32],
    ) -> Option<[u8; 32]>;

    fn commit_changes(&mut self, other: &StateStore);

    // Panics on u256 overflow
    // This represents NEAR tokens, so it can never _actually_ go above 2**128
    // That'd be silly.
    fn add_balance(&mut self, address: &Address, incr: U256) -> Option<U256> {
        let balance = self.balance_of(address);
        let new_balance = balance
            .checked_add(incr)
            .expect("overflow during add_balance");
        self.set_balance(address, new_balance)
    }

    // Panics if insufficient balance
    fn sub_balance(&mut self, address: &Address, decr: U256) -> Option<U256> {
        let balance = self.balance_of(address);
        let new_balance = balance
            .checked_sub(decr)
            .expect("underflow during sub_balance");
        self.set_balance(address, new_balance)
    }

    fn transfer_balance(&mut self, sender: &Address, recipient: &Address, amnt: U256) {
        self.sub_balance(sender, amnt);
        self.add_balance(recipient, amnt);
    }
}

#[derive(Default)]
pub struct StateStore {
    pub code: HashMap<[u8; 20], Vec<u8>>,
    pub balances: HashMap<[u8; 20], [u8; 32]>,
    pub nonces: HashMap<[u8; 20], [u8; 32]>,
    pub storages: HashMap<[u8; 20], HashMap<[u8; 32], [u8; 32]>>,
    pub logs: Vec<String>,
}

impl StateStore {
    pub fn commit_code(&mut self, other: &HashMap<[u8; 20], Vec<u8>>) {
        self.code
            .extend(other.iter().map(|(k, v)| (*k, v.clone())));
    }

    pub fn commit_balances(&mut self, other: &HashMap<[u8; 20], [u8; 32]>) {
        self.balances
            .extend(other.iter().map(|(k, v)| (*k, *v)));
    }

    pub fn commit_nonces(&mut self, other: &HashMap<[u8; 20], [u8; 32]>) {
        self.nonces
            .extend(other.iter().map(|(k, v)| (*k, *v)));
    }

    pub fn commit_storages(&mut self, other: &HashMap<[u8; 20], HashMap<[u8; 32], [u8; 32]>>) {
        for (k, v) in other.iter() {
            match self.storages.get_mut(k) {
                Some(contract_storage) => {
                    contract_storage.extend(v.iter().map(|(k, v)| (*k, *v)))
                }
                None => {
                    self.storages.insert(*k, v.clone());
                }
            }
        }
    }
}

pub struct SubState<'a> {
    pub msg_sender: &'a Address,
    pub state: &'a mut StateStore,
    pub parent: &'a dyn EvmState,
}

impl SubState<'_> {
    pub fn new<'a>(
        msg_sender: &'a Address,
        state: &'a mut StateStore,
        parent: &'a dyn EvmState,
    ) -> SubState<'a> {
        SubState {
            msg_sender,
            state,
            parent,
        }
    }

    pub fn contract_storage(&self, address: [u8; 20]) -> Option<&HashMap<[u8; 32], [u8; 32]>> {
        self.state.storages.get(&address)
    }

    pub fn mut_contract_storage(&mut self, address: [u8; 20]) -> &mut HashMap<[u8; 32], [u8; 32]> {
        self.state
            .storages
            .entry(address)
            .or_insert_with(HashMap::default)
    }
}

impl EvmState for SubState<'_> {
    fn code_at(&self, address: &Address) -> Option<Vec<u8>> {
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self.state
            .code
            .get(&internal_addr)
            .map_or_else(|| self.parent.code_at(address), |k| Some(k.to_vec()))
    }

    fn set_code(&mut self, address: &Address, bytecode: &[u8]) {
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self.state.code.insert(internal_addr, bytecode.to_vec());
    }

    fn _balance_of(&self, address: [u8; 20]) -> [u8; 32] {
        self.state
            .balances
            .get(&address)
            .map_or_else(|| self.parent._balance_of(address), |k| *k)
    }

    fn _set_balance(&mut self, address: [u8; 20], balance: [u8; 32]) -> Option<[u8; 32]> {
        self.state.balances.insert(address, balance)
    }

    fn _nonce_of(&self, address: [u8; 20]) -> [u8; 32] {
        self.state
            .nonces
            .get(&address)
            .map_or_else(|| self.parent._nonce_of(address), |k| *k)
    }

    fn _set_nonce(&mut self, address: [u8; 20], nonce: [u8; 32]) -> Option<[u8; 32]> {
        self.state.nonces.insert(address, nonce)
    }

    fn read_contract_storage(&self, address: &Address, key: [u8; 32]) -> Option<[u8; 32]> {
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self.contract_storage(internal_addr).map_or_else(
            || self.parent.read_contract_storage(address, key),
            |s| s.get(&key).copied(),
        )
    }

    fn set_contract_storage(
        &mut self,
        address: &Address,
        key: [u8; 32],
        value: [u8; 32],
    ) -> Option<[u8; 32]> {
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self.mut_contract_storage(internal_addr).insert(key, value)
    }

    fn commit_changes(&mut self, other: &StateStore) {
        self.state.commit_code(&other.code);
        self.state.commit_balances(&other.balances);
        self.state.commit_nonces(&other.nonces);
        self.state.commit_storages(&other.storages);
        self.state.logs.extend(other.logs.iter().cloned());
    }
}
