# Process from terminal:
# $ docker build . --target build -t near-evm:build
# $ docker build . --cache-from=near-evm:build -t near-evm:0.1.0

FROM ethereum/solc:0.5.17-alpine AS solc

FROM rust:1.43-buster AS build

WORKDIR /usr/src/near-evm

RUN rustup default nightly-2020-03-19

RUN apt-get update && \
    apt-get upgrade -y && \
    apt-get install -y clang && \
    apt-get install -y libssl-dev libclang-3.9-dev clang-3.9

COPY Cargo.toml ./
COPY Cargo.lock ./

RUN mkdir -p src/tests/contracts

# compile solidity
COPY --from=solc /usr/local/bin/solc /usr/local/bin/solc
COPY src/tests/build.sh ./src/tests/
COPY src/tests/contracts/SolTests.sol ./src/tests/contracts/
RUN cd src/tests && \
    ./build.sh && \
    cd ../../

# dummy test and src
RUN echo "fn main() {}" > src/lib.rs
RUN echo "#[test] fn test_mock() {assert_eq(4, 4)}" > src/tests/mod.rs

RUN cargo update
RUN cargo test --lib

FROM rust:1.43-buster

WORKDIR /usr/src/near-evm

RUN rustup default nightly-2020-03-19
RUN rm -rf src

RUN mkdir ./target
COPY --from=build /usr/src/near-evm/target/debug ./target/debug

RUN rm target/debug/deps/near_evm**

RUN rm -rf /usr/local/cargo
COPY --from=build /usr/local/cargo /usr/local/cargo

COPY --from=build /usr/src/near-evm/Cargo.toml ./
COPY --from=build /usr/src/near-evm/Cargo.lock ./

COPY src src
