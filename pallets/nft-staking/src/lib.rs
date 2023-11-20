#![cfg_attr(not(feature = "std"), no_std)]

//TODO: VALIDATE CALCUATION SECURITY, FIX IF NEEDED

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(any())]
mod mock;
#[cfg(any())]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use sp_std::{iter::Sum, ops::Add, vec::Vec as SpVec};

use codec::{Codec, HasCompact};
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, Imbalance, LockableCurrency, OnUnbalanced, ValidatorSet, WithdrawReasons},
	Twox64Concat,
};
use frame_system::pallet_prelude::*;
use pallet_nfts::Config as NftsConfig;
use pallet_session::{Config as SessionConfig, Pallet as SessionPallet};
use sp_runtime::{
	helpers_128bit::multiply_by_rational_with_rounding,
	traits::{Convert, Zero},
	FixedPointOperand, PerThing, Perbill, Rounding, Saturating,
};

use utils::{
	traits::{NftDelegation, NftStaking},
	SessionIndex,
};

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

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

	impl PermissionType {
		fn default_commission<T: Config>(self) -> Perbill {
			match self {
				Self::PoS => Perbill::from_percent(100),
				Self::DPoS => T::MinimumCommissionAllowed::get(),
			}
		}
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

	// TODO: wouldn't it be nice if we represented the mandatory 100% comission for PoS
	// validators on the type level?
	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct NodeDetails<Balance: HasCompact> {
		pub permission: PermissionType,
		pub commission: Perbill,
		pub own_exposure: Exposure<Balance>,
	}

	#[pallet::storage]
	pub type LockedCurrency<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::Balance>;

	#[pallet::storage]
	#[pallet::getter(fn inverse_slash)]
	pub type InverseSlashes<T: Config> = StorageMap<_, Twox64Concat, ValidatorId<T>, Perbill>;

	#[pallet::storage]
	pub type NextTotalStake<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	#[pallet::storage]
	pub type CurrentTotalStake<T: Config> = StorageValue<_, T::Balance, ValueQuery>;

	#[pallet::storage]
	pub type NextNodes<T: Config> =
		StorageMap<_, Twox64Concat, ValidatorId<T>, NodeDetails<T::Balance>, OptionQuery>;

	#[pallet::storage]
	pub type CurrentNodes<T: Config> =
		StorageMap<_, Twox64Concat, ValidatorId<T>, NodeDetails<T::Balance>, OptionQuery>;

	#[pallet::storage]
	pub type NextDelegations<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		ValidatorId<T>,
		Twox64Concat,
		DelegatorId<T>,
		Exposure<T::Balance>,
	>;

	#[pallet::storage]
	pub type CurrentDelegations<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		ValidatorId<T>,
		Twox64Concat,
		DelegatorId<T>,
		Exposure<T::Balance>,
	>;

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
		InvalidCommission,
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

	// TODO: seperate nft minting from staking genesis
	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			for (validator_id, permission, nominal_value) in &self.initial_staking_validators {
				assert!(!NextNodes::<T>::contains_key(validator_id));

				let item_id = T::NftStakingHandler::mint(validator_id, permission, nominal_value)
					.expect("couldn't mint new nft on genesis; this shouldn't happen");

				T::NftStakingHandler::bind(validator_id, &item_id)
					.expect("could't bind permission nft on genesis; this shouldn't happen");

				NextNodes::<T>::insert(
					validator_id,
					NodeDetails {
						commission: permission.default_commission::<T>(),
						permission: *permission,
						own_exposure: Exposure::default(),
					},
				);

				Pallet::<T>::do_stake_nft(validator_id, validator_id, *nominal_value)
					.expect("couldn't stake nft on genesis; this shouldn't happen");
			}

			Pallet::<T>::rotate_storage();
		}
	}

	impl<T: Config> Pallet<T> {
		// TODO: There must be a way more efficient way to do this.
		fn rotate_storage() {
			CurrentTotalStake::<T>::set(NextTotalStake::<T>::get());

			// Clear CurrentNodes
			loop {
				let mut cursor: Option<SpVec<u8>> = None;
				cursor = CurrentNodes::<T>::clear(u32::MAX, cursor.as_deref()).maybe_cursor;

				if cursor.is_none() {
					break;
				}
			}

			for (node_id, details) in NextNodes::<T>::iter() {
				CurrentNodes::<T>::insert(node_id, details);
			}

			// Clear CurrentDelegations
			loop {
				let mut cursor: Option<SpVec<u8>> = None;
				cursor = CurrentDelegations::<T>::clear(u32::MAX, cursor.as_deref()).maybe_cursor;

				if cursor.is_none() {
					break;
				}
			}

			for (node_id, delegator_id, exposure) in NextDelegations::<T>::iter() {
				CurrentDelegations::<T>::insert(node_id, delegator_id, exposure);
			}
		}

		fn do_stake_currency(
			node_id: &ValidatorId<T>,
			staker_id: &DelegatorId<T>,
			value: T::Balance,
		) -> DispatchResult {
			NextTotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_add(value);
			});

			NextNodes::<T>::try_mutate(node_id, |x| {
				let Some(details) = x else {
					return Err(Error::<T>::InvalidTarget);
				};

				if node_id == staker_id {
					details.own_exposure.currency =
						details.own_exposure.currency.saturating_add(value);
				} else {
					NextDelegations::<T>::mutate_exists(node_id, staker_id, |x| {
						let mut staker_exposure = x.clone().unwrap_or_default();
						staker_exposure.currency = staker_exposure.currency.saturating_add(value);
						*x = Some(staker_exposure);
					});
				}

				Ok(())
			})?;

			Self::lock_currency(staker_id, value);

			Ok(())
		}

		fn do_unstake_currency(
			node_id: &ValidatorId<T>,
			staker_id: &DelegatorId<T>,
			value: T::Balance,
		) -> DispatchResult {
			NextTotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_sub(value);
			});

			NextNodes::<T>::mutate(node_id, |x| {
				if node_id == staker_id {
					let Some(details) = x.as_mut() else {
						return Err(Error::<T>::InvalidTarget);
					};

					if value > details.own_exposure.currency {
						return Err(Error::<T>::NotEnoughFunds);
					}

					details.own_exposure.currency =
						details.own_exposure.currency.saturating_sub(value);
				} else {
					if x.is_none() {
						return Err(Error::<T>::InvalidTarget);
					};

					if let Some(mut staker_exposure) = NextDelegations::<T>::get(node_id, staker_id)
					{
						if value > staker_exposure.currency {
							return Err(Error::<T>::NotEnoughFunds);
						}

						staker_exposure.currency = staker_exposure.currency.saturating_sub(value);
						NextDelegations::<T>::insert(node_id, staker_id, staker_exposure);
					} else {
						return Err(Error::<T>::InvalidTarget);
					};
				}

				Self::release_currency(staker_id, value);

				Ok(())
			})?;

			Ok(())
		}

		fn do_stake_nft(
			node_id: &ValidatorId<T>,
			staker_id: &DelegatorId<T>,
			value: T::Balance,
		) -> DispatchResult {
			NextTotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_add(value);
			});

			NextNodes::<T>::mutate(node_id, |x| {
				let Some(details) = x else {
					return Err(Error::<T>::InvalidTarget);
				};

				if node_id == staker_id {
					details.own_exposure.nft = details.own_exposure.nft.saturating_add(value);
				} else {
					NextDelegations::<T>::mutate_exists(node_id, staker_id, |x| {
						let mut staker_exposure = x.clone().unwrap_or_default();
						staker_exposure.nft = staker_exposure.nft.saturating_add(value);
						*x = Some(staker_exposure);
					});
				};

				Ok(())
			})?;

			Ok(())
		}

		fn do_unstake_nft(
			node_id: &ValidatorId<T>,
			staker_id: &DelegatorId<T>,
			value: T::Balance,
		) -> DispatchResult {
			NextTotalStake::<T>::mutate(|balance| {
				*balance = (*balance).saturating_sub(value);
			});

			NextNodes::<T>::mutate_exists(node_id, |x| {
				let Some(ref mut details) = x else {
					return Err(Error::<T>::InvalidTarget);
				};

				if node_id == staker_id {
					if value >= details.own_exposure.exposure() {
						*x = None;
					} else {
						details.own_exposure.nft = details.own_exposure.nft.saturating_sub(value);

						*x = Some(details.clone());
					}
				} else {
					NextDelegations::<T>::mutate(node_id, staker_id, |x| {
						let Some(staker_exposure) = x else {
							return Err(Error::<T>::InvalidTarget);
						};

						staker_exposure.nft = staker_exposure.nft.saturating_sub(value);
						Ok(())
					})?;
				}

				Ok(())
			})?;

			Ok(())
		}

		fn do_kick_nfts(node_id: &ValidatorId<T>, delegator_id: &DelegatorId<T>) -> DispatchResult {
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
			let Some(exposure) = NextDelegations::<T>::get(node_id, delegator_id) else {
				return Err(Error::<T>::InvalidTarget.into());
			};

			Self::do_kick_nfts(node_id, delegator_id)?;
			Self::do_unstake_currency(node_id, delegator_id, exposure.currency)
		}

		fn do_kick_all(node_id: &ValidatorId<T>) -> DispatchResult {
			for (delegator_id, exposure) in NextDelegations::<T>::iter_prefix(node_id) {
				Self::do_kick_nfts(node_id, &delegator_id)?;
				Self::do_unstake_currency(node_id, &delegator_id, exposure.currency)?;
			}

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

		fn validator_total_exposure(
			node_id: &ValidatorId<T>,
			details: &NodeDetails<T::Balance>,
		) -> T::Balance {
			let own = details.own_exposure.exposure();
			let delegated: T::Balance =
				NextDelegations::<T>::iter_prefix(node_id).map(|(_, x)| x.exposure()).sum();

			own + delegated
		}

		/// Rewards a node and it's delegators.
		/// Returns the amount of new currency created.
		fn reward_node(
			validator_account_id: &T::AccountId,
			node_details: &mut NodeDetails<T::Balance>,
			total_stake: u128,
			session_reward: u128,
		) -> PositiveImbalanceOf<T> {
			let total_node_exposure: u128 =
				Self::validator_total_exposure(validator_account_id, node_details).into();
			let own_node_exposure: u128 = node_details.own_exposure.exposure().into();
			let delegated_node_exposure: u128 = total_node_exposure - own_node_exposure;
			let commission_part_per_billion: u128 = node_details.commission.deconstruct().into();
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
				validator_account_id,
				total_validator_cut.into(),
			);

			Self::lock_currency(validator_account_id, imbalance.peek());

			node_details.own_exposure.currency =
				node_details.own_exposure.currency.saturating_add(imbalance.peek());

			Self::deposit_event(Event::<T>::Rewarded {
				stash: validator_account_id.clone(),
				amount: imbalance.peek(),
			});

			total_imbalance.subsume(imbalance);

			let delegator_cut = total_reward - total_validator_cut;

			for (delegator_id, mut delegator_exposure) in
				NextDelegations::<T>::iter_prefix(validator_account_id)
			{
				let reward_amount = multiply_by_rational_with_rounding(
					delegator_cut,
					delegator_exposure.exposure().into(),
					delegated_node_exposure,
					Rounding::NearestPrefDown,
				)
				.unwrap();

				let imbalance =
					<T as Config>::Currency::deposit_creating(&delegator_id, reward_amount.into());

				Self::deposit_event(Event::<T>::Rewarded {
					stash: delegator_id.clone(),
					amount: imbalance.peek(),
				});

				Self::lock_currency(&delegator_id, imbalance.peek());

				delegator_exposure.currency =
					delegator_exposure.currency.saturating_add(imbalance.peek());

				NextDelegations::<T>::insert(
					validator_account_id,
					delegator_id,
					delegator_exposure,
				);

				total_imbalance.subsume(imbalance);
			}

			total_imbalance
		}

		/// Slash a node and it's delegators.
		/// Mutates node details and returns the amount of currency to be removed from total_stake
		fn slash_node(
			validator_account_id: &T::AccountId,
			node_details: &mut NodeDetails<T::Balance>,
			slash_proportion: Perbill,
		) -> T::Balance {
			let mut total_stake_slash = T::Balance::zero();

			// Slash own permission and delegator nfts
			if !node_details.own_exposure.nft.is_zero() {
				let new_perission_stake =
					T::NftStakingHandler::slash(validator_account_id, slash_proportion)
						.expect("TODO");
				let new_delegator_nft_stake = T::NftDelegationHandler::slash(
					validator_account_id,
					validator_account_id,
					slash_proportion,
				)
				.unwrap_or(Zero::zero());

				let new_nft_stake = new_perission_stake.saturating_add(new_delegator_nft_stake);

				total_stake_slash = total_stake_slash
					.saturating_add(node_details.own_exposure.nft.saturating_sub(new_nft_stake));
				node_details.own_exposure.nft = new_nft_stake;
			}

			// Slash own currency
			if !node_details.own_exposure.currency.is_zero() {
				let slash_amount = slash_proportion * node_details.own_exposure.currency;

				if <T as pallet::Config>::Currency::can_slash(validator_account_id, slash_amount) {
					let (actual_slash, _) =
						<T as pallet::Config>::Currency::slash(validator_account_id, slash_amount);

					node_details.own_exposure.currency =
						node_details.own_exposure.currency.saturating_sub(actual_slash.peek());

					total_stake_slash = total_stake_slash.saturating_add(actual_slash.peek());

					Self::release_currency(validator_account_id, actual_slash.peek());
				}
			}

			// Slash delegators
			for (delegator_id, mut delegator_exposure) in
				NextDelegations::<T>::iter_prefix(validator_account_id)
			{
				// Slash delegator nfts
				if !delegator_exposure.nft.is_zero() {
					let new_nft_stake = T::NftDelegationHandler::slash(
						&delegator_id,
						validator_account_id,
						slash_proportion,
					)
					.expect("TODO");

					total_stake_slash = total_stake_slash
						.saturating_add(delegator_exposure.nft.saturating_sub(new_nft_stake));
					delegator_exposure.nft = new_nft_stake;
				}

				// Slash delegator currency
				if !delegator_exposure.currency.is_zero() {
					let slash_amount = slash_proportion * delegator_exposure.currency;

					if <T as pallet::Config>::Currency::can_slash(&delegator_id, slash_amount) {
						let (actual_slash, _) =
							<T as pallet::Config>::Currency::slash(&delegator_id, slash_amount);

						delegator_exposure.currency =
							delegator_exposure.currency.saturating_sub(actual_slash.peek());
						total_stake_slash = total_stake_slash.saturating_add(actual_slash.peek());

						Self::release_currency(&delegator_id, actual_slash.peek());
					}
				}

				NextDelegations::<T>::insert(
					validator_account_id,
					delegator_id,
					delegator_exposure,
				);
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
			let target_variant = NextNodes::<T>::get(&target).map(|d| d.permission);

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
			let nominal_value = T::NftDelegationHandler::unbind(&who, &item_id)?;

			Self::do_unstake_nft(&target, &who, nominal_value)?;

			Ok(())
		}

		#[pallet::call_index(2)]
		pub fn bind_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(NextNodes::<T>::get(&who).is_none(), Error::<T>::AlreadyBound);

			let (permission, nominal_value) = T::NftStakingHandler::bind(&who, &item_id)?;

			NextNodes::<T>::insert(
				&who,
				NodeDetails {
					commission: permission.default_commission::<T>(),
					permission,
					own_exposure: Exposure::default(),
				},
			);

			Self::do_stake_nft(&who, &who, nominal_value)?;

			Ok(())
		}

		#[pallet::call_index(3)]
		pub fn unbind_nft(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let details = NextNodes::<T>::get(&who).ok_or(Error::<T>::NotBound)?;

			if details.permission == PermissionType::DPoS {
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
			NextNodes::<T>::remove(&who);

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
			let details = NextNodes::<T>::get(&target).ok_or(Error::<T>::InvalidTarget)?;

			ensure!(details.permission == PermissionType::DPoS, Error::<T>::TargetNotDPoS);

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
			ensure!(
				commission >= T::MinimumCommissionAllowed::get(),
				Error::<T>::InvalidCommission
			);

			// FIXME: Check for permission type and only allow to set if not bound (or chilled?)
			NextNodes::<T>::mutate(who, |x| {
				let Some(details) = x.as_mut() else {
					return Err(Error::<T>::InvalidTarget);
				};

				if details.permission == PermissionType::DPoS {
					details.commission = commission;
				} else {
					return Err(Error::<T>::InvalidTarget);
				}
				Ok(())
			})?;

			Ok(())
		}

		#[pallet::call_index(10)]
		pub fn chill_validator(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			T::NftStakingHandler::chill(&who)?;

			Ok(())
		}

		#[pallet::call_index(11)]
		pub fn unchill_validator(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			T::NftStakingHandler::unchill(&who)?;

			Ok(())
		}
	}

	impl<T: Config> utils::traits::SessionHook for Pallet<T>
	where
		ValidatorId<T>: From<<T as pallet_session::Config>::ValidatorId>,
	{
		fn session_ended(_: u32) -> DispatchResult {
			let total_stake: u128 = NextTotalStake::<T>::get().into();

			if total_stake == 0 {
				return Ok(());
			}

			let active_validators = SessionPallet::<T>::validators();

			// FIXME: replace active validator len with total number of blocks created in session
			// TODO: set this to a more sensible value
			let session_reward = 100 * active_validators.len() as u128;
			let mut total_stake_slash: T::Balance = Zero::zero();
			let mut total_imbalance = PositiveImbalanceOf::<T>::zero();

			for validator_id in active_validators.into_iter().map(ValidatorId::<T>::from) {
				NextNodes::<T>::mutate_extant(&validator_id, |details| {
					if let Some(inverse_slash) = InverseSlashes::<T>::take(&validator_id) {
						// Apply slash, skip rewarding this node
						let slash_proportion = inverse_slash.left_from_one();

						total_stake_slash = total_stake_slash.saturating_add(Self::slash_node(
							&validator_id,
							details,
							slash_proportion,
						));
					} else {
						// No need to slash, reward node
						total_imbalance.subsume(Self::reward_node(
							&validator_id,
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

			NextTotalStake::<T>::mutate(|s| {
				*s = s.saturating_add(total_imbalance.peek());
				*s -= total_stake_slash;
			});

			T::Reward::on_unbalanced(total_imbalance);

			Ok(())
		}

		fn session_started(_: u32) -> DispatchResult {
			Self::rotate_storage();
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
			_session: SessionIndex,
			_disable_strategy: sp_staking::offence::DisableStrategy,
		) -> frame_support::weights::Weight {
			for (o, slash) in offenders.iter().zip(slash_fraction.iter()) {
				InverseSlashes::<T>::mutate_exists(o.offender.0.clone(), |s| match *s {
					None => *s = Some(slash.left_from_one()),
					Some(ref mut acc) => *acc = *acc * slash.left_from_one(),
				});
			}

			Weight::default()
		}
	}
}
