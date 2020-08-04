use std::collections::{BTreeMap, HashMap, HashSet};

use ethereum_types::{Address, H256, U256};
use vm::CreateContractAddress;

use borsh::{BorshDeserialize, BorshSerialize};

use near_sdk::collections::{TreeMap as NearTreeMap, UnorderedMap as NearMap, UnorderedSet as NearSet};
use near_sdk::{env, ext_contract, near_bindgen as near_bindgen_macro, AccountId, Promise};

use crate::evm_state::{EvmState, StateStore};
use crate::utils::Balance;

#[cfg(test)]
mod tests;
#[cfg(test)]
#[macro_use]
extern crate lazy_static_include;
#[cfg(test)]
#[macro_use]
extern crate lazy_static;

mod builtins;
mod evm_state;
mod interpreter;
mod near_ext;
pub mod utils;

/// Represents the state of the EVM. All NearMaps are persisted to Near chain storage
///
/// The EVM contract public interface. Generally, the EVM handles ethereum-style 20-byte
/// hex-encoded addresses. External Near accountIDs are converted to EVM addresses by hashing
/// them, and taking the final 20 bytes of the hash. Which is to say, they roughly correspond to
/// Ethereum's externally-owned-account public keys.
///
/// The EVM holds NEAR and keeps an internal balances mapping to all EVM accounts. Therefore EVM
/// contracts can hold NEAR and interact with it.
///
/// # Note:
///
/// Logs are mapped to a byte vector by Length-prepending the topics and then appending the data.
/// E.g. an event with 3 topics will be serialized as `0x03[topic1][topic2][topic3][data]`.
#[near_bindgen_macro]
#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct EvmContract {
    code: NearMap<Vec<u8>, Vec<u8>>,
    balances: NearMap<Vec<u8>, [u8; 32]>,
    nonces: NearMap<Vec<u8>, [u8; 32]>,
    storages: NearTreeMap<Vec<u8>, [u8; 32]>,
    ecrecover_aliases: NearMap<Vec<u8>, [u8; 32]>,
    revoked_aliases: NearSet<Vec<u8>>
}

#[ext_contract]
pub trait Callback {
    fn finalize_retrieve_near(&mut self, addr: Address, amount: Vec<u8>);
}

impl EvmState for EvmContract {

    fn get_ecrecover_alias(&self, address: Address) -> Option<Address> {
        self.ecrecover_aliases.get(&address[..].to_vec()).map(|a| Address::from_slice(&a[12..]))
    }

    // Default code of None
    fn code_at(&self, address: &Address) -> Option<Vec<u8>> {
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self.code.get(&internal_addr.to_vec())
    }

    fn set_code(&mut self, address: &Address, bytecode: &[u8]) {
        let internal_addr = utils::evm_account_to_internal_address(*address);
        self.code
            .insert(&internal_addr.to_vec(), &bytecode.to_vec());
    }

    fn _set_balance(&mut self, address: [u8; 20], balance: [u8; 32]) -> Option<[u8; 32]> {
        self.balances.insert(&address.to_vec(), &balance)
    }

    // default balance of 0
    fn _balance_of(&self, address: [u8; 20]) -> [u8; 32] {
        self.balances.get(&address.to_vec()).unwrap_or([0u8; 32])
    }

    fn _set_nonce(&mut self, address: [u8; 20], nonce: [u8; 32]) -> Option<[u8; 32]> {
        self.nonces.insert(&address.to_vec(), &nonce)
    }

    // default nonce of 0
    fn _nonce_of(&self, address: [u8; 20]) -> [u8; 32] {
        self.nonces.get(&address.to_vec()).unwrap_or([0u8; 32])
    }

    // Default storage of None
    fn _read_contract_storage(&self, key: [u8; 52]) -> Option<[u8; 32]> {
        self.storages.get(&key.to_vec())
    }

    fn _set_contract_storage(&mut self, key: [u8; 52], value: [u8; 32]) -> Option<[u8; 32]> {
        self.storages.insert(&key.to_vec(), &value)
    }

    fn commit_changes(&mut self, other: &StateStore) {
        self.commit_self_destructs(&other.self_destructs);
        self.commit_self_destructs(&other.recreated);
        self.commit_code(&other.code);
        self.commit_balances(&other.balances);
        self.commit_nonces(&other.nonces);
        self.commit_storages(&other.storages);
        for log in &other.logs {
            near_sdk::env::log(format!("evm log: {}", log).as_bytes());
        }
    }
}

#[near_bindgen_macro]
impl EvmContract {
    /// Returns the storage at a particular slot, as a hex-encoded. Slots are 32-bytes wide,
    /// empty slots will be all 0s.
    ///
    /// # Arguments
    ///
    /// * `address` - the hex-encoded account address to read from.
    /// * `key` - the hex-encoded 32-byte storage key to read
    ///
    /// # Panics
    ///
    /// * When `address` or `key` is not valid hex.
    pub fn get_storage_at(&self, address: String, key: String) -> String {
        let mut key_buf = [0u8; 32];
        key_buf.copy_from_slice(&hex::decode(key).expect("invalid storage key"));
        let val = self
            .read_contract_storage(&utils::hex_to_evm_address(&address), key_buf)
            .unwrap_or([0u8; 32]);
        hex::encode(val)
    }

    /// Deploy a new EVM contract. Returns the hex-encoded EVM address where the contract was
    /// deployed.
    ///
    /// # Arguments
    ///
    /// * `bytecode` - the hex-encoded contract initcode as produced by solc or another compiler.
    /// You can find this in truffle build json files as `bytecode`. Note that this is NOT the
    /// contract code. It is the initcode that runs when deploying the contract code.
    ///
    /// # Panics
    ///
    /// * When `bytecode` is not valid hex.
    /// * When the contract encounters a revert during initialization
    #[payable]
    pub fn deploy_code(&mut self, bytecode: String) -> String {
        let code = hex::decode(bytecode).expect("invalid hex");
        let sender = utils::predecessor_as_evm();

        // TODO: move into create
        let nonce = self.next_nonce(&sender);
        let (contract_address, _) = utils::evm_contract_address(
            CreateContractAddress::FromSenderAndNonce,
            &sender,
            &nonce,
            &code,
        );

        let val = utils::attached_deposit_as_u256_opt().unwrap_or_else(U256::zero);
        self.add_balance(&sender, val);

        interpreter::deploy_code(self, &sender, &sender, val, 0, &contract_address, &code);
        hex::encode(&contract_address)
    }

    /// Get the code deployed at an address. Returns the hex-encoded code at that address, or the
    /// empty string if no code is deployed.
    ///
    /// # Arguments
    ///
    /// * `address` - the hex-encoded account address to read from.
    ///
    /// # Panics
    ///
    /// * When `address` is not valid hex.
    pub fn get_code(&self, address: String) -> String {
        let address = utils::hex_to_evm_address(&address);
        hex::encode(self.code_at(&address).unwrap_or_else(Vec::new))
    }

    /// Make an EVM transaction. Calls `contract_address` with `encoded_input`. Execution
    /// continues until all EVM messages have been processed. We expect this to behave identically
    /// to an Ethereum transaction, however there may be some edge cases.
    ///
    /// # Arguments
    ///
    /// * `contract_address` - the hex-encoded account address to call.
    /// * `encoded_input` - The hex-encoded data field of the Ethereum transaction. This typically
    /// contains the abi-encoded contract call arguments.
    ///
    /// # Panics
    ///
    /// * When `contract_address` or `encoded_input` is not valid hex.
    #[payable]
    pub fn call_contract(&mut self, contract_address: String, encoded_input: String) -> String {
        let contract_address = utils::hex_to_evm_address(&contract_address);
        let sender = utils::near_account_id_to_evm_address(&env::predecessor_account_id());

        let value = utils::attached_deposit_as_u256_opt();
        if let Some(val) = value {
            self.add_balance(&utils::predecessor_as_evm(), val);
        }
        let result =
            self.call_contract_internal(value, &contract_address, encoded_input, &sender, true);

        match result {
            Ok(v) => hex::encode(v),
            Err(s) => format!("internal call failed: {}", s),
        }
    }

    /// Move internal EVM balance to the EVM account corresponding to a specific Near account.
    /// This generally functions as an ethereum transfer, but will NOT trigger fallback functions.
    ///
    /// # Arguments
    ///
    /// * `address` - the NEAR account to credit EVM balance to.
    /// * `amount` - the number of yoctoNEAR to move
    ///
    /// # Panics
    ///
    /// * If the sender does not have sufficient NEAR balance in the EVM.
    pub fn move_funds_to_near_account(&mut self, address: AccountId, amount: Balance) -> bool {
        let sender = utils::predecessor_as_evm();
        let recipient = utils::near_account_id_to_evm_address(&address);
        let amount = utils::balance_to_u256(&amount);
        self.transfer_balance(&sender, &recipient, amount);
        true // transfer successful: a return val is needed for web3 compatibility
    }

    /// Move internal EVM balance to another EVM account.
    /// This generally functions as an ethereum transfer, but will NOT trigger fallback functions.
    ///
    /// # Arguments
    ///
    /// * `address` - the EVM account to credit EVM balance to.
    /// * `amount` - the number of yoctoNEAR to move
    ///
    /// # Panics
    ///
    /// * If `address` is not valid hex.
    /// * If the sender does not have sufficient NEAR balance in the EVM.
    pub fn move_funds_to_evm_address(&mut self, address: String, amount: Balance) -> bool {
        let recipient = utils::hex_to_evm_address(&address);
        let sender = utils::predecessor_as_evm();
        let amount = utils::balance_to_u256(&amount);
        self.sub_balance(&sender, amount);
        self.add_balance(&recipient, amount);
        true // transfer successful: a return val is needed for web3 compatibility
    }

    /// Returns the EVM balance of a Near AccountId.
    ///
    /// # Arguments
    ///
    /// * `address` - the Near account to check.
    pub fn balance_of_near_account(&self, address: AccountId) -> Balance {
        let addr = utils::near_account_id_to_evm_address(&address);
        utils::u256_to_balance(&self.balance_of(&addr))
    }

    /// Returns the EVM balance of an EVM address.
    ///
    /// # Arguments
    ///
    /// * `address` - the EVM account to check.
    ///
    /// # Panics
    ///
    /// * When `address` is not valid hex.
    pub fn balance_of_evm_address(&self, address: String) -> Balance {
        let addr = utils::hex_to_evm_address(&address);
        utils::u256_to_balance(&self.balance_of(&addr))
    }

    /// Credits near to the EVM account corresponding to the sending Near accountId. Used to fund
    /// EVM accounts so that the user can interact with contracts.
    ///
    /// # Panics
    ///
    /// * When no NEAR is attached to the call.
    #[payable]
    pub fn add_near(&mut self) -> Balance {
        let val = utils::attached_deposit_as_u256_opt().expect("Did not attach value");
        let addr = &utils::predecessor_as_evm();

        self.add_balance(&addr, val);
        utils::u256_to_balance(&self.balance_of(&addr))
    }

    /// Transfers NEAR out of the EVM to some Near account. Always transfers from the EVM account
    /// corresponding to the caller's near accountId.
    ///
    /// # Arguments
    ///
    /// * `recipient` - the Near accountId to which to transfer funds.
    /// * `amount` - the number of yoctoNEAR to transfer to `recipient`
    ///
    /// # Panics
    ///
    /// * If the sender does not have sufficient NEAR balance in the EVM.
    pub fn retrieve_near(&mut self, recipient: AccountId, amount: Balance) -> bool {
        let addr = utils::near_account_id_to_evm_address(&env::predecessor_account_id());

        if utils::u256_to_balance(&self.balance_of(&addr)) < amount {
            panic!("insufficient funds");
        }

        Promise::new(recipient)
            .transfer(amount.0)
            .then(callback::finalize_retrieve_near(
                addr,
                amount.to_be_bytes().to_vec(),
                &env::current_account_id(),
                0,
                (env::prepaid_gas() - env::used_gas()) / 2,
            ));
        true // retrieval successful: a return val is needed for web3 compatibility
    }

    /// Internal method. Updates EVM accounting.
    /// TODO: check safety
    pub fn finalize_retrieve_near(&mut self, addr: Address, amount: Vec<u8>) {
        let mut bin = [0u8; 16];
        bin.copy_from_slice(&amount[..]);
        // panics if called externally
        assert_eq!(
            env::current_account_id(),
            env::predecessor_account_id(),
            "caller is not self"
        );
        // panics if insufficient balance
        self.sub_balance(&addr, utils::balance_to_u256(&Balance::from_be_bytes(bin)));
    }

    /// Get the EVM nonce of a Near account. Unlike in the EVM, this is only incremented when
    /// deploying code. This function can be useful for predicting the address code will deploy
    /// at.
    ///
    /// # Arguments
    ///
    /// * `address` - the Near account to check.
    pub fn nonce_of_near_account(&self, address: AccountId) -> Balance {
        let addr = utils::near_account_id_to_evm_address(&address);
        utils::u256_to_balance(&self.nonce_of(&addr))
    }

    /// Get the EVM nonce of an EVM account. Unlike in the EVM, this is only incremented when
    /// deploying code. This function can be useful for predicting the address code will deploy
    /// at.
    ///
    /// # Arguments
    ///
    /// * `address` - the EVM account to check.
    ///
    /// # Panics
    ///
    /// * When `address` is not valid hex.
    pub fn nonce_of_evm_address(&self, address: String) -> Balance {
        let addr = utils::hex_to_evm_address(&address);
        utils::u256_to_balance(&self.nonce_of(&addr))
    }


    /// Register an ECDSA alias. This is an ECDSA key that is allowed to make updates and sign
    /// messages on behalf of your Near-EVM account. This allows Near-EVM contracts to effectively
    /// use EcRecover to seamlessly use Ethereum metatransactions and other signature-driven
    /// features without integrating Near's signature scheme into the Near-EVM.
    ///
    /// Users may register multiple aliases, which are all effective. To revoke an alias, use
    /// `revoke_ecdsa_alias`. The recommended usage is to give keys explicit, temporary lifespans
    /// in the application.
    ///
    /// To register an alias, the key must sign the Near-EVM address, AND the Near account
    /// corresponding to that address must submit the signature. This prevents replay attacks,
    /// and registering keys that are not actually controlled by the Near account holder.
    ///
    /// The message format uses the standard `Ethereum Signed Message` format, and can be produced
    /// by `eth_sign` or `eth_signPersonal`. The body of the message is the account ID prepended
    /// by the string "Near ecrecover alias: ". Note the lack of new line.
    ///
    /// # Note
    ///
    /// The `signature` argument is a hex string encoding the sigature in RSV format. This is the
    /// typical output of a call to `personal_sign` in Metamask. It is usually exactly 65 bytes
    /// long (130 hex characters), but may be longer if the chain ID is set to a high value. The
    /// encoded string is NOT prepended by "0x".
    ///
    /// This contrasts with the VRS representation used by the EVM internally, and the two should
    /// not be confused.
    ///
    /// # Arguments
    ///
    /// * `signature` - a valid ethereum-formatted signature
    ///
    /// # Returns
    ///
    /// True if succesfully registered. False if the key was previously revoked, or is registered elsewhere.
    ///
    /// # Panics
    ///
    /// * When `signature` is not valid hex.
    /// * When the signature is invalid.
    pub fn register_ecdsa_alias(&mut self, signature: String) -> bool {
        let body = format!("Near ecrecover alias: {}", env::predecessor_account_id());
        let mut message = format!("\x19Ethereum Signed Message:\n{}", body.len());
        message.push_str(&body);

        let signature_bytes = hex::decode(&signature).expect("valid hex input");
        let signature_bytes = utils::parse_rsv(&signature_bytes);

        // ecrecover expects digest, v, r, s to be 32 bytes each.
        let mut ecdsa_input = vec![];
        ecdsa_input.extend(keccak_hash::keccak(&message).as_bytes());
        ecdsa_input.extend(&signature_bytes[..]);

        let mut output = vec![];
        crate::builtins::run_stateless(
            &Address::from_low_u64_be(1),
            &ecdsa_input,
            &mut output
        );

        assert!(output.len() == 32, "Invalid ECDSA signature.");

        let signer = Address::from_slice(&output[12..32]);
        if self.revoked_aliases.contains(&signer[..].to_vec()) || self.ecrecover_aliases.get(&signer[..].to_vec()).is_some() {
            return false;
        }

        self.set_ecrecover_alias(
            &signer,
            &utils::predecessor_as_evm(),
        );
        true
    }


    /// Allow a Near account to permanently revoke an ECDSA alias it controls. The Near account
    /// must have previously registered the alias, and it must still be active (not previously
    /// revoked).
    ///
    /// Once an ECDSA alias is revoked, it may NEVER be used to alias any account again. This is a
    /// permanent, global, revocation.
    ///
    /// # Arguments
    ///
    /// * `signature` - a valid ethereum-formatted signature
    ///
    /// # Returns
    ///
    /// True if succesfully revoked False if the key was never registered.
    ///
    /// # Warning
    ///
    /// This CANNOT be undone. The key can NEVER be aliased again.
    ///
    /// # Panics
    ///
    /// - When alias_address is not a valid EVM address
    pub fn revoke_ecdsa_alias(&mut self, alias_address: String) -> bool {
        let sender = utils::predecessor_as_evm();
        let alias = utils::hex_to_evm_address(&alias_address);

        if self.get_ecrecover_alias(alias) == Some(sender) {
            self.set_ecrecover_alias(&alias, &alias);
            self.revoked_aliases.insert(&alias[..].to_vec());
            true
        } else {
            false
        }

    }
}

/// view_call_contract cannot implement #[near_bindgen_macro] because it is treated as a transaction call
/// and will result in: "View Functions Error: attached_deposit prohibited"
impl EvmContract {
    /// Make an EVM transaction. Calls `contract_address` with `encoded_input`. Execution
    /// continues until all EVM messages have been processed. We expect this to behave identically
    /// to an Ethereum transaction, however there may be some edge cases.
    ///
    /// This function serves the eth_call functionality, and will NOT apply state changes.
    ///
    /// # Arguments
    ///
    /// * `contract_address` - the hex-encoded account address to call.
    /// * `encoded_input` - the hex-encoded data field of the Ethereum transaction. This typically
    /// contains the abi-encoded contract call arguments.
    /// * `sender` - the hex-encoded sender of the view call. Used to pre-flight txns as if they
    /// were from the specified account.
    /// * `value` - the number of yoctoNEAR to simulate the call with. Sets msg.balance, but will
    /// NOT actually be transferred.
    ///
    /// # Panics
    ///
    /// * When `contract_address` or `encoded_input` or `sender` is not valid hex.
    pub fn view_call_contract(
        &mut self,
        contract_address: String,
        encoded_input: String,
        sender: String,
        value: Balance,
    ) -> String {
        let sender = utils::near_account_id_to_evm_address(&sender);
        let contract_address = utils::hex_to_evm_address(&contract_address);
        let val = match value {
            Balance(0) => None,
            Balance(v) => Some(U256::from(v)),
        };

        let result =
            self.call_contract_internal(val, &contract_address, encoded_input, &sender, false);

        match result {
            Ok(v) => hex::encode(v),
            Err(s) => format!("internal call failed: {}", s),
        }
    }
}
/// implement #[near_bindgen_macro] functionality for view_call_contract except for attached_deposit
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn view_call_contract() {
    near_sdk::env::setup_panic_hook();
    near_sdk::env::set_blockchain_interface(Box::new(near_blockchain::NearBlockchain {}));

    #[derive(serde::Deserialize, serde::Serialize)]
    struct Input {
        contract_address: String,
        encoded_input: String,
        sender: String,
        value: Balance,
    }
    let Input {
        contract_address,
        encoded_input,
        sender,
        value,
    }: Input = serde_json::from_slice(
        &near_sdk::env::input().expect("Expected input since method has arguments."),
    )
    .expect("Failed to deserialize input from JSON.");

    let mut contract: EvmContract = near_sdk::env::state_read().unwrap_or_default();
    let result = contract.view_call_contract(contract_address, encoded_input, sender, value);
    let result =
        serde_json::to_vec(&result).expect("Failed to serialize the return value using JSON.");
    near_sdk::env::value_return(&result);
}

impl EvmContract {
    fn set_ecrecover_alias(&mut self, address: &Address, alias_to: &Address) {
        self.ecrecover_aliases.insert(
            &address.as_bytes().to_vec(),
            &H256::from(*alias_to).to_fixed_bytes(),
        );
    }

    fn commit_code(&mut self, other: &HashMap<[u8; 20], Vec<u8>>) {
        self.code
            .extend(other.iter().map(|(k, v)| (k.to_vec(), v.clone())));
    }

    fn commit_balances(&mut self, other: &HashMap<[u8; 20], [u8; 32]>) {
        self.balances
            .extend(other.iter().map(|(k, v)| (k.to_vec(), *v)));
    }

    fn commit_nonces(&mut self, other: &HashMap<[u8; 20], [u8; 32]>) {
        self.nonces
            .extend(other.iter().map(|(k, v)| (k.to_vec(), *v)));
    }

    fn commit_storages(&mut self, other: &BTreeMap<Vec<u8>, [u8; 32]>) {
        for (k, v) in other.iter() {
            self.storages.insert(k, v);
        }
    }

    fn clear_contract_storage(&mut self, address_key: Vec<u8>) {
        let mut next_address_key = address_key.clone();
        *(next_address_key.last_mut().unwrap()) += 1;

        let range = (
            std::ops::Bound::Excluded(address_key),
            std::ops::Bound::Excluded(next_address_key),
        );

        let keys: Vec<_> = self.storages.range(range).map(|(k, _)| k).collect();
        for k in keys.iter() {
            self.storages.remove(k);
        }
    }

    fn clear_contract_info(&mut self, addr: &[u8; 20]) {
        let key = addr.to_vec();
        self.nonces.remove(&key);
        self.balances.remove(&key);
        self.code.remove(&key);
        self.clear_contract_storage(key);
    }

    fn commit_self_destructs(&mut self, other: &HashSet<[u8; 20]>) {
        for addr in other.iter() {
            self.clear_contract_info(addr)
        }
    }

    fn call_contract_internal(
        &mut self,
        value: Option<U256>,
        contract_address: &Address,
        encoded_input: String,
        sender: &Address,
        should_commit: bool,
    ) -> Result<Vec<u8>, String> {
        // decode
        let input = encoded_input;
        let input = hex::decode(input).expect("invalid hex");

        // run
        let result = interpreter::call(
            self,
            sender,
            sender,
            value,
            0, // call-stack depth
            &contract_address,
            &input,
            should_commit,
        );

        result.map(|v| v.to_vec())
    }
}
