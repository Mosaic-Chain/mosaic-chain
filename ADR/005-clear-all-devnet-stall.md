---
title: '"clear_all" devnet stall'
date: '2025-07-17'
status: approved
authors: ['vismate']
reviewers: ['wigy']
---

## Problem

On the 15th of July, 2025 around 8:15 AM UTC the devnet stopped producing blocks at the height of 2204659.
After some investigation we noticed that our nodes' CPU usage dramatically increased
around this time. Furthermore, we also noticed that the last successfully produced block was
the block before the current session end. For us this meant that the runtime was spinning in an
infinite loop when trying to end the session.

After searching the runtime code for possible loops we found
`utils::storage::ClearAll::clear_all` and `utils::storage::ClearAllPrefix::clear_all_prefix`
[link to sourcecode](https://gitlab.dlabs.hu/mosaic-chain/mosaic-chain/-/blob/0.7.1/utils/src/storage.rs#L24).

Both of these functions looked something like this:

```rust
fn clear_all(batch: u32) {
	let mut maybe_cursor = None;
	loop {
		maybe_cursor = Self::clear(batch, maybe_cursor.as_deref()).maybe_cursor;
		if maybe_cursor.is_none() {
			break;
		}
	}
} 
```

This code should terminate once the storage is fully emptied according to both the relevant documentation and intuition.
Since these functions were called upon session ending and starting (in `pallet_im_online` for example [here](https://gitlab.dlabs.hu/mosaic-chain/mosaic-chain/-/blob/0.7.1/pallets/im-online/src/lib.rs#L750))
and contained a `loop` we started testing it.

### Testing the theory

Looking at the function definition we can conclude that we only continue
looping around if `maybe_cursor` is `Some` after clearing a batch which should only happen
a limited number of times since there are a limited number of items in the storage.

Since for a relatively long time this code had been working (both on testnet and devnet) we had the suspicion
that the wrong behaviour only happens if the storage could not be cleared in a single batch.
When looking at the call-sites of these functions we saw that the provided batch size was 350 everywhere.
"Coincidentally" the number of validators on devnet surpassed 350 just recently...

To test what happens when multiple batches are needed to completely clear the storage we first wrote unit tests
where within the test function we filled a storage with 100 elements and tried to clear it in batches of 10. 

The code ran and exited almost instantly...

We now thought that the bug must only occur when using the real, file-based storage implementation which is not used
in unit tests.

To quickly test this theory we hacked in some new calls into a pallet.
One call would initialize a storage with 100 elements and an other one would
try to clear the storage in batches of 10.

After submitting the calls one after the other the results were paralyzing... for the runtime at least...
We successfully replicated the issue that caused the devnet to stop.

The bug (or undocumented feature) in `StorageMap::clear` and `StorageDoubleMap::clear_prefix` is that it assumes the "physical" storage has been updated
since the last batch and ignores the in-memory overlay of the current transaction.
So when we needed multiple batches to clear the storage, the subsequent iterations tried to delete the first batch again (ignoring the in-memory overlay
where those were already deleted) thus never progressing further.

Our hacked together tests can be found on a [separate branch](https://gitlab.dlabs.hu/mosaic-chain/mosaic-chain/-/tree/clear-all-tests).

## Solution

The issue in the code was relatively easy to fix by using another method of clearing the storage.
One method that works is using `StorageMap::drain` and `StorageDoubleMap::drain_prefix`.

Since the nodes were stuck and couldn't produce a new block in which a runtime upgrade could have been enacted, we
needed to use code substitutes to instruct the nodes to use the fixed code instead of the on-chain one.

First, we built a new runtime from the fixed code, but without changing the runtime version.
Next, we modified the **old** devnet chainspec to include the new code in the `codeSubstitutes` map with the key of the
last produced block (2204659).
Then, we [asked the paseo team](https://github.com/paseo-network/support/issues/138) to call `forceSetCurrentCode` on the relaychain,
so their nodes also run the fixed version of the code.
Finally we released a new debian package containing the new chainspec with the code substitute.

### Testing the solution

Before releasing the fixed code, we wanted to try what happens when the running nodes gradually receive the chainspec update.
We produced a version of the old code that would get into an infinite loop very soon (decreased the batch size to 2), then
started six solochain nodes running it.

Sure enough, at the end of the first session the blockchain stalled. Next, we stopped half of the nodes
and restarted them with the chainspec containing the code substitute. Because some of the restarted nodes
were part of the active set, they produced 10 or so blocks until the next session where (by coincidence)
all the nodes that had not yet been restarted were active. Then, we stopped the other half of the nodes
and restarted **a single one** with the correct code.
The node started producing blocks every 18 seconds. Perfect!
Finally, we reastarted the remaining two nodes and the block production went on without a hitch.

In conclusion the solution proved to be resilient no metter the rate or order in which the nodes
were restarted with the fixed code. 

## Resources

- [what is codesubstitutes in the chainspec - stackexchange](https://substrate.stackexchange.com/questions/5828/what-is-codesubstitutes-in-the-chain-spec/5925#5925)
- [fixed runtime deployed in code substitutes](https://mega.nz/file/VBZQHIRL#CbMX_ZND7z7D_8VLATfheGdXOrPM_a26XWBzJ10WBYA)
- [information collected during initial investigation - hackmd](https://hackmd.io/@Hyyhi1qjTYm0M5lcsNuvUQ/S1ktHo78xx)


## Appendix: Using `jq` to edit chainspec

### Obtain code from chainspec in hex

```sh
  cat chainspec.json | jq -r '.genesis.runtimeGenesis.code' > runtime.wasm.hex
```

### Add code substitute to chainspec from file

```sh
jq --rawfile code runtime.wasm.hex '.codeSubstitutes."<block height>" = $code' old-chainspec.json > old-chainspec-with-code-substitute.json
```
