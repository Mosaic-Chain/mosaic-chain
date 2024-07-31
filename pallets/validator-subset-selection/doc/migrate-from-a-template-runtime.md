# Migrating from a template runtime

Since the session pallet always plans a session ahead, "hot-swapping" `SessionManager` implementations can be awkward.
Do we skip to the the end of the current session? Do we skip to the end of the next? Solutions involving these don't seem optimal.

When upgrading from a runtime with `PeriodicSessions<Period, Offset>` as the
`SessionManager` implementation to `ValidatorSubsetSelection`
we can almost seemlessly transition between them by sleekly initializing the state of `ValidatorSubsetSelection`.

We suppose that in the current runtime the `SessionManager`, `EstimateNextSessionRotation` and `ShouldEndSession`
implementation is `PeriodicSessions<Period, Offset>`, where offset is 0.

During runtime upgrade we initialize the storages in `ValidatorSubsetSelection` as such:

```rust
let current_block = System::block_number();
let current_session_end = ((current_block + period - 1) / PERIOD) * PERIOD;

pallet_validator_subset_selection::CurrentSessionEnd::<Runtime>::put(
  current_session_end,
);

pallet_validator_subset_selection::NextSessionEnd::<Runtime>::put(
  current_session_end + PERIOD,
);

pallet_validator_subset_selection::CurrentSessionLength::<Runtime>::put(PERIOD);
pallet_validator_subset_selection::AvgSessionLength::<Runtime>::put(PERIOD);  
```

These storage values make `ValidatorSubsetSelection` behave consistently after upgrade.
Keep in mind, that right after the upgrade all rules from the new runtime apply to sessions started or
planned before the upgrade.


