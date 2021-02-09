use near_evm::backend::{Apply, ApplyBackend, Backend, Basic, Log};
use near_evm::types::{bytes_to_hex, log_to_bytes};
use primitive_types::{H160, H256, U256};
use std::collections::HashMap;

pub struct TestBackend {
    pub origin: H160,
    pub timestamp: U256,
    pub accounts: HashMap<H160, Basic>,
    pub codes: HashMap<H160, Vec<u8>>,
    pub storages: HashMap<H160, HashMap<H256, H256>>,
    pub logs: Vec<Vec<u8>>,
}

impl TestBackend {
    pub fn new(origin: H160) -> Self {
        Self {
            origin,
            timestamp: U256::zero(),
            accounts: Default::default(),
            codes: Default::default(),
            storages: Default::default(),
            logs: Default::default(),
        }
    }
}

impl Backend for TestBackend {
    fn gas_left(&self) -> U256 {
        U256::zero()
    }

    fn gas_price(&self) -> U256 {
        U256::zero()
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
        self.timestamp
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

    fn code_size(&self, address: H160) -> usize {
        self.codes.get(&address).unwrap_or(&vec![]).len()
    }

    fn code(&self, address: H160) -> Vec<u8> {
        println!("Read code {}", address);
        self.codes.get(&address).unwrap_or(&vec![]).clone()
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        println!("Read storage {} {}", address, index);
        self.storages
            .get(&address)
            .map(|s| s.get(&index).unwrap_or(&H256::zero()).clone())
            .unwrap_or(H256::zero())
    }
}

impl ApplyBackend for TestBackend {
    fn apply<A, I, L>(&mut self, values: A, logs: L, _delete_empty: bool)
    where
        A: IntoIterator<Item = Apply<I>>,
        I: IntoIterator<Item = (H256, H256)>,
        L: IntoIterator<Item = Log>,
    {
        println!("Apply");
        for value in values {
            match value {
                Apply::Modify {
                    address,
                    basic,
                    code,
                    storage,
                    reset_storage: _,
                } => {
                    println!(
                        "Address: {:?}, account: {:?}, code: {}",
                        address,
                        basic,
                        code.clone().map(|c| c.len()).unwrap_or(0)
                    );
                    self.accounts.insert(address, basic);
                    if let Some(code) = code {
                        self.codes.insert(address, code);
                    }
                    let s = self.storages.entry(address).or_default();
                    for (index, value) in storage {
                        println!("  {} {}", index, value);
                        s.insert(index, value);
                    }
                }
                Apply::Delete { address: _ } => {}
            }
        }
        for log in logs {
            self.logs
                .push(bytes_to_hex(&log_to_bytes(log)).into_bytes());
        }
    }
}
