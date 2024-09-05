# Upcoming chain-related tasks

NOTE: some tasks were moved forward from later milestones.
This is because we want to look at the solochain runtime as
internal integration test environment, so it should have the same
features as our parachains runtimes have.
Before going live either on paseo or polkadot internal tests
should be conducted using the solochain.

We can of course shedule these smartly to align better with
our current allocation plans.

## Implement tokenomics

- Finalize requirements (may need to reopen tasks)
  - https://app.clickup.com/t/9015416752/86bzrfjvm
  - https://app.clickup.com/t/9015416752/86c050e0t
- Implement needed changes
  - https://app.clickup.com/t/9015416752/86bzrfk40
  - https://app.clickup.com/t/9015416752/86by1p1uw

## Validator Onboarding

- Finalize (initial) protocol for minting / bridging NFTs
- Write, test, benchmark pallet Airdrop
  - https://app.clickup.com/t/86bzrfujn
  - https://app.clickup.com/t/86bxyaw3y

## Test pallets

- Review existing tests, write more if needed
- Resume writing staking tests
  - https://app.clickup.com/t/9015416752/86bxv6qfb - staking tests
- Fix failing test (currently filtered out) related to Default genesis config construction

## Benchmark runtime

- Write benchmarks for each pallet
  - https://app.clickup.com/t/9015416752/86bxvb9e0
- Do a runtime benchmark to define weights
  - The resulting values influence call fees
- Take pallets out of devmode

## Misc

- Support `SignedExtensions` for `CheckMetadata`
  - https://app.clickup.com/t/86bzmu58e
- Runtime parameters: some values are arbitrary and only make sense for testing. Go through and finalize them.
  - This requires our full attention, and doing it together would be preferred.
  - Ideally would be done in multiple runs over a period of time
- Haven't updated the dependencies for a while; do a round of updates
  - https://app.clickup.com/t/86bxquvd8
- Update old outdated docs and scripts
- Break up the 1000 line runtime code into modules and `mosaic-chain-common`
crate
