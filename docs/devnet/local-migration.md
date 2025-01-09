# v100 -> v101 Runtime Upgrade Guide

This document outlines the steps to test the runtime upgrade process from v100 to v101.

## Preparing the Local `paseo` Relaychain Specification

To streamline testing, we create a relaychain chainspec with the parachain onboarded in the genesis block.

(This step can be skipped as the required files are already available in this repository.)

1. Generate a plain `paseo-local` chainspec using our `paseo-runtimes` fork:

```bash
cargo build -r -p chain-spec-generator -F fast-runtime
./target/release/chain-spec-generator paseo-local > paseo-local.json
```

2. Add the parachain runtime code and genesis state:
   - Obtain the parachain runtime and genesis state files from the [public repository](<https://mega.nz/folder/ZdBzERzK#EiPAXQXxTV3ZRAvFixhovw>).
   - Modify `mosaic-devnet-plain.json` to replace devnet accounts with developer accounts (alice, bob, ...).
      - the edited file is saved under `chainspecs/devnet/devnet-local-v100.json`
   - Regenerate the genesis state:
     ```bash
     cargo build -r -p mosaic-chain-node
     ./target/release/mosaic-chain export-genesis-state --chain chaionspecs/devnet/devnet-local-v100.json > devnet-local.genesis-state
     ```
   - Add the following lines to `paseo-local.json` under the `"genesis"` section:
     ```json
     ...
     "paras": {
       "paras": [
         [
           3377,
           [
             "<genesis-state>",
             "<genesis-wasm>",
             true
           ]
         ]
       ]
     }
     ...
     ```

**Note:** the resulting file is saved under `chainspecs/paseo/paseo-local.json`.

## Generating the v101 Runtime WASM

To upgrade, obtain the runtime WASM for v101:

```bash
cargo build -r runtime-generator -F para-chain
./target/release/runtime-generator native build para-local > v101-local.json
./target/release/mosaic-chain export-genesis-wasm --chain v101-local.json > v101-local.wasm
```

**Note:** the use of `runtime-generator`'s native builder results in non-deterministic builds.
Use the srtool builder for production builds.

## Launching the Network

Start the relaychain:

```bash
./scripts/paseo-local-up.sh polkadot chainspecs/paseo/paseo-local.json
```

Start the parachain:

```bash
./scripts/parachain/test-up.sh chainspecs/devnet/devnet-local-v100.json chainspecs/paseo/paseo-local.json
```

## Performing the On-Chain Runtime Upgrade

1. Using polkadot.js, send the `system.setCode` extrinsic as sudo.
   For the `code` parameter upload `v101-local.wasm`
  
2. The runtime upgrade will take effect after a short delay.

## Updating Session Keys

To enable proper functionality, session keys (Aura and ImOnline) must be generated and inserted.

1. For each parachain node, use the `author.rotateKeys` RPC to generate new session keys.
2. Use the `session.setKeys` extrinsic with:
   - **Keys**: The output of `author.rotateKeys`.
   - **Proof**: `0x`.


**Note:** The keys returned by `author.rotateKeys` are concatenated.
We can use the following script to separate them:

```bash
input="<output_of_author_rotateKeys>"
echo "Aura: ${input:0:${#input}/2+1}"
echo "ImOnline: 0x${input:${#input}/2+1}"
```
