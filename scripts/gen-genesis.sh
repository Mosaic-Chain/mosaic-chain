#!/usr/bin/env bash

CHAIN=${1:-local}
NODE="./target/release/parachain-template-node"

if [ ! -f $NODE ]; then
  >&2 echo "Please compile parachain node in release mode!"
  exit 3
fi;

# TODO: change this to live!!
$NODE build-spec --chain $CHAIN > $CHAIN-plain.json

read -r -p 'Waiting for you to add bootstrap nodes.'

$NODE build-spec --chain $CHAIN-plain.json --raw > $CHAIN-raw.json
$NODE export-genesis-wasm --chain $CHAIN-raw.json $CHAIN-wasm
$NODE export-genesis-state --chain $CHAIN-raw.json $CHAIN-genesis-state
