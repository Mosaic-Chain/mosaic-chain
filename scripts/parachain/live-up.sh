#!/usr/bin/env bash
set -eu

if [[ $# != 2 ]]; then
  echo "Usage: $0 <raw parachain spec> <raw relay-chain spec>"
  exit 1
fi


PARA_SPEC="$1"
RELAY_SPEC="$2"
NODE="./target/release/mosaic-chain"

if [ ! -f $PARA_SPEC ]; then
  >&2 echo "Please provide a valid raw parachain spec file!"
  exit 2
fi;

if [ ! -f $RELAY_SPEC ]; then
  >&2 echo "Please provide a valid raw relay-chain spec file!"
  exit 3
fi;


if [ ! -f $NODE ]; then
  >&2 echo "Please compile parachain node in release mode!"
  exit 4
fi;

run_collator() {
  local ids=(alice bob charlie dave eve ferdie)
  local iden=${ids[$1]}
  local base_path="/tmp/live-parachain/$iden"

  $NODE --collator --chain $PARA_SPEC --base-path $base_path \
  --pruning archive --port $((50333 + $1)) --rpc-port $((7744 + $1)) -- \
  --chain "$RELAY_SPEC" --port $((50343 + $1)) --rpc-port $((7754 + $1))&

}

for i in $(seq 0 3); do 
  run_collator $i
done

trap "trap - SIGTERM && kill -9 -- $$" SIGINT SIGTERM EXIT
while true; do read; done
