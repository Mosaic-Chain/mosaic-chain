NODE="./target/release/parachain-template-node"

if [ ! -f $NODE ]; then
  >&2 echo "Please compile parachain node in release mode!"
  exit 3
fi;

$NODE export-genesis-wasm --chain parachain.json para-2000-wasm
$NODE export-genesis-state --chain parachain.json para-2000-genesis-state