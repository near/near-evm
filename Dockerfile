# Process from terminal:
# $ docker build . --target build -t near-evm:build
# $ docker build . --cache-from=near-evm:build -t near-evm:0.1.0

FROM rust:1.43-buster AS build

WORKDIR /usr/src/near-evm

RUN rustup default nightly-2020-03-19

COPY Cargo.toml ./
COPY Cargo.lock ./
COPY src/tests/build ./src/tests/build/
COPY build.sh ./

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y clang && \
    apt-get install -y libssl-dev libclang-3.9-dev clang-3.9 && \
    mkdir -p src && \
    echo "fn main() {}" > src/lib.rs && \
    mkdir -p src/tests && \
    echo "#[test] fn test_mock() {assert_eq(4, 4)}" > src/tests/mod.rs && \
    rustup target add wasm32-unknown-unknown && \
    cargo build --target wasm32-unknown-unknown --release && \
    ./build.sh && \
    cargo test --lib

FROM rust:1.43-buster

WORKDIR /usr/src/near-evm

RUN rustup default nightly-2020-03-19

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y clang && \
    apt-get install -y libssl-dev libclang-3.9-dev clang-3.9 && \
    rustup target add wasm32-unknown-unknown && rustup component add rustfmt


COPY --from=build /usr/src/near-evm/target/ ./target/
COPY --from=build /usr/src/near-evm/res/ ./res/
COPY --from=build /usr/src/near-evm/Cargo.toml ./Cargo.toml
COPY --from=build /usr/src/near-evm/Cargo.lock ./Cargo.lock


# this hacky hack worked for a smaller test repo, but this repo still rebuilds from scratch.
RUN cd target/debug/deps && rm *near_evm* && cd ../../..

COPY src/ ./src/

RUN cargo build --target wasm32-unknown-unknown --release && \
    cargo test --lib
