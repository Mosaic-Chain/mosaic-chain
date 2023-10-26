#!/bin/bash

SCRIPT_DIR=$(dirname $(readlink -f $0))

$SCRIPT_DIR/../../target/release/mosaic-chain \
    benchmark pallet \
    --chain=dev \
    --wasm-execution=compiled \
    --pallet=pallet_validator_subset_selection \
    --extrinsic=* \
    --steps=50 \
    --repeat=20 \
    --output=$SCRIPT_DIR/src/test_weights.rs
