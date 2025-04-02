#!/usr/bin/env bash
set -x

zepter format features &&
zepter lint propagate-feature --feature std,runtime-benchmarks,try-runtime --workspace &&
cargo fmt --check &&
cargo deny check &&
cargo clippy --color=always --tests --examples --bins --benches --release --all-features --no-deps -- -D warnings &&
cargo nextest run --release --all-features

if [ $? -ne 0 ]; then
    echo "Checks failed. Fix errors before committing. Try running ./scripts/fix.sh"
    exit 1
fi
