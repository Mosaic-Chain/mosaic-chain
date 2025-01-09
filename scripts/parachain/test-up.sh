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

node_key() {
  printf "%064d" $1
}

node_id() {  
  node_key $1 | $NODE key inspect-node-key
}

run_collator() {
  local ids=(alice bob charlie dave eve ferdie)
  local iden=${ids[$1]}
  local base_path="/tmp/test-parachain/$iden"

  $NODE --$(echo $iden) --collator --chain $PARA_SPEC --base-path $base_path --bootnodes "/ip4/127.0.0.1/tcp/40333/p2p/$(node_id 10)" \
  --pruning archive --port $((40333 + $1)) --rpc-port $((8844 + $1)) --node-key $(node_key $((10 + $1))) -- \
  --chain "$RELAY_SPEC" --port $((30343 + $1)) --rpc-port $((9977 + $1)) --bootnodes "/ip4/127.0.0.1/tcp/30333/p2p/$(node_id 1)" --unsafe-force-node-key-generation &

}

for i in $(seq 0 5); do 
  run_collator $i
done

trap "trap - SIGTERM && kill -9 -- $$" SIGINT SIGTERM EXIT
while true; do read; done
