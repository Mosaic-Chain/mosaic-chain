# build debug version of Mosaic chains
build:
  cargo build

# build release version of Mosaic chains
build-release:
  cargo build --release

# test a specified package (eg.: a pallet)
test $PACKAGE:
  cargo nextest run --release --lib --features try-runtime --package $PACKAGE

# test every package
test-all:
  cargo nextest run --release --lib --features try-runtime

# run clippy for lints
clippy:
  cargo clippy --all-features --bins --examples --tests --benches --release --no-deps --features runtime-benchmarks,try-runtime

# format the code
format:
  cargo fmt -- --check

# run a temporary test network with alice, bob and the others (6 nodes)
run-network: build-release
  #!/usr/bin/env bash
  ids=(alice bob charlie dave eve ferdie)

  max=6
  for (( i=0; i < $max; i++ ))
  do
    ./target/release/mosaic-testnet-solo purge-chain \
    --base-path /tmp/${ids[i]} \
    --chain local -y \

    ./target/release/mosaic-testnet-solo \
    --base-path /tmp/${ids[i]} \
    --chain local \
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

# install the nix package manager to make your life easier* 
install-nix:
  sh <(curl -L https://nixos.org/nix/install) --daemon

# create debian package for release
pkg: build-release
  cd packaging/release && ./make-package.sh

# create debian package for release candidacy
pkg-rc: build-release
  cd packaging/release-candidate && ./make-package.sh
