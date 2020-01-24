use std::convert::TryInto;
use std::sync::{Arc};

use actix::System;
use borsh::ser::BorshSerialize;
use futures::{Future, TryFutureExt, FutureExt};
use near_crypto::{PublicKey, Signer};
use near_jsonrpc_client::{ new_client};
use near_jsonrpc_client::JsonRpcClient;
use near_primitives::account::AccessKey;
use near_primitives::hash::CryptoHash;
use near_primitives::receipt::Receipt;
use near_primitives::serialize::{to_base, to_base64};
use near_primitives::transaction::{
    Action, AddKeyAction, CreateAccountAction, DeleteAccountAction, DeleteKeyAction,
    DeployContractAction, ExecutionOutcome, FunctionCallAction, SignedTransaction, StakeAction,
    TransferAction,
};
use near_primitives::types::{AccountId, Balance, Gas, MerkleHash, BlockHeight, MaybeBlockId, BlockId};
use near_primitives::views::{AccessKeyView, AccountView, BlockView, QueryResponse, StatusResponse, ViewStateResult, ExecutionErrorView, EpochValidatorInfo};
use near_primitives::views::{ExecutionOutcomeView, FinalExecutionOutcomeView};
use std::thread;
use futures::future::LocalBoxFuture;
use std::time::Duration;

pub trait User {
    fn view_account(&self, account_id: &AccountId) -> Result<AccountView, String>;

    fn view_balance(&self, account_id: &AccountId) -> Result<Balance, String> {
        Ok(self.view_account(account_id)?.amount)
    }

    fn view_state(&self, account_id: &AccountId, prefix: &[u8]) -> Result<ViewStateResult, String>;

    fn add_transaction(&self, signed_transaction: SignedTransaction) -> Result<(), String>;

    fn commit_transaction(
        &self,
        signed_transaction: SignedTransaction,
    ) -> Result<FinalExecutionOutcomeView, String>;

    fn add_receipt(&self, receipt: Receipt) -> Result<(), String>;

    fn get_access_key_nonce_for_signer(&self, account_id: &AccountId) -> Result<u64, String> {
        self.get_access_key(account_id, &self.signer().public_key())
            .map(|access_key| access_key.nonce)
    }

    fn get_best_height(&self) -> Result<BlockHeight, String>;

    fn get_best_block_hash(&self) -> Result<CryptoHash, String>;

    fn get_block(&self, height: BlockHeight) -> Option<BlockView>;

    fn get_transaction_result(&self, hash: &CryptoHash) -> ExecutionOutcomeView;

    fn get_transaction_final_result(&self, hash: &CryptoHash) -> FinalExecutionOutcomeView;

    fn get_state_root(&self) -> CryptoHash;

    fn get_access_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> Result<AccessKeyView, String>;

    fn signer(&self) -> Arc<dyn Signer>;

    fn set_signer(&mut self, signer: Arc<dyn Signer>);

    fn sign_and_commit_actions(
        &self,
        signer_id: AccountId,
        receiver_id: AccountId,
        actions: Vec<Action>,
    ) -> Result<FinalExecutionOutcomeView, String> {
        let block_hash = self.get_best_block_hash().unwrap_or(CryptoHash::default());
        let signed_transaction = SignedTransaction::from_actions(
            self.get_access_key_nonce_for_signer(&signer_id).unwrap_or_default() + 1,
            signer_id,
            receiver_id,
            &*self.signer(),
            actions,
            block_hash,
        );
        self.commit_transaction(signed_transaction)
    }

    fn send_money(
        &self,
        signer_id: AccountId,
        receiver_id: AccountId,
        amount: Balance,
    ) -> Result<FinalExecutionOutcomeView, String> {
        self.sign_and_commit_actions(
            signer_id,
            receiver_id,
            vec![Action::Transfer(TransferAction { deposit: amount })],
        )
    }

    fn deploy_contract(
        &self,
        signer_id: AccountId,
        code: Vec<u8>,
    ) -> Result<FinalExecutionOutcomeView, String> {
        self.sign_and_commit_actions(
            signer_id.clone(),
            signer_id,
            vec![Action::DeployContract(DeployContractAction { code })],
        )
    }

    fn function_call(
        &self,
        signer_id: AccountId,
        contract_id: AccountId,
        method_name: &str,
        args: Vec<u8>,
        gas: Gas,
        deposit: Balance,
    ) -> Result<FinalExecutionOutcomeView, String> {
        self.sign_and_commit_actions(
            signer_id,
            contract_id,
            vec![Action::FunctionCall(FunctionCallAction {
                method_name: method_name.to_string(),
                args,
                gas,
                deposit,
            })],
        )
    }

    fn create_account(
        &self,
        signer_id: AccountId,
        new_account_id: AccountId,
        public_key: PublicKey,
        amount: Balance,
    ) -> Result<FinalExecutionOutcomeView, String> {
        self.sign_and_commit_actions(
            signer_id,
            new_account_id,
            vec![
                Action::CreateAccount(CreateAccountAction {}),
                Action::Transfer(TransferAction { deposit: amount }),
                Action::AddKey(AddKeyAction { public_key, access_key: AccessKey::full_access() }),
            ],
        )
    }

    fn add_key(
        &self,
        signer_id: AccountId,
        public_key: PublicKey,
        access_key: AccessKey,
    ) -> Result<FinalExecutionOutcomeView, String> {
        self.sign_and_commit_actions(
            signer_id.clone(),
            signer_id,
            vec![Action::AddKey(AddKeyAction { public_key, access_key })],
        )
    }

    fn delete_key(
        &self,
        signer_id: AccountId,
        public_key: PublicKey,
    ) -> Result<FinalExecutionOutcomeView, String> {
        self.sign_and_commit_actions(
            signer_id.clone(),
            signer_id,
            vec![Action::DeleteKey(DeleteKeyAction { public_key })],
        )
    }

    fn swap_key(
        &self,
        signer_id: AccountId,
        old_public_key: PublicKey,
        new_public_key: PublicKey,
        access_key: AccessKey,
    ) -> Result<FinalExecutionOutcomeView, String> {
        self.sign_and_commit_actions(
            signer_id.clone(),
            signer_id,
            vec![
                Action::DeleteKey(DeleteKeyAction { public_key: old_public_key }),
                Action::AddKey(AddKeyAction { public_key: new_public_key, access_key }),
            ],
        )
    }

    fn delete_account(
        &self,
        signer_id: AccountId,
        receiver_id: AccountId,
    ) -> Result<FinalExecutionOutcomeView, String> {
        self.sign_and_commit_actions(
            signer_id.clone(),
            receiver_id,
            vec![Action::DeleteAccount(DeleteAccountAction { beneficiary_id: signer_id })],
        )
    }

    fn stake(
        &self,
        signer_id: AccountId,
        public_key: PublicKey,
        stake: Balance,
    ) -> Result<FinalExecutionOutcomeView, String> {
        self.sign_and_commit_actions(
            signer_id.clone(),
            signer_id,
            vec![Action::Stake(StakeAction { stake, public_key })],
        )
    }
}

/// Same as `User` by provides async API that can be used inside tokio.
pub trait AsyncUser: Send + Sync {
    fn view_account(
        &self,
        account_id: &AccountId,
    ) -> LocalBoxFuture<'static, Result<AccountView, String>>;

    fn view_balance(
        &self,
        account_id: &AccountId,
    ) -> LocalBoxFuture<'static, Result<Balance, String>> {
        self.view_account(account_id).map(|res| res.map(|acc| acc.amount)).boxed_local()
    }

    fn view_state(
        &self,
        account_id: &AccountId,
    ) -> LocalBoxFuture<'static, Result<ViewStateResult, String>>;

    fn add_transaction(
        &self,
        transaction: SignedTransaction,
    ) -> LocalBoxFuture<'static, Result<(), String>>;

    fn add_receipt(&self, receipt: Receipt) -> LocalBoxFuture<'static, Result<(), String>>;

    fn get_account_nonce(
        &self,
        account_id: &AccountId,
    ) -> LocalBoxFuture<'static, Result<u64, String>>;

    fn get_best_height(&self) -> LocalBoxFuture<'static, Result<BlockHeight, String>>;

    fn get_transaction_result(
        &self,
        hash: &CryptoHash,
    ) -> LocalBoxFuture<'static, Result<ExecutionOutcome, String>>;

    fn get_transaction_final_result(
        &self,
        hash: &CryptoHash,
    ) -> LocalBoxFuture<'static, Result<FinalExecutionOutcomeView, String>>;

    fn get_state_root(&self) -> LocalBoxFuture<'static, Result<MerkleHash, String>>;

    fn get_access_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> LocalBoxFuture<'static, Result<Option<AccessKey>, String>>;
}


pub struct RpcUser {
    account_id: AccountId,
    signer: Arc<dyn Signer>,
    addr: String,
}

impl RpcUser {
    fn actix<F, Fut, R>(&self, f: F) -> R
        where
            Fut: Future<Output = R> + 'static,
            F: FnOnce(JsonRpcClient) -> Fut + 'static,
    {
        let addr = self.addr.clone();
        System::new("actix")
            .block_on(async move { f(new_client(&format!("http://{}", addr))).await })
    }

    pub fn new(addr: &str, account_id: AccountId, signer: Arc<dyn Signer>) -> RpcUser {
        RpcUser { account_id, addr: addr.to_owned(), signer }
    }

    pub fn get_status(&self) -> Result<StatusResponse, String> {
        self.actix(|mut client| client.status()).map_err(|err| err.to_string())
    }

    pub fn query(&self, path: String, data: &[u8]) -> Result<QueryResponse, String> {
        let data = to_base(data);
        self.actix(move |mut client| client.query(path, data).map_err(|err| err.to_string()))
    }

    pub fn validators(&self, block_id: MaybeBlockId) -> Result<EpochValidatorInfo, String> {
        self.actix(move |mut client| client.validators(block_id).map_err(|err| err.to_string()))
    }
}

impl User for RpcUser {
    fn view_account(&self, account_id: &AccountId) -> Result<AccountView, String> {
        self.query(format!("account/{}", account_id), &[])?.try_into()
    }

    fn view_state(&self, account_id: &AccountId, prefix: &[u8]) -> Result<ViewStateResult, String> {
        self.query(format!("contract/{}", account_id), prefix)?.try_into()
    }

    fn add_transaction(&self, transaction: SignedTransaction) -> Result<(), String> {
        let bytes = transaction.try_to_vec().unwrap();
        let _ = self
            .actix(move |mut client| client.broadcast_tx_async(to_base64(&bytes)))
            .map_err(|err| err.to_string())?;
        Ok(())
    }

    fn commit_transaction(
        &self,
        transaction: SignedTransaction,
    ) -> Result<FinalExecutionOutcomeView, String> {
        let bytes = transaction.try_to_vec().unwrap();
        let result = self.actix(move |mut client| client.broadcast_tx_commit(to_base64(&bytes)));
        // Wait for one more block, to make sure all nodes actually apply the state transition.
        let height = self.get_best_height().unwrap();
        while height == self.get_best_height().unwrap() {
            thread::sleep(Duration::from_millis(50));
        }
        match result {
            Ok(outcome) => Ok(outcome),
            Err(err) => {
                match serde_json::from_value::<ExecutionErrorView>(err.clone().data.unwrap()) {
                    Ok(error_view) => Err(error_view.error_message),
                    Err(_) => Err(serde_json::to_string(&err).unwrap()),
                }
            }
        }
    }

    fn add_receipt(&self, _receipt: Receipt) -> Result<(), String> {
        // TDDO: figure out if rpc will support this
        unimplemented!()
    }

    fn get_best_height(&self) -> Result<BlockHeight, String> {
        self.get_status().map(|status| status.sync_info.latest_block_height)
    }

    fn get_best_block_hash(&self) -> Result<CryptoHash, String> {
        self.get_status().map(|status| status.sync_info.latest_block_hash)
    }

    fn get_block(&self, height: BlockHeight) -> Option<BlockView> {
        self.actix(move |mut client| client.block(BlockId::Height(height))).ok()
    }

    fn get_transaction_result(&self, _hash: &CryptoHash) -> ExecutionOutcomeView {
        unimplemented!()
    }

    fn get_transaction_final_result(&self, hash: &CryptoHash) -> FinalExecutionOutcomeView {
        let account_id = self.account_id.clone();
        let hash = *hash;
        self.actix(move |mut client| client.tx((&hash).into(), account_id)).unwrap()
    }

    fn get_state_root(&self) -> CryptoHash {
        self.get_status().map(|status| status.sync_info.latest_state_root).unwrap()
    }

    fn get_access_key(
        &self,
        account_id: &AccountId,
        public_key: &PublicKey,
    ) -> Result<AccessKeyView, String> {
        self.query(format!("access_key/{}/{}", account_id, public_key), &[])?.try_into()
    }

    fn signer(&self) -> Arc<dyn Signer> {
        self.signer.clone()
    }

    fn set_signer(&mut self, signer: Arc<dyn Signer>) {
        self.signer = signer;
    }
}
