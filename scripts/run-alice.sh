#! /bin/bash

./target/release/node-template purge-chain --base-path /tmp/alice --chain local -y

./target/release/node-template --base-path /tmp/alice --chain local --alice --port 30333 --ws-port 9945 --unsafe-ws-external --rpc-port 9933 --node-key 0000000000000000000000000000000000000000000000000000000000000001 --validator --rpc-methods=Unsafe --rpc-cors=all
