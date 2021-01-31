use near_evm::backend::{Apply, Backend, Basic, Log};
use near_evm::runner::BackendApply;
use primitive_types::{H160, H256, U256};
use std::collections::{BTreeMap, HashMap};

pub struct TestBackend {
    origin: H160,
    accounts: HashMap<H160, Basic>,
    codes: HashMap<H160, Vec<u8>>,
    storages: HashMap<H160, HashMap<H256, H256>>,
}

impl TestBackend {
    pub fn new(origin: H160) -> Self {
        Self {
            origin,
            accounts: Default::default(),
            codes: Default::default(),
            storages: Default::default(),
        }
    }
}

impl Backend for TestBackend {
    fn gas_left(&self) -> U256 {
        unimplemented!()
    }

    fn gas_price(&self) -> U256 {
        unimplemented!()
    }

    fn origin(&self) -> H160 {
        self.origin
    }

    fn block_hash(&self, _number: U256) -> H256 {
        unimplemented!()
    }

    fn block_number(&self) -> U256 {
        unimplemented!()
    }

    fn block_coinbase(&self) -> H160 {
        unimplemented!()
    }

    fn block_timestamp(&self) -> U256 {
        unimplemented!()
    }

    fn block_difficulty(&self) -> U256 {
        unimplemented!()
    }

    fn block_gas_limit(&self) -> U256 {
        unimplemented!()
    }

    fn chain_id(&self) -> U256 {
        unimplemented!()
    }

    fn exists(&self, _address: H160) -> bool {
        unimplemented!()
    }

    fn basic(&self, address: H160) -> Basic {
        self.accounts
            .get(&address)
            .unwrap_or(&Basic::default())
            .clone()
    }

    fn code_hash(&self, _address: H160) -> H256 {
        unimplemented!()
    }

    fn code_size(&self, _address: H160) -> usize {
        unimplemented!()
    }

    fn code(&self, address: H160) -> Vec<u8> {
        self.codes.get(&address).unwrap_or(&vec![]).clone()
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        self.storages
            .get(&address)
            .map(|s| s.get(&index).unwrap_or(&H256::zero()).clone())
            .unwrap_or(H256::zero())
    }
}

impl BackendApply for TestBackend {
    fn apply(
        &mut self,
        values: Vec<Apply<BTreeMap<H256, H256>>>,
        _logs: Vec<Log>,
        _delete_empty: bool,
    ) {
        println!("{:?}", values);
        for value in values {
            match value {
                Apply::Modify {
                    address,
                    basic,
                    code,
                    storage,
                    reset_storage: _,
                } => {
                    self.accounts.insert(address, basic);
                    if let Some(code) = code {
                        self.codes.insert(address, code);
                    }
                    let s = self.storages.entry(address).or_default();
                    for (index, value) in storage {
                        s.insert(index, value);
                    }
                }
                Apply::Delete { address: _ } => {}
            }
        }
    }
}
