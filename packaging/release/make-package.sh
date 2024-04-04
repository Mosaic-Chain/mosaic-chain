#!/usr/bin/env bash

set -eu

VER="$(grep -o 'version = "[^"]*"' ../../Cargo.toml | cut -d '"' -f2)-1"
DIR="mosaic-chain_`echo $VER`_amd64"

mkdir -p $DIR/DEBIAN
sed "s/xVERSION/`echo $VER`/g" ./control.template > $DIR/DEBIAN/control
mkdir -p $DIR/usr/bin
cp ../../target/release/mosaic-chain $DIR/usr/bin/mosaic-chain
dpkg --build $DIR >> /dev/null
echo "$DIR.deb"
