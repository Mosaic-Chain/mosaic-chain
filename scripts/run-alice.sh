#! /bin/bash

./target/release/node-template purge-chain \
--base-path \
/tmp/alice \
--chain local -y

./target/release/node-template \
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
--state-pruning archive-canonical \
&

trap "trap - SIGTERM && kill -9 -- -$$" SIGINT SIGTERM EXIT

while true; do read; done