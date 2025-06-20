# Mosaic Chain

<!--toc:start-->
- [Getting Started](#getting-started)
  - [Setup git hooks](#setup-git-hooks)
  - [Generate docs](#generate-docs)
- [Solo-chain](#solo-chain)
  - [Build solo-chain](#build-solo-chain)
  - [Embedded Docs](#embedded-docs)
  - [Single-Node development chain](#single-node-development-chain)
  - [Connect with Polkadot-JS Apps Front-End](#connect-with-polkadot-js-apps-front-end)
  - [Project Structure](#project-structure)
  - [Node](#node)
  - [Runtime](#runtime)
  - [Pallets](#pallets)
    - [Custom pallets](#custom-pallets)
  - [Preparing and running a solochain testnet](#preparing-and-running-a-solochain-testnet)
    - [Generating chainspec with `runtime-generator`](#generating-chainspec-with-runtime-generator)
    - [Generating node keys](#generating-node-keys)
    - [Scenario 1: development accounts spread across different servers](#scenario-1-development-accounts-spread-across-different-servers)
    - [Scenario 2: real accounts spread across different servers](#scenario-2-real-accounts-spread-across-different-servers)
    - [Useful links:](#useful-links)
- [Parachain](#parachain)
<!--toc:end-->

## Getting Started

Install the following dependencies from your preferred package manager:

For compilation / testing:

- `rust` (`rustup`)
- `clang` / `libclang`
- `pkg-config`
- `protobuf` (`protoc`)
- `librocksdb`
- `cargo-nextest`
- `zepter` (<https://github.com/ggwpez/zepter>)
- `zombienet` (<https://github.com/paritytech/zombienet>)
- `polkadot` (<https://github.com/paritytech/polkadot-sdk>)

For packaging:

- `git-cliff`
- `toml-cli`
- `cargo-deb`

NOTE: cargo-deb, cargo-nextest and zepter can be installed with `cargo install` if
they are not available in your package manager.

A `flake.nix` file is also included using which a complete development environment can be spawned.
It also serves as a complete list of dependencies together with `rust-toolchain.toml`.

### Setup git hooks

If you wish to contribute to the repository it might worth to automatically
do some checks using git hooks. To install default git hooks execute:

```sh
./scripts/setup-hooks.sh
  
```

### Generate docs

You can generate and view the [Rust Docs](https://doc.rust-lang.org/cargo/commands/cargo-doc.html) for this project with this command:

```sh
cargo doc --workspace --open
```

## Releasing

1. `cargo b -r -p runtime-generator --all-features`
2. `./target/release/runtime-generator srtool build solo-local > "chainspecs/testnet/chainspec-$(date +%s).json"`
3. `./scripts/release.sh`

## Solo-chain

The solochain is used for developing and testing new functionality locally or in a pipeline.
It can also used by other components that require interactions with the mosaic-chain runtime.

### Build solo-chain

Use the following command to build the node without launching it:

```sh
cargo build --release -p mosaic-testnet-solo-node -F dev-spec
```

This builds the solo-chain node with the `dev-spec` feature on, which means a development chain specification and runtime
will be included in the node.

If you'd like to get a more detailed view of how the chainspec is built and included into the node please read
[`runtime-generator`'s readme](runtime-generator/README.md)

### Embedded Docs

After you build the project, you can use the following command to explore its parameters and subcommands:

```sh
./target/release/mosaic-testnet-solo -h
```

### Single-Node development chain

The following command starts a single-node development chain that doesn't persist state:

```sh
./target/release/mosaic-testnet-solo --dev
```

To purge the development chain's state, run the following command:

```sh
./target/release/mosaic-testnet-solo purge-chain --dev
```

To start the development chain with detailed logging, run the following command:

```sh
RUST_BACKTRACE=1 ./target/release/mosaic-testnet-solo -ldebug --dev
```

Development chain:

- Maintain state in a `tmp` folder while the node is running.
- Use the **Alice**, **Bob**, **Charlie**, **Dave** and **Eve** accounts as default validator authorities.
- Are preconfigured with a genesis state that includes several prefunded development accounts.
- Is configured with shorter sessions and other waiting periods to make it convenient for testing.

To persist chain state between runs, specify a base path by running a command similar to the following:

```sh
# Create a folder to use as the db base path
$ mkdir my-chain-state

# Use of that folder to store the chain state
$ ./target/release/mosaic-testnet-solo --dev --base-path ./my-chain-state/

# Check the folder structure created inside the base path after running the chain
$ ls ./my-chain-state
chains
$ ls ./my-chain-state/chains/
dev
$ ls ./my-chain-state/chains/dev
db keystore network
```

### Connect with Polkadot-JS Apps Front-End

After you start the node locally, you can interact with it using the hosted version of the [Polkadot/Substrate Portal](https://polkadot.js.org/apps/#/explorer?rpc=ws://localhost:9944) front-end by connecting to the local node endpoint.
A hosted version is also available on [IPFS (redirect) here](https://dotapps.io/) or [IPNS (direct) here](ipns://dotapps.io/?rpc=ws%3A%2F%2F127.0.0.1%3A9944#/explorer).
You can also find the source code and instructions for hosting your own instance on the [polkadot-js/apps](https://github.com/polkadot-js/apps) repository.

### Project Structure

A Substrate project such as this consists of a number of components that are spread across a few directories.

### Node

A blockchain node is an application that allows users to participate in a blockchain network.
Substrate-based blockchain nodes expose a number of capabilities:

- Networking: Substrate nodes use the [`libp2p`](https://libp2p.io/) networking stack to allow the
  nodes in the network to communicate with one another.
- Consensus: Blockchains must have a way to come to [consensus](https://docs.substrate.io/fundamentals/consensus/) on the state of the network.
  Substrate makes it possible to supply custom consensus engines and also ships with several consensus mechanisms that have been built on top of Web3 Foundation research.
- RPC Server: A remote procedure call (RPC) server is used to interact with Substrate nodes.

There are several files in the `node` directory.
Take special note of the following:

[`service.rs`](./node/testnet-solo/src/service.rs): This file defines the node implementation.
Take note of the libraries that this file imports and the names of the functions it invokes.
In particular, there are references to consensus-related topics, such as the [block finalization and forks](https://docs.substrate.io/fundamentals/consensus/#finalization-and-forks) and other [consensus mechanisms](https://docs.substrate.io/fundamentals/consensus/#default-consensus-models) such as Aura for block authoring and GRANDPA for finality.

### Runtime

In Substrate, the terms "runtime" and "state transition function" are analogous.
Both terms refer to the core logic of the blockchain that is responsible for validating blocks and executing the state changes they define.
The Substrate project in this repository uses [FRAME](https://docs.substrate.io/fundamentals/runtime-development/#frame) to construct a blockchain runtime.
FRAME allows runtime developers to declare domain-specific logic in modules called "pallets".
At the heart of FRAME is a helpful [macro language](https://docs.substrate.io/reference/frame-macros/) that makes it easy to create pallets and flexibly compose them to create blockchains that can address [a variety of needs](https://substrate.io/ecosystem/projects/).

Review the [FRAME runtime implementation](./runtime/testnet-solo/src/lib.rs) and note the following:

- This file configures several pallets to include in the runtime.
  Each pallet configuration is defined by a code block that begins with `impl $PALLET_NAME::Config for Runtime`.
- The pallets are composed into a single runtime by way of the [`construct_runtime!`](https://crates.parity.io/frame_support/macro.construct_runtime.html) macro, which is part of the core FRAME Support [system](https://docs.substrate.io/reference/frame-pallets/#system-pallets) library.

### Pallets

The runtime in this project is constructed using many FRAME pallets that ship with the [core Substrate repository](https://github.com/paritytech/polkadot-sdk/tree/master/substrate/frame).

A FRAME pallet is compromised of a number of blockchain primitives:

- Storage: FRAME defines a rich set of powerful [storage abstractions](https://docs.substrate.io/build/runtime-storage/) that makes it easy to use Substrate's efficient key-value database to manage the evolving state of a blockchain.
- Dispatchables: FRAME pallets define special types of functions that can be invoked (dispatched) from outside of the runtime in order to update its state.
- Events: Substrate uses [events and errors](https://docs.substrate.io/build/events-and-errors/) to notify users of important changes in the runtime.
- Errors: When a dispatchable fails, it returns an error.
- Config: The `Config` configuration interface is used to define the types and parameters upon which a FRAME pallet depends.

#### Custom pallets

Mosaic Chain implements it's business logic in custom built pallets:

- [`pallet-nft-staking`](./pallets/nft-staking/README.md) ties staking and validation logic together, it's responsible for:
  - validator binding/unbinding
  - accepting nft and currency based delegation
  - reward calculation
  - slashing
  - providing the list of selectable validators to `validator-subset-selection`
- [`pallet-validator-subset-selection`](./pallets/validator-subset-selection/README.md) selects the active subset of validators who produce the block in the current session and drives session progression.
- [`pallet-nft-permission`](./pallets/nft-permission/README.md) owns permission NFTs and handles it's attributes.
- [`pallet-nft-delegation`](./pallets/nft-delegation/README.md) owns delegator NFTs and handles it's attributes.
- [`pallet-airdrop`](./pallets/airdrop/README.md) allows a minting authority to create permission and delegator NFTs, vesting schedules and free tokens via unsgined extrinsics.
- `pallet-doas` allows a custom origin to make calls in the name of arbitrary accounts, or even as root.
- `pallet-vesting-to-freeze` allows users to convert unvested tokens to immediately stakable frozen assets.
- `pallet-staking-incentive` distributes incentives from a predefined pool based on staking actions.

We also had to fork and modify some FRAME pallets:

- [`pallet-hold-vesting`](./pallets/hold-vesting/README.md) (forked from `pallet-vesting`) upgraded to fungible traits and using non-overlapping holds instead of locks.
- `pallet-im-online` allows nodes to announce and prove that they are operationl. The fork now lets any node with a session key to announce themselves.
- [`pallet-treasury`] upgraded to fungible traits and ensured that anyone can still propose spendings.

### Preparing and running a solochain testnet

To set up our solochain we generally need to agree upon a few things:

- The chain specification with runtime and genesis state
- Who are the bootnodes

#### Generating chainspec with `runtime-generator`

Build runtime generator:

```sh
  cargo b -r -p runtime-generator
```

Pull builder image (srtool):

```sh
./target/release/runtime-generator srtool pull
  
```

Build the runtime and generate raw chainspec:

```sh
./target/release/runtime-generator srtool build --raw solo-local > raw_chainspec.json  
```

Distribute the file amongst the nodes!

NOTE: available chainspec presets: mosaic-solo-local, mosaic-solo-live, mosaic-para-local, mosaic-para-live
NOTE: once a chain is started consequent nodes must also join with the same chainspec as the genesis hash must match.

#### Generating node keys

These keys are used on the libp2p layer and the public part is used to generate the node id.
For bootnodes knowing this id is important as it's part of their [multiaddress](https://docs.libp2p.io/concepts/fundamentals/addressing/).

```sh
  mosaic-testnet-solo key generate-node-key > nodekey
```

The above command generates a new node key and writes it to the `nodekey` file.
It also displays the derived node identity as well on `stderr`.

NOTE: `nodekey` is a private key and should handled as such

#### Scenario 1: development accounts spread across different servers

In this scenario we run **six** nodes across multiple machines.
We use the `mosaic-solo-local` chainspec preset with 6 dev accounts (alice, bob, charlie, dave, eve, ferdie).

NOTE: currently our chainspec presets only support running a minimum of 6 nodes.

1. generate chainspec as seen above and copy it to all machines
2. generate nodekeys as seen above and pick one to be the bootnode
   - I recommend naming nodekey files like this: `nodekey.alice`, `nodekey.bob`, ...
   - Let's pick alice to be the bootnode and note down her node id (printed to `stderr`) when
     generating `nodekey.alice`
3. define bootnode multiaddress: `/ip4/<ip>/tcp/<p2p port>/p2p/<node id>`
   - protocols can be mixed and matched, so a node can have multiple valid multiaddrs
   - we can also use dns name instead of raw ip: `/dns4/boot1.example.com/tcp/<p2p port>/p2p/<node id>`
4. start bootnode (for example):

```sh
mosaic-testnet-solo --chain <chainspec> --name <node name> --base-path <basepath> --state-pruning <pruning mode> \
  --validator --rpc-port <rpc port> --listen-addr /ip4/<ip>/tcp/<p2p port> --node-key-file <nodekey file>
```

5. start other nodes: same as starting the bootnode but additionally provide the boot multiaddr: `--bootnodes /ip4/<ip>/tcp/<p2p port>/p2p/<node id>`

NOTES:

- `<node name>` in this case is one of: Alice, Bob, Charlie, Dave, Eve, Ferdie.
- `<basepath>` should be unique to each node
- `<pruning mode>` should be `archive` for our nodes and `archive-canonical` for the validator nodes
  - `archive` keeps all state forever
  - `archive-canonical` only keeps data of finalized blocks
- if listening on an address that belongs to a VPN add these extra args: `--allow-private-ip --discover-local --no-mdns`
- further options can be found with `mosaic-testnet-solo --help` for example ones related to rpc availability.

#### Scenario 2: real accounts spread across different servers

In this scenario we run **six** nodes across multiple machines.
We use the `mosaic-solo-live` chainspec preset with baked in initial authorities.

Our runtime generator is not yet templatable, so we presume the `mosaic-solo-live`
preset already has the proper accounts and starter session keys. We also presume, that
the deployer has access to these secrets (suri) in form of files for each node's each session key like so:

```
  node1/aura
  node1/gran
  node1/imon
  node2/aura
  ...
```

1. generate chainspec as seen above and copy it to all machines
2. for each node, insert session keys into the node's local keystore:

  ```sh
  mosaic-testnet-solo key insert --chain <chainspec> --base-path <basepath> --scheme <scheme> --key-type <key type> --suri <suri file>
  ```

  | Key Type | Scheme  |
  |----------|---------|
  | aura     | sr25519 |
  | imon     | sr25519 |
  | gran     | ed25519 |

  NOTES:

  - when defining these keys in the chainspec (currently manually in `runtime-generator`'s source) we can use `mosaic-testnet-solo key generate` to do so.
  - currently only the solochain node has functionality related to key handling, parachain node needs to be updated.

3. follow steps from `Scenario 1`, but **DO NOT** use dev account names as node names!

#### Useful links

- <https://wiki.polkadot.network/docs/maintain-guides-how-to-validate-polkadot>
- <https://docs.substrate.io/deploy/keys-and-network-operations/>
- <https://multiformats.io/multiaddr/>
- <https://docs.libp2p.io/concepts/fundamentals/addressing/>

## Parachain

To be added...

## Benchmarking pallets

Rebuild the nodes and runtimes with enabled benchmarks:

```sh
cargo build --release --features runtime-benchmarks,dev-spec
```

This will take a while, feel free to read some links from the end of this document while you are waiting.

After that, run the benchmarks for all pallets:

```sh
scripts/benchmark.sh
```

If you add a new pallet that needs benchmarking, you need to add them to the `define_benchmarks!` macro call at the end of the
runtime `lib.rs`, and also add it to the script above.
