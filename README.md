:warning: This repository contains obsolete EVM contract experiments.

Find current development at: https://github.com/aurora-is-near/aurora-engine

# NEAR EVM

EVM interpreter as a NEAR smart contract. This uses the EVM interpreter from [SputnikVM].

Network  | Account
:------- | :-----------------------
LocalNet | `evm.test.near`
BetaNet  | `evm.$MYACCOUNT.betanet`
TestNet  | `evm.$MYACCOUNT.testnet`

### Prerequisites

To develop Rust contracts, change into the top-level directory in this
repository, and do the following:

1. Make sure you have the newest version of the [NEAR CLI] installed by running:

  ```shell
  npm install -g near-cli
  ```

2. Install [Rustup](https://rustup.rs):

  ```shell
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

3. Add a WebAssembly target to your Rust toolchain:

  ```shell
  rustup target add wasm32-unknown-unknown
  ```

### Building

```shell
./build.sh
```

This will build the contract code in `res/near_evm.wasm`.

### Deployment

Deploy the EVM contract:

* If you are using BetaNet or TestNet, execute `near login`.

* If you are using LocalNet, set the `NODE_ENV=local` environment variable
  prior to executing any of the commands below.

* Create the contract account:

  ```shell
  # LocalNet
  near create-account evm.test.near --masterAccount=test.near

  # BetaNet
  near create-account evm.myaccount.betanet --masterAccount=myaccount.betanet

  # TestNet
  near create-account evm.myaccount.testnet --masterAccount=myaccount.testnet
  ```

* Deploy the built contract from `res/near_evm.wasm`:

  ```shell
  # LocalNet
  near deploy --accountId=evm.test.near --wasmFile=res/near_evm.wasm

  # BetaNet
  near deploy --accountId=evm.myaccount.betanet --wasmFile=res/near_evm.wasm

  # TestNet
  near deploy --accountId=evm.myaccount.testnet --wasmFile=res/near_evm.wasm
  ```

### Testing

1. Build the EVM contract:

    1. Build the NEAR EVM contract binary:
      ```shell
      ./build.sh
      ```
    2. Ensure Truffle is installed:
      ```shell
      npm i -g truffle
      ```
    3. Build the test contracts:
      ```shell
      cd tests && ./build.sh
      ```

2. Run the all tests including integration tests:

      ```shell
      cargo test --lib
      ```

3. To run the RPC tests you must [run a local NEAR node](https://docs.near.org/docs/develop/node/running-a-node):

      1. Check out [`nearcore`](https://github.com/near/nearcore) from GitHub.
      2. Compile and run `nearcore`:
      ```shell
      cd nearcore && python scripts/start_unittest.py --local --release
      ```
    1. Run the tests from this directory in another terminal window:
      ```shell
      cargo test
      ```

### Troubleshooting

You may need to install `nightly` if you get an error similar to the following:

```shell
error[E0554]: `#![feature]` may not be used on the stable release channel
```

1. Install `nightly`:
  ```shell
  rustup toolchain install nightly
  ```
2. Run the [Testing](###Testing) commands again.

[NEAR CLI]:  https://docs.near.org/docs/tools/near-cli
[SputnikVM]: https://github.com/aurora-is-near/sputnikvm
