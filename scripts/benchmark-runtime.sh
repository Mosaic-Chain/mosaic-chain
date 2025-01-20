#!/usr/bin/env bash

if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Usage: $0 <mosaic-chain> <benchmark-pallet-weights.hbs> [doas ... vesting-to-freeze]"
    exit 1
fi

if [ "$#" -gt 0 ]; then
  node="$1"
  shift 1
else
  node="mosaic-chain-benchmark"
fi
if [ "$#" -gt 0 ]; then
  template="$1"
  shift 1
else
  template="/usr/share/mosaic-chain-benchmark/benchmark-runtime-weights.hbs"
fi

if [ $# -eq 0 ]; then
  pallets=(doas hold-vesting im-online nft-delegation nft-permission nft-staking staking-incentive treasury validator-subset-selection vesting-to-freeze)
else
  pallets=("$@")
fi

mkdir -p weights
for pallet in "${pallets[@]}"; do
    echo " ⏳ ⏱  Running runtime benchmarks for pallet-$pallet  ⏱ ⏳ "

    touch "weights/$pallet.rs"
    pallet_mod=$(echo $pallet | tr '-' '_')
    "$node" benchmark pallet --pallet "pallet-$pallet" --extrinsic '*' --steps=50 --repeat=20 --wasm-execution=compiled --output "weights/$pallet_mod.rs" --template "$template"
done
