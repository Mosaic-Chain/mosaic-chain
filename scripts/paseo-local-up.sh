#!/usr/bin/env bash
set -eu

if [[ $# -eq 0 ]]; then
  echo "Usage: relay-chain-up.sh <polkadot executable> <raw paseo chainspec>"
  exit 1
fi

POLKADOT="$1"
CHAIN="$2"

node_key() {
  printf "%064d" $1
}

node_id() {  
  node_key $1 | $POLKADOT key inspect-node-key
}

run_node() {
    local ids=(alice bob charlie dave eve ferdie)
    local iden=${ids[$1]}
    local base_path="/tmp/relay/$iden"

    $POLKADOT --$(echo $iden) --validator --pruning archive --base-path $base_path --chain $CHAIN \
    --port $((30333 + $1)) --rpc-port $((9944 + $1)) --node-key $(node_key $((1 + $1))) --bootnodes "/ip4/127.0.0.1/tcp/30333/p2p/$(node_id 1)"&

}

for i in $(seq 0 3); do
  run_node $i
done

trap "trap - SIGTERM && kill -9 -- $$" SIGINT SIGTERM EXIT
while true; do read; done
