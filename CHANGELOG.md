# Changelog

All notable changes to this project will be documented in this file.

## [0.9.0] - 2025-07-21

### 🚀 Features

- *node*: Providing symlinks to the latest chainspec in deb packages

### 📚 Documentation

- *adr*: Clear_all devnet stall

## [0.8.0] - 2025-07-17

### 🚀 Features

- Added code substitute in a new devnet chainspec

### 🐛 Bug Fixes

- Fixed clear_all bug related to multiple iterations in a single block

## [0.7.1] - 2025-07-01

### 🐛 Bug Fixes

- *airdrop*: Removed old weight for validate_unsigned

## [0.7.0] - 2025-06-20

### Runtime Compatibility Changes

In this update `pallet-airdrop` was modified to utilize signed extrinsics instead
of unsigned ones:

- The `airdrop` call is signed and does not take a `signature` argument.
- The 'rotate_key' call now takes a single `account_id: T::AccountId` argument.
- Definition of the `KeyRotated` event changed accordingly to the updates above.
- `MintingAuthority` and `Nonce` storages are removed and `MintingAuthorityId`
  is added to store the authority's `AccountId`.

### 🚀 Features

- *airdrop*: Added migrations from v100 and v101
- *airdrop*: Support setting nft metadata
- *benchmark*: Benchmark airdropping package with metadata
- Run benchmarks in ci

### 🐛 Bug Fixes

- *ci*: Cargo nextest dropped support for Rust 1.81 in a patch upgrade
- *airdrop*: Using the default minting authority seed in benchmarks
- *airdrop*: Using `Pays` to disable fees for the authorized account
- *testnet-solo*: Increment runtime version
- *testnet-solo*: Added runtime migration to executive

### 📚 Documentation

- *airdrop*: Mentioning nft metadata and guide to minting using polkadot.js

### 🧪 Testing

- *airdrop*: Tests for setting nft metadata

### ⚙️ Miscellaneous Tasks

- Ignoring wasmtime-jit-debug advisory
- Created new testnet chainspec

## [0.6.0] - 2025-06-10

### 🚀 Features

- *ci*: Check dependency licenses, versions and advisories
- Sentry integration

### 🐛 Bug Fixes

- Sentry sample_rate and pii

### ⚙️ Miscellaneous Tasks

- Flake lock update and pinning polkadot.nix to 2412-3
- Added wasmtime vulns to deny.toml

## [0.5.0] - 2025-03-27

### Runtime Compatibility Changes

- `pallet_system`
  - For the extrinsic events (`ExtrinsicSuccess` and `ExtrinsicFailed`),
    the type for its dispatch info argument was changed from
    `DispatchInfo` to `DispatchEventInfo`
- `pallet_identity`
  - Breaking changes for calls:
    - Arguments for `remove_username_authority` changed
    - New argument was added to `set_username_for`
    - Renamed `remove_dangling_username` to `unbind_username`
  - New calls added:
    - `remove_username`
    - `kill_username`
  - Breaking changes on several events
  - Four new errors added
  - Two new constants added
  - Breaking storage changes
- `pallet_collective`
  - Two new calls added: `kill` and `release_proposal_cost`
  - Three new events added: `Killed`, `ProposalCostBurned` and
    `ProposalCostReleased`
  - Added a new error: `ProposalActive`
  - Added new storage: `CostOf`

### 🚀 Features

- Upgraded polkadot-sdk to polkadot-stable-2412-2:
- *runtime-generator*: Added option to copy built wasm to some path
- Upgraded polkadot-sdk to polkadot-stable-2412-3
- *im-online*: Using pallet_session directly for getting session index as its
  already a hard dependency
- *im-online*: Only requesting the validator set once when a session starts
- Ss58 prefix is now set to 0 everywhere
- *testnet*: Increased runtime version

### 📚 Documentation

- Release checklist and documentation

### ⚙️ Miscellaneous Tasks

- Using correct name for rust-toolchain.toml

## [0.4.1] - 2025-01-24

### ⚙️ Miscellaneous Tasks

- Release script now commits the updated Cargo.lock
- Added 2 more devnet nodes and their addresses to chainspec

## [0.4.0] - 2025-01-22

### 🚀 Features

- *parachain*: Runtime on par with solochain
- *parachain*: Runtime migrations
- *parachain*: Dynamic SlashFraction and TokenGenerationFactor
- *parachain*: Updated runtime profiles
- Revised pallet configurations and parameters
- NFT collection descriptions
- *bench*: Runtime benchmark script and deb variant
- *bench*: Integrated benchmark results into runtimes

### 🐛 Bug Fixes

- *treasury*: Benchmarks ensure that spend amounts are at least equal to the ED
- *bench*: Removed redundant instances of collectives and membership from listed benchmarks
- *bench*: Custom nfts benchmark helper returns unused collection ids

### 📚 Documentation

- ADR-003 about many validators going offline

### ⚙️ Miscellaneous Tasks

- Rename parachain-template-* to mosaic-chain-*
- Moved from shell.nix to flake.nix and added missing dependencies for zombienet
- Reorganized imports and deduped pallet configuration between runtimes
- Runtime construction uses attribute macro
- Added live devnet chainspecs
- New testnet chainspec for v0.4

## [0.3.0] - 2025-01-16

### 🚀 Features

- *nft-staking*: Cursor based session ending
- *bench*: Pallet-nft-staking session-ending benchmarks

### ⚙️ Miscellaneous Tasks

- New testnet chainspec for v0.3

## [0.2.0] - 2024-12-12

### 🚀 Features

- Upgraded everything to substrate v1.11.0
- *parachain*: Incremented spec_version to 101 and set minimum period between blocks to 0
- *parachain*: Added pallet_collective and pallet_doas
- *runtime*: Added migration from v100 to v101
- *parachain*: Parachain node is now capable of using litep2p as network backend
- *testnet-solo*: Added testnet chainspec to node
- *testnet-solo*: Dynamic parameters, moved contents from tiny constants crate
- *im-online*: Our fork allows requiring passive validator nodes to be online
- *runtime-generator*: Basic generator for Mosaic chainspecs
- Added treasury as fork
- *testnet-solo*: Transaction fee is distributed and paid out
- Parachain key utils
- *testnet-solo*: Migration from template runtime
- *testnet-solo*: Checking previous and current runtime version before state migration
- Moved migration implementation to docs
- *testnet-solo*: Added funds and their collectives
- *staking*: A predetermined percentage of distributed reward now goes to the treasury
- *testnet-solo*: Session reward calculation
- *testnet-solo*: Applied constants from tokenomics
- *nft-delegation*: Items expiry now starts when its first bound
- *treasury*: Balance in pot is no longer inactive
- *testnet-solo*: Staking reward now uses Balances::active_issuance as the circulating supply
- *testnet-solo*: PoS validators no longer can self-stake currency or nft
- *airdrop*: Add vesting and our specialized airdrop pallet
- *treasury*: Upgraded to fungible interface
- *nft-staking*: Upgraded to fungible interface
- *vesting*: Forked from polkadot-sdk v1.11.0
- *vesting*: It now uses holds from fungible - tests, benchmarks, migrations removed
- *vesting-to-freeze*: Implemented basic pallet
- *vesting*: Schedule starting block is now optional
- Upgraded everything to polkadot-sdk stable2409 and removed baked-in chainspecs from nodes
- *airdrop*: The number of delegator nfts is now bound to a constant
- *nft-delegation*: Set_nominal_value no longer ensures the nft is currently bound
- *nft-staking*: More semantically correct errors
- Optional development chain-spec and runtime build into nodes
- *utils*: Added generalized run_until utility
- *nft-delegation*: Limit NFTs that can expire in a session
- *testnet-solo*: No longer using the currency trait when processing fees
- *airdrop*: Cheaper checks first in validate_unsigned
- *testnet-solo*: Added extra details to nft related events
- *nft-staking*: Currency staking hooks
- *staking-incentive*: Implemented the pallet
- *testnet-solo*: Fungible wrapper to add hold related events
- *bench*: Restored sudo weight benchmarks
- *bench*: Hold-vesting benchmarks converted from the old vesting ones
- *bench*: Im-online benchmarks converted from the old ones
- *bench*: Vesting-to-freeze benchmarks
- *bench*: Validator-subset-selection benchmarks
- *subset-selection*: Removed costly randomness source and storage getters
- *bench*: Extra benchmarks for nft-staking
- *bench*: Added benchmarks to pallet_airdrop

### 🐛 Bug Fixes

- *testnet-solo-runtime*: Upgraded testnet runtime and its mock.rs to substrate v1.9.0
- *parachain*: Imports and constans are now defined according to set feature flag in node
- *parachain*: Parachain chain_spec now contain valid json blob and tweaked scripts
- *solo-chain*: Fixed chain_spec json keys and implemented missing runtime api
- *ci*: Added missing cargo-deb dependency
- *ci*: Using yaml anchors to merge common commands
- *testnet-solo*: Implementing Defualt for RuntimeParameters for runtime-benchmarks
- Testnet dockerfile now starts a persistant chain
- *vesting-to-freeze*: Actually mutate storage and error when converting already expired schedule
- *nft-staking*: Rewarding empty accounts does not result in panic
- *airdrop*: Ensuring the endowed account will at least have the ed minted
- Parachain deb package variants
- Testnet deb package assets and docker
- Testnet deb package asset directory
- Dockerfile
- Deb package assets

### 📚 Documentation

- Collator deployment writeup
- Removed section about just from readme
- Added fixing-genesis-state-on-polkadot ADR
- *adr*: Decided not to merge node implementations
- Added solochain setup scenarios
- Added upcoming tasks to TODO.md
- Added dev dependencies to README.md explicitely
- Added state pruning arg to the appropriate section of README
- *validator-subset-selection*: Migrate from a template runtime
- Internal tokenomics
- Internal tokenomics v2
- Internal tokenomics v4

### 🧪 Testing

- *nft-staking*: Testing extrinsics, session interactions and trait implementations
- *doas*: Added tests for pallet
- *vesting-to-freeze*: Added tests
- *nft-staking*: Mock now uses `derive_impl` for some pallet configs
- *nft-staking*: Now using utils::run_until
- *nft-staking*: Fixed `delegate_nft::target_would_be_overdominant` test
- *validator-subset-selection*: Updated tests and fixed default genesis config build
- *nft-permission*: Updated and extended tests
- *nft-delegation*: Updated and extended tests
- *airdrop*: Added tests

### ⚙️ Miscellaneous Tasks

- Removed never-used tooling and added rust-analyzer to toolchain
- Removed crate version from path and git based dependencies
- Added cargo-watch to shell.nix
- Packaging nodes using cargo-deb and some package metadata additions
- Ever-increasing versions for rc deb packages
- Added docker image for local solo testnet
- Pushing testnet-solo-local to GCP
- Fetching tags before creating debian packages
- Solo local docker image refactor
- Release creating script
- Force unshallow fetch to make git-describe work properly
- Bumped rust version to 1.81.0
- Added zepter checks for features and disabled docker build for now
- Checking scripts and hooks, updated readme
- Testnet branching

## [0.1.0] - 2024-03-05

### 🧪 Testing

- *validator_subset_selection*: Fix and enable pallet unit-tests

### Refact

- *nft-delegation*: Removed validator_id parameter from NftDelegation::unbind

<!-- generated by git-cliff -->
