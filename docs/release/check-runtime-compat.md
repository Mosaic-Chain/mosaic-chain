# Checking Runtime Compatibility

When releasing a new version of the project it's important to notify
the users of the API about the introduced changes.
For other developers it's also important to know what exactly changed with the new release.

As described in `ADR/004-using-subwasm-for-more-detailed-changelogs.md` we wish to add every
change to `CHANGELOG.md`. 

We use `subwasm diff` to get the full list of changes that ought to be included in the changelog.
This document describes exactly how it should be done.

## 1. Obtain previous runtime code

```bash
git checkout origin/testnet
cargo b -r -p runtime-generator
./target/release/runtime-generator native build --out-wasm prev.wasm <profile> # this is deployed to testnet
```

In some cases (eg.: when reordering pallets) solochain and parachain runtimes might
have different changes/incompatibilities, so the above process should be repeated for all (deployed) profiles.

NOTE: the previous version of `runtime-generator` might not support the `--out-wasm` argument.
In this case we can set `--target-dir` to some known place and the binary can be found at
`<target-dir>/release/wbuild/<runtime>/<runtime>.compact.compressed.wasm`

## 2. Obtain current runtime code

```bash
git checkout develop
cargo b -r -p runtime-generator
./target/release/runtime-generator native build --out-wasm current.wasm <profile>
```

## 3. Run `subwasm diff`

```bash
subwasm diff prev.wasm current.wasm
```
