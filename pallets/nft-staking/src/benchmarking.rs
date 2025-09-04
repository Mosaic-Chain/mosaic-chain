//! Benchmarking setup for the permission NFT pallet

use super::*;
use crate::{
	session_ending::{
		commit::Sweep as CommitSweep,
		reward::{RewardValidator, Sweep as RewardSweep, ValidatorContext},
		slash::{OffenderContext, SlashOffender},
	},
	types::MAX_NFTS_PER_CONTRACT,
	Config as NftStakingConfig, Pallet as NftStakingPallet, PermissionType, SessionIndex,
};
use utils::traits::{NftPermission as NftPermissionT, OnDelegationNftExpire, SessionHook};

use core::{fmt, iter::IntoIterator};
use pallet_nft_delegation::Config as NftDelegationConfig;
use sdk::{
	frame_benchmarking::v2::*,
	frame_support::traits::{tokens::fungible::Mutate, Incrementable, ValidatorSet},
	frame_system::{pallet_prelude::*, RawOrigin},
	pallet_balances::{Config as BalancesConfig, Pallet as BalancesPallet},
	pallet_session::Config as SessionConfig,
	sp_staking::offence::{OffenceDetails, OnOffenceHandler},
	sp_std::{vec, vec::Vec as SpVec},
};

pub trait BenchmarkHelper<T: Config> {
	fn id_tuple_from_account(
		acc: T::AccountId,
	) -> <T as pallet_offences::Config>::IdentificationTuple;
}

const MAX_ACTIVE_VALIDATORS: u32 = 400;
const UNIT: u128 = 10u128.pow(18); // 1 MOS = 10^18 tile

#[derive(Debug, Clone)]
struct Account<T: NftStakingConfig> {
	id: T::AccountId,
}

impl<T: NftStakingConfig> Default for Account<T> {
	fn default() -> Self {
		let id: T::AccountId = account("unknown", 0, 0);
		Self { id }
	}
}

impl<T: NftStakingConfig> Account<T> {
	fn index(group: &'static str, i: u32) -> Self {
		let id: T::AccountId = account(group, i, 0);
		Self { id }
	}

	fn validator(i: u32) -> Self {
		Self::index("validator", i)
	}

	fn delegator(i: u32) -> Self {
		Self::index("delegator", i)
	}

	fn signed_origin(&self) -> RawOrigin<T::AccountId> {
		RawOrigin::Signed(self.id.clone())
	}
}

impl<T: NftStakingConfig + BalancesConfig> Account<T>
where
	T: NftStakingConfig<Balance = <T as BalancesConfig>::Balance>,
{
	fn endow(&self, amount: <T as NftStakingConfig>::Balance) {
		<BalancesPallet<T> as Mutate<_>>::mint_into(&self.id, amount).expect("Should succeed");
	}
}

#[derive(Debug, Clone)]
struct DelegatorNft<T: NftStakingConfig> {
	account: Account<T>,
	expiration: SessionIndex,
	nominal_value: T::Balance,
}

impl<T: NftStakingConfig> Default for DelegatorNft<T> {
	fn default() -> Self {
		let account = Default::default();
		let expiration = 24 /* hours */ * 30 /* days */ * 24u32 /* months */;
		let nominal_value: T::Balance = (50 * UNIT).into();

		Self { account, expiration, nominal_value }
	}
}

impl<T: NftStakingConfig> DelegatorNft<T> {
	fn mint(&self) -> T::ItemId {
		T::NftDelegationHandler::mint(&self.account.id, self.expiration, &self.nominal_value)
			.expect("should succeed")
	}
}

#[derive(Debug, Clone)]
struct PermissionNft<T: NftStakingConfig> {
	account: Account<T>,
	permission: PermissionType,
	nominal_value: T::Balance,
}

impl<T: NftStakingConfig> Default for PermissionNft<T> {
	fn default() -> Self {
		let account = Default::default();
		let permission = PermissionType::DPoS;
		let nominal_value: T::Balance = (100 * UNIT).into();

		Self { account, permission, nominal_value }
	}
}

impl<T: NftStakingConfig> PermissionNft<T> {
	fn signed_origin(&self) -> RawOrigin<T::AccountId> {
		self.account.signed_origin()
	}

	fn mint(&self) -> T::ItemId {
		T::NftStakingHandler::mint(&self.account.id, &self.permission, &self.nominal_value)
			.expect("should succeed")
	}

	fn mint_and_bind(&self) -> T::ItemId {
		let item_id = self.mint();
		NftStakingPallet::<T>::bind_validator(self.signed_origin().into(), item_id.clone())
			.expect("Should succeed");
		item_id
	}
}

struct Fixture<T: NftStakingConfig> {
	validator: PermissionNft<T>,
}

impl<T: NftStakingConfig + BalancesConfig> Default for Fixture<T>
where
	T: NftStakingConfig<Balance = <T as BalancesConfig>::Balance>,
	<T as BalancesConfig>::Balance: From<u128>,
{
	fn default() -> Self {
		let mut bootstrap_validator = PermissionNft::<T>::default();
		bootstrap_validator.account.id = account("bootstrap", 0, 0);
		bootstrap_validator.mint_and_bind(); // Without this the validator superset would become empty.
		bootstrap_validator.account.endow((10 * UNIT).into());
		assert!(!crate::SelectableValidators::<T>::validators().is_empty());
		let bootstrap_validator_id = T::ValidatorIdOf::convert(bootstrap_validator.account.id)
			.expect("Should be convertible");
		pallet_session::Validators::<T>::put::<SpVec<T::ValidatorId>>(vec![
			bootstrap_validator_id.clone()
		]);

		let validator = PermissionNft::<T>::default();
		validator.account.endow((10 * UNIT).into());

		Self { validator }
	}
}

impl<T: NftStakingConfig + SessionConfig> Fixture<T> {
	fn session_passed(&self) {
		let session = pallet_session::CurrentIndex::<T>::get();
		pallet_session::CurrentIndex::<T>::set(session + 1);
	}

	fn period_passed(&self) {
		let period = <T as NftStakingConfig>::MinimumStakingPeriod::get();
		let session = pallet_session::CurrentIndex::<T>::get();
		pallet_session::CurrentIndex::<T>::set(session + period.get());
	}
}

fn commit<T>() -> Weight
where
	T: NftStakingConfig,
	T::AccountId: From<<T as SessionConfig>::ValidatorId>,
{
	// we need to commit all staged data
	<Pallet<T> as SessionHook>::session_ended(0u32).expect("Should succeed");
	Pallet::<T>::on_idle(0u32.into(), Weight::MAX)
}

fn endow_mint_and_bind<T>(v: &Account<T>)
where
	T: BalancesConfig + NftStakingConfig<Balance = <T as BalancesConfig>::Balance>,
	<T as BalancesConfig>::Balance: From<u128>,
	T::AccountId: codec::EncodeLike<<T as SessionConfig>::ValidatorId>,
{
	let nft = PermissionNft::<T> { account: v.clone(), ..Default::default() };
	v.endow((10 * UNIT).into());
	nft.mint_and_bind();
}

fn set_session_validators<'a, T>(validators: impl IntoIterator<Item = &'a Account<T>>)
where
	T: SessionConfig + NftStakingConfig,
	T::AccountId: codec::EncodeLike<<T as SessionConfig>::ValidatorId>,
{
	let x = validators.into_iter().map(|acc| acc.id.clone()).collect::<SpVec<_>>();
	pallet_session::Validators::<T>::put(x);
}

fn session_end_fixture<T>(o: u32, v: u32)
where
	T: SessionConfig + NftStakingConfig,
	T::AccountId: codec::EncodeLike<<T as SessionConfig>::ValidatorId>,
{
	// It is not a StorageValue, but a Get<u128>, so we cannot T::SessionReward::set(5000u128).expect("reward set");
	let validators = (0..v).map(|i| Account::<T>::validator(i)).collect::<SpVec<_>>();
	set_session_validators(&validators);
	let slash = Perbill::from_rational(1u32, 1000u32);

	for v in validators.iter().take(o.min(v - 1) as usize) {
		InverseSlashes::<T>::insert(&v.id, slash);
	}
}

#[benchmarks(
	where
		T: BalancesConfig<Balance = <T as NftStakingConfig>::Balance> + SessionConfig + NftDelegationConfig<ItemId = <T as NftStakingConfig>::ItemId>,
		<T as NftStakingConfig>::ItemId: Incrementable + From<u32>,
		BlockNumberFor<T>: From<u32> + fmt::Display,
		<T as NftStakingConfig>::Balance: From<u32> + From<u128>,
		T::AccountId: From<<T as SessionConfig>::ValidatorId> + codec::EncodeLike<<T as SessionConfig>::ValidatorId>,
)]
mod benchmarks {

	use super::*;

	#[benchmark]
	fn bind_validator() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		let item_id = validator.mint();

		let origin = validator.signed_origin();
		#[extrinsic_call]
		_(origin, item_id);
	}

	#[benchmark]
	fn unbind_validator(c: Linear<1, { T::MaximumContractsPerValidator::get() }>) {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind(); // This always gives 1 contract, the self-contract

		let target = &validator.account.id;
		let Some(ValidatorDetails::DPoS {
			accept_delegations: true,
			min_staking_period,
			commission,
		}) = Validators::<T>::get(target)
		else {
			panic!("Should succeed");
		};
		for i in 1..c {
			let delegator = Account::<T>::delegator(i);
			let nft = DelegatorNft::<T> {
				account: delegator.clone(),
				expiration: 24 /* hours */ * 30 /* days */ * 24u32 + i, // we need to spread out expiration to different sessions
				nominal_value: (50 * UNIT).into(),
			};
			let item_id = nft.mint();
			let origin = delegator.signed_origin();
			NftStakingPallet::<T>::delegate_nft(
				origin.clone().into(),
				item_id.clone(),
				target.clone(),
				min_staking_period,
				commission,
			)
			.expect("Should succeed");

			delegator.endow((2000 * UNIT).into());
			let amount = (50 * UNIT).into();
			NftStakingPallet::<T>::delegate_currency(
				origin.into(),
				amount,
				target.clone(),
				min_staking_period,
				commission,
			)
			.expect("Should succeed");
		}
		NftStakingPallet::<T>::chill_validator(validator.signed_origin().into())
			.expect("Should succeed");
		fixture.period_passed();
		// An extra session is needed for unbinding
		fixture.session_passed();
		commit::<T>();
		let origin = validator.signed_origin();

		#[extrinsic_call]
		_(origin, c);
	}

	#[benchmark]
	fn enable_delegations() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		NftStakingPallet::<T>::disable_delegations(validator.signed_origin().into())
			.expect("Should succeed");
		let origin = validator.signed_origin();

		#[extrinsic_call]
		_(origin);
	}

	#[benchmark]
	fn disable_delegations() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let origin = validator.signed_origin();

		#[extrinsic_call]
		_(origin);
	}

	#[benchmark]
	fn chill_validator() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let origin = validator.signed_origin();

		#[extrinsic_call]
		_(origin);
	}

	#[benchmark]
	fn unchill_validator() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let origin = validator.signed_origin();
		NftStakingPallet::<T>::chill_validator(origin.into()).expect("Should succeed");
		let origin = validator.signed_origin();

		#[extrinsic_call]
		_(origin);
	}

	#[benchmark]
	fn self_stake_currency() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		validator.account.endow((2000 * UNIT).into());

		let origin = validator.signed_origin();
		let amount: <T as NftStakingConfig>::Balance = (50 * UNIT).into();
		#[extrinsic_call]
		_(origin, amount);
	}

	#[benchmark]
	fn self_stake_nft() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let nft = DelegatorNft::<T>::default();
		let item_id = nft.mint();

		let origin = validator.signed_origin();
		#[extrinsic_call]
		_(origin, item_id);
	}

	#[benchmark]
	fn self_unstake_currency() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		validator.account.endow((2000 * UNIT).into());
		let origin = validator.signed_origin();
		let amount: <T as NftStakingConfig>::Balance = (50 * UNIT).into();
		NftStakingPallet::<T>::self_stake_currency(origin.into(), amount).expect("Should succeed");
		fixture.period_passed();

		let origin = validator.signed_origin();
		#[extrinsic_call]
		_(origin, amount);
	}

	#[benchmark]
	fn self_unstake_nft() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let nft = DelegatorNft::<T>::default();
		let item_id = nft.mint();
		let origin = validator.signed_origin();
		NftStakingPallet::<T>::self_stake_nft(origin.into(), item_id.clone())
			.expect("Should succeed");
		fixture.period_passed();

		let origin = validator.signed_origin();
		#[extrinsic_call]
		_(origin, item_id);
	}

	#[benchmark]
	fn delegate_currency() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let delegator = Account::<T>::delegator(0);
		delegator.endow((2000 * UNIT).into());
		let amount = (50 * UNIT).into();
		let target = &validator.account.id;
		let Some(ValidatorDetails::DPoS {
			accept_delegations: true,
			min_staking_period,
			commission,
		}) = Validators::<T>::get(target)
		else {
			panic!("Should succeed");
		};

		let origin = delegator.signed_origin();
		#[extrinsic_call]
		_(origin, amount, target.clone(), min_staking_period, commission);
	}

	#[benchmark]
	fn delegate_nft() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let nft = DelegatorNft::<T> { account: Account::<T>::delegator(0), ..Default::default() };
		let item_id = nft.mint();
		let target = &validator.account.id;
		let Some(ValidatorDetails::DPoS {
			accept_delegations: true,
			min_staking_period,
			commission,
		}) = Validators::<T>::get(target)
		else {
			panic!("Validator is DPoS");
		};

		let origin = nft.account.signed_origin();
		#[extrinsic_call]
		_(origin, item_id, target.clone(), min_staking_period, commission);
	}

	#[benchmark]
	fn undelegate_currency() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let delegator = Account::<T>::delegator(0);
		delegator.endow((2000 * UNIT).into());
		let amount = (50 * UNIT).into();
		let target = &validator.account.id;
		let Some(ValidatorDetails::DPoS {
			accept_delegations: true,
			min_staking_period,
			commission,
		}) = Validators::<T>::get(target)
		else {
			panic!("Should succeed");
		};
		let origin = delegator.signed_origin();
		NftStakingPallet::<T>::delegate_currency(
			origin.into(),
			amount,
			target.clone(),
			min_staking_period,
			commission,
		)
		.expect("Should succeed");
		fixture.period_passed();

		let origin = delegator.signed_origin();
		#[extrinsic_call]
		_(origin, amount, target.clone());
	}

	#[benchmark]
	fn undelegate_nft() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let nft = DelegatorNft::<T> { account: Account::<T>::delegator(0), ..Default::default() };
		let item_id = nft.mint();
		let target = &validator.account.id;
		let Some(ValidatorDetails::DPoS {
			accept_delegations: true,
			min_staking_period,
			commission,
		}) = Validators::<T>::get(target)
		else {
			panic!("Should succeed");
		};
		let origin = nft.account.signed_origin();
		NftStakingPallet::<T>::delegate_nft(
			origin.into(),
			item_id.clone(),
			target.clone(),
			min_staking_period,
			commission,
		)
		.expect("Should succeed");
		fixture.period_passed();

		let origin = nft.account.signed_origin();
		#[extrinsic_call]
		_(origin, item_id, target.clone());
	}

	#[benchmark]
	fn set_minimum_staking_period() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let new_min_period = <T as NftStakingConfig>::MinimumStakingPeriod::get().into();

		let origin = validator.signed_origin();
		#[extrinsic_call]
		_(origin, new_min_period);
	}

	#[benchmark]
	fn set_commission() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let new_commission = <T as NftStakingConfig>::MinimumCommissionRate::get();

		let origin = validator.signed_origin();
		#[extrinsic_call]
		_(origin, new_commission);
	}

	#[benchmark]
	fn kick() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		validator.mint_and_bind();
		let target = &validator.account.id;
		let Some(ValidatorDetails::DPoS {
			accept_delegations: true,
			min_staking_period,
			commission,
		}) = Validators::<T>::get(target)
		else {
			panic!("Could not retrieve validator settings");
		};
		let delegator = Account::<T>::delegator(0);
		let origin = delegator.signed_origin();
		for _ in 1..=crate::types::MAX_NFTS_PER_CONTRACT {
			let nft = DelegatorNft::<T> { account: delegator.clone(), ..Default::default() };
			let item_id = nft.mint();
			NftStakingPallet::<T>::delegate_nft(
				origin.clone().into(),
				item_id.clone(),
				target.clone(),
				min_staking_period,
				commission,
			)
			.expect("Could not delegate NFT");
		}
		delegator.endow((200 * UNIT).into());
		let amount = (50 * UNIT).into();
		NftStakingPallet::<T>::delegate_currency(
			origin.into(),
			amount,
			target.clone(),
			min_staking_period,
			commission,
		)
		.expect("Could not delegate currency");
		fixture.period_passed();

		let origin = validator.account.signed_origin();
		#[extrinsic_call]
		_(origin, delegator.id.clone());
	}

	#[benchmark]
	fn topup() {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		let item_id = validator.mint_and_bind();
		let new_value = validator.nominal_value - (2 * UNIT).into();
		T::NftStakingHandler::set_nominal_value_of_bound(&validator.account.id, new_value)
			.expect("could set nominal value");
		validator.account.endow((200 * UNIT).into());
		let amount = (3 * UNIT).into();

		let origin = validator.account.signed_origin();
		#[extrinsic_call]
		_(origin, item_id, amount);
	}

	#[benchmark]
	fn total_committed_stake(v: Linear<1, { MAX_ACTIVE_VALIDATORS }>) {
		let validators = (0..v).map(|i| Account::<T>::validator(i)).collect::<SpVec<_>>();
		for v in validators {
			TotalValidatorStakes::<T>::insert(
				(StorageLayer::Committed, &v.id),
				TotalValidatorStake { total_stake: 5000u32.into(), contract_count: 1 },
			)
		}

		#[block]
		{
			RewardSweep::<T>::total_committed_stake();
		}
	}

	#[benchmark]
	fn maybe_reset_validator_state() {
		let v = Account::<T>::validator(0);
		endow_mint_and_bind(&v);
		commit::<T>();
		ValidatorStates::<T>::mutate_extant(&v.id, |s| {
			*s = ValidatorState::Faulted;
		});

		#[block]
		{
			RewardValidator::<T>::maybe_reset_validator_state(&v.id);
		}

		assert_eq!(ValidatorStates::<T>::get(&v.id), Some(ValidatorState::Normal));
	}

	#[benchmark]
	fn reward_contract(d: Linear<0, { MAX_NFTS_PER_CONTRACT }>) {
		let v = Account::<T>::validator(0);
		endow_mint_and_bind(&v);
		let delegator = Account::<T>::delegator(42);
		delegator.endow((500 * UNIT).into());
		// FIXME: Why is the 2nd contract missing if this call is not in here?
		NftStakingPallet::<T>::delegate_currency(
			delegator.signed_origin().into(),
			(50 * UNIT).into(),
			v.id.clone(),
			T::MinimumStakingPeriod::get().into(),
			T::MinimumCommissionRate::get(),
		)
		.expect("Currency could be delegated");
		for _i in 0..d {
			let nft = DelegatorNft::<T> { account: delegator.clone(), ..Default::default() };
			let item_id = nft.mint();

			NftStakingPallet::<T>::delegate_nft(
				delegator.signed_origin().into(),
				item_id.clone(),
				v.id.clone(),
				T::MinimumStakingPeriod::get().into(),
				T::MinimumCommissionRate::get(),
			)
			.expect("NFT could be delegated");
		}
		commit::<T>();

		let contract = Contracts::<T>::get((StorageLayer::Committed, &v.id, &delegator.id))
			.expect("Contract exists");
		let v_imbalance = 0u32.into();
		let mut context = ValidatorContext::<T> {
			validator: v.id,
			total_committed_stake: 80_000 * UNIT,
			session_reward: 20 * UNIT,
			total_contribution: 0u32.into(),
		};
		#[block]
		{
			RewardValidator::<T>::reward_contract(
				delegator.id,
				&contract,
				v_imbalance,
				&mut context,
			);
		}
	}

	#[benchmark]
	fn deposit_validator_reward() {
		let v = Account::<T>::validator(0);
		endow_mint_and_bind(&v);
		commit::<T>();

		let commission = (2 * UNIT).into();
		#[block]
		{
			RewardValidator::<T>::deposit_validator_reward(&v.id, commission);
		}
	}

	#[benchmark]
	fn deposit_contribution() {
		#[block]
		{
			RewardSweep::<T>::deposit_contribution(500u32.into());
		}
	}

	#[benchmark]
	fn chill_if_faulted() {
		let v = Account::<T>::validator(0);
		endow_mint_and_bind(&v);
		commit::<T>();
		ValidatorStates::<T>::mutate_extant(&v.id, |s| {
			*s = ValidatorState::Faulted;
		});

		#[block]
		{
			SlashOffender::<T>::chill_if_faulted(&v.id);
		}

		assert_eq!(ValidatorStates::<T>::get(&v.id), Some(ValidatorState::Chilled(0u32)));
	}

	#[benchmark]
	fn chill_if_disqualified() {
		let v = Account::<T>::validator(0);
		endow_mint_and_bind(&v);
		commit::<T>();
		T::NftStakingHandler::set_nominal_value_of_bound(&v.id, 0u32.into())
			.expect("Nominal value can be set");

		#[block]
		{
			SlashOffender::<T>::chill_if_disqualified(&v.id);
		}
	}

	#[benchmark]
	fn slash_contract(d: Linear<0, { MAX_NFTS_PER_CONTRACT }>) {
		let v = Account::<T>::validator(0);
		endow_mint_and_bind(&v);
		let delegator = Account::<T>::delegator(42);
		delegator.endow((500 * UNIT).into());
		// FIXME: Why is the 2nd contract missing if this call is not in here?
		NftStakingPallet::<T>::delegate_currency(
			delegator.signed_origin().into(),
			(50 * UNIT).into(),
			v.id.clone(),
			T::MinimumStakingPeriod::get().into(),
			T::MinimumCommissionRate::get(),
		)
		.expect("Currency could be delegated");
		for _i in 0..d {
			let nft = DelegatorNft::<T> { account: delegator.clone(), ..Default::default() };
			let item_id = nft.mint();

			NftStakingPallet::<T>::delegate_nft(
				delegator.signed_origin().into(),
				item_id.clone(),
				v.id.clone(),
				T::MinimumStakingPeriod::get().into(),
				T::MinimumCommissionRate::get(),
			)
			.expect("NFT could be delegated");
		}
		commit::<T>();

		let contract = Contracts::<T>::get((StorageLayer::Committed, &v.id, &delegator.id))
			.expect("Contract should exist");
		let mut context =
			OffenderContext::<T> { offender: v.id, slash: Perbill::from_rational(1u32, 1000u32) };
		#[block]
		{
			SlashOffender::<T>::slash_contract(delegator.id, &contract, &mut context);
		}
	}

	#[benchmark]
	fn rotate_staging_layer() {
		#[block]
		{
			Pallet::<T>::rotate_staging_layer();
		}
	}

	#[benchmark]
	fn commit_full_contract() {
		let stake = Stake {
			currency: 5000u32.into(),
			delegated_nfts: BoundedVec::truncate_from(
				(0..MAX_NFTS_PER_CONTRACT)
					.map(|i| (i.into(), 5000u32.into()))
					.collect::<SpVec<_>>(),
			),
			permission_nft: Some(50_000u32.into()),
		};
		assert!(!stake.is_empty());
		let contract = Contract {
			stake,
			commission: Perbill::from_rational(1u32, 100u32),
			min_staking_period_end: 1000u32,
		};
		let validator = Account::<T>::validator(0);
		let delegator = Account::<T>::delegator(42);

		#[block]
		{
			CommitSweep::<T>::commit_contract(validator.id, delegator.id, contract);
		}
	}

	#[benchmark]
	fn commit_empty_contract() {
		let stake = Stake {
			currency: 0u32.into(),
			delegated_nfts: BoundedVec::new(),
			permission_nft: None,
		};
		assert!(stake.is_empty());
		let contract = Contract {
			stake,
			commission: Perbill::from_rational(1u32, 100u32),
			min_staking_period_end: 1000u32,
		};
		let validator = Account::<T>::validator(0);
		let delegator = Account::<T>::delegator(42);

		#[block]
		{
			CommitSweep::<T>::commit_contract(validator.id, delegator.id, contract);
		}
	}

	#[benchmark]
	fn commit_total_validator_stakes() {
		let total_validator_stake =
			TotalValidatorStake { total_stake: 5000u32.into(), contract_count: 1 };
		let validator = Account::<T>::validator(0);

		#[block]
		{
			CommitSweep::<T>::commit_total_validator_stakes(&validator.id, total_validator_stake);
		}
	}

	#[benchmark]
	fn unlock_currency() {
		let account = Account::<T>::delegator(0);
		let amount: <T as NftStakingConfig>::Balance = (UNIT * 300).into();
		account.endow((UNIT * 500).into());
		Pallet::<T>::lock_currency(&account.id, amount, Precision::Exact)
			.expect("Currency can be locked");

		#[block]
		{
			Pallet::<T>::unlock_currency(&account.id, amount);
		}
	}

	#[benchmark]
	fn unlock_delegator_nft() {
		let nft = DelegatorNft::<T> { account: Account::<T>::delegator(0), ..Default::default() };
		let item_id = nft.mint();
		T::NftDelegationHandler::bind(&nft.account.id, &item_id, Account::<T>::default().id)
			.expect("Delegator NFT can be bound");

		#[block]
		{
			T::NftDelegationHandler::unbind(&nft.account.id, &item_id)
				.expect("Bound NFT could be unbound");
		}
	}

	#[benchmark(extra)]
	fn nft_expire(c: Linear<1, { MAX_NFTS_PER_CONTRACT }>) {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		let _id = validator.mint_and_bind();

		let delegator = Account::<T>::delegator(42);

		let mut expiring_nft = (Incrementable::initial_value().unwrap(), Default::default());

		for i in 0..c {
			let nft = DelegatorNft::<T> { account: delegator.clone(), ..Default::default() };
			let item_id = nft.mint();

			NftStakingPallet::<T>::delegate_nft(
				delegator.signed_origin().into(),
				item_id.clone(),
				validator.account.id.clone(),
				T::MinimumStakingPeriod::get().into(),
				T::MinimumCommissionRate::get(),
			)
			.expect("Should succeed");

			if (c / 2 + 1) == i {
				expiring_nft = (item_id, nft.nominal_value);
			}
		}

		#[block]
		{
			Pallet::<T>::on_expire(
				&delegator.id,
				Some(validator.account.id.clone()),
				&expiring_nft.0,
				&expiring_nft.1,
			);
		}
	}

	#[benchmark(extra)]
	fn on_offence(c: Linear<1, { MAX_ACTIVE_VALIDATORS / 4 }>) {
		let offenders = (0..c)
			.map(|i| {
				let offender =
					T::BenchmarkHelper::id_tuple_from_account(Account::<T>::validator(i).id);
				OffenceDetails { offender, reporters: vec![] }
			})
			.collect::<SpVec<_>>();
		let slash_fraction =
			core::iter::repeat_n(Perbill::from_percent(1), c as usize).collect::<SpVec<_>>();

		#[block]
		{
			Pallet::<T>::on_offence(offenders.as_slice(), &slash_fraction, 0);
		}
	}

	#[benchmark]
	fn session_ended(
		o: Linear<0, { MAX_ACTIVE_VALIDATORS / 4 }>,
		v: Linear<1, { MAX_ACTIVE_VALIDATORS }>,
	) {
		session_end_fixture::<T>(o, v);

		#[block]
		{
			<Pallet<T> as utils::traits::SessionHook>::session_ended(1).expect("Hook must succeed");
		}
	}

	#[benchmark]
	fn on_idle() {
		session_end_fixture::<T>(MAX_ACTIVE_VALIDATORS / 4, MAX_ACTIVE_VALIDATORS);
		<Pallet<T> as utils::traits::SessionHook>::session_ended(1).expect("Hook must succeed");

		#[block]
		{
			// We only want to measure the overhead, since everything else is already
			// measured in functions called in `session_ending::State::make_progress`
			Pallet::<T>::on_idle(42u32.into(), Weight::zero());
		}
	}
}
