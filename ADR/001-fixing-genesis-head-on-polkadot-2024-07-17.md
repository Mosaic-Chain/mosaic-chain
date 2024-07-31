---
title: 'Fixing genesis-head on polkadot'
date: '2024-07-17'
status: approved
authors: ['vismate']
reviewers: ['wigy']
---

## Problem

When registering parathread on polkadot we mistakenly generated the `genesis-state` from the wrong chainspec.
The chainspec used was modified for local testing as follows:

- Bootnodes were added with ip `localhost`
- Relaychain was set to "paseo-local"
- ParaId was set to 2000
- ParaId in parachain_info's genesis config was set to 2000

The first three modifications do not result in any change in the produced genesis-state.

Why?

A chainspec consists of two parts:

- Client config
- Genesis config

The client config contains information needed by the node to connect to and communicate with the blockchain network.
The genesis configuration contains the runtime code (at genesis) and the configuration of pallets.

When we generate the raw chainspec, the client config is copied as is,
and the runtime code is used to build the initial storage state from the provided values.
The storage is then encoded into this new raw chainspec.

Then we use this raw chainspec to export the genesis-state and the runtime code.

Thus we can conclude that only genesis configuration values contribute to the generated genesis-state.

## Solution

The first solution that comes to mind is to just correct the chainspec and rebuild the genesis-state with the
version of the code tagged with `mosaic-mainnet-runtime-v100`.

This is _almost_ right. According to [this article](https://docs.substrate.io/build/build-a-deterministic-runtime/)
it is important for nodes to run the **exact same** runtime.

The problem with rebuilding the runtime is that rust builds are non-deterministic (thats why we'll need to use srtool from now on).
To solve this, we also need to replace the runtime code with the code uploaded to polkadot.

In the past I haven't had any problems with non-deterministically built runtimes. They do generate the same genesis
state and of course behave exactly the same. On the other hand we can't be too sure with these things...

So we need a raw chainspec with correct client and genesis config **and** the exact same runtime code we uploaded
to polkadot.

We can either create a new chainspec and replace the code, or use the original raw chainspec (which we thankfully have)
and modify the relevant parts.

Since I'm more comfortable with editing a few lines than 3kb's of code, I chose to replace the storage value.

## Original raw chainspec

Upon discovering we've made a mistake I've gathered the raw chainspec and the uploaded files and saved them to a .zip file.
I'll make this file available for the reader.

To confirm these are the files used for parathread registration I checked three things:

1. file metadata: the files dated back to parathread registration
2. the genesis-state and genesis-wasm match uploaded files
  - genesis-state confirmed through polkadot.js
  - both files match the ones I sent to Peti for upload
3. The raw chainspec generates the exact genesis-state and contains exactly the same code
  - used node to export code and state
  - these matched the ones we have

## Determining what needs to be changed

In the mosaic-chain repo:

```sh
git checkout tags/mosaic-mainnet-runtime-v100
cargo b -r  
```

Now generate a test chainspec:

```sh
./target/release/parachain-template-node build-spec --chain live > plain.json  
```

Generate control raw spec:

```sh
./target/release/parachain-template-node build-spec --chain plain.json --raw > control.json
```

Now change `plain.json`:

```json
...
"parachainInfo": {
  "parachainId": 3377
}
...
```

to

```json
...
"parachainInfo": {
  "parachainId": 2000
}
...
```

Now generate new raw chainspec:

```sh
./target/release/parachain-template-node build-spec --chain plain.json --raw > modified.json  
```

Comparing two raw chainspecs:

```sh
diff control.json modified.json
```

Expected output:

```diff
  21c21
<         "0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f": "0x310d0000",
---
>         "0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f": "0xd0070000",
```

If we take a look at the file, we can see that this is in fact the fist key-value pair
under `"genesis": { "raw": { "top": {...`.

We can also deduce that the para_id "2000" is encoded as "0xd0070000" and para_id "3377" is encoded as "0x310d0000".
If we look into the original raw chainspec (from the .zip) we can see that the value is "0xd0070000" which really
corresponds to "2000".

## Producing correct chainspec

In the client config section we change:

- "bootNodes": ip needs to be set to deployed nodes' ip.
- "relay-chain": from "paseo-local" to "polkadot"
- "para_id": from 2000 to 3377

In the genesis section:

- "0x0d715f2646c8f85767b5d2764bb2782604a74d81251e398fd8a0a4d55023bb3f": from "0xd0070000" to "0x310d0000"

Now a new genesis-state can be built using the modified raw chainspec:

```sh
./target/release/parachain-template-node export-genesis-head --chain live-raw.json > genesis-state
```

_NOTE: this chainspec is the one that nodes must use on our mainnet._

## Updating genesis-state on polkadot

With our multisig account we call `registrar_setCurrentHead` with `para` 3377 and the newly
generated genesis-state.

## Testing

**Goal:** by fixing genesis state parachain nodes running the **correct**
chainspec start producing blocks and these blocks are finalized.

**Non goal:** test running **incorrect** chainspec.

### Dependencies

- `mosaic-chain` repo (+build-deps)
- `paseo-runtimes` repo (own fork, +build-deps)
- `polkadot` v1.11.0 (main binary + workers)

### Test log

1. **checkout the `mosaic-mainnet-runtime-v100` tag of mosaic-chain**
  - `git checkout tags/mosaic-mainnet-runtime-v100`
2. compile `parachain-template-node` in release mode
  - `cargo b -r -p parachain-template-node`
3. **produce local plain chainspec**
  - `./target/release/parachain-template-node build-spec --chain local > test-plain.json`
4. **produce correct raw chainspec and genesis state**
  - `./target/release/parachain-template-node build-spec --chain test-plain.json --raw > test-correct.json`
  - `./target/release/parachain-template-node export-genesis-head --chain test-correct.json > test-correct-head`
5. **produce incorrect raw chainspec and genesis state**
  - edit `test-plain.json` "parachainInfo": { "parachainId": 2001 }
  - (client options can be edited too, but not necessary)
  - `./target/release/parachain-template-node build-spec --chain test-plain.json --raw > test-incorrect.json`
  - `./target/release/parachain-template-node export-genesis-head --chain test-incorrect.json > test-incorrect-head`
  - (notice that the only difference between `test-correct-head` and `test-incorrect-head` is one storage entry)
6. **generate genesis-wasm**
  - `./target/release/parachain-template-node export-genesis-wasm --chain test-correct.json > test-wasm`
7. **compile `paseo-runtimes` in release mode with the `fast-runtime` feature turned on.**
  - `cargo b -r -F fast-runtime`
8. **generate relay chainspec**
  - `./scripts/gen-chainspec.sh`
9. **start relaychain**
  - `./scripts/relay-chain-up.sh <polkadot binary> paseo-local.json`
10. **register parathread with wrong genesis state**
  - connect to a relaychain node with polkadot.js (localhost:9945)
  - navigate to Network > Parachains > Parathreads
  - claim paraid (eg. with bob's account, "+ ParaId" button)
  - register parathread ("+ ParaThread" button)
  - upload `test-wasm` and `test-incorrect-head`
  - wait for onboarding (approx. 3 mins, can be tracked: Network > Parachains > Parathreads)
11. **start auction**
  - navigate to Developer > Sudo
  - using Alice's account call `auctions_newAuction` with default params
12. **bid**
  - quickly navigate to Network > Parachains > Auctions
  - bid for bob's 2000 paraid on period 0-7 with some PAS ("+Bid" button)
  - wait for auction to end (30 sec ending period +length of session, approx 3 minutes)
  - after winning auction it takes around 3 mins to activate (upgrade) parathread
13. **start parachain with correct chainspec**
  - on the checked out tag the helper script uses `local-raw.json`,
    so either the script needs a touch up, or `test-correct.json` should be renamed.
  - `./scripts/test-up.sh <paseo-local.json>`
  - (`paseo-local.json` was generated in step 8.)
14. **observe blocks are not produced**
  - after the 3 min upgrade period ends the parachain should complain about genesis-state mismatch
    and should not produce blocks (best height remains 0)
15. **fix genesis head**
  - (still connected to relaychain on polkadot.js)
  - navigate to Developer > Extrinsics
  - call (using Bob's account) `registrar_setCurrentHead` with paraid 2000 and upload `test-correct-head`.
16. **observe blocks are produced**
  - Now the parachain should start producing blocks
  - Finalization should also happen

### Conclusion

Using the above method we changed the value in storage of parachain_info
(genesis.runtimeGenesis.patch.parachainInfo.parachainId) from 0xd0070000 to 0x310d0000.

Then we re-ran the test again and confirmed the fix will work.

**Uploaded genesis-head:**
0x0000000000000000000000000000000000000000000000000000000000000000001f9d7cd4f90f072af243082040b30097a498cc426b67e278bb91071901f0974c03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c11131400

**raw-chainspec (SHA256):**
fbad4de9c906bb447aae1a5b4b714ece33fa691714729be3a40b27f35fc89348

#### Initial call

**Sender:** 1saFMaLSrzaG4sC7n2eaZhm2QT4fFWopBM1j4rMNNZ76pM3 (vismate)
**Block:** #21671803
**Block hash:** 0x2fbba283a1ca2a7102487dbf334341b0dc8656abfe4f6b945915375af0797738
**Timestamp:** 2024-07-16 13:00:30 (+UTC)
**Subscan: <https://polkadot.subscan.io/extrinsic/21671803-3>**

**Call data:**
0x1d0000f25418311f6f3b70b3ebe74fc714aba44e177d104b6c2defb27f9b9936da17b9004608310d000089010000000000000000000000000000000000000000000000000000000000000000001f9d7cd4f90f072af243082040b30097a498cc426b67e278bb91071901f0974c03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c11131400

**Code:**

```json
[
  {
    "name": "threshold",
    "type": "U16",
    "type_name": "u16",
    "value": 2
  },
  {
    "name": "other_signatories",
    "type": "Vec<AccountId>",
    "type_name": "Vec<AccountId>",
    "value": [
      "0x56635c6ed226344060eae9313b47472ef7bdc4ca9e4d3eb8ccee73badfe1ec42",
      "0x6efe1c45d2ad830ff71c16c26e005b6d0f6f40feadb72cfa6e010a9541444b77",
      "0xa04ed7f7003af5b1f2cdc596c58ef31a58d2a12899f8e2b0ebe0ea10b5dcfa72",
      "0xaa3c8082e71c64e4652e2e5b11d0f1b8aa0dcab0e48bfbc6aa395978630acf48",
      "0xb4dd5b7a774828c2a267175a43347d0b06c28cdb9fd4443954ca167e07a1af2b"
    ]
  },
  {
    "name": "maybe_timepoint",
    "type": "option<pallet_multisig:Timepoint>",
    "type_name": "Option<Timepoint<BlockNumberFor>>",
    "value": null
  },
  {
    "name": "call_hash",
    "type": "[U8; 32]",
    "type_name": "[u8; 32]",
    "value": "0xc092272a8fff36ee2caf2cc85c16cdb204e8e5b30e54795a5240e2062295ae4a"
  },
  {
    "name": "max_weight",
    "type": "sp_weights:weight_v2:Weight",
    "type_name": "Weight",
    "value": {
      "proof_size": 4706,
      "ref_time": 227552054
    }
  }
]
```

### Approval

**Sender:** 1569NpEohXAJXJucn2hTeBXcJaFtD5eWittZ4iYPiCHo47PD (mudlee)
**Block:** #21671886
**Block hash:**
0x47eb6707016fa0d5f6b9478b51ac3651b6a909ea9fb3a7869c8dccd716526f0f
**Timestamp:** 2024-07-16 13:08:48 (+UTC)
**Subscan:** <https://polkadot.subscan.io/extrinsic/21671886-2>

```json
[
  {
    "name": "threshold",
    "type": "U16",
    "type_name": "u16",
    "value": 2
  },
  {
    "name": "other_signatories",
    "type": "Vec<AccountId>",
    "type_name": "Vec<AccountId>",
    "value": [
      "0x269230f6b7087977655840a851769f109af8a28a7c2900604805141025b7d123",
      "0x56635c6ed226344060eae9313b47472ef7bdc4ca9e4d3eb8ccee73badfe1ec42",
      "0x6efe1c45d2ad830ff71c16c26e005b6d0f6f40feadb72cfa6e010a9541444b77",
      "0xa04ed7f7003af5b1f2cdc596c58ef31a58d2a12899f8e2b0ebe0ea10b5dcfa72",
      "0xaa3c8082e71c64e4652e2e5b11d0f1b8aa0dcab0e48bfbc6aa395978630acf48"
    ]
  },
  {
    "name": "maybe_timepoint",
    "type": "option<pallet_multisig:Timepoint>",
    "type_name": "Option<Timepoint<BlockNumberFor>>",
    "value": {
      "height": 21671803,
      "index": 3
    }
  },
  {
    "name": "call",
    "type": "Call",
    "type_name": "Box<<T as Config>::RuntimeCall>",
    "value": {
      "call_index": "1d00",
      "call_module": "Proxy",
      "call_name": "proxy",
      "params": [
        {
          "name": "real",
          "type": "sp_runtime:multiaddress:MultiAddress",
          "value": {
            "Id": "0xf25418311f6f3b70b3ebe74fc714aba44e177d104b6c2defb27f9b9936da17b9"
          }
        },
        {
          "name": "force_proxy_type",
          "type": "option<polkadot_runtime:ProxyType>",
          "value": null
        },
        {
          "name": "call",
          "type": "Call",
          "value": {
            "call_index": "4608",
            "call_module": "Registrar",
            "call_name": "set_current_head",
            "params": [
              {
                "name": "para",
                "type": "U32",
                "value": 3377
              },
              {
                "name": "new_head",
                "type": "Vec<U8>",
                "value": "0x0000000000000000000000000000000000000000000000000000000000000000001f9d7cd4f90f072af243082040b30097a498cc426b67e278bb91071901f0974c03170a2e7597b7b7e3d84c05391d139a62b157e78786d8c082f29dcf4c11131400"
              }
            ]
          }
        }
      ]
    }
  },
  {
    "name": "max_weight",
    "type": "sp_weights:weight_v2:Weight",
    "type_name": "Weight",
    "value": {
      "proof_size": 4706,
      "ref_time": 227552054
    }
  }
]
```
