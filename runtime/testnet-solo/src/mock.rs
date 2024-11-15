// construct_runtime! macro creates some non-camel-case type names.
#![allow(non_camel_case_types)]

use sdk::{
	frame_support, frame_system, pallet_balances, pallet_collective, pallet_membership, sp_core,
	sp_io, sp_runtime,
};

use frame_support::{
	parameter_types,
	traits::{ConstU64, EitherOfDiverse},
	weights::Weight,
};

use frame_system::{EventRecord, Phase};
use sp_core::{bounded_vec, ConstU128, ConstU32, H256};
use sp_runtime::{
	traits::{AccountIdLookup, BlakeTwo256, IdentifyAccount, Verify},
	BuildStorage, MultiSignature,
};

pub type Signature = MultiSignature;
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
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
	}
);

pub type Balance = u128;

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
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type EnsureOrigin = pallet_collective::EnsureProportionAtLeast<AccountId, TestCouncil, 2, 3>;
	type WeightInfo = ();
}

pub fn account(id: u64) -> AccountId {
	let id_as_bytes = id.to_ne_bytes();
	let zeros: [u8; 24] = [0; 24];
	let ret: [u8; 32] = [&id_as_bytes[..], &zeros[..]].concat().try_into().unwrap();

	ret.into()
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
			members: bounded_vec![account(ALICE), account(BOB), account(CHARLIE)],
			phantom: Default::default(),
		},
	}
	.build_storage()
	.unwrap()
	.into();
	ext.execute_with(|| System::set_block_number(1));
	ext
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
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
}
