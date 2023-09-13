#!/bin/bash

./target/release/node-template \
    benchmark pallet \
    --chain=dev \
    --wasm-execution=compiled \
    --pallet=pallet_validator_subset_selection \
    --extrinsic=* \
    --steps=50 \
    --repeat=20 \
    --output=pallets/validator-subset-selection/src/test_weights.rs