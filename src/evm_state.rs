use std::collections::HashMap;

use ethereum_types::U256;

pub trait EvmState {
    fn code_at(&self, address: &Vec<u8>) -> Option<Vec<u8>>;
    fn set_code(&mut self, address: &Vec<u8>, bytecode: &Vec<u8>);

    fn _set_balance(&mut self, address: &Vec<u8>, balance: [u8; 32]) -> Option<[u8; 32]>;
    fn set_balance(&mut self, address: &Vec<u8>, balance: U256) -> Option<U256>;

    fn _balance_of(&self, address: &Vec<u8>) -> [u8; 32];
    fn balance_of(&self, address: &Vec<u8>) -> U256;

    fn read_contract_storage(&self, address: &Vec<u8>, key: &Vec<u8>) -> Option<Vec<u8>>;
    fn set_contract_storage(
        &mut self,
        address: &Vec<u8>,
        key: &Vec<u8>,
        value: &Vec<u8>,
    ) -> Option<Vec<u8>>;

    fn commit_changes(&mut self, other: &StateStore);

    // panics on overflow (this seems unlikely)
    // TODO: ensure this never becomes larger than 2 ** 128
    fn add_balance(&mut self, address: &Vec<u8>, incr: U256) -> Option<U256> {
        let balance = self.balance_of(address);
        let new_balance = balance
            .checked_add(incr)
            .expect("overflow during add_balance");
        self.set_balance(address, new_balance)
    }

    // panics if insufficient balance
    fn sub_balance(&mut self, address: &Vec<u8>, decr: U256) -> Option<U256> {
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
    pub state: &'a mut StateStore,
    pub parent: &'a dyn EvmState,
}

impl SubState<'_> {
    pub fn new<'a>(state: &'a mut StateStore, parent: &'a dyn EvmState) -> SubState<'a> {
        SubState { state, parent }
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
    fn code_at(&self, address: &Vec<u8>) -> Option<Vec<u8>> {
        self.state
            .code
            .get(address)
            .map_or_else(|| self.parent.code_at(address), |k| Some(k.to_vec()))
    }

    fn set_code(&mut self, address: &Vec<u8>, bytecode: &Vec<u8>) {
        self.state.code.insert(address.to_vec(), bytecode.to_vec());
    }

    fn _balance_of(&self, address: &Vec<u8>) -> [u8; 32] {
        self.state
            .balances
            .get(address)
            .map_or_else(|| self.parent._balance_of(address), |k| k.clone())
    }

    fn balance_of(&self, address: &Vec<u8>) -> U256 {
        self._balance_of(address).into()
    }

    fn _set_balance(&mut self, address: &Vec<u8>, balance: [u8; 32]) -> Option<[u8; 32]> {
        self.state.balances.insert(address.to_vec(), balance)
    }

    fn set_balance(&mut self, address: &Vec<u8>, balance: U256) -> Option<U256> {
        let mut bin = [0u8; 32];
        balance.to_big_endian(&mut bin);
        self._set_balance(address, bin).map(|v| v.into())
    }

    fn read_contract_storage(&self, address: &Vec<u8>, key: &Vec<u8>) -> Option<Vec<u8>> {
        self.contract_storage(address).map_or_else(
            || self.parent.read_contract_storage(address, key),
            |s| s.get(key).map(|v| v.clone()),
        )
    }

    fn set_contract_storage(
        &mut self,
        address: &Vec<u8>,
        key: &Vec<u8>,
        value: &Vec<u8>,
    ) -> Option<Vec<u8>> {
        self.mut_contract_storage(address)
            .insert(key.to_vec(), value.to_vec())
    }

    fn commit_changes(&mut self, other: &StateStore) {
        self.state.commit_code(&other.code);
        self.state.commit_balances(&other.balances);
        self.state.commit_storages(&other.storages);
    }
}
