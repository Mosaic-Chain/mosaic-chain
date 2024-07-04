#!/usr/bin/env bash
set -eu

if [[ $# -eq 0 ]]; then
  echo "Usage: $0 <raw relay-chain spec>"
  exit 1
fi

RELAY_SPEC="$1"
NODE="./target/release/parachain-template-node"


if [ ! -f $RELAY_SPEC ]; then
  >&2 echo "Please provide a valid raw relay-chain spec file!"
  exit 2
fi;


if [ ! -f $NODE ]; then
  >&2 echo "Please compile parachain node in release mode!"
  exit 3
fi;

run_collator() {
  local ids=(alice bob charlie dave eve ferdie)
  local iden=${ids[$1]}
  local base_path="/tmp/parachain2/$iden"

  $NODE --collator --chain live-raw.json --base-path $base_path \
  --pruning archive --port $((50333 + $1)) --rpc-port $((7744 + $1)) --network-backend litep2p -- \
  --chain "$RELAY_SPEC" --port $((50343 + $1)) --rpc-port $((7754 + $1)) --network-backend litep2p&

}

for i in $(seq 0 3); do 
  run_collator $i
done

trap "trap - SIGTERM && kill -9 -- $$" SIGINT SIGTERM EXIT
while true; do read; done
