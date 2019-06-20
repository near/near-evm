use std::convert::From;
use std::convert::TryInto;

use actix::System;
use near_jsonrpc_client::{JsonRpcClient, new_client};
use near_primitives::account::AccessKey;
use near_primitives::crypto::signature::PublicKey;
use near_primitives::hash::CryptoHash;
use near_primitives::receipt::ReceiptInfo;
use near_primitives::rpc::{AccountViewCallResult, QueryResponse, ViewStateResult, StatusResponse};
use near_primitives::serialize::{BaseEncode, to_base};
use near_primitives::transaction::{
    FinalTransactionResult, ReceiptTransaction, SignedTransaction, TransactionResult,
};
use near_primitives::types::{AccountId, Balance, MerkleHash};
use near_protos::signed_transaction as transaction_proto;
use protobuf::Message;

pub trait User {
    fn view_account(&self, account_id: &AccountId) -> Result<AccountViewCallResult, String>;

    fn view_balance(&self, account_id: &AccountId) -> Result<Balance, String> {
        Ok(self.view_account(account_id)?.amount)
    }

    fn view_state(&self, account_id: &AccountId) -> Result<ViewStateResult, String>;

    fn add_transaction(&self, transaction: SignedTransaction) -> Result<(), String>;

    fn commit_transaction(
        &self,
        transaction: SignedTransaction,
    ) -> Result<FinalTransactionResult, String>;

    fn add_receipt(&self, receipt: ReceiptTransaction) -> Result<(), String>;

    fn get_account_nonce(&self, account_id: &AccountId) -> Option<u64>;

    fn get_best_block_index(&self) -> Option<u64>;

    //    fn get_block(&self, index: u64) -> Option<Block>;

    fn get_transaction_result(&self, hash: &CryptoHash) -> TransactionResult;

    fn get_transaction_final_result(&self, hash: &CryptoHash) -> FinalTransactionResult;

    fn get_state_root(&self) -> MerkleHash;

    fn get_receipt_info(&self, hash: &CryptoHash) -> Option<ReceiptInfo>;

    fn get_access_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> Result<Option<AccessKey>, String>;
}

pub struct RpcUser {
    addr: String,
}

impl RpcUser {
    pub fn actix<F, R>(&self, f: F) -> R
    where
        R: Send + 'static,
        F: Send + 'static,
        F: FnOnce(JsonRpcClient) -> R,
    {
        let addr = self.addr.clone();
        let thread = std::thread::spawn(move || {
            let client = new_client(&format!("http://{}", addr));
            let res = f(client);
            res
        });
        thread.join().unwrap()
    }

    pub fn new(addr: &str) -> RpcUser {
        RpcUser {
            addr: addr.to_string(),
        }
    }

    pub fn get_status(&self) -> Option<StatusResponse> {
        self.actix(move |mut client| {
            System::new("actix").block_on(futures::lazy(|| client.status()))
        })
        .ok()
    }

    pub fn query(&self, path: String, data: Vec<u8>) -> Result<QueryResponse, String> {
        self.actix(move |mut client| {
            System::new("actix").block_on(futures::lazy(|| client.query(path, to_base(&data))))
        })
    }
}

impl User for RpcUser {
    fn view_account(&self, account_id: &AccountId) -> Result<AccountViewCallResult, String> {
        let x = self
            .query(format!("account/{}", account_id), vec![])?
            .try_into();
        println!("OOOHO {:?}", x);
        x
    }

    fn view_state(&self, account_id: &AccountId) -> Result<ViewStateResult, String> {
        self.query(format!("contract/{}", account_id), vec![])?
            .try_into()
    }

    fn add_transaction(&self, transaction: SignedTransaction) -> Result<(), String> {
        let proto: transaction_proto::SignedTransaction = transaction.into();
        let bytes = to_base(&proto.write_to_bytes().unwrap());
        let _ = self.actix(move |mut client| {
            System::new("actix").block_on(futures::lazy(|| client.broadcast_tx_async(bytes)))
        })?;
        Ok(())
    }

    fn commit_transaction(
        &self,
        transaction: SignedTransaction,
    ) -> Result<FinalTransactionResult, String> {
        let proto: transaction_proto::SignedTransaction = transaction.into();
        let bytes = to_base(&proto.write_to_bytes().unwrap());
        self.actix(move |mut client| {
            System::new("actix").block_on(futures::lazy(|| client.broadcast_tx_commit(bytes)))
        })
    }

    fn add_receipt(&self, _receipt: ReceiptTransaction) -> Result<(), String> {
        // TDDO: figure out if rpc will support this
        unimplemented!()
    }

    fn get_account_nonce(&self, account_id: &String) -> Option<u64> {
        self.view_account(account_id)
            .map_err(|e| println!("ERROR {}", e))
            .ok()
            .map(|acc| acc.nonce)
    }

    fn get_best_block_index(&self) -> Option<u64> {
        self.get_status()
            .map(|status| status.sync_info.latest_block_height)
    }

    //    fn get_block(&self, index: u64) -> Option<Block> {
    //        System::new("actix").block_on(self.client.write().unwrap().block(index)).ok()
    //    }

    fn get_transaction_result(&self, hash: &CryptoHash) -> TransactionResult {
        let hash = hash.clone();
        self.actix(move |mut client| {
            System::new("actix")
                .block_on(futures::lazy(|| client.tx_details(String::from(&hash))))
                .unwrap()
        })
    }

    fn get_transaction_final_result(&self, hash: &CryptoHash) -> FinalTransactionResult {
        let hash = hash.clone();
        self.actix(move |mut client| {
            System::new("actix")
                .block_on(futures::lazy(|| client.tx(String::from(&hash))))
                .unwrap()
        })
    }

    fn get_state_root(&self) -> MerkleHash {
        self.get_status()
            .map(|status| status.sync_info.latest_state_root)
            .unwrap()
    }

    fn get_receipt_info(&self, _hash: &CryptoHash) -> Option<ReceiptInfo> {
        // TDDO: figure out if rpc will support this
        unimplemented!()
    }

    fn get_access_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> Result<Option<AccessKey>, String> {
        self.query(
            format!("access_key/{}/{}", account_id, public_key.to_base()),
            vec![],
        )?
        .try_into()
    }
}
