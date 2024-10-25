#!/usr/bin/env bash

set -eu

SCRIPT_DIR=$(dirname $0)

cp "$SCRIPT_DIR/check.sh" "$SCRIPT_DIR/../.git/hooks/pre-commit"
