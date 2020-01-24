#!/bin/bash

sed -i "s|# crate\-type|crate\-type|g" Cargo.toml

RUSTFLAGS='-C link-arg=-s' cargo +stable build --target wasm32-unknown-unknown --release
mkdir -p res
cp target/wasm32-unknown-unknown/release/near_evm.wasm ./res/

sed -i "s|crate\-type|# crate\-type|g" Cargo.toml

# wasm-opt -Oz --output ./res/near_evm.wasm ./res/near_evm.wasm
# rm -rf target
