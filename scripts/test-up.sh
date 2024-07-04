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
  local base_path="/tmp/parachain/$iden"

  $NODE --$(echo $iden) --collator --chain local-raw.json --base-path $base_path \
  --pruning archive --port $((40333 + $1)) --rpc-port $((8844 + $1)) --unsafe-force-node-key-generation -- \
  --chain "$RELAY_SPEC" --port $((30343 + $1)) --rpc-port $((9977 + $1)) --unsafe-force-node-key-generation&

}

for i in $(seq 0 5); do 
  run_collator $i
done

trap "trap - SIGTERM && kill -9 -- $$" SIGINT SIGTERM EXIT
while true; do read; done
