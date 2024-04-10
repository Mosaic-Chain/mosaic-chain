#!/usr/bin/env bash

set -eu

HASH=$(git describe --always)
VER="$(grep -o -m 1 '^version = "[^"]*"' ../../Cargo.toml | cut -d '"' -f2)+$HASH-1"
DIR="mosaic-chain-rc_"$VER"_amd64"

mkdir -p "$DIR/DEBIAN"
sed "s/xVERSION/$VER/g" ./control.template > "$DIR/DEBIAN/control"
mkdir -p "$DIR/usr/bin"
mkdir -p "$DIR/var/lib/mosaic/"
mkdir -p "$DIR/lib/systemd/system/"
cp ../../target/release/mosaic-chain "$DIR/usr/bin/"
cp ./start-node "$DIR/usr/bin/"
cp ./mosaic-chain.service "$DIR/lib/systemd/system/"
dpkg --build "$DIR" >> /dev/null
echo "$DIR.deb"
