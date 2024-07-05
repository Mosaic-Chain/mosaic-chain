#!/usr/bin/env bash

set -eu

HASH=$(git describe --always)
echo "$(grep -o -m 1 '^version = "[^"]*"' Cargo.toml | cut -d '"' -f2)+$HASH"
