use std::collections::HashMap;

use ethereum_types::{Address, U256};

use crate::utils;

pub trait EvmState {
    fn code_at(&self, address: &Address) -> Option<Vec<u8>>;
    fn set_code(&mut self, address: &Address, bytecode: &Vec<u8>);

    fn _set_balance(&mut self, address: &Vec<u8>, balance: [u8; 32]) -> Option<[u8; 32]>;
    fn set_balance(&mut self, address: &Address, balance: U256) -> Option<U256> {
        let mut bin = [0u8; 32];
        balance.to_big_endian(&mut bin);
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self._set_balance(&internal_addr, bin).map(|v| v.into())
    }

    fn _balance_of(&self, address: &Vec<u8>) -> [u8; 32];
    fn balance_of(&self, address: &Address) -> U256 {
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self._balance_of(&internal_addr).into()
    }

    fn _set_nonce(&mut self, address: &Vec<u8>, nonce: [u8; 32]) -> Option<[u8; 32]>;
    fn set_nonce(&mut self, address: &Address, nonce: U256) -> Option<U256> {
        let mut bin = [0u8; 32];
        nonce.to_big_endian(&mut bin);
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self._set_balance(&internal_addr, bin).map(|v| v.into())
    }

    fn _nonce_of(&self, address: &Vec<u8>) -> [u8; 32];
    fn nonce_of(&self, address: &Address) -> U256 {
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self._nonce_of(&internal_addr).into()
    }

    fn next_nonce(&mut self, address: &Address)  -> U256 {
        let next = self.nonce_of(address) + 1;
        self.set_nonce(address, next);
        next
    }

    fn read_contract_storage(&self, address: &Address, key: &Vec<u8>) -> Option<Vec<u8>>;
    fn set_contract_storage(
        &mut self,
        address: &Address,
        key: &Vec<u8>,
        value: &Vec<u8>,
    ) -> Option<Vec<u8>>;

    fn commit_changes(&mut self, other: &StateStore);

    // panics on overflow (this seems unlikely)
    // TODO: ensure this never becomes larger than 2 ** 128
    fn add_balance(&mut self, address: &Address, incr: U256) -> Option<U256> {
        let balance = self.balance_of(address);
        let new_balance = balance
            .checked_add(incr)
            .expect("overflow during add_balance");
        self.set_balance(address, new_balance)
    }

    // panics if insufficient balance
    fn sub_balance(&mut self, address: &Address, decr: U256) -> Option<U256> {
        let balance = self.balance_of(address);
        let new_balance = balance
            .checked_sub(decr)
            .expect("underflow during sub_balance");
        self.set_balance(address, new_balance)
    }
}

#[derive(Default)]
pub struct StateStore {
    pub code: HashMap<Vec<u8>, Vec<u8>>,
    pub balances: HashMap<Vec<u8>, [u8; 32]>,
    pub nonces: HashMap<Vec<u8>, [u8; 32]>,
    pub storages: HashMap<Vec<u8>, HashMap<Vec<u8>, Vec<u8>>>,
}

impl StateStore {
    pub fn commit_code(&mut self, other: &HashMap<Vec<u8>, Vec<u8>>) {
        self.code
            .extend(other.into_iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    pub fn commit_balances(&mut self, other: &HashMap<Vec<u8>, [u8; 32]>) {
        self.balances
            .extend(other.into_iter().map(|(k, v)| (k.clone(), v.clone())));
    }

    pub fn commit_storages(&mut self, other: &HashMap<Vec<u8>, HashMap<Vec<u8>, Vec<u8>>>) {
        for (k, v) in other.iter() {
            match self.storages.get_mut(k) {
                Some(contract_storage) => {
                    contract_storage.extend(v.into_iter().map(|(k, v)| (k.clone(), v.clone())))
                }
                None => {
                    self.storages.insert(k.to_vec(), v.clone());
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
    pub fn new<'a>(msg_sender: &'a Address, state: &'a mut StateStore, parent: &'a dyn EvmState) -> SubState<'a> {
        SubState { msg_sender, state, parent }
    }

    pub fn contract_storage(&self, address: &Vec<u8>) -> Option<&HashMap<Vec<u8>, Vec<u8>>> {
        self.state.storages.get(&address.to_vec())
    }

    pub fn mut_contract_storage(&mut self, address: &Vec<u8>) -> &mut HashMap<Vec<u8>, Vec<u8>> {
        self.state
            .storages
            .entry(address.to_vec())
            .or_insert(HashMap::default())
    }
}

impl EvmState for SubState<'_> {
    fn code_at(&self, address: &Address) -> Option<Vec<u8>> {
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self.state
            .code
            .get(&internal_addr)
            .map_or_else(|| self.parent.code_at(address), |k| Some(k.to_vec()))
    }

    fn set_code(&mut self, address: &Address, bytecode: &Vec<u8>) {
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self.state.code.insert(internal_addr.to_vec(), bytecode.to_vec());
    }

    fn _balance_of(&self, address: &Vec<u8>) -> [u8; 32] {
        self.state
            .balances
            .get(address)
            .map_or_else(|| self.parent._balance_of(address), |k| k.clone())
    }

    fn _set_balance(&mut self, address: &Vec<u8>, balance: [u8; 32]) -> Option<[u8; 32]> {
        self.state.balances.insert(address.to_vec(), balance)
    }

    fn _nonce_of(&self, address: &Vec<u8>) -> [u8; 32] {
        self.state
            .nonces
            .get(address)
            .map_or_else(|| self.parent._nonce_of(address), |k| k.clone())
    }

    fn _set_nonce(&mut self, address: &Vec<u8>, nonce: [u8; 32]) -> Option<[u8; 32]> {
        self.state.nonces.insert(address.to_vec(), nonce)
    }

    fn read_contract_storage(&self, address: &Address, key: &Vec<u8>) -> Option<Vec<u8>> {
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self.contract_storage(&internal_addr).map_or_else(
            || self.parent.read_contract_storage(address, key),
            |s| s.get(key).map(|v| v.clone()),
        )
    }

    fn set_contract_storage(
        &mut self,
        address: &Address,
        key: &Vec<u8>,
        value: &Vec<u8>,
    ) -> Option<Vec<u8>> {
        let internal_addr = utils::eth_account_to_internal_address(*address);
        self.mut_contract_storage(&internal_addr)
            .insert(key.to_vec(), value.to_vec())
    }

    fn commit_changes(&mut self, other: &StateStore) {
        self.state.commit_code(&other.code);
        self.state.commit_balances(&other.balances);
        self.state.commit_storages(&other.storages);
    }
}
