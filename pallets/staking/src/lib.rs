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
	use frame_support::pallet_prelude::*;
	use frame_support::{
		dispatch::Codec,
		traits::{Currency, Imbalance, LockableCurrency, OnUnbalanced, WithdrawReasons},
	};
	use frame_system::pallet_prelude::*;
	use pallet_nfts::Config as NftsConfig;
	use pallet_session::{Config as SessionConfig, Pallet as SessionPallet};
	use sp_runtime::{
		helpers_128bit::multiply_by_rational_with_rounding, traits::Convert, FixedPointOperand,
		Perbill, Rounding, Saturating,
	};
	use sp_std::vec::Vec as SpVec;

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

		fn kick(
			validator_id: &AccountId,
			delegator_id: &AccountId,
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

		fn kick(
			_validator_id: &AccountId,
			_delegator_id: &AccountId,
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
			+ From<u128>
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

		type Reward: OnUnbalanced<PositiveImbalanceOf<Self>>;

		type WeightInfo: WeightInfo;
	}

	type ValidatorId<T> = <T as frame_system::Config>::AccountId;
	type DelegatorId<T> = <T as frame_system::Config>::AccountId;
	type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::PositiveImbalance;

	#[pallet::storage]
	#[pallet::getter(fn total_stake)]
	pub type TotalStake<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn account_exposure)]
	pub type AccountExposure<T: Config> =
		StorageMap<_, Twox64Concat, ValidatorId<T>, T::Balance, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn nft_exposure)]
	pub type NftExposure<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		ValidatorId<T>,
		Twox64Concat,
		DelegatorId<T>,
		T::Balance,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn currency_exposure)]
	pub type CurrencyExposure<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		ValidatorId<T>,
		Twox64Concat,
		DelegatorId<T>,
		T::Balance,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn account_variant)]
	pub type AccountVariant<T: Config> =
		StorageMap<_, Twox64Concat, ValidatorId<T>, PermissionType>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Rewarded { stash: T::AccountId, amount: T::Balance },
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
		AlreadyInQueue,
	}

	impl<T: Config> Pallet<T> {
		fn do_stake_currency(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			TotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_add(value);
			});
			AccountExposure::<T>::mutate(node_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
				} else {
					*x = Some(value);
				}
			});
			CurrencyExposure::<T>::mutate(node_id, staker_id, |x| {
				let balance = if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
					*balance
				} else {
					*x = Some(value);
					value
				};
				Self::update_lock(staker_id, balance);
			});

			Ok(())
		}

		fn do_unstake_currency(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			TotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_sub(value);
			});
			AccountExposure::<T>::mutate_exists(node_id, |x| {
				let balance = x.unwrap_or_default();
				if value >= balance {
					*x = None
				} else {
					*x = Some(balance.saturating_sub(value))
				}
			});

			CurrencyExposure::<T>::mutate_exists(node_id, staker_id, |x| {
				let balance = x.unwrap_or_default();
				if value >= balance {
					*x = None
				} else {
					*x = Some(balance.saturating_sub(value))
				}
				Self::update_lock(staker_id, balance);
			});

			Ok(())
		}

		fn do_stake_nft(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			TotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_add(value);
			});
			AccountExposure::<T>::mutate(node_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
				} else {
					*x = Some(value);
				}
			});
			NftExposure::<T>::mutate(node_id, staker_id, |x| {
				if let Some(balance) = x {
					*balance = (*balance).saturating_add(value);
				} else {
					*x = Some(value);
				}
			});

			Ok(())
		}

		fn do_unstake_nft(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			TotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_sub(value);
			});
			AccountExposure::<T>::mutate_exists(node_id, |x| {
				let balance = x.unwrap_or_default();
				if value >= balance {
					*x = None
				} else {
					*x = Some(balance.saturating_sub(value))
				}
			});
			NftExposure::<T>::mutate_exists(node_id, staker_id, |x| {
				let balance = x.unwrap_or_default();
				if value >= balance {
					*x = None
				} else {
					*x = Some(balance.saturating_sub(value))
				}
			});
			Ok(())
		}

		fn do_kick_nft(node_id: &ValidatorId<T>, delegator_id: &DelegatorId<T>) -> DispatchResult {
			let nominal_value = T::NftDelegationHandler::kick(node_id, delegator_id)?;
			Self::do_unstake_nft(node_id, delegator_id, nominal_value)?;
			Ok(())
		}

		fn do_kick_currency(
			node_id: &ValidatorId<T>,
			delegator_id: &DelegatorId<T>,
			value: T::Balance,
		) -> DispatchResult {
			Self::do_unstake_currency(node_id, delegator_id, value)?;
			Ok(())
		}

		fn do_kick(node_id: &ValidatorId<T>, delegator_id: &DelegatorId<T>) -> DispatchResult {
			if NftExposure::<T>::contains_key(node_id, delegator_id) {
				Self::do_kick_nft(node_id, delegator_id)?;
			}
			// u128 max will unstake everything automatically because of `saturating_sub`
			Self::do_unstake_currency(node_id, delegator_id, u128::MAX.into())?;

			Ok(())
		}

		fn do_kick_all(node_id: &ValidatorId<T>) -> DispatchResult {
			NftExposure::<T>::iter_key_prefix(node_id)
				.try_for_each(|delegator| Self::do_kick_nft(node_id, &delegator))?;

			CurrencyExposure::<T>::iter_prefix(node_id).try_for_each(|(delegator, value)| {
				Self::do_kick_currency(node_id, &delegator, value)
			})?;
			Ok(())
		}

		fn update_lock(staker_id: &T::AccountId, amount: T::Balance) {
			<T as pallet::Config>::Currency::set_lock(
				[0; 8], // TODO: fix id
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
			let who = ensure_signed(origin)?;

			let active_validators = SessionPallet::<T>::validators();
			let queued_validators = SessionPallet::<T>::queued_keys()
				.into_iter()
				.map(|(v, _)| v)
				.collect::<SpVec<T::ValidatorId>>();

			let validator_set = active_validators
				.into_iter()
				.chain(queued_validators.into_iter())
				.collect::<SpVec<T::ValidatorId>>();

			// HACK/FIXME: this is bad
			let validator_id = T::ValidatorIdOf::convert(who.clone()).unwrap();
			ensure!(!validator_set.contains(&validator_id), Error::<T>::AlreadyInQueue);

			let permission_type = AccountVariant::<T>::get(&who).ok_or(Error::<T>::NotBound)?;

			if permission_type == PermissionType::DPoS {
				Self::do_kick_all(&who)?;
			}

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

			// TODO: check for MinimumStakingDuration
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

			// TODO: check for MinimumStakingDuration
			Self::do_unstake_currency(&who, &target, value)?;

			Ok(())
		}

		#[pallet::call_index(8)]
		pub fn kick(origin: OriginFor<T>, target: ValidatorId<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_kick(&who, &target)?;

			Ok(())
		}
	}

	trait DummySessionHook {
		fn on_before_session_end();
	}

	impl<T: Config> DummySessionHook for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<<T as pallet_session::Config>::ValidatorId>,
	{
		fn on_before_session_end() {
			let active_validators = SessionPallet::<T>::validators();
			// HACK: naive implementation, ideally we would want to add the currency and nft exposures together,
			// to avoid having to call the payout function twice for each category;
			// although this shouldn't be much of a problem in the short term, since the number of payout calls per account *at most* is 2,
			// it could add up in the long term at worse O(2n) instead of O(n)
			let payout_candidates = active_validators.iter().map(|validator_id| {
				let nft = NftExposure::<T>::iter_prefix(T::AccountId::from(validator_id.clone()));
				let currency =
					CurrencyExposure::<T>::iter_prefix(T::AccountId::from(validator_id.clone()));
				(validator_id, nft.chain(currency))
			});

			let total_stake_amount: u128 = TotalStake::<T>::get().into();
			// FIXME: replace active validator len with total number of blocks created in session
			let session_reward = u128::pow(10, 9) * active_validators.len() as u128;

			let mut total_imbalance = PositiveImbalanceOf::<T>::zero();

			// this is O(n) because we iterate over `candidates` in sequence
			for (validator_id, candidates) in payout_candidates {
				// calculate account exposure for commission calculation
				let account_exposure_amount: u128 =
					AccountExposure::<T>::get(T::AccountId::from(validator_id.clone()))
						.unwrap()
						.into();

				// validator gets its cut first
				let total_validator_reward_amount = multiply_by_rational_with_rounding(
					session_reward,
					account_exposure_amount,
					total_stake_amount,
					Rounding::NearestPrefDown,
				)
				.unwrap();

				let validator_commission_amount = multiply_by_rational_with_rounding(
					total_validator_reward_amount,
					50,
					1000,
					Rounding::NearestPrefDown,
				)
				.unwrap();

				let imbalance = <T as Config>::Currency::deposit_creating(
					&(*validator_id).clone().into(),
					validator_commission_amount.into(),
				);
				total_imbalance.subsume(imbalance);

				// calculate the rest of the payouts from the account's remaining rewards
				let remaining_account_reward =
					total_validator_reward_amount - validator_commission_amount;

				for (dest_account, individual_exposure_amount) in candidates {
					let reward_amount = multiply_by_rational_with_rounding(
						individual_exposure_amount.into(),
						remaining_account_reward,
						account_exposure_amount,
						Rounding::NearestPrefDown,
					)
					.unwrap();

					let imbalance = <T as Config>::Currency::deposit_creating(
						&dest_account,
						reward_amount.into(),
					);
					Self::deposit_event(Event::<T>::Rewarded {
						stash: dest_account,
						amount: imbalance.peek(),
					});
					total_imbalance.subsume(imbalance);
				}
			}
			T::Reward::on_unbalanced(total_imbalance);
		}
	}
}
