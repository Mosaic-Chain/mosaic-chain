#!/usr/bin/env bash
  ids=(alice bob charlie dave eve ferdie)

  max=6
  for (( i=0; i < $max; i++ ))
  do
    ./target/release/node-template purge-chain \
    --base-path /tmp/${ids[i]} \
    --chain local -y \

    ./target/release/node-template \
    --base-path /tmp/${ids[i]} \
    --chain local \
    --network-backend litep2p \
    --${ids[i]} \
    --port $((30333 + i)) \
    --unsafe-rpc-external \
    --rpc-port $((9945 +i)) \
    --node-key 000000000000000000000000000000000000000000000000000000000000000$((1 + i)) \
    --validator \
    --rpc-methods=Unsafe \
    --rpc-cors=all \
   &
  done

  trap "trap - SIGTERM && kill -9 -- $$" SIGINT SIGTERM EXIT
  while true; do read; done
