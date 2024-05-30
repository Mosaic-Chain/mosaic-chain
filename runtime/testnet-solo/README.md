# Get the node running

```bash
mosaic-chain --chain <chain_spec> --node-key-file <node_key_file> --rpc-port <rpc_port> --port <p2p_port> --name <name> --no-telemetry --validator
```

- `chain_spec`:
  - One of the built-in specs or file
  - Contains bootnodes and other info about the chain

- `node_key_file`:
  - path to a file containing pre-generated **secret** key
  - node ids identify a node in (lib)p2p networks

- `rpc_port`:
  - port through which our node accepts RPC calls

- `p2p_port`:
  - port through which our node communicates with other (lib)p2p nodes

- `name`:
  - human readable node name

## Generate `node_key_file`

In this step we generate a libp2p node id. This is optional as node ids are randomly generated if not provided. However it is recommended to have a well-known static node id that persists between restarts.


```bash
mosaic-chain key generate-node-key > node_key_file
```

_(the derived node ID will be written to stderr, the secret key to the file)_

## Session keys

Session keys are used to sign consensus related messages. These are hot keys, so it is recommended to rotate them regularly. Session keys are tied to a validator node and act on behalf of your stash account shielding it from beeing exposed too much.

To generate initial session keys we recommend using

```bash
# aura / imonline
mosaic-chain key generate --scheme sr25519
# grandpa (for now)
mosaic-chain key generate --scheme ed25519
```

_Note: `author_rotateKeys` can be used to generate new keys but it returns the keys concatenated and SCALE encoded._

### Insert keys:

Now we tell our running node about our keys so it may use them to sign messages.
There are a couple of ways to insert keys into the node's keystore. Now we'll use jsonRPC.


```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "author_insertKey",
  "params": ["<aura/gran/imon>", "<mnemonic phrase>", "<public key>"]
}
```

```bash
curl http://localhost:<rpc_port> -H "Content-Type:application/json;charset=utf-8" -d '<rpc>'"
```

Expected result:
```json
{ "jsonrpc": "2.0", "result": null, "id": 1 }
```

At a later stage we will also broadcast our public keys to other nodes using `pallet_session`.

## Set up accounts

Two accounts need to be set up:
  - stash account (cold, holds funds and permission NFT)
  - controller account (not so cold, used as a proxy for the stash account)

_Note: for convenience controller can be a **hard** derivation of stash account_

TODO: how will the validators receive their NFT? \
TODO: how will the accounts be pre-funded (for fees)?

### Add proxy

By now it's important to have free balance on both the stash and controller
accounts.

Extrinsic: `proxy.addProxy` \
Params:
```
delegate: <controller_account>
proxyType: Staking # or superset of
delay: 0
```

## Start validating

### Set session keys in pallet session

Use proxied call (controller acts on behalf of stash)!

Extrinsic: `session.setKeys` \
Params:
```
keys:
  aura: <aura pubkey(hex)>,
  grandpa: <grandpa pubkey(hex)>,
  ImOnline: <imonline pubkey(hex)>
proof: 0x00
```

### Bind permission NFT

Use proxied call (controller acts on behalf of stash)!

Extrinsic: `nftStaking.bindValidator` \
Params:
```
itemId: <permission_NFT_id>
```

After the NFT is bound the validator can be chosen into the active set and produce blocks.⏎