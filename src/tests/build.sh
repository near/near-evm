#!/bin/bash

truffle compile || exit 1

cat build/contracts/SolTests.json | \
  jq .bytecode | \
  awk '{ print substr($1,4,length($1)-4) }' | \
  tr -d '\n' \
  > build/SolTests.bin

cat build/contracts/SolTests.json | \
  jq .abi \
  > build/SolTests.abi

cat build/contracts/SubContract.json | \
  jq .bytecode | \
  awk '{ print substr($1,4,length($1)-4) }' | \
  tr -d '\n' \
  > build/SubContract.bin

cat build/contracts/SubContract.json | \
  jq .abi \
  > build/SubContract.abi

cat build/contracts/Create2Factory.json | \
  jq .bytecode | \
  awk '{ print substr($1,4,length($1)-4) }' | \
  tr -d '\n' \
  > build/Create2Factory.bin

cat build/contracts/Create2Factory.json | \
  jq .abi \
  > build/Create2Factory.abi

cat build/contracts/SelfDestruct.json | \
  jq .bytecode | \
  awk '{ print substr($1,4,length($1)-4) }' | \
  tr -d '\n' \
  > build/SelfDestruct.bin

cat build/contracts/SelfDestruct.json | \
  jq .abi \
  > build/SelfDestruct.abi

rm -rf build/contracts
