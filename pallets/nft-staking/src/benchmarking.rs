//! Benchmarking setup for the permission NFT pallet

use super::*;
use crate::{
	types::MAX_NFTS_PER_CONTRACT, Config as NftStakingConfig, Pallet as NftStakingPallet,
	PermissionType, SessionIndex,
};
use utils::traits::{NftStaking, OnDelegationNftExpire, SessionHook};

use core::fmt;
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

const UNIT: u128 = 1_000_000_000_000_000_000; // 1 MOS = 10^18 tile

#[derive(Debug, Clone)]
struct Account<T: NftStakingConfig> {
	id: T::AccountId,
}

impl<T: NftStakingConfig> Default for Account<T> {
	fn default() -> Self {
		let id: T::AccountId = whitelisted_caller();
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
		let nominal_value: T::Balance = (10 * UNIT).into();

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

// Setup a complex scenario for `*_complex` benchmarks.
//
// Scenario:
//
// We have 400 active validators.
//   - 100 of them are very popular, so they have a 1000 contracts.
//   - another 100 has 500 contracts
//   - another 100 has 250 contracts
//   - and the 100 least reliable validators have only 125 contracts.
// Thus the pallet handles a total of 187,500 contracts.
//
// A contract contains some currency and multiple NFTs.
//
// 10 of each class of validators are getting slashed in the current session.
// 200 delegators have decided to undelegate some currency and NFTs in this session.
fn complex_setup<T>()
where
	T: SessionConfig + BalancesConfig,
	T: NftStakingConfig<Balance = <T as BalancesConfig>::Balance>,
	<T as BalancesConfig>::Balance: From<u128>,
	T::AccountId: From<<T as SessionConfig>::ValidatorId>
		+ codec::EncodeLike<<T as SessionConfig>::ValidatorId>,
{
	let validators = (0..400).map(|i| Account::<T>::validator(i)).collect::<SpVec<_>>();
	let delegators = (0..999).map(|i| Account::<T>::delegator(i)).collect::<SpVec<_>>();

	pallet_session::Validators::<T>::put(
		validators.iter().map(|acc| acc.id.clone()).collect::<SpVec<_>>(),
	);

	for d in &delegators {
		d.endow((5000 * UNIT).into());
	}

	for (i, v) in validators.iter().enumerate() {
		let nft = PermissionNft::<T> { account: v.clone(), ..Default::default() };
		v.endow((10 * UNIT).into());
		nft.mint_and_bind();

		// The validator already has a self-contract...
		let delegator_count = [125, 250, 500, 1000][i / 100] - 1;

		for (j, d) in delegators.iter().take(delegator_count).enumerate() {
			let nft_id = DelegatorNft::<T> {
				account: d.clone(),
				// Too many NFTs can't expire in the same session
				// so we spread it out.
				expiration: (1000 * (i + 1) + j) as u32,
				..Default::default()
			}
			.mint();
			Pallet::<T>::delegate_currency(
				d.signed_origin().into(),
				(2 * UNIT).into(),
				v.id.clone(),
				T::MinimumStakingPeriod::get().into(),
				T::MinimumCommissionRate::get(),
			)
			.unwrap();
			Pallet::<T>::delegate_nft(
				d.signed_origin().into(),
				nft_id,
				v.id.clone(),
				T::MinimumStakingPeriod::get().into(),
				T::MinimumCommissionRate::get(),
			)
			.unwrap();
		}

		// 10 of each group is slashed
		if i % 100 < 10 {
			InverseSlashes::<T>::insert(&v.id, Perbill::from_percent(98));
		}
	}

	commit::<T>();

	for (i, d) in delegators.iter().take(200).enumerate() {
		Pallet::<T>::lock_currency(&d.id, (2 * UNIT).into(), Precision::Exact).unwrap();
		UnlockingCurrency::<T>::insert(&d.id, <T as Config>::Balance::from(2 * UNIT));

		let nft_id =
			DelegatorNft::<T> { account: d.clone(), expiration: i as u32, ..Default::default() }
				.mint();

		T::NftDelegationHandler::bind(&d.id, &nft_id, Account::<T>::default().id).unwrap();
		UnlockingDelegatorNfts::<T>::insert(nft_id, &d.id);
	}
}

#[expect(
	clippy::multiple_bound_locations,
	reason = "The benchmarks attribute macro injects T: NftStakingConfig which conflicts with any T: .."
)]
#[benchmarks(
	where
		T: BalancesConfig<Balance = <T as NftStakingConfig>::Balance> + SessionConfig + NftDelegationConfig<ItemId = <T as NftStakingConfig>::ItemId>,
		<T as NftStakingConfig>::ItemId: Incrementable,
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
				nominal_value: (10 * UNIT).into(),
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
			let amount = (10 * UNIT).into();
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
		let amount: <T as NftStakingConfig>::Balance = (10 * UNIT).into();
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
		let amount: <T as NftStakingConfig>::Balance = (10 * UNIT).into();
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
		let amount = (10 * UNIT).into();
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
			panic!("Should succeed");
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
		let amount = (10 * UNIT).into();
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
		let amount = (10 * UNIT).into();
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

	// session_ended
	// on_idle
	//
	// total_committed_stake
	// maybe_reset_validator_state
	// reward_contract(delegated_nft_count)
	// deposit_validator_reward
	// deposit_contribution
	//
	// chill_if_faulted
	// slash_contract(is_disqualified, delegated_nft_count)
	//
	// rotate_staging_layer
	// commit_contract
	// commit_total_validator_stakes
	// unlock_currency
	// unlock_delegator_nft

	#[benchmark]
	fn rotate_staging_layer() {
		#[block]
		{
			Pallet::<T>::rotate_staging_layer()
		}
	}

	#[benchmark(extra)]
	fn nft_expire(c: Linear<1, { MAX_NFTS_PER_CONTRACT }>) {
		let fixture = Fixture::<T>::default();
		let validator = &fixture.validator;
		let _ = validator.mint_and_bind();

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
	fn on_offence(c: Linear<1, 2000>) {
		let offenders = (0..c)
			.map(|i| {
				let offender =
					T::BenchmarkHelper::id_tuple_from_account(Account::<T>::validator(i).id);
				OffenceDetails { offender, reporters: vec![] }
			})
			.collect::<SpVec<_>>();
		let slash_fraction = core::iter::repeat(Perbill::from_percent(1))
			.take(c as usize)
			.collect::<SpVec<_>>();

		#[block]
		{
			Pallet::<T>::on_offence(offenders.as_slice(), &slash_fraction, 0);
		}
	}

	#[benchmark(extra)]
	fn session_ended_complex() {
		complex_setup::<T>();

		#[block]
		{
			<Pallet<T> as utils::traits::SessionHook>::session_ended(42).unwrap();
		}
	}

	// #[benchmark(extra)]
	// fn reward_complex() {
	// 	complex_setup::<T>();

	// 	#[block]
	// 	{
	// 		let active_validators = SessionPallet::<T>::validators();
	// 		let session_reward = T::SessionReward::get();

	// 		let rewarded = active_validators.into_iter().filter_map(|v| {
	// 			let v = T::AccountId::from(v);
	// 			(!InverseSlashes::<T>::contains_key(&v)).then_some(v)
	// 		});

	// 		Pallet::<T>::do_reward_participants(rewarded, session_reward);
	// 	}
	// }

	// #[benchmark(extra)]
	// fn slashing_complex() {
	// 	complex_setup::<T>();

	// 	#[block]
	// 	{
	// 		Pallet::<T>::do_slash_participants();
	// 	}
	// }

	// #[benchmark(extra)]
	// fn commit_storage_complex() {
	// 	complex_setup::<T>();

	// 	#[block]
	// 	{
	// 		Pallet::<T>::commit_storage();
	// 	}
	// }

	// Scenario:
	//
	// At least `n` transactions modified `n` contracts, validator stakes and unlocking assets,
	// and now we are committing the changes.
	// #[benchmark(extra)]
	// fn commit_storage_n_staged(n: Linear<1, 1000>) {
	// 	for i in 1..=n {
	// 		let validator = Account::<T>::validator(i);
	// 		let delegator = Account::<T>::delegator(i);

	// 		Pallet::<T>::stage_contract(
	// 			&validator.id,
	// 			&delegator.id,
	// 			Contract {
	// 				stake: Stake { currency: 100u32.into(), ..Default::default() },
	// 				..Default::default()
	// 			},
	// 		);

	// 		Pallet::<T>::stage_total_validator_stake(
	// 			&validator.id,
	// 			TotalValidatorStake { total_stake: 100u32.into(), ..Default::default() },
	// 		);

	// 		delegator.endow((3 * UNIT).into());
	// 		Pallet::<T>::lock_currency(&delegator.id, (2 * UNIT).into(), Precision::Exact).unwrap();
	// 		UnlockingCurrency::<T>::insert(&delegator.id, <T as Config>::Balance::from(2 * UNIT));

	// 		let nft_id = DelegatorNft::<T> {
	// 			account: delegator.clone(),
	// 			expiration: i,
	// 			..Default::default()
	// 		}
	// 		.mint();

	// 		T::NftDelegationHandler::bind(&delegator.id, &nft_id, Account::<T>::default().id)
	// 			.unwrap();
	// 		UnlockingDelegatorNfts::<T>::insert(nft_id, &delegator.id);
	// 	}

	// 	#[block]
	// 	{
	// 		Pallet::<T>::commit_storage();
	// 	}
	// }
}
