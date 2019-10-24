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
1. Run the all tests including integration test
    1. `cargo test --features env_test -- --nocapture` -
    this will deploy the evm contract, then deploy cryptozombies and run some functions.
