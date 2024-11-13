//! Benchmarking setup for the permission NFT pallet

use super::*;
use crate::{Config as NftStakingConfig, Pallet as NftStakingPallet, PermissionType, SessionIndex};
use utils::traits::NftStaking;

use core::fmt;
use pallet_nft_delegation::Config as NftDelegationConfig;
use sdk::{
	frame_benchmarking::v2::*,
	frame_support::traits::{tokens::fungible::Mutate, Incrementable, ValidatorSet},
	frame_system::{pallet_prelude::*, RawOrigin},
	pallet_balances::{Config as BalancesConfig, Pallet as Balances},
	pallet_session::Config as SessionConfig,
	sp_std::{vec, vec::Vec as SpVec},
};

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
	fn index(i: u32) -> Self {
		let id: T::AccountId = account("delegator", i, 0);
		Self { id }
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
		<Balances<T> as Mutate<_>>::mint_into(&self.id, amount).expect("Should succeed");
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
			let delegator = Account::<T>::index(i);
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
		let delegator = Account::<T>::index(0);
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
		let nft = DelegatorNft::<T> { account: Account::<T>::index(0), ..Default::default() };
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
		let delegator = Account::<T>::index(0);
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
		let nft = DelegatorNft::<T> { account: Account::<T>::index(0), ..Default::default() };
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
		let delegator = Account::<T>::index(0);
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
}
