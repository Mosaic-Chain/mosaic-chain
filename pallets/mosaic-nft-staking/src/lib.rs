#![cfg_attr(not(feature = "std"), no_std)]
pub use impls::{SelectableValidators, SlashableValidators};
/// Mosaic's very own staking pallet
/// Note: functions might have an (immediate) or a (staged) qualifier to signify when the change is going to occur.
/// # Missing features:
///   - Tests and benchmarks
pub use pallet::*;
pub use types::PermissionType;

mod impls;
mod reward;
mod slash;
mod types;

use core::num::NonZeroU32;

use codec::Codec;
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, ExistenceRequirement, LockableCurrency, OnUnbalanced, WithdrawReasons},
	Twox64Concat,
};
use frame_system::pallet_prelude::*;
use pallet_session::{Config as SessionConfig, Pallet as SessionPallet};
use sp_std::vec::Vec as SpVec;

use sp_runtime::{
	traits::{Convert, Zero},
	FixedPointOperand, Perbill, Saturating,
};

use sp_staking::SessionIndex;

use types::{
	ChillReason, Contract, KickReason, PositiveImbalanceOf, Staging, Stake, TotalValidatorStake,
	ValidatorDetails, ValidatorState,
};
use utils::traits::{NftDelegation, NftStaking};

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + SessionConfig + pallet_offences::Config {
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
			+ Saturating;

		type ItemId: Parameter;

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

		type NftDelegationHandler: NftDelegation<
			Self::AccountId,
			Self::Balance,
			Self::ItemId,
			Self::AccountId,
		>;

		type OffenderToValidatorId: Convert<Self::IdentificationTuple, Self::AccountId>;

		/// Period after which a chilled validator is considered slacking
		type SlackingPeriod: Get<u32>;
		/// A percent under which a validator is disqualified
		type NominalValueThreshold: Get<Perbill>;
		type MinimumStakingPeriod: Get<NonZeroU32>;
		type MinimumCommissionRate: Get<Perbill>;
		type MinimumStakingAmount: Get<Self::Balance>;
		type MaximumStakePercentage: Get<Perbill>;
		type MaximumContractsPerValidator: Get<u32>;

		// Amount of **Tiles** to be rewarded in a given session.
		type SessionReward: Get<u128>;
		type OnReward: OnUnbalanced<PositiveImbalanceOf<Self>>;

		#[pallet::constant]
		type PalletId: Get<frame_support::PalletId>;
	}

	#[pallet::storage]
	pub type TotalValidatorStakes<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		Staging<TotalValidatorStake<T::Balance>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type Validators<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, ValidatorDetails, OptionQuery>;

	#[pallet::storage]
	pub type Contracts<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		T::AccountId,
		Staging<Contract<T::Balance, T::ItemId>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type LockedCurrency<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::Balance>;

	#[pallet::storage]
	pub type UnlockingCurrency<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::Balance>;

	#[pallet::storage]
	pub type UnlockingDelegatorNfts<T: Config> =
		StorageMap<_, Twox64Concat, T::ItemId, T::AccountId>;

	#[pallet::storage]
	pub type InverseSlashes<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Perbill>;

	#[pallet::storage]
	pub type ValidatorStates<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ValidatorState>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ContractReward {
			validator: T::AccountId,
			staker: T::AccountId,
			reward: T::Balance,
			commission: T::Balance,
		},

		Slash {
			offender: T::AccountId,
			staker: T::AccountId,
			currency: T::Balance,
			delegator_nfts: SpVec<(T::ItemId, T::Balance)>,
			permission_nft: Option<T::Balance>,
		},

		ValidatorChilled {
			validator: T::AccountId,
			reason: ChillReason,
		},

		ValidatorUnchilled(T::AccountId),

		ValidatorBound(T::AccountId),

		ValidatorUnbound(T::AccountId),

		StakerKicked {
			validator: T::AccountId,
			staker: T::AccountId,
			reason: KickReason,
		},

		DelegationEnabled(T::AccountId),

		DelegationDisabled(T::AccountId),

		CurrencyStaked {
			validator: T::AccountId,
			staker: T::AccountId,
			amount: T::Balance,
		},

		CurrencyUnstaked {
			validator: T::AccountId,
			staker: T::AccountId,
			amount: T::Balance,
		},

		NftDelegated {
			validator: T::AccountId,
			staker: T::AccountId,
			item_id: T::ItemId,
		},

		NftUndelegated {
			validator: T::AccountId,
			staker: T::AccountId,
			item_id: T::ItemId,
		},

		MinimumStakingPeriodChanged {
			validator: T::AccountId,
			new_period: SessionIndex,
		},

		CommissionChanged {
			validator: T::AccountId,
			new_commission: Perbill,
		},

		PermissionNftTopup {
			validator: T::AccountId,
			item: T::ItemId,
			cost: T::Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		AlreadyBound,
		NotBound,
		ValidatorAlreadySelected,
		BindingContractExists,
		TargetNotDPoS,
		CallerNotDPoS,
		InvalidCaller,
		InvalidTarget,
		AlreadyAcceptsDelegations,
		AlreadyDeniesDelegations,
		TargetDeniesDelegations,
		CallerIsChilled,
		CallerIsNotChilled,
		TargetIsChilled,
		TargetIsNotChilled,
		TooSmallStake,
		InsufficientFunds,
		NftExpiresEarly,
		LowNominalValue,
		TooSmallUnstake,
		NoContract,
		EarlyUnstake,
		InvalidStakingPeriod,
		InvalidCommission,
		EarlyKick,
		ValidatorDisqualified,
		TopupWrongOwner,
		SlippageExceeded,
		OverdominantStake,
		TooManyNftDelegatedToContract,
		ContractLimitReached,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_staking_validators: SpVec<(T::AccountId, PermissionType, T::Balance)>,
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
				assert!(!Validators::<T>::contains_key(validator_id));

				let item_id = T::NftStakingHandler::mint(validator_id, permission, nominal_value)
					.expect("couldn't mint new nft on genesis; this shouldn't happen");

				T::NftStakingHandler::bind(validator_id, &item_id)
					.expect("could't bind permission nft on genesis; this shouldn't happen");

				let validator = match permission {
					PermissionType::PoS => ValidatorDetails::PoS,
					PermissionType::DPoS => ValidatorDetails::DPoS {
						commission: T::MinimumCommissionRate::get(),
						min_staking_period: T::MinimumStakingPeriod::get().into(),
						accept_delegations: true,
					},
				};

				Validators::<T>::insert(validator_id, validator);
				ValidatorStates::<T>::insert(validator_id, ValidatorState::Normal);

				let self_contract = Contract {
					stake: Stake { permission_nft: Some(*nominal_value), ..Default::default() },
					commission: Perbill::from_percent(100),
					min_staking_period_end: T::MinimumStakingPeriod::get().into(),
				};

				Contracts::<T>::insert(validator_id, validator_id, Staging::new(self_contract));

				TotalValidatorStakes::<T>::insert(
					validator_id,
					Staging::new(TotalValidatorStake {
						total_stake: *nominal_value,
						contract_count: 1,
					}),
				);
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub(crate) fn drafted_validators() -> impl Iterator<Item = <T as SessionConfig>::ValidatorId>
		{
			// Currently active + queued validators
			SessionPallet::<T>::queued_keys()
				.into_iter()
				.map(|(v, _)| v)
				.chain(SessionPallet::<T>::validators())
		}

		fn ensure_not_overdominant(validator: &T::AccountId) -> DispatchResult {
			let total_stake = TotalValidatorStakes::<T>::get(validator)
				.current()
				.ok_or(Error::<T>::NotBound)?
				.total_stake;

			let ratio = Perbill::from_rational(
				total_stake,
				<T as pallet::Config>::Currency::total_issuance(),
			);

			ensure!(ratio <= T::MaximumStakePercentage::get(), Error::<T>::OverdominantStake);
			Ok(())
		}

		/// PANIC: account does not exist in ValidatorStates
		fn is_chilled(account: &T::AccountId) -> bool {
			let state = ValidatorStates::<T>::get(account).unwrap();
			matches!(state, ValidatorState::Chilled(_))
		}

		/// PANIC: account does not exist in ValidatorStates
		fn is_slacking(account: &T::AccountId) -> bool {
			let state = ValidatorStates::<T>::get(account).unwrap();
			match state {
				ValidatorState::Chilled(session_index) => {
					let now = SessionPallet::<T>::current_index();
					now - session_index > T::SlackingPeriod::get()
				},
				_ => false,
			}
		}

		/// Returns a chilled validator state and publishes an event of the chill.
		pub(crate) fn chill_state(validator: T::AccountId, reason: ChillReason) -> ValidatorState {
			Self::deposit_event(Event::<T>::ValidatorChilled { validator, reason });
			ValidatorState::Chilled(SessionPallet::<T>::current_index())
		}

		/// Commits all storage values. Removes values where there is neither a staged nor a committed value.
		/// We also remove empty contracts
		pub(crate) fn commit_storage() {
			Contracts::<T>::translate(
				|validator, _, mut s: Staging<Contract<T::Balance, T::ItemId>>| {
					if let Some(contract) = s.current() {
						if contract.stake.is_empty() {
							Self::relinquish_contract(&validator);
							None
						} else {
							s.commit();
							Some(s)
						}
					} else {
						None
					}
				},
			);

			// Must be after relinquish_contract calls
			TotalValidatorStakes::<T>::translate_values(
				|mut s: Staging<TotalValidatorStake<_>>| {
					s.exists().then(|| {
						s.commit();
						s
					})
				},
			);

			for (staker, amount) in UnlockingCurrency::<T>::drain() {
				Self::unlock_currency(&staker, amount);
			}

			for (item_id, staker) in UnlockingDelegatorNfts::<T>::drain() {
				if let Err(e) = T::NftDelegationHandler::unbind(&staker, &item_id) {
					log::error!("failed to unbind delegator nft({item_id:?}) of {staker:?}: {e:?}");
				}
			}
		}

		/// Grows total stake by amount
		/// (staged)
		pub(crate) fn grow_total_validator_stake_by(validator: &T::AccountId, value: T::Balance) {
			if value.is_zero() {
				return;
			}

			TotalValidatorStakes::<T>::mutate(validator, |s| {
				if let Some(TotalValidatorStake { total_stake: current, .. }) =
					s.ensure_staging_mut()
				{
					*current = current.saturating_add(value);
				} else {
					s.stage(TotalValidatorStake { total_stake: value, ..Default::default() });
				}
			});
		}

		/// Shrinks total stake by amount
		/// (staged)
		pub(crate) fn shrink_total_validator_stake_by(validator: &T::AccountId, value: T::Balance) {
			if value.is_zero() {
				return;
			}

			TotalValidatorStakes::<T>::mutate(validator, |s| {
				if let Some(TotalValidatorStake { total_stake: current, .. }) =
					s.ensure_staging_mut()
				{
					*current = current.saturating_sub(value);
				}
			});
		}

		// Requests the creation of a new contract for a given validator
		// Fails if contract limit is already reached for the validator
		fn request_new_contract(validator: &T::AccountId) -> DispatchResult {
			TotalValidatorStakes::<T>::mutate(validator, |s| {
				if let Some(TotalValidatorStake { contract_count: current, .. }) =
					s.ensure_staging_mut()
				{
					if *current < T::MaximumContractsPerValidator::get() {
						*current += 1;
					} else {
						return Err(Error::<T>::ContractLimitReached.into());
					}
				} else {
					s.stage(TotalValidatorStake { contract_count: 1, ..Default::default() });
				}

				Ok(())
			})
		}

		// Make a promise to destroy a contract for the given validator
		// This decreases the number of contracts associated with the validator
		fn relinquish_contract(validator: &T::AccountId) {
			TotalValidatorStakes::<T>::mutate(validator, |s| {
				if let Some(TotalValidatorStake { contract_count: current, .. }) =
					s.ensure_staging_mut()
				{
					*current = current.saturating_sub(1);
				}
			});
		}

		/// Updates currency lock to a new value
		/// (immediate)
		fn update_lock(account_id: &T::AccountId, amount: T::Balance) {
			<T as pallet::Config>::Currency::set_lock(
				<T as Config>::PalletId::get().0,
				account_id,
				amount,
				WithdrawReasons::all(),
			);

			LockedCurrency::<T>::insert(account_id, amount);
		}

		/// Releases all locked currency and destroys lock
		/// (immediate)
		fn release_lock(account_id: &T::AccountId) {
			<T as pallet::Config>::Currency::remove_lock(
				<T as Config>::PalletId::get().0,
				account_id,
			);

			LockedCurrency::<T>::remove(account_id);
		}

		/// Adds the provided amount to the account's lock.
		/// Possible side-effects: creates an entry in LockedCurrency, locks currency
		/// (immediate)
		pub(crate) fn lock_currency(
			account_id: &T::AccountId,
			amount: T::Balance,
		) -> DispatchResult {
			if amount.is_zero() {
				return Ok(());
			}

			let locked = LockedCurrency::<T>::get(account_id)
				.unwrap_or(Zero::zero())
				.saturating_add(amount);

			let free_balance = <T as pallet::Config>::Currency::free_balance(account_id);
			ensure!(locked <= free_balance, Error::<T>::InsufficientFunds);

			Self::update_lock(account_id, locked);
			Ok(())
		}

		/// Removes the provided amount from the account's lock.
		/// (immediate)
		pub(crate) fn unlock_currency(account_id: &T::AccountId, amount: T::Balance) {
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

		/// Removes the provided amount from the account's lock.
		/// (staged)
		fn stage_unlock_currency(account_id: &T::AccountId, amount: T::Balance) {
			if amount.is_zero() {
				return;
			}

			UnlockingCurrency::<T>::mutate_exists(account_id, |unlocking| {
				*unlocking = Some(unlocking.unwrap_or_default().saturating_add(amount));
			});
		}

		/// Generic currency staking
		/// Locks currency (immediate)
		/// Increases stake (staged)
		fn do_stake_currency(
			validator: &T::AccountId,
			validator_details: ValidatorDetails,
			staker: &T::AccountId,
			amount: T::Balance,
		) -> DispatchResult {
			ensure!(!Self::is_chilled(validator), Error::<T>::TargetIsChilled);

			ensure!(amount >= T::MinimumStakingAmount::get(), Error::<T>::TooSmallStake);

			Self::lock_currency(staker, amount)?;
			Self::grow_total_validator_stake_by(validator, amount);
			Self::ensure_not_overdominant(validator)?;

			let min_staking_period_end =
				SessionPallet::<T>::current_index() + validator_details.min_staking_period::<T>();

			Contracts::<T>::mutate(validator, staker, |s| {
				let stake = match s.current() {
					Some(Contract { stake: old, .. }) => {
						Stake { currency: old.currency.saturating_add(amount), ..old.clone() }
					},

					None => {
						Self::request_new_contract(validator)?;
						Stake { currency: amount, ..Default::default() }
					},
				};

				let contract = Contract {
					stake,
					commission: validator_details.commission(),
					min_staking_period_end,
				};

				s.stage(contract);

				Ok(())
			})
		}

		fn do_stake_nft(
			validator: &T::AccountId,
			validator_details: ValidatorDetails,
			staker: &T::AccountId,
			item_id: &T::ItemId,
		) -> DispatchResult {
			ensure!(!Self::is_chilled(validator), Error::<T>::TargetIsChilled);

			let (expiry, nominal_value) =
				T::NftDelegationHandler::bind(staker, item_id, validator.clone())?;

			ensure!(nominal_value >= T::MinimumStakingAmount::get(), Error::<T>::LowNominalValue);

			let min_staking_period_end =
				SessionPallet::<T>::current_index() + validator_details.min_staking_period::<T>();

			// Note: nft is still considered valid until the end of the session it expires on.
			ensure!(expiry >= min_staking_period_end, Error::<T>::NftExpiresEarly);

			Self::grow_total_validator_stake_by(validator, nominal_value);
			Self::ensure_not_overdominant(validator)?;

			Contracts::<T>::mutate(validator, staker, |s| {
				let stake = match s.current() {
					Some(Contract { stake: old, .. }) => {
						let mut new = old.clone();
						new.delegated_nfts
							.try_push((item_id.clone(), nominal_value))
							.map(|()| new)
							.map_err(|_| Error::<T>::TooManyNftDelegatedToContract)
					},

					None => {
						Self::request_new_contract(validator)?;
						let mut delegated_nfts = BoundedVec::new();
						// Force push is safe, as (1) its truncating and does not panic,
						// (2) we just created the vec and the bound is non-zero
						delegated_nfts.force_push((item_id.clone(), nominal_value));

						Ok(Stake { delegated_nfts, ..Default::default() })
					},
				}?;

				let contract = Contract {
					stake,
					commission: validator_details.commission(),
					min_staking_period_end,
				};

				s.stage(contract);
				Ok(())
			})
		}

		fn do_unstake_currency(
			validator: &T::AccountId,
			staker: &T::AccountId,
			amount: T::Balance,
		) -> DispatchResult {
			Contracts::<T>::mutate(validator, staker, |s| {
				let Some(contract) = s.ensure_staging_mut() else {
					return Err(Error::<T>::NoContract.into());
				};

				let session = SessionPallet::<T>::current_index();
				ensure!(
					session >= contract.min_staking_period_end
						|| (validator != staker && Self::is_slacking(validator)),
					Error::<T>::EarlyUnstake
				);

				let minimum_staking_amount = T::MinimumStakingAmount::get();

				ensure!(
					amount >= contract.stake.currency.min(minimum_staking_amount),
					Error::<T>::TooSmallUnstake
				);
				ensure!(amount <= contract.stake.currency, Error::<T>::InsufficientFunds);

				contract.stake.currency = contract.stake.currency.saturating_sub(amount);

				Self::shrink_total_validator_stake_by(validator, amount);
				Self::stage_unlock_currency(staker, amount);

				Ok(())
			})
		}

		fn do_unstake_nft(
			validator: &T::AccountId,
			staker: &T::AccountId,
			item_id: &T::ItemId,
		) -> DispatchResult {
			Contracts::<T>::mutate(validator, staker, |s| {
				let Some(contract) = s.ensure_staging_mut() else {
					return Err(Error::<T>::NoContract.into());
				};

				let session = SessionPallet::<T>::current_index();
				ensure!(
					session >= contract.min_staking_period_end
						|| (validator != staker && Self::is_slacking(validator)),
					Error::<T>::EarlyUnstake
				);

				let nominal_value = T::NftDelegationHandler::nominal_value(item_id)?;
				UnlockingDelegatorNfts::<T>::insert(item_id.clone(), staker.clone());

				if let Some(index) =
					contract.stake.delegated_nfts.iter().position(|(x, _)| x == item_id)
				{
					contract.stake.delegated_nfts.remove(index);
				} else {
					return Err(Error::<T>::NotBound.into());
				}

				Self::shrink_total_validator_stake_by(validator, nominal_value);

				Ok(())
			})
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		pub fn bind_validator(origin: OriginFor<T>, item_id: T::ItemId) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(!Validators::<T>::contains_key(&caller), Error::<T>::AlreadyBound);

			let (permission, nominal_value) = T::NftStakingHandler::bind(&caller, &item_id)?;

			ensure!(
				T::NftStakingHandler::nominal_factor_of(&caller)? > T::NominalValueThreshold::get(),
				Error::<T>::ValidatorDisqualified
			);

			let details = match permission {
				PermissionType::PoS => ValidatorDetails::PoS,
				PermissionType::DPoS => ValidatorDetails::DPoS {
					commission: T::MinimumCommissionRate::get(),
					min_staking_period: T::MinimumStakingPeriod::get().into(),
					accept_delegations: true,
				},
			};

			Validators::<T>::insert(&caller, details);
			ValidatorStates::<T>::insert(&caller, ValidatorState::Normal);
			Self::grow_total_validator_stake_by(&caller, nominal_value);

			Self::request_new_contract(&caller)?;
			let self_contract = Contract {
				stake: Stake { permission_nft: Some(nominal_value), ..Default::default() },
				commission: Perbill::from_percent(100),
				min_staking_period_end: SessionPallet::<T>::current_index()
					+ u32::from(T::MinimumStakingPeriod::get()),
			};

			Contracts::<T>::insert(&caller, &caller, Staging::new_staged(self_contract));

			Self::deposit_event(Event::<T>::ValidatorBound(caller));

			Ok(())
		}

		#[pallet::call_index(1)]
		pub fn unbind_validator(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Validators::<T>::contains_key(&caller), Error::<T>::NotBound);
			ensure!(Self::is_chilled(&caller), Error::<T>::CallerIsNotChilled);

			let _ = T::NftStakingHandler::unbind(&caller)?;

			let validator_id = T::ValidatorIdOf::convert(caller.clone())
				.expect("caller address can always be converted to validator id");

			ensure!(
				Self::drafted_validators().all(|id| id != validator_id),
				Error::<T>::ValidatorAlreadySelected
			);

			let session = SessionPallet::<T>::current_index();

			let contracts =
				Contracts::<T>::drain_prefix(&caller).filter_map(|(contractee, contract)| {
					contract.current().map(|c| (contractee, c.clone()))
				});

			// Note: this includes the self-contract as well
			for (contractee, contract) in contracts {
				// Note: since we are terminating the contract immediately we have to check for equality too
				if contract.min_staking_period_end >= session {
					return Err(Error::<T>::BindingContractExists.into());
				}

				// We immediately release currency and NFTs so delegators needn't wait until the end of session.
				// Note: they would not receive rewards for this contract this session.
				Self::unlock_currency(&contractee, contract.stake.currency);
				for (item_id, _) in &contract.stake.delegated_nfts {
					T::NftDelegationHandler::unbind(&contractee, item_id)?;
				}

				Self::deposit_event(Event::<T>::StakerKicked {
					validator: caller.clone(),
					staker: contractee,
					reason: KickReason::Unbind,
				});
			}

			// Note: we can remove immediately as no reward would be given.
			// Caveat: slashes in current session? We can forgive them, right?
			Validators::<T>::remove(&caller);
			ValidatorStates::<T>::remove(&caller);
			TotalValidatorStakes::<T>::remove(&caller);
			InverseSlashes::<T>::remove(&caller);

			Self::deposit_event(Event::<T>::ValidatorUnbound(caller));

			Ok(())
		}

		#[pallet::call_index(2)]
		pub fn enable_delegations(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			Validators::<T>::mutate(&caller, |v: &mut Option<_>| {
				let details = v.as_mut().ok_or(Error::<T>::NotBound)?;

				if let ValidatorDetails::DPoS { accept_delegations, .. } = details {
					ensure!(!*accept_delegations, Error::<T>::AlreadyAcceptsDelegations);

					*accept_delegations = true;
				} else {
					return Err(Error::<T>::CallerNotDPoS.into());
				};

				Self::deposit_event(Event::<T>::DelegationEnabled(caller.clone()));

				Ok(())
			})
		}

		#[pallet::call_index(3)]
		pub fn disable_delegations(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			Validators::<T>::mutate(&caller, |v: &mut Option<_>| {
				let details = v.as_mut().ok_or(Error::<T>::NotBound)?;

				if let ValidatorDetails::DPoS { accept_delegations, .. } = details {
					ensure!(*accept_delegations, Error::<T>::AlreadyDeniesDelegations);

					*accept_delegations = false;
				} else {
					return Err(Error::<T>::CallerNotDPoS.into());
				};

				Self::deposit_event(Event::<T>::DelegationDisabled(caller.clone()));

				Ok(())
			})
		}

		#[pallet::call_index(4)]
		pub fn chill_validator(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ValidatorStates::<T>::mutate(&caller, |validator_state| {
				let validator_state = validator_state.as_mut().ok_or(Error::<T>::NotBound)?;
				if let ValidatorState::Chilled(_) = validator_state {
					Err(Error::<T>::CallerIsChilled.into())
				} else {
					*validator_state = Self::chill_state(caller.clone(), ChillReason::Manual);
					Ok(())
				}
			})
		}

		#[pallet::call_index(5)]
		pub fn unchill_validator(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ValidatorStates::<T>::mutate(&caller, |validator_state| {
				let validator_state = validator_state.as_mut().ok_or(Error::<T>::NotBound)?;
				match validator_state {
					ValidatorState::Chilled(_) => {
						*validator_state = ValidatorState::Normal;

						Self::deposit_event(Event::<T>::ValidatorUnchilled(caller.clone()));
						Ok(())
					},
					_ => Err(Error::<T>::CallerIsNotChilled),
				}
			})?;

			ensure!(
				T::NftStakingHandler::nominal_factor_of(&caller)? > T::NominalValueThreshold::get(),
				Error::<T>::ValidatorDisqualified
			);

			Ok(())
		}

		#[pallet::call_index(6)]
		pub fn self_stake_currency(origin: OriginFor<T>, amount: T::Balance) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let details = Validators::<T>::get(&caller).ok_or(Error::<T>::NotBound)?;

			Self::do_stake_currency(&caller, details, &caller, amount)?;

			Self::deposit_event(Event::<T>::CurrencyStaked {
				validator: caller.clone(),
				staker: caller,
				amount,
			});

			Ok(())
		}

		#[pallet::call_index(7)]
		pub fn self_stake_nft(origin: OriginFor<T>, item_id: T::ItemId) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let details = Validators::<T>::get(&caller).ok_or(Error::<T>::NotBound)?;

			Self::do_stake_nft(&caller, details, &caller, &item_id)?;

			Self::deposit_event(Event::<T>::NftDelegated {
				validator: caller.clone(),
				staker: caller,
				item_id,
			});

			Ok(())
		}

		#[pallet::call_index(8)]
		pub fn self_unstake_currency(origin: OriginFor<T>, amount: T::Balance) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Validators::<T>::contains_key(&caller), Error::<T>::NotBound);

			Self::do_unstake_currency(&caller, &caller, amount)?;

			Self::deposit_event(Event::<T>::CurrencyUnstaked {
				validator: caller.clone(),
				staker: caller,
				amount,
			});

			Ok(())
		}

		#[pallet::call_index(9)]
		pub fn self_unstake_nft(origin: OriginFor<T>, item_id: T::ItemId) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Validators::<T>::contains_key(&caller), Error::<T>::NotBound);

			Self::do_unstake_nft(&caller, &caller, &item_id)?;

			Self::deposit_event(Event::<T>::NftUndelegated {
				validator: caller.clone(),
				staker: caller,
				item_id,
			});

			Ok(())
		}

		#[pallet::call_index(10)]
		pub fn delegate_currency(
			origin: OriginFor<T>,
			amount: T::Balance,
			target: T::AccountId,
			observed_staking_period: u32,
			observed_commission: Perbill,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidTarget);

			let details = Validators::<T>::get(&target).ok_or(Error::<T>::NotBound)?;

			let ValidatorDetails::DPoS { accept_delegations, min_staking_period, commission } =
				details
			else {
				return Err(Error::<T>::TargetNotDPoS.into());
			};

			ensure!(accept_delegations, Error::<T>::TargetDeniesDelegations);
			ensure!(
				min_staking_period == observed_staking_period && commission == observed_commission,
				Error::<T>::SlippageExceeded
			);

			Self::do_stake_currency(&target, details, &caller, amount)?;

			Self::deposit_event(Event::<T>::CurrencyStaked {
				validator: target,
				staker: caller,
				amount,
			});

			Ok(())
		}

		#[pallet::call_index(11)]
		pub fn delegate_nft(
			origin: OriginFor<T>,
			item_id: T::ItemId,
			target: T::AccountId,
			observed_staking_period: u32,
			observed_commission: Perbill,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidTarget);

			let details = Validators::<T>::get(&target).ok_or(Error::<T>::NotBound)?;

			let ValidatorDetails::DPoS { accept_delegations, min_staking_period, commission } =
				details
			else {
				return Err(Error::<T>::TargetNotDPoS.into());
			};

			ensure!(accept_delegations, Error::<T>::TargetDeniesDelegations);
			ensure!(
				min_staking_period == observed_staking_period && commission == observed_commission,
				Error::<T>::SlippageExceeded
			);

			Self::do_stake_nft(&target, details, &caller, &item_id)?;

			Self::deposit_event(Event::<T>::NftDelegated {
				validator: target,
				staker: caller,
				item_id,
			});

			Ok(())
		}

		#[pallet::call_index(12)]
		pub fn undelegate_currency(
			origin: OriginFor<T>,
			amount: T::Balance,
			target: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidTarget);

			let details = Validators::<T>::get(&target).ok_or(Error::<T>::NotBound)?;
			ensure!(details.permission() == PermissionType::DPoS, Error::<T>::TargetNotDPoS);

			Self::do_unstake_currency(&target, &caller, amount)?;

			Self::deposit_event(Event::<T>::CurrencyUnstaked {
				validator: target,
				staker: caller,
				amount,
			});

			Ok(())
		}

		#[pallet::call_index(13)]
		pub fn undelegate_nft(
			origin: OriginFor<T>,
			item_id: T::ItemId,
			target: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidTarget);

			let permission = Validators::<T>::get(&target)
				.map(|details| details.permission())
				.ok_or(Error::<T>::NotBound)?;

			ensure!(permission == PermissionType::DPoS, Error::<T>::TargetNotDPoS);

			Self::do_unstake_nft(&target, &caller, &item_id)?;

			Self::deposit_event(Event::<T>::NftUndelegated {
				validator: target,
				staker: caller,
				item_id,
			});

			Ok(())
		}

		#[pallet::call_index(14)]
		pub fn set_minium_staking_period(
			origin: OriginFor<T>,
			new_min_period: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			Validators::<T>::mutate(&caller, |v: &mut Option<_>| {
				let details = v.as_mut().ok_or(Error::<T>::NotBound)?;

				ensure!(!Self::is_chilled(&caller), Error::<T>::CallerIsChilled);

				let ValidatorDetails::DPoS { min_staking_period, .. } = details else {
					return Err(Error::<T>::CallerNotDPoS.into());
				};

				ensure!(
					new_min_period >= T::MinimumStakingPeriod::get().into(),
					Error::<T>::InvalidStakingPeriod
				);

				*min_staking_period = new_min_period;

				Self::deposit_event(Event::<T>::MinimumStakingPeriodChanged {
					validator: caller.clone(),
					new_period: new_min_period,
				});

				Ok(())
			})
		}

		#[pallet::call_index(15)]
		pub fn set_commission(origin: OriginFor<T>, new_commission: Perbill) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			Validators::<T>::mutate(&caller, |v: &mut Option<_>| {
				let details = v.as_mut().ok_or(Error::<T>::NotBound)?;

				ensure!(!Self::is_chilled(&caller), Error::<T>::CallerIsChilled);

				let ValidatorDetails::DPoS { commission, .. } = details else {
					return Err(Error::<T>::CallerNotDPoS.into());
				};

				ensure!(
					new_commission >= T::MinimumCommissionRate::get(),
					Error::<T>::InvalidCommission
				);

				*commission = new_commission;

				Self::deposit_event(Event::<T>::CommissionChanged {
					validator: caller.clone(),
					new_commission,
				});

				Ok(())
			})
		}

		#[pallet::call_index(16)]
		pub fn kick(origin: OriginFor<T>, target: T::AccountId) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidCaller);

			let details = Validators::<T>::get(&caller).ok_or(Error::<T>::NotBound)?;

			ensure!(details.permission() == PermissionType::DPoS, Error::<T>::CallerNotDPoS);

			ensure!(!Self::is_chilled(&caller), Error::<T>::CallerIsChilled);

			Contracts::<T>::mutate(&caller, &target, |s| {
				let contract = s.ensure_staging_mut().ok_or(Error::<T>::NoContract)?;

				if contract.min_staking_period_end > SessionPallet::<T>::current_index() {
					return Err(Error::<T>::EarlyKick.into());
				}

				Self::stage_unlock_currency(&target, contract.stake.currency);
				for (item_id, _) in &contract.stake.delegated_nfts {
					UnlockingDelegatorNfts::<T>::insert(item_id.clone(), target.clone());
				}

				// this is a staged action, so in the current session this delegator's stake is still included.
				Self::shrink_total_validator_stake_by(&caller, contract.stake.total());

				// Note: contracts with empty stake are removed in next session,
				// but the delegator can always just restake even in this session
				contract.stake = Stake::default();

				Self::deposit_event(Event::<T>::StakerKicked {
					validator: caller.clone(),
					staker: target.clone(),
					reason: KickReason::Manual,
				});

				Ok(())
			})
		}

		#[pallet::call_index(17)]
		pub fn topup(
			origin: OriginFor<T>,
			item_id: T::ItemId,
			allowed_amount: T::Balance,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(T::NftStakingHandler::owner(&item_id)? == caller, Error::<T>::TopupWrongOwner);
			let nominal_value = T::NftStakingHandler::nominal_value(&item_id)?;
			let issued_nominal_value = T::NftStakingHandler::issued_nominal_value(&item_id)?;

			let imbalance = issued_nominal_value - nominal_value;

			ensure!(imbalance <= allowed_amount, Error::<T>::SlippageExceeded);

			Contracts::<T>::mutate(&caller, &caller, |s| {
				let Some(contract) = s.ensure_staging_mut() else {
					// No self-contract -> Not bound
					return;
				};

				contract.stake.permission_nft = Some(issued_nominal_value);

				Self::grow_total_validator_stake_by(&caller, imbalance);

				ValidatorStates::<T>::mutate_extant(&caller, |vstate| {
					if let ValidatorState::Faulted = vstate {
						*vstate = ValidatorState::Normal;
					}
				});
			});

			let withdrawn = <T as pallet::Config>::Currency::withdraw(
				&caller,
				imbalance,
				WithdrawReasons::all(),
				ExistenceRequirement::KeepAlive,
			)?;

			// Modifies total issuance
			drop(withdrawn);

			T::NftStakingHandler::set_nominal_value(&item_id, issued_nominal_value)?;

			Self::deposit_event(Event::<T>::PermissionNftTopup {
				validator: caller,
				item: item_id,
				cost: imbalance,
			});

			Ok(())
		}
	}
}
