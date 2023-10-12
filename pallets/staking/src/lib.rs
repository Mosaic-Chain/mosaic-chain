#![cfg_attr(not(feature = "std"), no_std)]

//TODO: VALIDATE CALCUATION SECURITY, FIX IF NEEDED

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
	use frame_support::Twox64Concat;
	use frame_support::{
		dispatch::Codec,
		traits::{
			Currency, Imbalance, LockableCurrency, OnUnbalanced, ValidatorSet, WithdrawReasons,
		},
	};
	use frame_system::pallet_prelude::*;
	use pallet_nfts::Config as NftsConfig;
	use pallet_session::{Config as SessionConfig, Pallet as SessionPallet};
	use sp_runtime::{
		helpers_128bit::multiply_by_rational_with_rounding,
		traits::{Convert, Zero},
		DispatchError, FixedPointOperand, PerThing, Perbill, Rounding, Saturating,
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
	// TODO: remove im-online dependency
	pub trait Config:
		frame_system::Config
		+ NftsConfig
		+ SessionConfig
		+ pallet_offences::Config
		+ pallet_im_online::Config
	{
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

		type MinimumCommissionAllowed: Get<Perbill>;

		#[pallet::constant]
		type PalletId: Get<frame_support::PalletId>;

		type WeightInfo: WeightInfo;
	}

	type ValidatorId<T> = <T as frame_system::Config>::AccountId;
	type DelegatorId<T> = <T as frame_system::Config>::AccountId;
	type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::PositiveImbalance;

	#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Exposure<Balance: HasCompact> {
		#[codec(compact)]
		pub currency: Balance,
		#[codec(compact)]
		pub nft: Balance,
	}

	impl<Balance> Exposure<Balance>
	where
		Balance: HasCompact + Add<Output = Balance> + Copy,
	{
		fn exposure(&self) -> Balance {
			self.currency + self.nft
		}
	}
	impl<Balance: Default + HasCompact> Default for Exposure<Balance> {
		fn default() -> Self {
			Self { currency: Default::default(), nft: Default::default() }
		}
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct NodeDetails<AccountId, Balance: HasCompact> {
		pub own: Exposure<Balance>,
		pub delegators: SpVec<(AccountId, Exposure<Balance>)>,
		// inverse of the unapplied slash in the current session
		pub inverse_slash: Option<Perbill>,
	}

	impl<AccountId: Ord, Balance> NodeDetails<AccountId, Balance>
	where
		Balance: HasCompact + Add<Output = Balance> + Sum + Copy,
	{
		fn own_exposure(&self) -> Balance {
			self.own.exposure()
		}

		fn get_delegator(&mut self, delegator_id: &AccountId) -> Option<&mut Exposure<Balance>> {
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

	impl<AccountId, Balance: Default + HasCompact> Default for NodeDetails<AccountId, Balance> {
		fn default() -> Self {
			Self { own: Exposure::default(), delegators: SpVec::new(), inverse_slash: None }
		}
	}

	#[pallet::storage]
	#[pallet::getter(fn total_stake)]
	pub type TotalStake<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	#[pallet::storage]
	pub type Nodes<T: Config> = StorageMap<
		_,
		Twox64Concat,
		ValidatorId<T>,
		NodeDetails<T::AccountId, T::Balance>,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn account_variant)]
	pub type AccountVariant<T: Config> =
		StorageMap<_, Twox64Concat, ValidatorId<T>, PermissionType>;

	#[pallet::storage]
	#[pallet::getter(fn validator_commission)]
	pub type ValidatorCommission<T: Config> = StorageMap<
		_,
		Twox64Concat,
		ValidatorId<T>,
		Perbill,
		ValueQuery,
		T::MinimumCommissionAllowed,
	>;

	#[pallet::storage]
	pub type LockedCurrency<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::Balance>;

	// TODO: add slashed event
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Rewarded { stash: T::AccountId, amount: T::Balance },
		RewardCurrencyCreated { amount: T::Balance },
		TotalSlashThisSession { amount: T::Balance },
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
		NotEnoughFunds,
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

				assert!(Nodes::<T>::get(validator_id).is_none());
				assert!(AccountVariant::<T>::get(validator_id).is_none());

				T::NftStakingHandler::bind(validator_id, &item_id)
					.expect("could bind newly made permission nft");
				AccountVariant::<T>::insert(validator_id, permission);
				ValidatorCommission::<T>::insert(
					validator_id.clone(),
					if *permission == PermissionType::PoS {
						Perbill::from_percent(100)
					} else {
						T::MinimumCommissionAllowed::get()
					},
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
			Nodes::<T>::mutate(node_id, |x| {
				if node_id == staker_id {
					let Some(exposure) = x else {
						return Err(Error::<T>::InvalidTarget);
					};
					exposure.own.currency = exposure.own.currency.saturating_add(value);
				} else {
					let Some(exposure) = x else {
						return Err(Error::<T>::InvalidTarget);
					};
					if let Some(delegator_exposure) = exposure.get_delegator(staker_id) {
						delegator_exposure.currency =
							delegator_exposure.currency.saturating_add(value);
					} else {
						let delegator_exposure = Exposure::<<T as pallet::Config>::Balance> {
							currency: value,
							..Default::default()
						};
						exposure.delegators.push((staker_id.clone(), delegator_exposure));
					};
				}

				Self::lock_currency(staker_id, value);
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
			Nodes::<T>::mutate_exists(node_id, |x| {
				if node_id == staker_id {
					let mut exposure = x.clone().unwrap_or_default();
					if value > exposure.own.currency {
						return Err(Error::<T>::NotEnoughFunds);
					}
					exposure.own.currency = exposure.own.currency.saturating_sub(value);
					*x = Some(exposure.clone());
				} else {
					let Some(exposure) = x else {
						return Err(Error::<T>::InvalidTarget);
					};

					if let Some(delegator_exposure) = exposure.get_delegator(staker_id) {
						if value > delegator_exposure.currency {
							return Err(Error::<T>::NotEnoughFunds);
						}
						delegator_exposure.currency =
							delegator_exposure.currency.saturating_sub(value);
					} else {
						return Err(Error::<T>::InvalidTarget);
					};
				}

				Self::release_currency(staker_id, value);
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
			Nodes::<T>::mutate(node_id, |x| {
				if node_id == staker_id {
					if let Some(exposure) = x {
						exposure.own.nft = exposure.own.nft.saturating_add(value);
					} else {
						let mut exposure = NodeDetails::<T::AccountId, T::Balance>::default();
						exposure.own.nft = exposure.own.nft.saturating_add(value);
						*x = Some(exposure);
					}
				} else {
					let Some(exposure) = x else {
						return Err(Error::<T>::InvalidTarget);
					};
					if let Some(delegator_exposure) = exposure.get_delegator(staker_id) {
						delegator_exposure.nft = delegator_exposure.nft.saturating_add(value);
					} else {
						let delegator_exposure = Exposure::<<T as pallet::Config>::Balance> {
							nft: value,
							..Default::default()
						};
						exposure.delegators.push((staker_id.clone(), delegator_exposure.clone()));
					};
				}
				Ok(())
			})?;
			Ok(())
		}

		pub fn do_bind_nft(
			node_id: ValidatorId<T>,
			node_variant: PermissionType,
			nominal_value: T::Balance,
		) -> DispatchResult {
			Self::do_stake_nft(&node_id, &node_id, nominal_value)?;
			AccountVariant::<T>::insert(node_id.clone(), node_variant);
			ValidatorCommission::<T>::insert(node_id, T::MinimumCommissionAllowed::get());
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
			Nodes::<T>::mutate_exists(node_id, |x| {
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

					if let Some(delegator_exposure) = exposure.get_delegator(staker_id) {
						delegator_exposure.nft = delegator_exposure.nft.saturating_sub(value);
					} else {
						return Err(Error::<T>::InvalidTarget);
					};
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
			match Nodes::<T>::get(node_id) {
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
			Nodes::<T>::get(node_id).expect("TODO").delegators.iter().try_for_each(
				|(delegator, individual_exposure)| {
					Self::do_kick_currency(node_id, delegator, individual_exposure.currency)?;
					Self::do_kick_nft(node_id, delegator)?;
					Ok::<(), DispatchError>(())
				},
			)?;
			Ok(())
		}

		fn update_lock(account_id: &T::AccountId, amount: T::Balance) {
			<T as pallet::Config>::Currency::set_lock(
				<T as Config>::PalletId::get().0,
				account_id,
				amount,
				WithdrawReasons::all(),
			);

			LockedCurrency::<T>::insert(account_id, amount);
		}

		fn release_lock(account_id: &T::AccountId) {
			<T as pallet::Config>::Currency::remove_lock(
				<T as Config>::PalletId::get().0,
				account_id,
			);

			LockedCurrency::<T>::remove(account_id);
		}

		/// Adds the provided amount to the account's lock.
		/// Possible side-effects: creates an entry in LockedCurrency, locks currency
		fn lock_currency(account_id: &T::AccountId, amount: T::Balance) {
			if amount.is_zero() {
				return;
			}

			let locked = LockedCurrency::<T>::get(account_id)
				.unwrap_or(Zero::zero())
				.saturating_add(amount);

			Self::update_lock(account_id, locked);
		}

		/// Removes the provided amount from the account's lock.
		/// Possible side-effects: removes an entry from LockedCurrency, releases lock
		fn release_currency(account_id: &T::AccountId, amount: T::Balance) {
			if amount.is_zero() {
				return;
			}

			if let Some(locked) = LockedCurrency::<T>::get(account_id) {
				let locked = locked.saturating_sub(amount);
				if locked.is_zero() {
					Self::release_lock(account_id);
				} else {
					Self::update_lock(account_id, locked);
				}
			}
		}

		/// Rewards a node and it's delegators.
		/// Returns the amount of new currency created.
		fn reward_node(
			validator_account_id: T::AccountId,
			node_details: &mut NodeDetails<T::AccountId, T::Balance>,
			total_stake: u128,
			session_reward: u128,
		) -> PositiveImbalanceOf<T> {
			let total_node_exposure: u128 = node_details.total_exposure().into();
			let own_node_exposure: u128 = node_details.own.exposure().into();
			let delegated_node_exposure: u128 = total_node_exposure - own_node_exposure;

			let commission_part_per_billion =
				ValidatorCommission::<T>::get(validator_account_id.clone()).deconstruct() as u128;
			let mut total_imbalance = PositiveImbalanceOf::<T>::zero();

			// validator gets its cut first
			let total_reward = multiply_by_rational_with_rounding(
				session_reward,
				total_node_exposure,
				total_stake,
				Rounding::NearestPrefDown,
			)
			.expect("no arithmetic error");

			let inherent_validator_cut = multiply_by_rational_with_rounding(
				total_reward,
				own_node_exposure,
				total_node_exposure,
				Rounding::NearestPrefDown,
			)
			.expect("no arithmetic error");

			let commission_validator_cut = multiply_by_rational_with_rounding(
				total_reward - inherent_validator_cut,
				commission_part_per_billion,
				u128::pow(10, 9),
				Rounding::NearestPrefDown,
			)
			.expect("no arithmetic error");

			let total_validator_cut = inherent_validator_cut + commission_validator_cut;

			let imbalance = <T as Config>::Currency::deposit_creating(
				&validator_account_id,
				total_validator_cut.into(),
			);

			Self::lock_currency(&validator_account_id, imbalance.peek());
			node_details.own.currency = node_details.own.currency.saturating_add(imbalance.peek());

			Self::deposit_event(Event::<T>::Rewarded {
				stash: validator_account_id,
				amount: imbalance.peek(),
			});

			total_imbalance.subsume(imbalance);

			let delegator_cut = total_reward - total_validator_cut;

			for (delegator_id, delegator_exposure) in &mut node_details.delegators {
				let reward_amount = multiply_by_rational_with_rounding(
					delegator_cut,
					delegator_exposure.exposure().into(),
					delegated_node_exposure,
					Rounding::NearestPrefDown,
				)
				.unwrap();

				let imbalance =
					<T as Config>::Currency::deposit_creating(delegator_id, reward_amount.into());

				Self::deposit_event(Event::<T>::Rewarded {
					stash: delegator_id.clone(),
					amount: imbalance.peek(),
				});

				Self::lock_currency(delegator_id, imbalance.peek());
				delegator_exposure.currency =
					delegator_exposure.currency.saturating_add(imbalance.peek());
				total_imbalance.subsume(imbalance);
			}

			total_imbalance
		}

		/// Slash a node and it's delegators.
		/// Mutates node details and returns the amount of currency to be removed from total_stake
		fn slash_node(
			validator_account_id: T::AccountId,
			node_details: &mut NodeDetails<T::AccountId, T::Balance>,
			slash_proportion: Perbill,
		) -> T::Balance {
			let mut total_stake_slash = T::Balance::zero();

			// Slash own permission and delegator nfts
			if !node_details.own.nft.is_zero() {
				let new_perission_stake =
					T::NftStakingHandler::slash(&validator_account_id, slash_proportion)
						.expect("TODO");
				let new_delegator_nft_stake = T::NftDelegationHandler::slash(
					&validator_account_id,
					&validator_account_id,
					slash_proportion,
				)
				.unwrap_or(Zero::zero());

				let new_nft_stake = new_perission_stake.saturating_add(new_delegator_nft_stake);

				total_stake_slash = total_stake_slash
					.saturating_add(node_details.own.nft.saturating_sub(new_nft_stake));
				node_details.own.nft = new_nft_stake;
			}

			// Slash own currency
			if !node_details.own.currency.is_zero() {
				let slash_amount = slash_proportion * node_details.own.currency;

				if <T as pallet::Config>::Currency::can_slash(&validator_account_id, slash_amount) {
					let (actual_slash, _) =
						<T as pallet::Config>::Currency::slash(&validator_account_id, slash_amount);
					node_details.own.currency =
						node_details.own.currency.saturating_sub(actual_slash.peek());

					total_stake_slash = total_stake_slash.saturating_add(actual_slash.peek());
					Self::release_currency(&validator_account_id, actual_slash.peek());
				}
			}

			// Slash delegators
			for (delegator_id, exposure) in &mut node_details.delegators {
				// Slash delegator nfts
				if !exposure.nft.is_zero() {
					let new_nft_stake = T::NftDelegationHandler::slash(
						delegator_id,
						&validator_account_id,
						slash_proportion,
					)
					.expect("TODO");

					total_stake_slash = total_stake_slash
						.saturating_add(exposure.nft.saturating_sub(new_nft_stake));
					exposure.nft = new_nft_stake;
				}

				// Slash delegator currency
				if !exposure.currency.is_zero() {
					let slash_amount = slash_proportion * exposure.currency;
					if <T as pallet::Config>::Currency::can_slash(delegator_id, slash_amount) {
						let (actual_slash, _) =
							<T as pallet::Config>::Currency::slash(delegator_id, slash_amount);
						exposure.currency = exposure.currency.saturating_sub(actual_slash.peek());
						total_stake_slash = total_stake_slash.saturating_add(actual_slash.peek());

						Self::release_currency(delegator_id, actual_slash.peek());
					}
				}
			}

			total_stake_slash
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

			ensure!(Nodes::<T>::get(&who).is_none(), Error::<T>::AlreadyBound);
			ensure!(AccountVariant::<T>::get(&who).is_none(), Error::<T>::AlreadyBound);

			let (variant, nominal_value) = T::NftStakingHandler::bind(&who, &item_id)?;

			ValidatorCommission::<T>::insert(
				&who,
				if variant == PermissionType::PoS {
					Perbill::from_percent(100)
				} else {
					T::MinimumCommissionAllowed::get()
				},
			);

			AccountVariant::<T>::insert(&who, variant);
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
			Nodes::<T>::remove(&who);

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
		pub fn set_commission(origin: OriginFor<T>, commission: Perbill) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// FIXME: Check for permission type
			ValidatorCommission::<T>::set(who, commission);

			Ok(())
		}
	}

	impl<T: Config> utils::traits::SessionHook for Pallet<T>
	where
		<T as frame_system::Config>::AccountId: From<<T as pallet_session::Config>::ValidatorId>,
	{
		fn session_ended(_: u32) -> DispatchResult {
			let total_stake: u128 = TotalStake::<T>::get().into();

			if total_stake == 0 {
				return Ok(());
			}

			let active_validators = SessionPallet::<T>::validators();

			// FIXME: replace active validator len with total number of blocks created in session
			// TODO: set this to a more sensible value
			let session_reward = 100 * active_validators.len() as u128;

			let mut total_stake_slash: T::Balance = Zero::zero();
			let mut total_imbalance = PositiveImbalanceOf::<T>::zero();

			for validator_id in active_validators {
				let validator_account_id = T::AccountId::from(validator_id.clone());

				Nodes::<T>::mutate_extant(validator_account_id.clone(), |details| {
					if let Some(inverse_slash) = details.inverse_slash.take() {
						// Apply slash, skip rewarding this node
						let slash_proportion = inverse_slash.left_from_one();

						total_stake_slash = total_stake_slash.saturating_add(Self::slash_node(
							validator_account_id,
							details,
							slash_proportion,
						));
					} else {
						// No need to slash, reward node
						total_imbalance.subsume(Self::reward_node(
							validator_account_id,
							details,
							total_stake,
							session_reward,
						));
					}
				});
			}

			Self::deposit_event(Event::<T>::RewardCurrencyCreated {
				amount: total_imbalance.peek(),
			});

			Self::deposit_event(Event::<T>::TotalSlashThisSession { amount: total_stake_slash });

			TotalStake::<T>::mutate(|s| {
				*s = s.saturating_add(total_imbalance.peek());
				*s -= total_stake_slash;
			});

			T::Reward::on_unbalanced(total_imbalance);
			Ok(())
		}
	}

	// TODO: make id tuple more generic
	// TODO: define weights
	impl<T: Config>
		sp_staking::offence::OnOffenceHandler<
			<T as frame_system::Config>::AccountId,
			pallet_im_online::IdentificationTuple<T>,
			frame_support::weights::Weight,
		> for Pallet<T>
	where
		<<T as pallet_im_online::Config>::ValidatorSet as ValidatorSet<
			<T as frame_system::Config>::AccountId,
		>>::ValidatorId: codec::EncodeLike<<T as frame_system::Config>::AccountId>,
	{
		fn on_offence(
			offenders: &[sp_staking::offence::OffenceDetails<
				<T as frame_system::Config>::AccountId,
				pallet_im_online::IdentificationTuple<T>,
			>],
			slash_fraction: &[Perbill],
			_session: sp_staking::SessionIndex,
			_disable_strategy: sp_staking::offence::DisableStrategy,
		) -> frame_support::weights::Weight {
			for (o, slash) in offenders.iter().zip(slash_fraction.iter()) {
				Nodes::<T>::mutate_extant(o.offender.0.clone(), |node_details| match node_details
					.inverse_slash
				{
					None => node_details.inverse_slash = Some(slash.left_from_one()),
					Some(ref mut acc) => *acc = *acc * slash.left_from_one(),
				});
			}

			Default::default()
		}
	}
}
