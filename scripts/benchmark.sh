#!/usr/bin/env bash

pallets=(nft-permission nft-delegation)
for pallet in "${pallets[@]}"; do
    target/release/mosaic-testnet-solo benchmark pallet --pallet "pallet-$pallet" --extrinsic '*' --steps=50 --repeat=20 --wasm-execution=compiled --output "pallets/$pallet/src/weights.rs" --template scripts/benchmark-pallet-weights.hbs
done
