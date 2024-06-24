# Deploying a mosaic bootstrap node

This document describes the required steps to deploy our own collators for
test / main net.

Things are going to change in the future a bit, but we'll keep updating this document with all the latest information.
Because of this I won't try to be too generic here just yet.

## Prerequisites
- Obtain and install parachain node binary/package (debian package made by CI)
- Obtain _secrets_: initial session keys and pre-generated node-keys.
  - **todo:** figure out access control
- Obtain relaychain specific **raw** chain spec.
  - `polkadot_cli` has some built in chain specs, generating those are not necessary. 
    [(built in specs)](https://github.com/paritytech/polkadot-sdk/blob/feeec7b9030027847ff21da18bf0bb9df6b15dd9/polkadot/cli/src/command.rs#L76)
  - for generating custom relay chainspecs (eg. when we customize relay chain runtime) 
    usually generators are available in the relay's repos. (`polkadot-runtimes`, `paseo-runtimes`)
- Obtain parachain spec: this should be already pre-generated and extended with soon-to-be deployed bootnodes.
  - Generating this file should not be in the scope for whoever deploys the nodes as this spec stores potentially consensus critical data.
  - ...but generally:
    - `parachain-node build-spec --chain live > plain.json`
    - edit plain.json and add node multiaddresses to bootnodes.
    - `parachain-node build-spec --chain plain.json --raw > raw.json`
- determine `base_path` location: all of node's data will be stored here, this place needs to be known for configuration.
 
## Configure node-key

The node-key is used by the libp2p layer for peer identification (pubkey derivative) and signing messages on that level.
For us it is crucial to have static and known node-ids, as these ids are a required part of a bootnode's [multiaddress](https://docs.libp2p.io/concepts/fundamentals/addressing/).
The _secret_ pre-generated node keys are available as hax-encoded string, BUT our blockchain node only accepts them in binary.

For convenience the _secret_ file contains the binary files named appropriataly.

```
  secrets.zip
    - mosaic-collators.txt # contains all required data
    - node01
      - secret_ed25529 # node-key named appropriately
    - node02
      - secret_ed25529
    - node03
      - secret_ed25529
    - node04
      - secret_ed25529
```

When deploying `node01` for example we "install" the node-key like so:

```bash
cp node01/secret_ed25529 $BASE_PATH/chains/mosaic/network/
  
```

Where `BASE_PATH` is the pre-determined location.

## Start node
After ensuring we have access to the parachain-node binary (eg.: through installed deb package)
we now have to start it with the following command:

```bash
  parachain-node --collator --chain $PARA_CHAINSPEC --base-path $BASE_PATH \
  --pruning archive --port $PARA_PORT --rpc-port $PARA_RPC_PORT -- --chain $RELAY_CHAINSPEC \
  --port $RELAY_PORT --rpc-port $RELAY_RPC_PORT
```

| Variable / value | Description                                   | Recommended value(s)        |
|------------------|-----------------------------------------------|-----------------------------|
| PARA_CHAINSPEC   | raw chain spec in .json file                  | raw.json                    |
| BASE_PATH        | where persistent data is stored               | ~/.local/share/mosaic-chain |
| PARA_PORT        | port used by libp2p of para chain instance    | 40333                       |
| PARA_RCP_PORT    | rpc port of parachain instance                | 8845                        |
| RELAY_CHAINSPEC  | raw chainspec in .json file OR built-in chain | paseo-raw.json, polkadot    |
| RELAY_PORT       | port used by libp2p of relay chain instance   | 50333                       |
| RELAY_RPC_PORT   | rpc port of relay chain instance              | 9945                        |

_Note:_ some options can be reconsidered and adjusted. These are only my recommendations.

## Insert initial session key

After the node is up and running, their initial session-key(s) must be set.
In the current version of the parachain runtime we only need one session-key: aura.
In the future additional session keys will need to be set separately.

The session keys are used to sign consensus related data without requiring the address of
the validator/collator/block-producer to be hot. The session keys ought to be rotated regularly, but when
deploying our nodes we need to provide the initial ones. The public keys in the _secret_ file are 
baked into the chain spec, there is no need for dissemination. They are already tied to the collator's address.

The only thing left to do is to insert the private key(s) into our node's keystore.
This can be done via JSON-RPC:

```bash
curl -H "Content-Type: application/json" \
   --data '{ "jsonrpc":"2.0", "method":"author_insertKey", "params":["aura", "'"${SECRET_PHRASE}"'", "'"${PUBLIC_KEY}"'"],"id":1 }' "${RPC_ENDPOINT}"
```

Alternatively we can use polkadot.js to construct and send this rpc method call to our node.
 - Connect to node via side panel
 - Use Developer -> RPC calls
