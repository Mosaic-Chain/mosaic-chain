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
	use frame_support::traits::LockableCurrency;
	use frame_support::traits::WithdrawReasons;
	use frame_system::pallet_prelude::*;
	use pallet_nfts::Config as NftsConfig;
	use pallet_session::Config as SessionConfig;
	use pallet_session::Pallet as SessionPallet;
	use sp_runtime::traits::Convert;
	use sp_runtime::FixedPointOperand;
	use sp_runtime::Perbill;
	use sp_runtime::Saturating;

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
	pub enum PermissionType {
		PoS,
		DPoS,
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + NftsConfig + SessionConfig {
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
			+ FixedPointOperand
			+ Into<u128>
			+ sp_runtime::traits::Saturating;

		type Currency: frame_support::traits::LockableCurrency<
			Self::AccountId,
			Moment = BlockNumberFor<Self>,
			Balance = Self::Balance,
		>;

		type NftStakingHandler: NftStaking<
			Self::AccountId,
			Self::Balance,
			PermissionType,
			Self::ItemId,
		>;

		type NftDelegationHandler: NftDelegation<Self::AccountId, Self::Balance, Self::ItemId>;

		type MinimumStakingDuration: Get<u32>;

		type WeightInfo: WeightInfo;
	}

	type ValidatorId<T> = <T as frame_system::Config>::AccountId;
	type DelegatorId<T> = <T as frame_system::Config>::AccountId;

	#[pallet::storage]
	#[pallet::getter(fn total_stake)]
	pub type TotalStake<T: Config> = StorageValue<_, T::Balance>;

	#[pallet::storage]
	#[pallet::getter(fn account_exposure)]
	pub type AccountExposure<T: Config> = StorageMap<_, Twox64Concat, ValidatorId<T>, T::Balance>;

	#[pallet::storage]
	#[pallet::getter(fn individual_exposure)]
	pub type NftExposure<T: Config> =
		StorageDoubleMap<_, Twox64Concat, ValidatorId<T>, Twox64Concat, DelegatorId<T>, T::Balance>;

	#[pallet::storage]
	#[pallet::getter(fn currency_exposure)]
	pub type CurrencyExposure<T: Config> =
		StorageDoubleMap<_, Twox64Concat, ValidatorId<T>, Twox64Concat, DelegatorId<T>, T::Balance>;

	#[pallet::storage]
	#[pallet::getter(fn account_variant)]
	pub type AccountVariant<T: Config> =
		StorageMap<_, Twox64Concat, ValidatorId<T>, PermissionType>;

	#[pallet::event]
	// #[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
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
		ExpiresEarly,
	}

	impl<T: Config> Pallet<T> {
		fn do_stake_currency(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			let currency_exposure =
				CurrencyExposure::<T>::get(node_id, node_id).unwrap_or_default();
			Self::update_lock(staker_id, currency_exposure + value);

			TotalStake::<T>::mutate(|x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
				}
			});
			AccountExposure::<T>::mutate(node_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
				}
			});
			CurrencyExposure::<T>::mutate(node_id, staker_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
				}
			});
			Ok(())
		}

		fn do_unstake_currency(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			let currency_exposure =
				CurrencyExposure::<T>::get(node_id, node_id).unwrap_or_default();

			Self::update_lock(staker_id, currency_exposure.saturating_sub(value));

			TotalStake::<T>::mutate(|x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_sub(value);
				}
			});
			AccountExposure::<T>::mutate(node_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_sub(value);
				}
			});
			CurrencyExposure::<T>::mutate(node_id, staker_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_sub(value);
				}
			});

			Ok(())
		}

		fn do_stake_nft(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			TotalStake::<T>::mutate(|x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
				}
			});
			AccountExposure::<T>::mutate(node_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
				}
			});
			NftExposure::<T>::mutate(node_id, staker_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
				}
			});

			Ok(())
		}

		fn do_unstake_nft(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			TotalStake::<T>::mutate(|x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_sub(value);
				}
			});
			AccountExposure::<T>::mutate(node_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_sub(value);
				}
			});
			NftExposure::<T>::mutate(node_id, staker_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_sub(value);
				}
			});
			Ok(())
		}

		fn update_lock(staker_id: &T::AccountId, amount: T::Balance) {
			<T as pallet::Config>::Currency::set_lock(
				[0; 8],
				staker_id,
				amount,
				WithdrawReasons::all(),
			);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		pub fn delegate_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
			target: ValidatorId<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let target_variant = AccountVariant::<T>::get(&target);
			ensure!(target_variant.is_some(), Error::<T>::InvalidTarget);
			ensure!(target_variant == Some(PermissionType::DPoS), Error::<T>::TargetNotDPoS);

			let (expiry_in_session, nominal_value) =
				T::NftDelegationHandler::bind(&who, &target, &item_id)?;

			ensure!(
				expiry_in_session
					>= SessionPallet::<T>::current_index() + T::MinimumStakingDuration::get(),
				Error::<T>::ExpiresEarly
			);

			Self::do_stake_nft(&who, &target, nominal_value)?;
			Ok(())
		}

		#[pallet::call_index(1)]
		pub fn undelegate_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
			target: ValidatorId<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let nominal_value = T::NftDelegationHandler::unbind(&who, &target, &item_id)?;
			Self::do_unstake_nft(&who, &target, nominal_value)?;
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

			let (variant, nominal_value) = T::NftStakingHandler::bind(&who, &item_id)?;
			Self::do_stake_nft(&who, &who, nominal_value)?;
			AccountVariant::<T>::insert(&who, variant);

			Ok(())
		}

		#[pallet::call_index(3)]
		pub fn unbind_nft(origin: OriginFor<T>, _validator_id: ValidatorId<T>) -> DispatchResult {
			// TODO: clean up stake and delegation logic (for dpos)
			let who = ensure_signed(origin)?;

			let _variant = AccountVariant::<T>::get(&who).ok_or(Error::<T>::NotBound)?;
			let nominal_value = T::NftStakingHandler::unbind(&who)?;
			Self::do_unstake_nft(&who, &who, nominal_value)?;
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
			Self::do_stake_currency(&who, &who, value)?;

			Ok(())
		}

		#[pallet::call_index(5)]
		pub fn unstake_currency(
			origin: OriginFor<T>,
			#[pallet::compact] value: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_unstake_currency(&who, &who, value)?;

			Ok(())
		}

		#[pallet::call_index(6)]
		pub fn delegate_currency(
			origin: OriginFor<T>,
			#[pallet::compact] value: T::Balance,
			target: ValidatorId<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let target_variant = AccountVariant::<T>::get(&target);
			ensure!(target_variant.is_some(), Error::<T>::InvalidTarget);
			ensure!(target_variant == Some(PermissionType::DPoS), Error::<T>::TargetNotDPoS);

			let stash_balance = <T as pallet::Config>::Currency::free_balance(&who);
			let value = value.min(stash_balance);
			Self::do_stake_currency(&who, &target, value)?;

			Ok(())
		}

		#[pallet::call_index(7)]
		pub fn undelegate_currency(
			origin: OriginFor<T>,
			#[pallet::compact] value: T::Balance,
			target: ValidatorId<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_unstake_currency(&who, &target, value)?;

			Ok(())
		}
	}
}
