// construct_runtime! macro creates some non-camel-case type names.
#![allow(non_camel_case_types)]

use sdk::{
	frame_support, frame_system, pallet_balances, pallet_collective, pallet_membership,
	pallet_offences, pallet_session, sp_core, sp_io, sp_runtime, sp_std,
};

use frame_support::{
	parameter_types,
	traits::{ConstU64, EitherOfDiverse},
	weights::Weight,
};

use frame_system::{EventRecord, Phase};
use sp_core::{bounded_vec, ConstU128, ConstU32, H256};
use sp_runtime::{
	traits::{AccountIdLookup, BlakeTwo256, ConvertInto},
	BuildStorage, Perbill,
};
use sp_std::num::NonZeroU32;
use utils::mocking;

pub type AccountId = u64;
pub type Balance = u128;
pub type ItemId = u64;

type Block = frame_system::mocking::MockBlock<Test>;

// this is needed, otherwise fmt will remove the :: from ::<Instance1>
#[rustfmt::skip::macros(construct_runtime)]
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		TestCouncilCollective: pallet_collective::<Instance1>,
		TestCouncilCollectiveMembership: pallet_membership::<Instance1>,
		DoAs: pallet_doas,
		Offences: pallet_offences,
		Session: pallet_session,
		HoldVesting: pallet_hold_vesting,
		VestingToFreeze: pallet_vesting_to_freeze,
		NftStaking: pallet_nft_staking,
	}
);

impl mocking::MockConfig for Test {
	type AccountId = AccountId;
	type ItemId = ItemId;
	type Balance = Balance;
	type PermissionType = pallet_nft_staking::PermissionType;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = BlockWeights;
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = AccountIdLookup<AccountId, ()>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type ExtensionsWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type RuntimeTask = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

fn default_max_proposal_weight() -> Weight {
	sp_runtime::Perbill::from_percent(80) * BlockWeights::get().max_block
}

pub fn record(event: RuntimeEvent) -> EventRecord<RuntimeEvent, H256> {
	EventRecord { phase: Phase::Initialization, event, topics: vec![] }
}

pub type MaxMembers = ConstU32<100>;

parameter_types! {
	pub const MotionDuration: u64 = 3;
	pub const MaxProposals: u32 = 257;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(Weight::MAX);
	pub static MaxProposalWeight: Weight = default_max_proposal_weight();
}

type EnsureRootOrTwoThirdCouncil = EitherOfDiverse<
	frame_system::EnsureRoot<AccountId>,
	pallet_collective::EnsureProportionAtLeast<AccountId, TestCouncil, 2, 3>,
>;

type TestCouncil = pallet_collective::Instance1;
impl pallet_collective::Config<TestCouncil> for Test {
	type RuntimeOrigin = RuntimeOrigin;
	type Proposal = RuntimeCall;
	type RuntimeEvent = RuntimeEvent;
	type MotionDuration = MotionDuration;
	type MaxProposals = MaxProposals;
	type MaxMembers = MaxMembers;
	type DefaultVote = pallet_collective::PrimeDefaultVote;
	type WeightInfo = ();
	type SetMembersOrigin = frame_system::EnsureRoot<AccountId>;
	type MaxProposalWeight = MaxProposalWeight;
	type DisapproveOrigin = EnsureRootOrTwoThirdCouncil;
	type KillOrigin = EnsureRootOrTwoThirdCouncil;
	type Consideration = ();
}

type TestCouncilMembership = pallet_membership::Instance1;
impl pallet_membership::Config<TestCouncilMembership> for Test {
	type RuntimeEvent = RuntimeEvent;
	type AddOrigin = EnsureRootOrTwoThirdCouncil;
	type RemoveOrigin = EnsureRootOrTwoThirdCouncil;
	type SwapOrigin = EnsureRootOrTwoThirdCouncil;
	type ResetOrigin = EnsureRootOrTwoThirdCouncil;
	type PrimeOrigin = EnsureRootOrTwoThirdCouncil;
	type MembershipInitialized = TestCouncilCollective;
	type MembershipChanged = TestCouncilCollective;
	type MaxMembers = ConstU32<10>;
	type WeightInfo = pallet_membership::weights::SubstrateWeight<Test>;
}

impl pallet_doas::Config for Test {
	type RuntimeCall = RuntimeCall;
	type EnsureOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, TestCouncil, 2, 3>;
	type WeightInfo = ();
}

impl pallet_offences::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = AccountId;
	type OnOffenceHandler = NftStaking;
}

impl pallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = Self::AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = mocking::session::AlwaysEndSession;
	type NextSessionRotation = ();
	type SessionManager = mocking::session::DummySessionManager<
		Test,
		(mocking::nft_delegation_handler::NftDelegationExpiry<Test, NftStaking>, NftStaking),
	>;
	type SessionHandler = mocking::session::EmptySessionHandler<Test>;
	type Keys = mocking::MockSessionKeys;
	type DisablingStrategy = ();
	type WeightInfo = ();
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = RuntimeFreezeReason;
	type MaxFreezes = ConstU32<50>;
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type DoneSlashHandler = ();
}

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl pallet_nft_staking::BenchmarkHelper<Test> for BenchmarkHelper {
	fn id_tuple_from_account(acc: AccountId) -> AccountId {
		acc
	}
}

parameter_types! {
	pub const MinimumCommission: Perbill = Perbill::from_percent(1);
	pub const MinimumStakingAmount: Balance = 10;
	pub const MinimumStakingPeriod: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(200) }; // approx. 1 week
	pub const NominalValueThreshold: Perbill = Perbill::from_percent(80);
	pub const MaximumStakePercentage: Perbill = Perbill::from_percent(15);
	pub const MaximumContractsPerValidator: u32 = 1000;
	pub const MaximumBoundValidators: u32 = 4000;
	pub const SlackingPeriod: u32 = 5;
	pub const ContributionPercentage: Perbill = Perbill::from_percent(20);
}

impl pallet_nft_staking::Config for Test {
	type RuntimeHoldReason = RuntimeHoldReason;
	type Balance = Balance;
	type ItemId = ItemId;
	type Fungible = Balances;
	type NftStakingHandler = mocking::nft_staking_handler::NftStakingHandler<Test>;
	type NftDelegationHandler = mocking::nft_delegation_handler::NftDelegationHandler<Test>;
	type SlackingPeriod = SlackingPeriod;
	type NominalValueThreshold = NominalValueThreshold;
	type MinimumStakingPeriod = MinimumStakingPeriod;
	type MinimumCommissionRate = MinimumCommission;
	type MinimumStakingAmount = MinimumStakingAmount;
	type MaximumStakePercentage = MaximumStakePercentage;
	type MaximumContractsPerValidator = MaximumContractsPerValidator;
	type MaximumBoundValidators = MaximumBoundValidators;
	type SessionReward = mocking::session::SessionRewardInstance;
	type Hooks = ();
	type OffenderToValidatorId = ConvertInto;
	type ContributionPercentage = ContributionPercentage;
	type ContributionDestination = ();
	type WeightInfo = ();

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BenchmarkHelper;
}

impl pallet_hold_vesting::Config for Test {
	type BlockNumberToBalance = ConvertInto;
	type Balance = Balance;
	type Fungible = Balances;
	type MinVestedTransfer = ConstU128<50>;
	type BlockNumberProvider = System;
	type RuntimeHoldReason = RuntimeHoldReason;
	type WeightInfo = ();
	const MAX_VESTING_SCHEDULES: u32 = 3;
}

impl pallet_vesting_to_freeze::Config for Test {
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type Balance = Balance;
	type Fungible = Balances;
	type VestingSchedule = HoldVesting;
	type BlockNumberToBalance = ConvertInto;
	type BlockNumberProvider = System;
	type MaxFrozenSchedules = ConstU32<3>;
	type MaxFreezes = ConstU32<5>;
	type MaxVestingSchedules = ConstU32<3>;
	type WeightInfo = ();
}

pub const ALICE: u64 = 1;
pub const BOB: u64 = 2;
pub const CHARLIE: u64 = 3;
pub const DAVE: u64 = 4;

pub(crate) fn new_test_ext() -> sp_io::TestExternalities {
	let mut ext: sp_io::TestExternalities = RuntimeGenesisConfig {
		system: frame_system::GenesisConfig::default(),
		balances: pallet_balances::GenesisConfig::default(),
		test_council_collective: Default::default(),
		test_council_collective_membership: pallet_membership::GenesisConfig {
			members: bounded_vec![ALICE, BOB, CHARLIE],
			phantom: Default::default(),
		},
		nft_staking: pallet_nft_staking::GenesisConfig::default(),
		session: pallet_session::GenesisConfig::default(),
		hold_vesting: pallet_hold_vesting::GenesisConfig::default(),
	}
	.build_storage()
	.unwrap()
	.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
