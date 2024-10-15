#!/usr/bin/env bash
  set -e

  if [ -z "$1" ]; then echo "Please provide a chainspec file" && exit 1; fi
  
  spec="$1"
  ids=(alice bob charlie dave eve ferdie)

  max=6
  for (( i=0; i < $max; i++ ))
  do
    ./target/release/mosaic-testnet-solo purge-chain \
    --base-path /tmp/${ids[i]} \
    --chain $spec -y \

    ./target/release/mosaic-testnet-solo \
    --base-path /tmp/${ids[i]} \
    --chain $spec \
    --network-backend litep2p \
    --${ids[i]} \
    --port $((30333 + i)) \
    --unsafe-rpc-external \
    --rpc-port $((9945 +i)) \
    --node-key 000000000000000000000000000000000000000000000000000000000000000$((1 + i)) \
    --bootnodes "/ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp" \
    --validator \
    --execution wasm \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
   &
  done

  trap "trap - SIGTERM && kill -9 -- $$" SIGINT SIGTERM EXIT
  while true; do read; done
