# Upcoming chain-related tasks

NOTE: some tasks were moved forward from later milestones.
This is because we want to look at the solochain runtime as
internal integration test environment, so it should have the same
features as our parachains runtimes have.
Before going live either on paseo or polkadot internal tests
should be conducted using the solochain.

We can of course schedule these smartly to align better with
our current allocation plans.

## Test pallets

- Resume testing pallets
  - https://app.clickup.com/t/86c0kqap5

## Benchmark runtime

- Do a runtime benchmark to define weights
  - The resulting values influence call fees

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
