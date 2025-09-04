#![cfg_attr(not(feature = "std"), no_std)]

use sdk::frame_support::{
	pallet_prelude::{DispatchError, DispatchResult, Encode, TypeInfo},
	traits::{
		fungible::{
			hold::DoneSlash, Balanced, BalancedHold, Credit, Debt, Dust, Inspect, InspectFreeze,
			InspectHold, Mutate, MutateFreeze, MutateHold, Unbalanced, UnbalancedHold,
		},
		tokens::{
			DepositConsequence, Fortitude, Precision, Preservation, Provenance, Restriction,
			WithdrawConsequence,
		},
	},
};

pub use pallet::*;

#[sdk::frame_support::pallet]
mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		type RuntimeHoldReason: Clone + core::fmt::Debug + PartialEq + Encode + TypeInfo + 'static;
		type Fungible: Inspect<Self::AccountId>
			+ InspectHold<Self::AccountId, Reason = Self::RuntimeHoldReason>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		HoldSlashed {
			who: T::AccountId,
			amount: <T::Fungible as Inspect<T::AccountId>>::Balance,
			reason: <T::Fungible as InspectHold<T::AccountId>>::Reason,
		},
		HoldIncreased {
			reason: T::RuntimeHoldReason,
			who: T::AccountId,
			amount: <T::Fungible as Inspect<T::AccountId>>::Balance,
		},
		HoldBurned {
			reason: T::RuntimeHoldReason,
			who: T::AccountId,
			amount: <T::Fungible as Inspect<T::AccountId>>::Balance,
		},
		HoldReleased {
			reason: T::RuntimeHoldReason,
			who: T::AccountId,
			amount: <T::Fungible as Inspect<T::AccountId>>::Balance,
		},
		TransferOnHold {
			reason: T::RuntimeHoldReason,
			source: T::AccountId,
			dest: T::AccountId,
			amount: <T::Fungible as Inspect<T::AccountId>>::Balance,
		},
		TransferAndHold {
			reason: T::RuntimeHoldReason,
			source: T::AccountId,
			dest: T::AccountId,
			transferred: <T::Fungible as Inspect<T::AccountId>>::Balance,
		},
	}
}

impl<T: Config> Inspect<T::AccountId> for Pallet<T>
where
	T::Fungible: Inspect<T::AccountId>,
{
	type Balance = <T::Fungible as Inspect<T::AccountId>>::Balance;

	delegate::delegate! {
		to T::Fungible {
			fn total_issuance() -> Self::Balance;
			fn minimum_balance() -> Self::Balance;
			fn total_balance(who: &T::AccountId) -> Self::Balance;
			fn balance(who: &T::AccountId) -> Self::Balance;
			fn reducible_balance(
				who: &T::AccountId,
				preservation: Preservation,
				force: Fortitude,
			) -> Self::Balance;
			fn can_deposit(
				who: &T::AccountId,
				amount: Self::Balance,
				provenance: Provenance,
			) -> DepositConsequence;
			fn can_withdraw(
				who: &T::AccountId,
				amount: Self::Balance,
			) -> WithdrawConsequence<Self::Balance>;
		}
	}
}

impl<T: Config> Unbalanced<T::AccountId> for Pallet<T>
where
	T::Fungible: Unbalanced<T::AccountId>,
{
	delegate::delegate! {
		to T::Fungible {
			fn handle_raw_dust(amount: Self::Balance);
			fn write_balance(
				who: &T::AccountId,
				amount: Self::Balance,
			) -> Result<Option<Self::Balance>, DispatchError>;
			fn set_total_issuance(amount: Self::Balance);
			fn decrease_balance(
				who: &T::AccountId,
				amount: Self::Balance,
				precision: Precision,
				preservation: Preservation,
				force: Fortitude,
			) -> Result<Self::Balance, DispatchError>;
			fn increase_balance(
				who: &T::AccountId,
				amount: Self::Balance,
				precision: Precision,
			) -> Result<Self::Balance, DispatchError>;
			fn deactivate(amount: Self::Balance);
			fn reactivate(amount: Self::Balance);
		}
	}

	/// If `T::Fungible` implements `Balanced` this correctly
	/// delegates to `OnDropCredit`.
	fn handle_dust(dust: Dust<T::AccountId, Self>) {
		T::Fungible::handle_dust(Dust(dust.0));
	}
}

impl<T: Config> Balanced<T::AccountId> for Pallet<T>
where
	T::Fungible: Balanced<T::AccountId>,
{
	type OnDropDebt = <T::Fungible as Balanced<T::AccountId>>::OnDropDebt;
	type OnDropCredit = <T::Fungible as Balanced<T::AccountId>>::OnDropCredit;

	delegate::delegate! {
		to T::Fungible {
			fn rescind(amount: Self::Balance) -> Debt<T::AccountId, Self>;
			fn issue(amount: Self::Balance) -> Credit<T::AccountId, Self>;
			fn pair(
				amount: Self::Balance,
			) -> Result<(Debt<T::AccountId, Self>, Credit<T::AccountId, Self>), DispatchError>;
			fn deposit(
				who: &T::AccountId,
				value: Self::Balance,
				precision: Precision,
			) -> Result<Debt<T::AccountId, Self>, DispatchError>;
			fn withdraw(
				who: &T::AccountId,
				value: Self::Balance,
				precision: Precision,
				preservation: Preservation,
				force: Fortitude,
			) -> Result<Credit<T::AccountId, Self>, DispatchError>;
			fn resolve(
				who: &T::AccountId,
				credit: Credit<T::AccountId, Self>,
			) -> Result<(), Credit<T::AccountId, Self>>;
			fn settle(
				who: &T::AccountId,
				debt: Debt<T::AccountId, Self>,
				preservation: Preservation,
			) -> Result<Credit<T::AccountId, Self>, Debt<T::AccountId, Self>>;
			fn done_rescind(amount: Self::Balance);
			fn done_issue(amount: Self::Balance);
			fn done_deposit(who: &T::AccountId, amount: Self::Balance);
			fn done_withdraw(who: &T::AccountId, amount: Self::Balance);
		}
	}
}

impl<T: Config> Mutate<T::AccountId> for Pallet<T>
where
	T::Fungible: Mutate<T::AccountId>,
{
	delegate::delegate! {
			to T::Fungible {
				fn mint_into(who: &T::AccountId, amount: Self::Balance) -> Result<Self::Balance, DispatchError>;
				fn burn_from(
					who: &T::AccountId,
					amount: Self::Balance,
					preservation: Preservation,
					precision: Precision,
					force: Fortitude,
				) -> Result<Self::Balance, DispatchError>;
				fn shelve(who: &T::AccountId, amount: Self::Balance) -> Result<Self::Balance, DispatchError>;
				fn restore(who: &T::AccountId, amount: Self::Balance) -> Result<Self::Balance, DispatchError>;
				fn transfer(
					source: &T::AccountId,
					dest: &T::AccountId,
					amount: Self::Balance,
					preservation: Preservation,
				) -> Result<Self::Balance, DispatchError>;
				fn set_balance(who: &T::AccountId, amount: Self::Balance) -> Self::Balance;
				fn done_mint_into(who: &T::AccountId, amount: Self::Balance);
				fn done_burn_from(who: &T::AccountId, amount: Self::Balance);
				fn done_shelve(who: &T::AccountId, amount: Self::Balance);
				fn done_restore(who: &T::AccountId, amount: Self::Balance);
				fn done_transfer(source: &T::AccountId, dest: &T::AccountId, amount: Self::Balance);
		}
	}
}

impl<T: Config> InspectHold<T::AccountId> for Pallet<T>
where
	T::Fungible: InspectHold<T::AccountId>,
{
	type Reason = <T::Fungible as InspectHold<T::AccountId>>::Reason;

	delegate::delegate! {
		to T::Fungible {
			fn total_balance_on_hold(who: &T::AccountId) -> Self::Balance;
			fn reducible_total_balance_on_hold(who: &T::AccountId, force: Fortitude) -> Self::Balance;
			fn balance_on_hold(reason: &Self::Reason, who: &T::AccountId) -> Self::Balance;
			fn hold_available(reason: &Self::Reason, who: &T::AccountId) -> bool;
			fn ensure_can_hold(
				reason: &Self::Reason,
				who: &T::AccountId,
				amount: Self::Balance,
			) -> DispatchResult;
			fn can_hold(reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) -> bool;
		}
	}
}

impl<T: Config> UnbalancedHold<T::AccountId> for Pallet<T>
where
	T::Fungible: UnbalancedHold<T::AccountId>,
{
	delegate::delegate! {
		to T::Fungible {
			fn set_balance_on_hold(
				reason: &Self::Reason,
				who: &T::AccountId,
				amount: Self::Balance,
			) -> DispatchResult;
			fn decrease_balance_on_hold(
				reason: &Self::Reason,
				who: &T::AccountId,
				amount: Self::Balance,
				precision: Precision,
			) -> Result<Self::Balance, DispatchError>;
			fn increase_balance_on_hold(
				reason: &Self::Reason,
				who: &T::AccountId,
				amount: Self::Balance,
				precision: Precision,
			) -> Result<Self::Balance, DispatchError>;
		}
	}
}

impl<T: Config> BalancedHold<T::AccountId> for Pallet<T>
where
	T::Fungible: BalancedHold<T::AccountId>,
{
	fn slash(
		reason: &Self::Reason,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> (Credit<T::AccountId, Self>, Self::Balance) {
		let ret = <T::Fungible as BalancedHold<_>>::slash(reason, who, amount);
		Self::done_slash(reason, who, amount);
		ret
	}
}

impl<T: Config, Reason, Balance> DoneSlash<Reason, T::AccountId, Balance> for Pallet<T>
where
	T::Fungible: Inspect<T::AccountId, Balance = Balance>,
	T::Fungible: InspectHold<T::AccountId, Reason = Reason>,
	Reason: Clone,
{
	fn done_slash(
		reason: &<Self as InspectHold<T::AccountId>>::Reason,
		who: &T::AccountId,
		amount: <Self as Inspect<T::AccountId>>::Balance,
	) {
		Self::deposit_event(Event::<T>::HoldSlashed {
			who: who.clone(),
			amount,
			reason: reason.clone(),
		});
	}
}

impl<T: Config> MutateHold<T::AccountId> for Pallet<T>
where
	T::Fungible: MutateHold<T::AccountId>,
{
	fn hold(reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) -> DispatchResult {
		T::Fungible::hold(reason, who, amount)?;
		Self::done_hold(reason, who, amount);
		Ok(())
	}

	fn release(
		reason: &Self::Reason,
		who: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
	) -> Result<Self::Balance, DispatchError> {
		let ret = T::Fungible::release(reason, who, amount, precision)?;
		Self::done_release(reason, who, ret);
		Ok(ret)
	}

	fn set_on_hold(
		reason: &Self::Reason,
		who: &T::AccountId,
		amount: Self::Balance,
	) -> DispatchResult {
		T::Fungible::set_on_hold(reason, who, amount)
	}

	fn release_all(
		reason: &Self::Reason,
		who: &T::AccountId,
		precision: Precision,
	) -> Result<Self::Balance, DispatchError> {
		let ret = T::Fungible::release_all(reason, who, precision)?;
		Self::done_release(reason, who, ret);
		Ok(ret)
	}

	fn burn_held(
		reason: &Self::Reason,
		who: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
		force: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		let ret = T::Fungible::burn_held(reason, who, amount, precision, force)?;
		Self::done_burn_held(reason, who, ret);
		Ok(ret)
	}

	fn burn_all_held(
		reason: &Self::Reason,
		who: &T::AccountId,
		precision: Precision,
		force: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		let ret = T::Fungible::burn_all_held(reason, who, precision, force)?;
		Self::done_burn_held(reason, who, ret);
		Ok(ret)
	}

	fn transfer_on_hold(
		reason: &Self::Reason,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
		mode: Restriction,
		force: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		let ret =
			T::Fungible::transfer_on_hold(reason, source, dest, amount, precision, mode, force)?;
		Self::done_transfer_on_hold(reason, source, dest, ret);
		Ok(ret)
	}

	fn transfer_and_hold(
		reason: &Self::Reason,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
		precision: Precision,
		expendability: Preservation,
		force: Fortitude,
	) -> Result<Self::Balance, DispatchError> {
		let ret = T::Fungible::transfer_and_hold(
			reason,
			source,
			dest,
			amount,
			precision,
			expendability,
			force,
		)?;
		Self::done_transfer_and_hold(reason, source, dest, ret);
		Ok(ret)
	}

	fn done_hold(reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::HoldIncreased {
			reason: reason.clone(),
			who: who.clone(),
			amount,
		});
	}

	fn done_release(reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::HoldReleased {
			reason: reason.clone(),
			who: who.clone(),
			amount,
		});
	}

	fn done_burn_held(reason: &Self::Reason, who: &T::AccountId, amount: Self::Balance) {
		Self::deposit_event(Event::<T>::HoldBurned {
			reason: reason.clone(),
			who: who.clone(),
			amount,
		});
	}

	fn done_transfer_on_hold(
		reason: &Self::Reason,
		source: &T::AccountId,
		dest: &T::AccountId,
		amount: Self::Balance,
	) {
		Self::deposit_event(Event::<T>::TransferOnHold {
			reason: reason.clone(),
			source: source.clone(),
			dest: dest.clone(),
			amount,
		});
	}

	fn done_transfer_and_hold(
		reason: &Self::Reason,
		source: &T::AccountId,
		dest: &T::AccountId,
		transferred: Self::Balance,
	) {
		Self::deposit_event(Event::<T>::TransferAndHold {
			reason: reason.clone(),
			source: source.clone(),
			dest: dest.clone(),
			transferred,
		});
	}
}

impl<T: Config> InspectFreeze<T::AccountId> for Pallet<T>
where
	T::Fungible: InspectFreeze<T::AccountId>,
{
	type Id = <T::Fungible as InspectFreeze<T::AccountId>>::Id;

	delegate::delegate! {
		to T::Fungible {
			fn balance_frozen(id: &Self::Id, who: &T::AccountId) -> Self::Balance;
			fn balance_freezable(who: &T::AccountId) -> Self::Balance;
			fn can_freeze(id: &Self::Id, who: &T::AccountId) -> bool;
		}
	}
}

impl<T: Config> MutateFreeze<T::AccountId> for Pallet<T>
where
	T::Fungible: MutateFreeze<T::AccountId>,
{
	delegate::delegate! {
		to T::Fungible {
			fn set_freeze(id: &Self::Id, who: &T::AccountId, amount: Self::Balance) -> DispatchResult;
			fn extend_freeze(id: &Self::Id, who: &T::AccountId, amount: Self::Balance) -> DispatchResult;
			fn thaw(id: &Self::Id, who: &T::AccountId) -> DispatchResult;
			fn set_frozen(
				id: &Self::Id,
				who: &T::AccountId,
				amount: Self::Balance,
				fortitude: Fortitude,
			) -> DispatchResult;
			fn ensure_frozen(
				id: &Self::Id,
				who: &T::AccountId,
				amount: Self::Balance,
				fortitude: Fortitude,
			) -> DispatchResult;
			fn decrease_frozen(id: &Self::Id, who: &T::AccountId, amount: Self::Balance) -> DispatchResult;
			fn increase_frozen(id: &Self::Id, who: &T::AccountId, amount: Self::Balance) -> DispatchResult;
		}
	}
}
