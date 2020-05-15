#!/bin/bash

mkdir -p build
solc -o build --bin --abi --overwrite ./contracts/SolTests.sol
