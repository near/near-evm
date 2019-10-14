# Near-evm

EVM interpreter as a NEAR smart contract.

It uses the EVM interpreter from [parity-ethereum](https://github.com/paritytech/parity-ethereum/).

### Building

```shell
$ ./build.sh
```

This will build the contract code in `res/near_evm.wasm`.


### Usage

1. Run a local NEAR node
    1. checkout `nearcore`
    1. `python scripts/start_unittest.py --local` 
1. Build the evm contract
    1. `./build.sh` - this will build the contract code in `res/near_evm.wasm`
1. Run the cryptozombies integration test
    1. Remove `cdylib` from crate-type in Cargo.toml
    1. `cargo test -p near-evm --test cryptozombies_rpc test_zombie -- --exact --nocapture` - 
    this will deploy the evm contract, then deploy cryptozombies and run some functions.
