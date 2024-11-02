#!/usr/bin/env bash
set -eu

zepter lint propagate-feature --feature std,runtime-benchmarks,try-runtime --workspace --fix
zepter format features --fix
cargo fmt
