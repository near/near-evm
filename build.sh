#!/bin/bash

RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/near_evm.wasm ./res/
#wasm-opt -Oz --output ./res/cross_contract.wasm ./res/near_evm.wasm
rm -rf target
