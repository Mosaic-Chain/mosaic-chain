#let title = [Mosaic Chain Tokenomics]
#let version = [v4]
#let date = datetime.today()

#set text(font: "Fira Sans")
#show heading: it => pad(bottom: 0.35em, top: 0.35em, it)


#set page(
  header: place(top, dy: 12pt)[
    #text(size: 12pt, fill: gray, [#version - #date.display("[year].[month].[day]")])
    #h(1fr)
    #text(size: 12pt, fill: red.darken(20%), [For internal use only])
  ],
  margin: 4em
)

#align(center, [
  #text(size: 28pt, pad(top: 1em, bottom: 0.25em)[*#title*])
])

= Glossary

*pretoken*: tokens given out on a weekly bases in weboffice\
*premined*: everything we mint on launch except pretokens\
*total supply*: already minted tokens whether locked or not\
*circulating supply*: already minted, not locked tokens except staking incentive pool\
*max supply*: theoretical maximum token supply\
*staking incentive*: tokens given at a later stage to reward long-term staking\
*token generation event*: a point in time (session) from where block rewards are produced

= Constants

- 1 block = 6 sec
- 1 session = ~1 hour
- 1 MOS = $10^18$ tile
- max supply = 2,000,000,000 MOS
- validator slacking period = 72 sessions
- global minimum staking period = 28 days = 672 sessions
- global minimum commission rate = 1%
- global minimum staking amount = 1 MOS
- max contracts per validator: 1000
- max stake per validator = 5% of total supply.
- slash amount: 0.1% of user's stake.
- treasury / fund spending period = 28 days = 672 sessions
- validator subset selection target = 200 validator / session

= Premined token allocation

Premined tokens are allocated for various purposes according to the following table:

#table(
  columns: (auto, auto),
  inset: 7pt,
  align: horizon,
  table.header(
    [*Purpose*], [*Amount (MOS)*],
  ),
  
  [Staking incentive],
  [\~500,000,000],
  [Pretokens],
  [\~100,000,000],
  [Treasury],
  [10,000,000],
  [Development and Innovation],
  [24,000,000],
  [Financial and Liquidity Funds],
  [20,000,000],
  [Marketing and Community Building],
  [20,000,000],
  [Team and Advisors Support],
  [8,000,000],
  [Security and Compliance],
  [4,000,000],
  [Education and Documentation],
  [2,400,000]
)

== Staking incentive

We aim to incentivize long-term staking by enabling stakers to claim rewards from the _staking incentive pool_
for having their MOS staked for a long time.

After the _token generation event_ the rewards can be claimed approximately in every year from the 4th through the 8th. 

On each payout we distribute a pre-defined portion of the _staking incentive pool_. A staker's reward depends on a _score_ defined by the algorithm below.
The rewarded amount is received as a one year long vesting schedule that may be converted to stakable frozen assets.

The funds of the _staking incentive pool_ are initially owned by an account that belongs to the _staking incentive pallet_.

We define an algorithm such that the incentive grows linearly with the staked tokens and super-linearly with the time the amount is staked for. Processing a _staking action_ and payout is of constant time complexity.

=== Concepts

==== Staking action

The algorithm reacts to changes in the user's *currency* stake.\
A change can be either

- _stake_, when the user's stake increases, or
- _unstake_, when it decreases.

We call these events _staking actions_.

==== Active score ($A$)
The _active score_ grows exponentially over time to reward long-term staking.

$ A_n = A_(n-1) * 10^f $

Where $n$ refers to the current block index and $f$ is scaling factor. Defining $f$ as $0.30103/5259600$ results in a \~2x growth per year which is suitable for our purposes.\

The base of this score is the total amount currently staked by the user.\

==== Passive score ($P$)

Score can be made passive when the user unstakes some amount, but later it might be reactivated when they stake again. 

The _passive score_ does not grow over time, thus it's a setback in comparison to the _active score_.

==== Gained score ($G$)

The _gained score_ is the _active score_ minus the total amount staked by the user. In other words this is the amount that was accumulated via the exponential growth.

$ G_n := A_n - S_n $

Where $A_n$ is the current _active score_ and $S_n$ is the amount currently staked by the user.

#pagebreak()
==== Effective score ($E$)

The effective score is used to determine the cut the user gets on payout.

$ E_n := G_n + P_n $

Where $G_n$ is the current _gained score_ and $P_n$ is the current _passive score_.\
On payout the user may claim some of the _incentive reward_ proportional to their _effective score_.

==== Passivation threshold ($T_p$)

When unstaking, if the new staked amount would fall below this threshold, we make a proportional amount of _gained score_ passive. We define this threshold as such:

$ T_(p,n) := G_n / (10^(f*(n - k)) - 1) $

Where $k$ is the block index of the user's first _staking action_, $f$ is a predefined scaling factor and $G_n$ is the current _gained score_.

==== Activation threshold ($T_a$)

When staking, if the new staked amount reaches this threshold, we reactivate *all* passive score, otherwise we reactivate an amount proportional to the difference.

$ T_(a,n) := P_n / (10^(f*(n - k)) - 1) $

Where $k$ is the block index of the user's first _staking action_, $f$ is a predefined scaling factor and $P_n$ is the current _passive score_.

=== Staking action: _stake_

When staking amount $m$ we first determine how much score we have to reactivate.\ 

$ D := cases(
  0 &"if" "first staking action ",
  P_n &"if" S_n + m >= T_(a,n),
  (m P_n) / (T_(a,n) - S_n) &"else",
) $

Then we update the variables:

$ 
  S_n^' = S_n + m\
  A_n^' = A_n + D + m\
  P_n^' = P_n - D
$

 In future calculations following this _staking action_ we always use the updated values.

=== Staking action: _unstake_

When unstaking amount $m$ we first determine how much score we have to make passive.

$ D = max((T_(p,n) - (S_n - m))/T_(p,n), 0) * G_n $

Then we update the variables:

$ 
  S_n^' = S_n - m\
  A_n^' = A_n - D - m\
  P_n^' = P_n + D
$

=== Payout

Upon payout on block $n$ the amount to be distributed for user $x$ is

$ E_(n,x)/ (sum_(forall u in U) E_(n,u)) * I_n $

Where $E_(n,u)$ denotes the _effective score_ of user $u$ on block $n$, $U$ is the set of all staking users and $I_n$ is the amount to be distributed amongst all users in block $n$.

This amount can be calculated for a given user without iterating through all users as the sum of _effective scores_ can be kept track of globally.

To encourage newcomers to take part even in later payouts we cut back scores upon payout.

$ 
  A_n^' = G_n * c + S_n\
  P_n^' = P_n * c
$

Where $c in RR[0,1]$.

=== Notes

Because we want to avoid iterating the set of all staking users, we defer payout for a given user until they explicitly ask for it, or they perform a _staking action_. It is possible that this happens after *multiple* payouts, thus the implementation must be equipped to handle multiple skipped payouts as well. 

== Pretokens

Distribution of pretokens are part of on-boarding weboffice users.
The tokens are not made available on the account all at once, instead it is delivered in a vesting schedule.

The duration of the vesting schedule depends on the amount of tokens to be endowed.
The vested amount unlocks linearly in $Y$ years (around 2-4), where:

$ L := min(7, max(0, log("amount"))) $
$ Y := (L*4 + (7-L) * 2) / 7 $

This value is calculated before the on-boarding begins on-chain.

We'd also like to offer the recipients an option to convert their still vested pretokens to stakable MOS locked until the end of the original schedule. These specially locked tokens can be staked freely but cannot be used for anything else.


== Treasury

The funds in the _treasury_ are allocated by the chain's governance.
Anyone (in exchange for a deposit) can propose a spending which if authorized by the governance
becomes available at the end of the current spending period.

The treasury is funded by 20% of the block rewards and 10% of transaction fees.

Every 6 months in accordance with the governance's decision a portion of the treasury's funds are burnt to incentivize
approving spends during spending periods. There is also a 1% automatic burn rate set per spending period.

== Funds

The remainder of the allocation table concerns specific funds that are reserved for the given purpose.
These work similarly to the _treasury_ (in fact they are implemented as such) but without any stable income and
periodic burn of unused funds on top of the 1% per spending period.

Each fund is governed by a separate collective and not the chain's governance. 
The fund may be replenished via the treasury.

= Staking

There are two changes we make to the staking specification:

1. PoS validator cannot self-stake if their total stake reaches their NFT initial nominal value.
2. Delegator NFT expiry starts when its first staked.

= Expansion

Expansion is synonymous with average _block reward_ per block and we define it as such:
$ F :=12.5 * sqrt(("max_supply" - max("circulating_supply", 100,000,000)) / ("max_supply" - 100,000,000)) $

To achieve this target we have to consider the fact that in any given session
we only pay reward after the currently active validators' stake. To counteract this we consider the number of bound validators ($B$) and the number of currently active validators ($A$):

$  (B F) / A $

This yields the corrected _reward per block_ for the given session. To calculate the reward proportional to the total stake for a given session we
multiply be the session length ($L$).

$  (L B F) / A $

At the end of each session we evaluate this function and reward active validators for all blocks they produced in 
the past session.

We skim off 20% from the distributed reward and send it to the _treasury_.

= Transaction fees and tips

Transaction fees are split into 3 parts:

- 40% is burnt
- 50% goes to the validator which produces the current block
- 10% goes to the treasury

The tip (anything on top of the mandatory fee) goes to the validator in it's entirety.

= Discount validators

Validator rights sold at an unauthorized discount will not be treated specially on-chain,
rather they will be "punished" by receiving their NFT with a decreased nominal value. 
