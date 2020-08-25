#!/bin/bash
./build.sh
NEAR_ENV=local near create-account evm.test.near --initialBalance 10000000 --masterAccount test.near --key-path=/Users/mike/.near/local/validator_key.json
NEAR_ENV=local near create-account 1597857539367.test.near --initialBalance 10000000 --masterAccount test.near --key-path=/Users/mike/.near/local/validator_key.json
NEAR_ENV=local near deploy --accountId=evm.test.near --wasmFile=/Users/mike/near/near-evm/res/near_evm.wasm