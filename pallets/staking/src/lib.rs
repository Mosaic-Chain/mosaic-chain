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
	use codec::HasCompact;
	use frame_support::pallet_prelude::{ValueQuery, *};
	use frame_support::{
		dispatch::Codec,
		traits::{Currency, Imbalance, LockableCurrency, OnUnbalanced, WithdrawReasons},
	};
	use frame_system::pallet_prelude::*;
	use pallet_nfts::Config as NftsConfig;
	use pallet_session::{Config as SessionConfig, Pallet as SessionPallet};
	use sp_runtime::DispatchError;
	use sp_runtime::{
		helpers_128bit::multiply_by_rational_with_rounding, traits::Convert, FixedPointOperand,
		Perbill, Rounding, Saturating,
	};
	use sp_std::{iter::Sum, ops::Add, vec::Vec as SpVec};

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	pub trait NftStaking<AccountId, Balance, Variant, ItemId> {
		fn mint(
			account_id: &AccountId,
			permission: &Variant,
			nominal_value: &Balance,
		) -> Result<ItemId, DispatchError>;

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
		fn mint(
			_account_id: &AccountId,
			_permission: &Variant,
			_nominal_value: &Balance,
		) -> Result<ItemId, DispatchError> {
			unimplemented!()
		}

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
			+ Sum
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

		type MinimumCommissionAllowed: Get<u128>;

		type WeightInfo: WeightInfo;
	}

	type ValidatorId<T> = <T as frame_system::Config>::AccountId;
	type DelegatorId<T> = <T as frame_system::Config>::AccountId;
	type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::PositiveImbalance;

	#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct IndividualExposure<Balance: HasCompact> {
		#[codec(compact)]
		pub currency: Balance,
		#[codec(compact)]
		pub nft: Balance,
	}

	impl<Balance> IndividualExposure<Balance>
	where
		Balance: HasCompact + Add<Output = Balance> + Copy,
	{
		fn exposure(&self) -> Balance {
			self.currency + self.nft
		}
	}
	impl<Balance: Default + HasCompact> Default for IndividualExposure<Balance> {
		fn default() -> Self {
			Self { currency: Default::default(), nft: Default::default() }
		}
	}
	// TODO: I think we should rename this, as it's no longer just exposure.
	// Suggestion: NodeDetails, ValidatorDetails. These names follow substrate conventions.
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Exposure<AccountId, Balance: HasCompact> {
		pub own: IndividualExposure<Balance>,
		pub delegators: SpVec<(AccountId, IndividualExposure<Balance>)>,
	}

	impl<AccountId: Ord, Balance> Exposure<AccountId, Balance>
	where
		Balance: HasCompact + Add<Output = Balance> + Sum + Copy,
	{
		fn own_exposure(&self) -> Balance {
			self.own.exposure()
		}

		fn get_delegator(
			&mut self,
			delegator_id: &AccountId,
		) -> Option<&mut IndividualExposure<Balance>> {
			self.delegators
				.iter_mut()
				.find(|(account_id, _)| account_id == delegator_id)
				.map(|(_, exposure)| exposure)
		}

		fn total_exposure(&self) -> Balance {
			self.own_exposure()
				+ self.delegators.iter().map(|(_, exposure)| exposure.exposure()).sum()
		}
	}

	impl<AccountId, Balance: Default + HasCompact> Default for Exposure<AccountId, Balance> {
		fn default() -> Self {
			Self { own: IndividualExposure::default(), delegators: SpVec::new() }
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn total_stake)]
	pub type TotalStake<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn individual_exposure)]
	pub type NodeExposure<T: Config> = StorageMap<
		_,
		Twox64Concat,
		ValidatorId<T>,
		Exposure<T::AccountId, T::Balance>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn account_variant)]
	pub type AccountVariant<T: Config> =
		StorageMap<_, Twox64Concat, ValidatorId<T>, PermissionType>;

	#[pallet::storage]
	#[pallet::getter(fn validator_commission)]
	pub type ValidatorCommission<T: Config> =
		StorageMap<_, Twox64Concat, ValidatorId<T>, u128, ValueQuery, T::MinimumCommissionAllowed>;

	// TODO: add slashed event
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

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_staking_validators: SpVec<(ValidatorId<T>, PermissionType, T::Balance)>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_staking_validators: SpVec::new() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (validator_id, permission, nominal_value) in &self.initial_staking_validators {
				let item_id = T::NftStakingHandler::mint(validator_id, permission, nominal_value)
					.expect("could mint new permission nft");

				assert!(NodeExposure::<T>::get(validator_id).is_none());
				assert!(AccountVariant::<T>::get(validator_id).is_none());

				T::NftStakingHandler::bind(validator_id, &item_id)
					.expect("could bind newly made permission nft");
				AccountVariant::<T>::insert(validator_id, permission);
				ValidatorCommission::<T>::insert(
					validator_id.clone(),
					T::MinimumCommissionAllowed::get(),
				);
				Pallet::<T>::do_stake_nft(validator_id, validator_id, *nominal_value)
					.expect("could stake nft");
			}
		}
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
			NodeExposure::<T>::mutate(node_id, |x| {
				if node_id == staker_id {
					let Some(exposure) = x else {
						return Err(Error::<T>::InvalidTarget);
					};
					exposure.own.currency = exposure.own.currency.saturating_add(value);
					Self::update_lock(staker_id, exposure.own.currency);
				} else {
					let Some(exposure) = x else {
						return Err(Error::<T>::InvalidTarget);
					};
					let delegator_exposure = if let Some(delegator_exposure) =
						exposure.get_delegator(staker_id)
					{
						delegator_exposure.currency =
							delegator_exposure.currency.saturating_add(value);
						delegator_exposure.clone()
					} else {
						let delegator_exposure =
							IndividualExposure::<<T as pallet::Config>::Balance> {
								currency: value,
								..Default::default()
							};
						exposure.delegators.push((staker_id.clone(), delegator_exposure.clone()));
						delegator_exposure
					};
					Self::update_lock(staker_id, delegator_exposure.currency);
				}
				Ok(())
			})?;
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
			NodeExposure::<T>::mutate_exists(node_id, |x| {
				if node_id == staker_id {
					let mut exposure = x.clone().unwrap_or_default();
					if value >= exposure.own_exposure() {
						*x = None;
					} else {
						exposure.own.currency = exposure.own.currency.saturating_sub(value);
						*x = Some(exposure.clone());
					}
					Self::update_lock(staker_id, exposure.own.currency);
				} else {
					let Some(exposure) = x else {
						return Err(Error::<T>::InvalidTarget);
					};
					let delegator_exposure =
						if let Some(delegator_exposure) = exposure.get_delegator(staker_id) {
							delegator_exposure.currency =
								delegator_exposure.currency.saturating_sub(value);
							delegator_exposure
						} else {
							return Err(Error::<T>::InvalidTarget);
						};
					Self::update_lock(staker_id, delegator_exposure.currency);
				}
				Ok(())
			})?;
			Ok(())
		}

		pub fn do_stake_nft(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			TotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_add(value);
			});
			NodeExposure::<T>::mutate(node_id, |x| {
				if node_id == staker_id {
					if let Some(exposure) = x {
						exposure.own.nft = exposure.own.nft.saturating_add(value);
					} else {
						let mut exposure = Exposure::<T::AccountId, T::Balance>::default();
						exposure.own.nft = exposure.own.nft.saturating_add(value);
						*x = Some(exposure);
					}
				} else {
					let Some(exposure) = x else {
						return Err(Error::<T>::InvalidTarget);
					};
					let delegator_exposure = if let Some(delegator_exposure) =
						exposure.get_delegator(staker_id)
					{
						delegator_exposure.nft = delegator_exposure.nft.saturating_add(value);
						delegator_exposure.clone()
					} else {
						let delegator_exposure =
							IndividualExposure::<<T as pallet::Config>::Balance> {
								nft: value,
								..Default::default()
							};
						exposure.delegators.push((staker_id.clone(), delegator_exposure.clone()));
						delegator_exposure
					};
					Self::update_lock(staker_id, delegator_exposure.nft);
				}
				Ok(())
			})?;
			Ok(())
		}

		pub fn do_unstake_nft(
			node_id: &ValidatorId<T>,
			staker_id: &T::AccountId,
			value: T::Balance,
		) -> DispatchResult {
			TotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_sub(value);
			});
			NodeExposure::<T>::mutate_exists(node_id, |x| {
				if node_id == staker_id {
					let mut exposure = x.clone().unwrap_or_default();
					if value >= exposure.own_exposure() {
						*x = None;
					} else {
						exposure.own.nft = exposure.own.nft.saturating_sub(value);
						*x = Some(exposure);
					}
				} else {
					let Some(exposure) = x else {
						return Err(Error::<T>::InvalidTarget);
					};
					let delegator_exposure =
						if let Some(delegator_exposure) = exposure.get_delegator(staker_id) {
							delegator_exposure.currency =
								delegator_exposure.currency.saturating_sub(value);
							delegator_exposure
						} else {
							return Err(Error::<T>::InvalidTarget);
						};
					Self::update_lock(staker_id, delegator_exposure.currency);
				}
				Ok(())
			})?;
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
			Self::do_unstake_currency(node_id, delegator_id, value)
		}

		fn do_kick(node_id: &ValidatorId<T>, delegator_id: &DelegatorId<T>) -> DispatchResult {
			match NodeExposure::<T>::get(node_id) {
				Some(mut exposure) => {
					if exposure.get_delegator(delegator_id).is_some() {
						Self::do_kick_nft(node_id, delegator_id)?;
					}
				},
				None => return Err(Error::<T>::InvalidTarget.into()),
			};

			// u128 max will unstake everything automatically because of `saturating_sub`
			Self::do_unstake_currency(node_id, delegator_id, u128::MAX.into())
		}

		fn do_kick_all(node_id: &ValidatorId<T>) -> DispatchResult {
			NodeExposure::<T>::get(node_id)
				.ok_or(Error::<T>::BadState)?
				.delegators
				.iter()
				.try_for_each(|(delegator, individual_exposure)| {
					Self::do_kick_currency(node_id, delegator, individual_exposure.currency)?;
					Self::do_kick_nft(node_id, delegator)?;
					Ok::<(), DispatchError>(())
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

			// TODO: check for MinimumStakingDuration
			let nominal_value = T::NftDelegationHandler::unbind(&who, &target, &item_id)?;
			Self::do_unstake_nft(&target, &who, nominal_value)?;
			Ok(())
		}

		#[pallet::call_index(2)]
		pub fn bind_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(NodeExposure::<T>::get(&who).is_none(), Error::<T>::AlreadyBound);
			ensure!(AccountVariant::<T>::get(&who).is_none(), Error::<T>::AlreadyBound);

			let (variant, nominal_value) = T::NftStakingHandler::bind(&who, &item_id)?;
			AccountVariant::<T>::insert(&who, variant);
			ValidatorCommission::<T>::insert(who.clone(), T::MinimumCommissionAllowed::get());
			Self::do_stake_nft(&who, &who, nominal_value)?;

			Ok(())
		}

		#[pallet::call_index(3)]
		pub fn unbind_nft(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let permission_type = AccountVariant::<T>::get(&who).ok_or(Error::<T>::NotBound)?;

			if permission_type == PermissionType::DPoS {
				Self::do_kick_all(&who)?;
			}

			// TODO: check for MinimumStakingDuration
			let active_validators = SessionPallet::<T>::validators();
			let queued_validators = SessionPallet::<T>::queued_keys()
				.into_iter()
				.map(|(v, _)| v)
				.collect::<SpVec<T::ValidatorId>>();

			let validator_set = active_validators
				.into_iter()
				.chain(queued_validators.into_iter())
				.collect::<SpVec<T::ValidatorId>>();

			let validator_id = T::ValidatorIdOf::convert(who.clone()).expect("TODO");
			ensure!(!validator_set.contains(&validator_id), Error::<T>::AlreadyInQueue);

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
			Self::do_stake_currency(&target, &who, value)?;

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
			Self::do_unstake_currency(&target, &who, value)?;

			Ok(())
		}

		#[pallet::call_index(8)]
		pub fn kick(origin: OriginFor<T>, target: ValidatorId<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_kick(&who, &target)?;

			Ok(())
		}

		#[pallet::call_index(9)]
		pub fn set_commission(
			origin: OriginFor<T>,
			commission_in_part_per_billion: u128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ValidatorCommission::<T>::set(who, commission_in_part_per_billion);

			Ok(())
		}
	}

	impl<T: Config> utils::traits::SessionHook for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<<T as pallet_session::Config>::ValidatorId>,
	{
		fn session_ended(_: u32) -> DispatchResult {
			let total_stake_amount: u128 = TotalStake::<T>::get().into();

			if total_stake_amount == 0 {
				return Ok(());
			}

			let active_validators = SessionPallet::<T>::validators();

			let payout_candidates = active_validators.iter().map(|validator_id| {
				let exposure = NodeExposure::<T>::get(T::AccountId::from(validator_id.clone()))
					.unwrap_or_default();
				(validator_id, exposure)
			});

			// FIXME: replace active validator len with total number of blocks created in session
			let session_reward = u128::pow(10, 18) * active_validators.len() as u128;

			let mut total_imbalance = PositiveImbalanceOf::<T>::zero();

			// this is O(n) because we iterate over `exposure.delegators` in sequence
			for (validator_id, exposure) in payout_candidates {
				// calculate account exposure for commission calculation
				let node_exposure_amount: u128 = exposure.total_exposure().into();

				// validator gets its cut first
				let total_validator_reward_amount = multiply_by_rational_with_rounding(
					session_reward,
					node_exposure_amount,
					total_stake_amount,
					Rounding::NearestPrefDown,
				)
				.unwrap();

				let commission_part_per_billion =
					ValidatorCommission::<T>::get(T::AccountId::from(validator_id.clone()));

				let validator_commission_amount = multiply_by_rational_with_rounding(
					total_validator_reward_amount,
					commission_part_per_billion,
					u128::pow(10, 9),
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

				for (delegator_id, delegator_exposure) in exposure.delegators {
					let reward_amount = multiply_by_rational_with_rounding(
						delegator_exposure.exposure().into(),
						remaining_account_reward,
						node_exposure_amount,
						Rounding::NearestPrefDown,
					)
					.unwrap();

					let imbalance = <T as Config>::Currency::deposit_creating(
						&delegator_id,
						reward_amount.into(),
					);
					Self::deposit_event(Event::<T>::Rewarded {
						stash: delegator_id,
						amount: imbalance.peek(),
					});
					total_imbalance.subsume(imbalance);
				}
			}
			T::Reward::on_unbalanced(total_imbalance);
			Ok(())
		}
	}
}
