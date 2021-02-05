#!/bin/bash

cargo +nightly build -p near-evm --lib --target wasm32-unknown-unknown --release --no-default-features --features=contract -Z avoid-dev-deps || exit 1

mkdir -p res
cp target/wasm32-unknown-unknown/release/near_evm.wasm ./res/near_evm_symbols.wasm

# wasm-opt -Oz --output ./res/near_evm.wasm ./res/near_evm.wasm
ls -lh res/
# rm -rf target
