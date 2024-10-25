#!/usr/bin/env bash

zepter format features --workspace &&
zepter lint propagate-feature --feature std,runtime-benchmarks,try-runtime --workspace &&
cargo fmt --check &&
cargo clippy --color=always --tests --examples --bins --benches --release --all-features --no-deps -- -D warnings &&
cargo nextest run --release --features try-runtime -E "not (test(test_genesis_config_builds))" # These tests fail if mock runtime can't be built from *default* genesis config

if [ $? -ne 0 ]; then
    echo "Checks failed. Fix errors before committing. Try running ./scripts/fix.sh"
    exit 1
fi
