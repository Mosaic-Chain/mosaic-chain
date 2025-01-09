#!/usr/bin/env bash

set -e

if [ -z "$1" ]; then echo "Please provide a chainspec file" && exit 1; fi

CHAIN="$1"
NODE="./target/release/mosaic-chain"

if [ ! -f $NODE ]; then
  >&2 echo "Please compile parachain node in release mode!"
  exit 3
fi;

$NODE build-spec --chain $CHAIN > $CHAIN-plain.json

read -r -p 'Waiting for you to add bootstrap nodes.'

$NODE build-spec --chain $CHAIN-plain.json --raw > $CHAIN-raw.json
$NODE export-genesis-wasm --chain $CHAIN-raw.json $CHAIN-wasm
$NODE export-genesis-state --chain $CHAIN-raw.json $CHAIN-genesis-state
