# Staking API
## Design principles:

- Least suprise for the end user
- "Strict API" - no soft failures:
  - A call either succeeds according to the provided paramaters or it fails.
- API is primarily for the dev, secondarily for the end user
  - API should be easy to implement and maintain
  - It should be high level enough to enable freedom of impementation and should be easy to test
  - It should be ergonomic for the fronted devs.
  - We'll provide a frontend with good UX for the end users.

## Definitions:
(Here I ommit the basic ones that everybody from the dev team knows about)

- **Staking / delegation**:
  - I'll refer to all amounts as "stake" and the act of modifying stake as "staking" and "unstaking".
  - If the caller and the target accounts differ we can call staking "delegation" and unstaking "undelegation".
  - We wish to treat delegations and "self-staking" similarly, usually the same rules should apply.

_(NOTE: even in the current implementation we have these two cases and have to treat them seperately in almost every function.\
The goal is to make the two cases more similar so that we can finally abstract these away at some point)_

- **Contract:** a contract encapsulates the terms of a staking agreement between accounts.
  - There is at most **one** contract per validator-staker pair.
  - Terms: staked amount, min.staking period and commission rate.
  - The terms do not change if the contract is untouched.
  - The current terms set by the validator are applied every time an amount is added to the stake.
    - These new terms become active at the begginning of the next session.
    - The start of the new staking period is also reset to the upcoming session.

- **Binding / unbinding:** TODO

- **Freezing / Unfreezing:** TODO

## Constants
- default minimal staking period
- default minimal staking amount
 - _(Note: we do not guarantee, that all contracts have at least this stake at all times. There are edgecases, like autochilled validators)_ (TODO ?)
- minimum nominal percent

## Public API:
We have many distinct cases when staking depending on whether the caller stakes currency or nft or whether they self-stake or delegate.

We can represent all these cases in our public API, as I suspect many of these would be completely
seperate functionalities of the front-end. _(I should ask frontend devs about this)_

_(In the actual implementation reducing the number of cases as soon as possible is desired
so we don't need to handle them separately deeper down in the impl.)_

### `bind_validator(caller, item)`

Preconditions:
- `item` is an existing permission nft owned by `caller`
- `caller` does not have an other item bound
- `item`'s nominal value is at least minimal nominal percent of it's initial nominal value. (this value is greater than the minimal staking amount)
_(Note: parters might have tokens with 0 nominal value, this should not cause an issue)_

Effects:
- `item` is "immediately" bound.
- `item`'s nominal value is scheduled to be added to both `caller`'s and the global stake.

### `unbind_validator(caller)`

Preconditions:
- `caller` is a bound, chilled validator
- `caller` is not in the active validator set
- `caller` is not inevitably scheduled to be in the next validator set
- non of the `caller`'s contracts' staking period is below the minimum staking period. (including not yet active contracts)
_(Note this includes the validator's self-contract as well)_
_(Note: this is important, so validators don't just unbind then quickly rebind to reset terms. esp. PoS.)_

Effects:
- `caller`'s permission nft is unbound
- `caller`'s currency based self-stake is unlocked
- `caller`'s self-bound delegator nfts are unbound
- `caller`'s stake is **scheduled** to be removed from global stake (TODO: should we do it immediately? if preconditions are met we owe nothing to the validator this session)
- If `caller` is DPoS:
  - all delegators' currency based stake is unlocked (these can happen "immediately")
  - all delegators' bound delegator nft is unbound
  - all delegators' delegation to `caller` is **scheduled** to be removed from global stake

### `disable_delegations(caller)`

Preconditions:
- `caller` is a bound validator
- `caller` is DPoS validator
- `caller` has not yet disabled delegations (delegations can be disabled for other reasons, eg.: validator is chilled)

Effects:
- `caller` does not accept any more delegations after this call

### `enable_delegations(caller)`
Preconditions:
- `caller` is a bound validator
- `caller` is DPoS validator
- `caller` has disabled delegations with the above extrinsic

Effects:
- `caller` does now accept delegations after this call

### `chill(caller)`

Preconditions:
- `caller` is a bound validator
- `caller` is not yet chilled

Effects:
- a request is sent to subset selection not to select `caller` in the future
- `caller` is now considered chilled

### `unchill(caller)`

Preconditions:
- `caller` is bound validator
- `caller` is (auto)chilled
- `caller`'s bound permission token is still valid, its nominal value is greater then minimal nominal percent of the original. (TODO will this be possible? )

Effects:
- a request is sent to subset selecton to start considering `caller` for selection again
- `caller` is now not considered chilled

### `self_stake_currency(caller, amount)`

Preconditions:
- `caller` is a bound, not chilled  validator
- `amount` is greater or equal to the minimum staking amount
- `caller` has at least `amount` of free and lockable balance

Effects:
- the `amount` is locked "immediately" on the `caller`'s account
- the `amount` is scheduled to be added to both `caller`'s stake and the global stake in the next session
- the `caller`'s staking period is scheduled to be reset in the next session

_(Note: subsequent operations within a session are not always commutable as operations often check staged (scheduled) values instead of committed ones._
Here, because the staking period is scheduled to be reset a subsequent call to `self_unstake_currency` would fail)_

### `self_stake_nft(caller, item)`

Preconditions:
- `caller` is a bound, not chilled validator
- `item` is an existing delegator nft owned by `caller`
- `item` is not bound to another validator
- the nominal value of `item` is greater or equal to the minimum staking amount
- the `item` will not expire before the minimum staking period ends

Effects:
- the `item` is bound "immediately" to the `caller` (TODO: is this valid currently in nft_delegation?)
- the nft's nominal value is scheduled to be added to both `caller`'s stake and the global stake in the next session
- the `caller`'s own contract with themselves is scheduled to be updated: the staking period resets

### `self_unstake_currency(caller, amount)`

Preconditions:
- `caller` is a bound validator
- the `callers`'s staking period is greater or equal to the minimum staking period
- `amount` is greater or equal to the minimum staking amount
- `amount` can be deducted from the `caller`'s **currency based** stake
- the remaining currency based stake would still be greater or equal to the minimum staking amount OR be zero

Effects:
- the `amount` is **scheduled** to be unlocked on _(or added to)_ the `caller`'s account
- the `amount` is schduled to be removed from both the `caller`'s stake and the global stake

### `self_unstake_nft(caller, item)`

Preconditions:
- `caller` is a bound validator
- `item` is an existing delegator nft owned by `caller`
- `item` is bound to `caller`
- the `caller`'s staking period is greater or equal to the minimum staking period

Effects:
- the `item`'s nominal value is scheduled to be removed from both the `caller`'s stake and the global stake
- the `item` is **immediately** unbound from the `caller`'s account. 
  - _(Note: we can unbind immediately, as nfts can only be used for staking and consequent binds would only affect rewards in the following sessions)_

### `delegate_currency(caller, amount, target)`
_TODO: slippage_

Preconditions:
- `target` is a bound, not chilled validator
- `target` is a DPoS validator
- `target` accpets delegations
- `caller` and `target` are not the same (use `self_stake_currency` instead)
- `amount` is more than or equal to the minimum staking amount
- `caller` has at least `amount` lockable, free currency

Effects:
- `amount` is locked "immediately" on `caller`'s account
- `amount` is scheduled to be added to both `target`'s stake and global stake
- terms of the contract between `caller` and `target` are scheduled to be updated:
  - staking period resets (possibly to a new value) and the new commission is applied (if changed)

### `delegate_nft(caller, item, target)`
_TODO: There has been discussions about limiting delegator nft / validator_
_TODO: slippage_

Preconditions:
- `target` is a bound, not chilled validator
- `target` is a DPoS validator
- `target` accepts delegations
- `target` and `caller` are not the same (use `self_stake_nft` instead)
- `item` is a delegator nft owned by `caller`
- `item` is not bound to any other validator
- `item`'s nominal value is greater or equal to the minimal staking amount
- `item` will not expire before the minimum staking period ends

Effects:
- `item` is bound to `target` "immediately"
- `item`'s nominal value is scheduled to be added to both `target`'s stake and the global stake.
- terms of the contract between `caller` and `target` are scheduled to be updated:
  -  staking period resets (possibly to a new value), commision changes (if it's changed)

### `undelegate_currency(caller, amount, target)`

Preconditions:
- `target` is a bound validator
- `target` is a DPoS validator
- `target` and `caller` are not the same (use `self_unstake_currency` instead)
- `target` has a staking contract with `caller`
- the minimal staking period is over on the contract
- `amount` is greater or equal to the minimum staking amount
- the `amount` is available as **currency based** stake on the contract
- the remaining currency based stake would be more than or equal to the minimal staking amount OR be exactly 0

Effects:
- `amount` currency is scheduled to be unlocked on _(or added to)_ the `caller`'s account
- `amount` currency is scheduled to be removed from both the `target`'s stake and global stake

### `undelegate_nft(caller, item, target)`

Preconditions:
- `target` is a bound validator
- `target` is a DPoS validator
- `target` and `caller` are not the same (use `self_unstake_nft` instead)
- `item` is a delegator nft owned by `caller`
- `item` is bound to `target`
- the minimum staking period has already ended for this contract

Effects:
- `item` is unbound from `target`
- `item`'s nominal value is scheduled to be removed from both `target`'s stake and the global stake

### `set_minimum_staking_period(caller, new_min_period)` (TODO should we let them set this? also TODO should we let them set their own min stake as well?)

Preconditions:
- `caller` is a bound, not chilled validator
- `caller` is a DPoS validator
- `new_min_period` is longer or equal to the default staking period
- `new_min_period` is not unreasonably long (TODO what's unreasonable?)

Effects:
- `caller`'s new contract term is updated immediately.
  - _(Note: we should only do this immidiately after implementing slippage)_
  - _(Note: Existing contracts don't get updated)_


### `set_commission(caller, new_commission)`

Preconditions:
- `caller` is a bound, not chilled validator
- `caller` is a DPoS validator
- `new_commission` is larger or equal to minimum commission

Effects:
- `caller`'s new contract term is immediately updated
- _(Note: we should only do this after implementing slippage)_

### `kick(caller, target)`

Preconditions:
- `caller` is a bound, not chilled validator
- `caller` is a DPoS validator
- `caller` is not `target` (use `self_unstake_nft` instead)
- `caller` has a contract with `target`
- the minimal staking period has passed on this contract

Effects:
- `target`'s currency based delegation scheduled to be freed 
- `target`'s delegated nfts are unbound "immediately"
- `target`'s stake is **scheduled** to be removed both from `caller`'s stake and the global stake

TODO what should happen if target renews the contract in the same session? I guess "first come, first served, second come not served".

## Mechanisms
These details are not part of the public callable API, but are behaviours that must be defined well.

### Reward calcualtion
Rewards increase stake by default.

### Slashing
Currency/nft based stake slashed under the minimal staking amount is dusted.

### Auto-freezing

### Auto unbinding

### Delegator NFT expiration
- NFTS are still valid in the session they are expiring (consistent with staging)

## Implementation guide
- All additions and subtractions should be saturating
- Be careful with schduled (staged) and commited values.
  - Which one should be checked?
  - Be sure to stage modifications!
- What "scheduling contract with 0 stake for deletion" could mean in practice:
  - when committing changes at the beggininng of a session if you see a staged contract with 0 stake the contract should be deleted
  - _after scheduling for deletion the user can stake more in the same session. this case needs no special logic this way_
- Implementing the core logic using side effect free functions is encouraged (testability)
- We don't need to test each every addition, so don't write messy, confusing code just to satisfy the above guideline.
- Try to encode as much of the preconditions in the type system ~~as possible~~ as it makes sense.
- Try to define such a model where as many cases can be handled by the same code as possible.
