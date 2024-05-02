#!/usr/bin/env bash

set -eu

VER="$(grep -o -m 1 'version = "[^"]*"' ../../Cargo.toml | cut -d '"' -f2)-1"
DIR="mosaic-testnet-solo_`echo $VER`_amd64"

mkdir -p "$DIR/DEBIAN"
sed "s/xVERSION/`echo $VER`/g" ./control.template > "$DIR/DEBIAN/control"
mkdir -p "$DIR/usr/bin"
cp ../../target/release/mosaic-testnet-solo "$DIR/usr/bin/mosaic-testnet-solo"
dpkg --build "$DIR" >> /dev/null
echo "$DIR.deb"
