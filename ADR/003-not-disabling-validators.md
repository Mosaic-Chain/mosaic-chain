---
title: 'Not Disabling Validators'
date: '2025-01-06'
status: approved
authors: ['wigy']
reviewers: ['vismate']
---

## Context and Problem Statement

The original `pallet_staking` in the Polkadot SDK creates a new era
(reelects active validators) if too many validators are disabled for any
reasons. We could get to that situation if too many validators are chilled.

Though at the moment we are not changing the set of active validators
mid-session, and we just stage any changes to the coming subset selections.
Also, not only the current session, but also the next one is already planned.

What should we do in order to avoid problems when too many active validators go
offline?

## Considered Options

* Chilling could disable active validators mid-session -> Validators would just
  chill their offline validators to avoid slashing.
* We could introduce an extrinsic to start a new session -> We have an on-chain
  council, so it would still require multiple extrinsics included in a block to
  execute that code. In emergencies block generation could slow down, so this
  extra complexity would only solve this partially.
* Whitelist some collators that can always produce blocks even when they are not
  active -> Could solve the chain halting problem.

## Decision Outcome

We believe we should just live with the fact that if most of the active
validators go offline, the chain will proceed with the few validators that are
still online.

In a solochain the finalization of the blocks created in this session will
likely happen in the next session with possibly more online validators.

In a parachain we might still have fewer blocks generated in that hour, but as
long as the online collators can send those blocks to relay chain validators,
the finalization will happen there independently from our active set. So if we
set a few validators to be invulnarable in `pallet-collator-selection`, those
validators can pull out the parachain from a deadend.
