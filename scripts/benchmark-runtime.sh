#!/usr/bin/env bash

if [ "$1" = "--help" ] || [ "$1" = "-h" ]; then
    echo "Usage: $0 <mosaic-chain> <benchmark-pallet-weights.hbs> [pallet_doas ... pallet_vesting_to_freeze]"
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
  readarray -t pallets < <($node benchmark pallet --list=pallets --no-csv-header)
else
  pallets=("$@")
fi

mkdir -p weights
for pallet_prefixed in "${pallets[@]}"; do
    echo " ⏳ ⏱  Running runtime benchmarks for $pallet_prefixed  ⏱ ⏳ "

    pallet_mod="${pallet_prefixed#pallet_}"
    pallet_mod="${pallet_mod#cumulus_pallet_}"
    pallet_arg=$(echo $pallet_prefixed | tr '_' '-')
    
    touch "weights/$pallet_mod.rs"
    "$node" benchmark pallet --pallet "$pallet_arg" --extrinsic '*' --steps=50 --repeat=20 --wasm-execution=compiled --output "weights/$pallet_mod.rs" --template "$template"
done
