use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sdk::{
	frame_support, frame_system, pallet_balances, pallet_offences, pallet_session, sp_core, sp_io,
	sp_runtime, sp_staking, sp_std,
};

use frame_support::{
	derive_impl,
	traits::ValidatorSet as _,
	weights::{RuntimeDbWeight, Weight},
};
use sp_core::{ConstU128, Get, RuntimeDebug};
use sp_runtime::{
	traits::{parameter_types, ConvertInto},
	BuildStorage, Perbill,
};

pub use utils::{
	mocking::{self, MockConfig, NftDelegationHandlerError, NftStakingHandlerError, SessionReward},
	run_until::ToSession,
};

use sp_std::num::NonZeroU32;

use crate::{self as pallet_nft_staking, PermissionType, SlashableValidators, WeightInfo};

// To avoid changes in all other files:
pub type NftStakingHandler = mocking::NftStakingHandler<Test>;
pub type NftDelegationHandler = mocking::NftDelegationHandler<Test>;
pub type NftDelegationHandlerStore = mocking::NftDelegationHandlerStore<Test>;
pub type ValidatorSet = mocking::ValidatorSet<Test>;

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

impl MockConfig for Test {
	type AccountId = AccountId;
	type ItemId = ItemId;
	type Balance = Balance;
	type PermissionType = PermissionType;
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
	type DbWeight = UnitWeights;
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
	type ShouldEndSession = mocking::AlwaysEndSession;
	type NextSessionRotation = ();
	type SessionManager =
		mocking::DummySessionManager<Test, (mocking::NftDelegationExpiry<Test, Staking>, Staking)>;
	type SessionHandler = mocking::EmptySessionHandler<Test>;
	type Keys = mocking::MockSessionKeys;
	type DisablingStrategy = ();
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
	type SessionReward = mocking::SessionRewardInstance;
	type Hooks = ();
	type OffenderToValidatorId = ConvertInto;
	type ContributionPercentage = ContributionPercentage;
	type ContributionDestination = ();
	type WeightInfo = UnitWeights;

	#[cfg(feature = "runtime-benchmarks")]
	type BenchmarkHelper = BenchmarkHelper;
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum SessionEndingLogEntry {
	OnIdle,
	CalculateTotalStake,
	MaybeResetValidatorState,
	RewardContract,
	DepositValidatorReward,
	DepositContribution,
	ChillIfFaulted,
	ChillIfDisqualified,
	SlashContract,
	RotateStagingLayers,
	CommitContract,
	CommitTotalValidatorStakes,
	UnlockCurrency,
	UnlockDelegatorNfts,
}

pub struct SessionEndingLog;
impl frame_support::traits::StorageInstance for SessionEndingLog {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "SessionEndingLog";
}

pub type SessionEndingLogStorage = frame_support::storage::types::StorageValue<
	SessionEndingLog,
	Vec<SessionEndingLogEntry>,
	frame_support::pallet_prelude::ValueQuery,
>;

impl SessionEndingLog {
	pub fn push(e: SessionEndingLogEntry) {
		SessionEndingLogStorage::mutate(|v| v.push(e));
	}

	pub fn reset() {
		SessionEndingLogStorage::put(Vec::<SessionEndingLogEntry>::new());
	}

	pub fn entries() -> Vec<SessionEndingLogEntry> {
		SessionEndingLogStorage::get()
	}
}

pub struct UnitWeights;

impl WeightInfo for UnitWeights {
	fn bind_validator() -> Weight {
		Weight::from_all(1)
	}

	fn unbind_validator(c: u32) -> Weight {
		Weight::from_all(c.into())
	}

	fn enable_delegations() -> Weight {
		Weight::from_all(1)
	}

	fn disable_delegations() -> Weight {
		Weight::from_all(1)
	}

	fn chill_validator() -> Weight {
		Weight::from_all(1)
	}

	fn unchill_validator() -> Weight {
		Weight::from_all(1)
	}

	fn self_stake_currency() -> Weight {
		Weight::from_all(1)
	}

	fn self_stake_nft() -> Weight {
		Weight::from_all(1)
	}

	fn self_unstake_currency() -> Weight {
		Weight::from_all(1)
	}

	fn self_unstake_nft() -> Weight {
		Weight::from_all(1)
	}

	fn delegate_currency() -> Weight {
		Weight::from_all(1)
	}

	fn delegate_nft() -> Weight {
		Weight::from_all(1)
	}

	fn undelegate_currency() -> Weight {
		Weight::from_all(1)
	}

	fn undelegate_nft() -> Weight {
		Weight::from_all(1)
	}

	fn set_minimum_staking_period() -> Weight {
		Weight::from_all(1)
	}

	fn set_commission() -> Weight {
		Weight::from_all(1)
	}

	fn kick() -> Weight {
		Weight::from_all(1)
	}

	fn topup() -> Weight {
		Weight::from_all(1)
	}

	fn set_minimum_staking_amount() -> Weight {
		Weight::from_all(1)
	}

	fn total_committed_stake(v: u32) -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::CalculateTotalStake);
		Weight::from_all(v.into())
	}

	fn maybe_reset_validator_state() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::MaybeResetValidatorState);
		Weight::from_all(1)
	}

	fn reward_contract(d: u32) -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::RewardContract);
		Weight::from_all(d.into())
	}

	fn deposit_validator_reward() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::DepositValidatorReward);
		Weight::from_all(1)
	}

	fn deposit_contribution() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::DepositContribution);
		Weight::from_all(1)
	}

	fn chill_if_faulted() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::ChillIfFaulted);
		Weight::from_all(1)
	}

	fn chill_if_disqualified() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::ChillIfDisqualified);
		Weight::from_all(1)
	}

	fn slash_contract(d: u32) -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::SlashContract);
		Weight::from_all(d.into())
	}

	fn rotate_staging_layer() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::RotateStagingLayers);
		Weight::from_all(1)
	}

	fn commit_full_contract() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::CommitContract);
		Weight::from_all(1)
	}

	fn commit_empty_contract() -> Weight {
		Weight::from_all(1)
	}

	fn commit_total_validator_stakes() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::CommitTotalValidatorStakes);
		Weight::from_all(1)
	}

	fn unlock_currency() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::UnlockCurrency);
		Weight::from_all(1)
	}

	fn unlock_delegator_nft() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::UnlockDelegatorNfts);
		Weight::from_all(1)
	}

	fn session_ended(o: u32, v: u32) -> Weight {
		Weight::from_all((o + v).into())
	}

	fn on_idle() -> Weight {
		SessionEndingLog::push(SessionEndingLogEntry::OnIdle);
		Weight::from_all(1)
	}
}

impl Get<RuntimeDbWeight> for UnitWeights {
	fn get() -> RuntimeDbWeight {
		RuntimeDbWeight { read: 1, write: 1 }
	}
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	let predefined_keys = (0..16)
		.map(|n| {
			let account = n;
			let keys = mocking::MockSessionKeys::from_index(n);
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
