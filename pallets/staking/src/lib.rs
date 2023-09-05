#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

//#[cfg(test)]
//mod mock;
//
//#[cfg(test)]
//mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_support::dispatch::Codec;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::Currency;
	use frame_system::pallet_prelude::*;
	use pallet_nfts::Config as NftsConfig;
	use sp_runtime::FixedPointOperand;
	use sp_runtime::Perbill;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	pub trait NftStaking<AccountId, Balance, Variant, ItemId> {
		fn bind(
			account_id: &AccountId,
			item_id: &ItemId,
		) -> Result<(Variant, Balance), DispatchError>;

		fn unbind(account_id: &AccountId) -> Result<Balance, DispatchError>;

		fn slash(
			account_id: &AccountId,
			slash_proportion: Perbill,
		) -> Result<Balance, DispatchError>;

		fn chill(account_id: &AccountId) -> DispatchResult;

		fn unchill(account_id: &AccountId) -> DispatchResult;
	}

	// TODO: Who stores what state? How could storage be optimized?
	// Should we consider slashing each delegator token from this pallet?
	pub trait NftDelegation<AccountId, Balance, ItemId> {
		fn bind(
			delegator_id: &AccountId,
			validator_id: &AccountId,
			item_id: &ItemId,
		) -> Result<(sp_staking::SessionIndex, Balance), DispatchError>;

		fn unbind(
			delegator_id: &AccountId,
			validator_id: &AccountId,
			item_id: &ItemId,
		) -> Result<Balance, DispatchError>;

		// TODO: How will the stake be updated after slashing multiple items?
		// Will both pallets slash?
		fn slash(
			delegator_id: &AccountId,
			validator_id: &AccountId,
			slash_proportion: Perbill,
		) -> Result<Balance, DispatchError>;
	}

	impl<AccountId, Balance, Variant, ItemId> NftStaking<AccountId, Balance, Variant, ItemId> for () {
		fn bind(
			_account_id: &AccountId,
			_item_id: &ItemId,
		) -> Result<(Variant, Balance), DispatchError> {
			unimplemented!()
		}

		fn unbind(_account_id: &AccountId) -> Result<Balance, DispatchError> {
			unimplemented!()
		}

		fn slash(
			_account_id: &AccountId,
			_slash_proportion: Perbill,
		) -> Result<Balance, DispatchError> {
			unimplemented!()
		}

		fn chill(_account_id: &AccountId) -> DispatchResult {
			unimplemented!()
		}

		fn unchill(_account_id: &AccountId) -> DispatchResult {
			unimplemented!()
		}
	}

	impl<AccountId, Balance, ItemId> NftDelegation<AccountId, Balance, ItemId> for () {
		fn bind(
			_validator_id: &AccountId,
			_account_id: &AccountId,
			_item_id: &ItemId,
		) -> Result<(sp_staking::SessionIndex, Balance), DispatchError> {
			unimplemented!()
		}

		fn unbind(
			_validator_id: &AccountId,
			_account_id: &AccountId,
			_item_id: &ItemId,
		) -> Result<Balance, DispatchError> {
			unimplemented!()
		}

		fn slash(
			_validator_id: &AccountId,
			_account_id: &AccountId,
			_slash_proportion: Perbill,
		) -> Result<Balance, DispatchError> {
			unimplemented!()
		}
	}

	#[derive(
		Copy,
		Clone,
		PartialEq,
		Eq,
		Encode,
		Decode,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
		serde::Serialize,
		serde::Deserialize,
	)]
	pub enum ValidatorVariant {
		PoS,
		DPoS,
	}

	#[derive(
		Copy,
		Clone,
		PartialEq,
		Eq,
		Encode,
		Decode,
		RuntimeDebug,
		TypeInfo,
		MaxEncodedLen,
		serde::Serialize,
		serde::Deserialize,
	)]
	enum NftVariant<T: Config> {
		Permission(ValidatorVariant),
		Delegation(T::ItemId),
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + NftsConfig {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Used for the nominal value of permission tokens
		type Balance: Parameter
			+ Member
			+ sp_runtime::traits::AtLeast32BitUnsigned
			+ Codec
			+ Default
			+ Copy
			+ MaybeSerializeDeserialize
			+ MaxEncodedLen
			+ TypeInfo
			+ FixedPointOperand;

		type Currency: frame_support::traits::LockableCurrency<
			Self::AccountId,
			Moment = BlockNumberFor<Self>,
			Balance = Self::Balance,
		>;

		type NftStakingHandler: NftStaking<
			Self::AccountId,
			Self::Balance,
			ValidatorVariant,
			Self::ItemId,
		>;

		type NftDelegationHandler: NftDelegation<Self::AccountId, Self::Balance, Self::ItemId>;

		type MinimumStakingDuration: Get<u32>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn total_stake)]
	pub type TotalStake<T: Config> = StorageValue<_, T::Balance>;

	#[pallet::storage]
	#[pallet::getter(fn account_exposure)]
	pub type AccountExposure<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::Balance>;

	#[pallet::storage]
	#[pallet::getter(fn individual_exposure)]
	pub type IndividualExposure<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId, // ValidatorId
		Twox64Concat,
		T::AccountId, // DelegatorId
		T::Balance,
	>;

	#[pallet::storage]
	#[pallet::getter(fn account_variant)]
	pub type AccountVariant<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, ValidatorVariant>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	// #[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored { who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		BadState,
		AlreadyBound,
		TargetNotDPoS,
		InvalidTarget,
		NotBound,
	}

	impl<T: Config> Pallet<T> {
		fn do_stake_validator(validator_id: &T::AccountId, value: &T::Balance) -> DispatchResult {
			let total = TotalStake::<T>::get().ok_or(Error::<T>::BadState)?;
			let account_exposure = AccountExposure::<T>::get(&validator_id).unwrap_or_default();
			let individual_exposure =
				IndividualExposure::<T>::get(&validator_id, &validator_id).unwrap_or_default();

			TotalStake::<T>::put(total + *value);
			AccountExposure::<T>::set(&validator_id, Some(account_exposure + *value));
			IndividualExposure::<T>::set(
				&validator_id,
				&validator_id,
				Some(individual_exposure + *value),
			);
			Ok(())
		}

		fn do_unstake_validator(validator_id: &T::AccountId, value: &T::Balance) -> DispatchResult {
			let total = TotalStake::<T>::get().ok_or(Error::<T>::BadState)?;
			let account_exposure =
				AccountExposure::<T>::get(&validator_id).ok_or(Error::<T>::BadState)?;
			let individual_exposure = IndividualExposure::<T>::get(&validator_id, &validator_id)
				.ok_or(Error::<T>::BadState)?;

			TotalStake::<T>::put(total - *value);
			AccountExposure::<T>::set(&validator_id, Some(account_exposure - *value));
			IndividualExposure::<T>::set(
				&validator_id,
				&validator_id,
				Some(individual_exposure - *value),
			);
			Ok(())
		}

		fn do_stake_delegator(
			delegator_id: &T::AccountId,
			validator_id: &T::AccountId,
			value: &T::Balance,
		) -> DispatchResult {
			let total = TotalStake::<T>::get().ok_or(Error::<T>::BadState)?;
			let account_exposure =
				AccountExposure::<T>::get(&validator_id).ok_or(Error::<T>::BadState)?;
			let individual_exposure =
				IndividualExposure::<T>::get(&validator_id, &delegator_id).unwrap_or_default();

			TotalStake::<T>::put(total + *value);
			AccountExposure::<T>::set(&validator_id, Some(account_exposure + *value));
			IndividualExposure::<T>::set(
				&validator_id,
				&delegator_id,
				Some(individual_exposure + *value),
			);
			Ok(())
		}

		fn do_unstake_delegator(
			validator_id: &T::AccountId,
			delegator_id: &T::AccountId,
			value: &T::Balance,
		) -> DispatchResult {
			let total = TotalStake::<T>::get().ok_or(Error::<T>::BadState)?;
			let account_exposure =
				AccountExposure::<T>::get(&validator_id).ok_or(Error::<T>::BadState)?;
			let individual_exposure = IndividualExposure::<T>::get(&validator_id, &delegator_id)
				.ok_or(Error::<T>::BadState)?;

			TotalStake::<T>::put(total - *value);
			AccountExposure::<T>::set(&validator_id, Some(account_exposure - *value));
			IndividualExposure::<T>::set(
				&validator_id,
				&delegator_id,
				Some(individual_exposure - *value),
			);
			Ok(())
		}

		fn do_bind_permission(
			who: &T::AccountId,
			item_id: &T::ItemId,
		) -> Result<(ValidatorVariant, T::Balance), DispatchError> {
			T::NftStakingHandler::bind(&who, &item_id)
		}

		fn do_bind_delegation(
			who: &T::AccountId,
			validator_id: &T::AccountId,
			item_id: &T::ItemId,
		) -> Result<(sp_staking::SessionIndex, T::Balance), DispatchError> {
			T::NftDelegationHandler::bind(who, validator_id, item_id)
		}

		fn do_unbind_permission(
			who: &T::AccountId,
			_validator_variant: ValidatorVariant,
		) -> Result<T::Balance, DispatchError> {
			T::NftStakingHandler::unbind(&who)
		}
		fn do_unbind_delegation(
			who: &T::AccountId,
			validator_id: &T::AccountId,
			item_id: <T as NftsConfig>::ItemId,
		) -> Result<T::Balance, DispatchError> {
			T::NftDelegationHandler::unbind(who, validator_id, &item_id)
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		pub fn delegate_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
			target: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let target_variant = AccountVariant::<T>::get(&target);
			ensure!(target_variant.is_some(), Error::<T>::InvalidTarget);
			ensure!(target_variant == Some(ValidatorVariant::DPoS), Error::<T>::TargetNotDPoS);

			let (_expiry_in_session, nominal_value) =
				Self::do_bind_delegation(&who, &target, &item_id)?;
			// TODO: check whether expiry date falls outside of lock duration, if so return error
			Self::do_stake_delegator(&who, &target, &nominal_value)?;
			Ok(())
		}

		#[pallet::call_index(1)]
		pub fn undelegate_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
			target: T::AccountId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let nominal_value = Self::do_unbind_delegation(&who, &target, item_id)?;
			Self::do_unstake_delegator(&who, &target, &nominal_value)?;
			Ok(())
		}

		#[pallet::call_index(2)]
		pub fn bind_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(AccountExposure::<T>::get(&who).is_none(), Error::<T>::AlreadyBound);
			ensure!(AccountVariant::<T>::get(&who).is_none(), Error::<T>::AlreadyBound);

			let (variant, nominal_value) = Self::do_bind_permission(&who, &item_id)?;
			Self::do_stake_validator(&who, &nominal_value)?;

			AccountVariant::<T>::insert(&who, &variant);

			Ok(())
		}

		#[pallet::call_index(3)]
		pub fn unbind_nft(origin: OriginFor<T>, _validator_id: T::AccountId) -> DispatchResult {
			// TODO: clean up stake and delegation logic (for dpos)
			let who = ensure_signed(origin)?;

			let variant = AccountVariant::<T>::get(&who).ok_or(Error::<T>::NotBound)?;
			let nominal_value = Self::do_unbind_permission(&who, variant)?;
			Self::do_unstake_validator(&who, &nominal_value)?;
			AccountVariant::<T>::remove(&who);
			Ok(())
		}

		#[pallet::call_index(4)]
		pub fn stake_currency(
			origin: OriginFor<T>,
			#[pallet::compact] value: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			frame_system::Pallet::<T>::inc_consumers(&who).map_err(|_| Error::<T>::BadState)?;

			let stash_balance = <T as pallet::Config>::Currency::free_balance(&who);
			let value = value.min(stash_balance);
			Self::do_stake_validator(&who, &value)?;

			Ok(())
		}

		#[pallet::call_index(5)]
		pub fn unstake_currency(origin: OriginFor<T>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let _who = ensure_signed(origin)?;

			Ok(())
		}
	}
}
