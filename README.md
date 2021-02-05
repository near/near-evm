# NEAR EVM

EVM interpreter as a NEAR smart contract.

It uses the EVM interpreter from [SputnikVM](https://github.com/rust-blockchain/evm).

### Prerequisites

To develop Rust contracts you would need to:

1. Install [Rustup](https://rustup.rs):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Add a WebAssembly target to your Rust toolchain:
```bash
rustup target add wasm32-unknown-unknown --toolchain stable
```

### Building

```shell
$ ./build.sh
```

This will build the contract code in `res/near_evm.wasm`.

### Usage

Deploy contract on TestNet:

* Make sure you have the newest version of the NEAR CLI installed by running:

```shell
npm install -g near-cli
```

* If you are using TestNet, call `near login` (if you are using local node use `NODE_ENV=development` before commands below).

* Create contract's account, e.g. we will use `evm`:
```shell
near create_account evm --masterAccount=<account you used in near login/test.near for local>
```

for *testnet* example (for example basic logged in account: `myaccount.testnet``):
```shell
near create_account subname.myaccount.testnet --masterAccount=myaccount.testnet
```

* Deploy the compiled contract from `res/near_evm.wasm` at the building step:
```shell
near deploy --accountId=evm --wasmFile=res/near_evm.wasm
```

for `testnet`:
```shell
near deploy --accountId=subname.myaccount.testnet --wasmFile=res/near_evm.wasm
```

* TODO: hackery to actually deploy your EVM contract

### Testing

1. Build the evm contract
    1. Build the Near EVM contract binary
      ```sh
      ./build.sh`
      ```
    2. Ensure truffle is installed
      ```sh
      npm i -g truffle
      ```
    3. Build the test contracts
      ```sh
      cd tests && ./build.sh
      ```

2. Run the all tests including integration test
      ```sh
      cargo test --lib
      ```

3. To run the RPC tests you must [run a local NEAR node](https://docs.near.org/docs/local-setup/local-dev-node):
      1. Check out [`nearcore`](https://github.com/nearprotocol/nearcore) from Github
      2. Compile and run `nearcore`
      ```sh
      cd nearcore && python scripts/start_unittest.py --local --release
      ```
    1. Run the tests from this directory in another terminal window:
      ```sh
      cargo test
      ```

#### Troubleshooting

You may need to install `nightly` if you get an error similar to the following:

```sh
error[E0554]: `#![feature]` may not be used on the stable release channel
```

1. Install `nightly`
  ```sh
  rustup toolchain install nightly`
  ```
2. Run the [Testing](###Testing) commands again
