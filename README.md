# Near-evm

EVM interpreter as a NEAR smart contract.

It uses the EVM interpreter from [parity-ethereum](https://github.com/paritytech/parity-ethereum/).

### Building

Using `wasm-pack`:
```shell
$ wasm-pack build --no-typescript --release
```

This will build the contract code in `pkg/near_evm_bg.wasm`.


### Usage

1. Run a local NEAR node
    1. `cargo run -p near -- init --test-seed seed0` to init the local node
    1. `cargo run -p near -- --verbose run --produce-empty-blocks=false` to run the local node
1. Build the evm contract
    1. `wasm-pack build --no-typescript --release` - this will build the contract code in `pkg/near_evm_bg.wasm`
1. Run the cryptozombies integration test
    1. Remove `cdylib` from crate-type in Cargo.toml
    1. `cargo test -p near-evm --test cryptozombies_rpc test_zombie -- --exact --nocapture` - 
    this will deploy the evm contract, then deploy cryptozombies and run some functions.
