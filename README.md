# Near-evm

EVM interpreter as a NEAR smart contract.

It uses the EVM interpreter from [parity-ethereum](https://github.com/paritytech/parity-ethereum/).

### Pre-requisites
To develop Rust contracts you would need to:
* Install [Rustup](https://rustup.rs/):
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

* Add wasm target to your toolchain (if you already have installed Rust make sure to switch to `rustup default stable`):
```bash
rustup target add wasm32-unknown-unknown
```

### Building

```shell
$ ./build.sh
```

This will build the contract code in `res/near_evm.wasm`.

### Usage

Deploy contract on TestNet:

* Make sure you have the newest version of near-shell installed by running:
```shell
npm install -g near-shell
```

* If you are using TestNet, call `near login` (if you are using local node use `NODE_ENV=development` before commands below).

* Create contract's account, e.g. we will use `evm`:
```shell
near create_account evm --masterAccount=<account you used in near login/test.near for local>
```

* Deploy the compiled contract from `res/near_evm.wasm` at the building step:
```shell
near deploy --accountId=evm --wasmFile=res/near_evm.wasm
```

* TODO: hackery to actually deploy your EVM contract

### Testing

1. Build the evm contract
    1. Ensure truffle is installed
      ```sh
      npm i -g truffle
      ```
    1. Build the test contracts
      ```sh
      cd src/tests && ./build.sh
      ```
    2. Build the Near EVM contract binary
      ```sh
      cd ../.. && ./build.sh`
      ```
2. Run the all tests including integration test
      ```sh
      cargo test --lib`
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
