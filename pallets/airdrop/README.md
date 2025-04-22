# Airdrop API

On the blockchain the Airdrop pallet is responsible for creating certain assets on behalf of
the **Minting Authority** for a given account.

These assets include:

- Instantly available balance
- Vesting (balance that gradually unlocks in a given period)
- Permission NTF (granting PoS or DPoS rights to user)
- Delegator NFTs (can be used for staking)

Balance and nominal value is measured in _tiles_. 1 MOS = 10^18 tiles.

## Prerequisites

- Securely stored private key (sr25519) whose public pair is known by the runtime as the **Minting Authority**.
- A way to sign an arbitrary blob of data with the private key
- Ability to submit (un)signed transactions to a blockchain node and check for success

## Airdropping packages

Like in the case of most interactions with the blockchain runtime this too happens by
submitting an extrinsic to a running node. In this case though, we submit an **unsigned**
extrinsic. This is to ensure that the **Minting Authority** does not need an address or pay fees.

The pallet exposes a single call that handles the creation of all assets above:

- Method: `airdrop`
- Parameters:
  - `package` _(required)_: airdrop package
    - nonce _(required)_: ever-increasing sequance number for this call (u64)
    - account_id _(required)_: ss58 address of the recipient (AccountId)
    - balance _(optional)_: immediately accessible tokens in _tiles_ (u128)
    - vesting _(optional)_: gradually unlocking tokens
      - amount _(required)_: balance that unlocks overtime in _tiles_ (u128)
      - unlock_per_block _(required)_: how much of the tokens unlock per block in _tiles_ (u128)
      - start_block _(required)_: number of the block where the unlocking starts (u32)
    - permission_nft _(optional)_: permission nft details
      - permission _(required)_: "PoS" or "DPoS"
      - nominal_value _(required)_: what is the nft worth in _tiles_ (u128)
      - metadata _(optional)_: nft metadata (eg.: link to image) ([u8])
    - delegator_nfts _(optional)_: a **list** of delegator nft details
      - Each item:
        - expiration _(required)_: number of _sessions_ the nft is valid starting from it's first use (u32)
        - nominal_value _(required)_: what is the nft worth in _tiles_ (u128)
        - metadata _(optional)_: nft metadata (eg.: link to image) ([u8])
  - `signature` _(required)_: the sr25519 signature of the `data` parameter using the **Minting Authority's** private key
  
Example `package` in json (for demonstration only):

```json
{
    "nonce": 0,
    "account": "5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY",
    "balance": 5e+21,
    "permission_nft": {
      "permission": "PoS",
      "nominal_value": 5e+22
      "metadata": "ipfs://ipfs/bafybeihlyqahjigpjtswtqzzu2p3az5jro2gbkvbjzywgx6s65bq6bcv74"
    },
    "delegator_nfts": [
      {
        "expiration": 10,
        "nominal_value": 1e+22
      }
    ],
    "vesting": {
      "amount": 2e+23,
      "unlock_per_block": 1000000,
      "start_block": 16000
    }
}
```

_NOTE_: The package needs to be **SCALE encoded**. The json blob above is only to give an example of a possible package.

_NOTE:_ Only a certain number of pending transactions of this type are allowed and transactions
must be submitted ordered by the nonce. The current nonce can be read from the pallet's storage.

## Rotating the key

For security measures it might be worth rotating the **Minting Authority** keys periodically.
This can be done by submitting the new key using the `set_key` call.
This call requires `RootOrigin` and is _not_ signed by the **Minting Authority**.

Make sure there are no pending airdrops when rotating the key.
Otherwise they might get invalidated.


## Testing using subkey and polkadot.js

It is possible to mint a package without specialized tooling or scripts if we know the suri of
the **Minting Authority**'s private key and we can connect to a running node where this pallet is
deployed with polkadot.js.

1. Connect to the running node with polkadot.js
2. Navigate to Developer -> Chain state
3. Select the `airdrop` pallet's `nonce` storage and query it's value
4. Navigate to Developer -> Extrinsics
5. Select the `airdrop` pallet's `airdrop` extrinsic
6. Use the UI to assemble the desired package leaving the signature field empty
7. Copy `encoded call data`
8. Strip the first 4 characters after the `0x` prefix (call index) and the last 128 zeros (signature)
9. Sign using the remaining data using subkey: `subkey sign --suri <suri> --hex --message 0x...`
10. Paste signature and use `Submit Unsigned` to submit

## Resources

- <https://wiki.polkadot.network/docs/build-transaction-construction>
- <https://wiki.polkadot.network/docs/learn-transactions#types-of-extrinsics>
