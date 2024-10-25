#!/usr/bin/env bash
set -eu

zepter format features --workspace --fix
zepter lint propagate-feature --feature std,runtime-benchmarks,try-runtime --workspace --fix
cargo fmt
