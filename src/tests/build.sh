#!/bin/bash

truffle compile

cat build/contracts/SolTests.json | \
  jq .bytecode | \
  awk '{ print substr($1,4,length($1)-4) }' | \
  tr -d '\n' \
  > build/soltest.bin

cat build/contracts/SolTests.json | \
  jq .abi \
  > build/soltest.abi

cat build/contracts/SubContract.json | \
  jq .bytecode | \
  awk '{ print substr($1,4,length($1)-4) }' | \
  tr -d '\n' \
  > build/subcontract.bin

cat build/contracts/SubContract.json | \
  jq .abi \
  > build/subcontract.abi
