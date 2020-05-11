# Process from terminal:
# $ docker build . --target build -t near-evm:build
# $ docker build . --cache-from=near-evm:build -t near-evm:0.1.0

FROM rust:1.43-buster AS build

WORKDIR /usr/src/near-evm

RUN rustup default nightly-2020-03-19

COPY Cargo.toml ./
COPY Cargo.lock ./

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y clang && \
    apt-get install -y libssl-dev libclang-3.9-dev clang-3.9 && \
    mkdir -p src && \
    echo "fn main() {}" > src/lib.rs && \
    mkdir -p src/tests && \
    echo "#[test] fn test_mock() {assert_eq(4, 4)}" > src/tests/mod.rs && \
    cargo build -Z unstable-options --out-dir /output && \
    cargo test --lib

FROM rust:1.43-buster

WORKDIR /usr/src/near-evm

RUN rustup default nightly-2020-03-19

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y clang && \
    apt-get install -y libclang-3.9-dev clang-3.9

COPY --from=build /usr/src/near-evm/target/debug/deps ./target/debug/deps
# this hacky hack worked for a smaller test repo, but this repo still rebuilds from scratch.
RUN cd target/debug/deps && rm *near_evm* && cd ../../..

COPY . .
RUN cargo test --lib
