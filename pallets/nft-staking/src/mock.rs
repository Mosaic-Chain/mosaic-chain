mod nft_delegation_handler;
mod nft_staking_handler;
mod session;

pub use nft_delegation_handler::{
	Error as NftDeleationHandlerError, NftDelegationHandler, NftDelegationHandlerStore,
};
pub use nft_staking_handler::{Error as StakingHandlerError, NftStakingHandler};
pub use session::{
	run_to_block, MockSessionKeys, SessionReward, SessionRewardInstance, ValidatorSet,
};

use sdk::{
	frame_support, frame_system, pallet_balances, pallet_offences, pallet_session, sp_core, sp_io,
	sp_runtime, sp_staking, sp_std,
};

use frame_support::{pallet_prelude::*, traits::ValidatorSet as _};
use pallet_session::SessionManager;
use sp_core::{ConstU128, ConstU16, ConstU64, H256};
use sp_runtime::{
	testing::UintAuthorityId,
	traits::{parameter_types, BlakeTwo256, ConvertInto, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature, Perbill,
};

use sp_std::num::NonZeroU32;
use utils::traits::SessionHook;

use crate::{self as pallet_nft_staking, SlashableValidators};

type Block = frame_system::mocking::MockBlock<Test>;
type ItemId = u32;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		Staking: pallet_nft_staking,
		Offences: pallet_offences,
		Session: pallet_session,
	}
);

pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;
pub type Balance = u128;

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
	type RuntimeTask = RuntimeTask;
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

impl pallet_offences::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type IdentificationTuple = AccountId;
	type OnOffenceHandler = Staking;
}

pub struct Offence {
	pub offenders: Vec<AccountId>,
	pub session: utils::SessionIndex,
}

impl sp_staking::offence::Offence<AccountId> for Offence {
	const ID: sp_staking::offence::Kind = *b"mos-test-offence";

	type TimeSlot = utils::SessionIndex;

	fn offenders(&self) -> Vec<AccountId> {
		self.offenders.clone()
	}

	fn session_index(&self) -> utils::SessionIndex {
		self.session
	}

	fn validator_set_count(&self) -> u32 {
		SlashableValidators::<Test>::validators().len() as u32
	}

	fn time_slot(&self) -> Self::TimeSlot {
		self.session
	}

	fn slash_fraction(&self, _offenders_count: u32) -> Perbill {
		Perbill::from_percent(1)
	}
}

parameter_types! {
	pub const MinimumCommission: Perbill = Perbill::from_percent(1);
	pub const MinimumStakingAmount: Balance = 10;
	pub const MinimumStakingPeriod: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(200) }; // approx. 1 week
	pub const NominalValueThreshold: Perbill = Perbill::from_percent(80);
	pub const MaximumStakePercentage: Perbill = Perbill::from_percent(15);
	pub const MaximumContractsPerValidator: u32 = 1000;
	pub const SlackingPeriod: u32 = 5;
	pub const ContributionPercentage: Perbill = Perbill::from_percent(20);
}

impl pallet_balances::Config for Test {
	type Balance = Balance;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = RuntimeHoldReason;
	type RuntimeFreezeReason = ();
	type MaxLocks = ConstU32<50>;
}

impl pallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = Self::AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = session::AlwaysEndSession;
	type NextSessionRotation = ();
	type SessionManager = session::DummySessionManager<AccountId, (NftDelegationHandler, Staking)>;
	type SessionHandler = session::EmptySessionHandler;
	type Keys = session::MockSessionKeys;
	type WeightInfo = ();
}

impl pallet_nft_staking::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
	type Balance = Balance;
	type ItemId = ItemId;
	type Fungible = Balances;
	type NftStakingHandler = NftStakingHandler;
	type NftDelegationHandler = NftDelegationHandler;
	type SlackingPeriod = SlackingPeriod;
	type NominalValueThreshold = NominalValueThreshold;
	type MinimumStakingPeriod = MinimumStakingPeriod;
	type MinimumCommissionRate = MinimumCommission;
	type MinimumStakingAmount = MinimumStakingAmount;
	type MaximumStakePercentage = MaximumStakePercentage;
	type MaximumContractsPerValidator = MaximumContractsPerValidator;
	type SessionReward = SessionRewardInstance;
	type OnReward = ();
	type OffenderToValidatorId = ConvertInto;
	type ContributionPercentage = ContributionPercentage;
	type ContributionDestination = ();
}

pub fn account(id: u64) -> AccountId {
	let id_as_bytes = id.to_ne_bytes();
	let zeros: [u8; 24] = [0; 24];
	let ret: [u8; 32] = [&id_as_bytes[..], &zeros[..]].concat().try_into().unwrap();

	ret.into()
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let predefined_keys = (0..16)
		.map(|n| {
			let account = account(n);

			let keys = MockSessionKeys { dummy: n.into() };
			(account.clone(), account, keys)
		})
		.collect::<Vec<_>>();

	pallet_session::GenesisConfig::<Test> { keys: predefined_keys, ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
