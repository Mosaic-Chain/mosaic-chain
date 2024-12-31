mod nft_delegation_handler;
mod nft_staking_handler;
mod session;

pub use nft_delegation_handler::{
	Error as NftDeleationHandlerError, NftDelegationHandler, NftDelegationHandlerStore,
};
pub use nft_staking_handler::{Error as StakingHandlerError, NftStakingHandler};
pub use session::{MockSessionKeys, SessionReward, SessionRewardInstance, ToSession, ValidatorSet};

use sdk::{
	frame_support, frame_system, pallet_balances, pallet_offences, pallet_session, sp_core, sp_io,
	sp_runtime, sp_staking, sp_std,
};

use frame_support::{derive_impl, pallet_prelude::*, traits::ValidatorSet as _};
use pallet_session::SessionManager;
use sp_core::ConstU128;
use sp_runtime::{
	traits::{parameter_types, ConvertInto},
	BuildStorage, Perbill,
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

// pub type Signature = MultiSignature;
// pub type AccountPublic = <Signature as Verify>::Signer;
pub type AccountId = <Test as frame_system::Config>::AccountId;
pub type Balance = u128;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
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
	pub const MaximumBoundValidators: u32 = 4000;
	pub const SlackingPeriod: u32 = 5;
	pub const ContributionPercentage: Perbill = Perbill::from_percent(20);
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
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

#[cfg(feature = "runtime-benchmarks")]
pub struct BenchmarkHelper;

#[cfg(feature = "runtime-benchmarks")]
impl super::BenchmarkHelper<Test> for BenchmarkHelper {
	fn id_tuple_from_account(acc: AccountId) -> AccountId {
		acc
	}
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
	type MaximumBoundValidators = MaximumBoundValidators;
	type SessionReward = SessionRewardInstance;
	type Hooks = ();
	type OffenderToValidatorId = ConvertInto;
	type ContributionPercentage = ContributionPercentage;
	type ContributionDestination = ();
	type WeightInfo = ();

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BenchmarkHelper;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let predefined_keys = (0..16)
		.map(|n| {
			let account = n;

			let keys = MockSessionKeys { dummy: n.into() };
			(account, account, keys)
		})
		.collect::<Vec<_>>();

	pallet_session::GenesisConfig::<Test> { keys: predefined_keys, ..Default::default() }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
