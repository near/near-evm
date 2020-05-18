#!/bin/bash

truffle compile

cat build/contracts/SolTests.json | \
  jq .bytecode | \
  awk '{ print substr($1,4,length($1)-4) }' | \
  tr -d '\n' \
  > build/SolTests.bin

cat build/contracts/SolTests.json | \
  jq .abi \
  > build/SolTest.abi

cat build/contracts/SubContract.json | \
  jq .bytecode | \
  awk '{ print substr($1,4,length($1)-4) }' | \
  tr -d '\n' \
  > build/SubContract.bin

cat build/contracts/SubContract.json | \
  jq .abi \
  > build/SubContract.abi

rm -rf build/contracts
