use frame_support::traits::{
	fungible::{Inspect, Mutate},
	tokens::{Fortitude, Precision, Preservation},
};
use sp_runtime::traits::{CheckedAdd, CheckedSub};
use staging_xcm_executor::{
	traits::{ConvertLocation, Error as MatchError, MatchesFungible, TransactAsset},
	AssetsInHolding,
};
use xcm::latest::{Asset, Error as XcmError, Location, Result as XcmResult, XcmContext};

use super::*;

pub struct AssetTransactor<T, Matcher, AccountIdConverter>(
	core::marker::PhantomData<(T, Matcher, AccountIdConverter)>,
);

impl<T: Config, Matcher, AccountIdConverter> AssetTransactor<T, Matcher, AccountIdConverter> {
	fn can_accrue_checked(amount: <T::Fungible as Inspect<T::AccountId>>::Balance) -> XcmResult {
		CheckedOutBalance::<T>::get()
			.checked_add(&amount)
			.map(|_| ())
			.ok_or(XcmError::NotDepositable)
	}

	fn can_reduce_checked(amount: <T::Fungible as Inspect<T::AccountId>>::Balance) -> XcmResult {
		CheckedOutBalance::<T>::get()
			.checked_sub(&amount)
			.map(|_| ())
			.ok_or(XcmError::NotWithdrawable)
	}

	fn accrue_checked(amount: <T::Fungible as Inspect<T::AccountId>>::Balance) {
		let res = CheckedOutBalance::<T>::try_mutate(|v| match v.checked_add(&amount) {
			Some(new) => {
				*v = new;
				Ok(())
			},
			None => Err(()),
		});
		debug_assert!(
			res.is_ok(),
			"`can_accrue_checked` must have returned `Ok` immediately prior; qed"
		);
	}

	fn reduce_checked(amount: <T::Fungible as Inspect<T::AccountId>>::Balance) {
		let res = CheckedOutBalance::<T>::try_mutate(|v| match v.checked_sub(&amount) {
			Some(new) => {
				*v = new;
				Ok(())
			},
			None => Err(()),
		});
		debug_assert!(
			res.is_ok(),
			"`can_reduce_checked` must have returned `Ok` immediately prior; qed"
		);
	}
}

impl<T, Matcher, AccountIdConverter> TransactAsset
	for AssetTransactor<T, Matcher, AccountIdConverter>
where
	T: Config,
	T::Fungible: Mutate<T::AccountId> + Inspect<T::AccountId>,
	Matcher: MatchesFungible<<T::Fungible as Inspect<T::AccountId>>::Balance>,
	AccountIdConverter: ConvertLocation<T::AccountId>,
{
	fn can_check_in(_origin: &Location, what: &Asset, _context: &XcmContext) -> XcmResult {
		// Check we handle this asset
		let amount = Matcher::matches_fungible(what).ok_or(MatchError::AssetNotHandled)?;
		Self::can_reduce_checked(amount)
	}

	fn check_in(origin: &Location, what: &Asset, _context: &XcmContext) {
		if let Some(amount) = Matcher::matches_fungible(what) {
			Self::reduce_checked(amount);
			Pallet::<T>::deposit_event(Event::<T>::CheckedIn { from: origin.clone(), amount });
		}
	}

	fn can_check_out(_dest: &Location, what: &Asset, _context: &XcmContext) -> XcmResult {
		let amount = Matcher::matches_fungible(what).ok_or(MatchError::AssetNotHandled)?;
		Self::can_accrue_checked(amount)
	}

	fn check_out(dest: &Location, what: &Asset, _context: &XcmContext) {
		if let Some(amount) = Matcher::matches_fungible(what) {
			Self::accrue_checked(amount);
			Pallet::<T>::deposit_event(Event::<T>::CheckedOut { to: dest.clone(), amount });
		}
	}

	fn deposit_asset(what: &Asset, who: &Location, _context: Option<&XcmContext>) -> XcmResult {
		let amount = Matcher::matches_fungible(what).ok_or(MatchError::AssetNotHandled)?;
		let who = AccountIdConverter::convert_location(who)
			.ok_or(MatchError::AccountIdConversionFailed)?;
		Pallet::<T>::mint_into(&who, amount)
			.map_err(|error| XcmError::FailedToTransactAsset(error.into()))?;
		Ok(())
	}

	fn withdraw_asset(
		what: &Asset,
		who: &Location,
		_context: Option<&XcmContext>,
	) -> Result<AssetsInHolding, XcmError> {
		let amount = Matcher::matches_fungible(what).ok_or(MatchError::AssetNotHandled)?;
		let who = AccountIdConverter::convert_location(who)
			.ok_or(MatchError::AccountIdConversionFailed)?;
		Pallet::<T>::burn_from(
			&who,
			amount,
			Preservation::Expendable,
			Precision::Exact,
			Fortitude::Polite,
		)
		.map_err(|error| XcmError::FailedToTransactAsset(error.into()))?;
		Ok(what.clone().into())
	}
}
