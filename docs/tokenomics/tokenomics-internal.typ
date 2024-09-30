#let title = [Mosaic Chain Tokenomics]
#let version = [v2]
#let date = datetime.today()

#set text(font: "Fira Sans")
#show heading: it => pad(bottom: 0.25em, top: 0.25em, it)


#set page(
  header: place(top, dy: 12pt)[
    #text(size: 12pt, fill: gray, [#version - #date.display("[year].[month].[day]")])
    #h(1fr)
    #text(size: 12pt, fill: red.darken(20%), [For internal use only])
  ],
  margin: 3.5em
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

- 1 block = 6 sec\
- 1 session = ~1 hour\
- 1 MOS = $10^18$ tile\
- max supply = 2,000,000,000 MOS\
- validator slacking period = 72 sessions\
- global minimum staking period = 28 days = 672 sessions\
- global minimum commission rate = 1%\
- global minimum staking amount = 1 MOS\
- max contracts per validator: 1000\
- max stake per validator = 5% of total supply
- treasury / fund spending period = 28 days = 672 sessions
- validator subset selection target = 200 validator / session

= Premined token allocation

Premined tokens are allocated for various purposes according to the following table:

#table(
  columns: (auto, auto),
  inset: 10pt,
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

The rewards can be claimed from the 4th section after the _token generation event_ through the 8th.

A section's length is defined as 5,733,816 blocks.

In each section we distribute a pre-defined portion of the _staking incentive pool_. A staker's reward in these sections
depend on the amount staked and the time the amount is staked for.

The funds of the _staking incentive pool_ are initially owned by an account in control of the _staking incentive pallet_.

#text(fill:red, [Method TBA])

== Pretokens

Distribution of pretokens are part of on-boarding weboffice users.
The tokens are not made available on the account all at once, instead it is delivered in a vesting schedule.

The duration of the vesting schedule depends on the amount of tokens to be endowed.
The vested amount unlocks linearly in $Y$ years (around 2-4), where:

$ L = min(7, max(0, log("amount"))) $
$ Y = (L*4 + (7-L) * 2) / 7 $

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

#pagebreak()

= Staking

There are two changes we make to the staking specification:

1. PoS validator cannot self-stake if their total stake reaches their NFT initial nominal value.
2. Delegator NFT expiry starts when its first staked.

= Expansion

Expansion is synonymous with average _block reward_ per block and we define it as such:
$ F = 12.5 * sqrt(("max_supply" - max("circulating_supply", 100,000,000)) / ("max_supply" - 100,000,000)) $

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
