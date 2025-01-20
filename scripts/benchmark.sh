#!/usr/bin/env bash

if [ $# -eq 0 ]; then
  pallets=(doas hold-vesting im-online nft-delegation nft-permission nft-staking staking-incentive treasury validator-subset-selection vesting-to-freeze)
else
  pallets=("$@")
fi

for pallet in "${pallets[@]}"; do
    echo " ⏳ ⏱  Running benchmarks for pallet-$pallet  ⏱ ⏳ "

    target/release/mosaic-testnet-solo benchmark pallet --pallet "pallet-$pallet" --extrinsic '*' --steps=50 --repeat=20 --wasm-execution=compiled --output "pallets/$pallet/src/weights.rs" --template scripts/benchmark-pallet-weights.hbs
done
