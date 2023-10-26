#! /bin/bash

./target/release/mosaic-chain purge-chain \
--base-path \
/tmp/alice \
--chain local -y

./target/release/mosaic-chain \
--base-path /tmp/alice \
--chain local \
--alice \
--port 30333 \
--unsafe-rpc-external \
--rpc-port 9945 \
--node-key 0000000000000000000000000000000000000000000000000000000000000001 \
--validator \
--rpc-methods=Unsafe \
--rpc-cors=all \
--state-pruning archive-canonical
