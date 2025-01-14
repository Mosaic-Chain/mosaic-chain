#![cfg_attr(not(feature = "std"), no_std)]
// Expect lints caused by procmacros
#![expect(clippy::manual_inspect)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;

mod impls;
mod session_ending;
mod types;

use sdk::{frame_support, frame_system, pallet_offences, pallet_session, sp_runtime, sp_std};

use core::num::NonZeroU32;

use codec::Codec;
use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{Balanced, BalancedHold, Credit, Inspect, InspectHold, MutateHold},
		tokens::{Fortitude, Precision, Preservation},
		OnUnbalanced,
	},
	weights::WeightMeter,
	Twox64Concat,
};
use frame_system::pallet_prelude::*;
use pallet_session::{Config as SessionConfig, Pallet as SessionPallet};
use sp_std::{vec, vec::Vec as SpVec};

use sp_runtime::{
	traits::{Convert, Zero},
	FixedPointOperand, Perbill, Saturating,
};

use session_ending::{Progress, SessionEnding};
use types::{
	ChillReason, Contract, KickReason, StagingLayer, Stake, StorageLayer, TotalValidatorStake,
	ValidatorDetails, ValidatorState,
};

use utils::{
	traits::{NftDelegation, NftPermission, StakingHooks},
	SessionIndex,
};

pub use impls::{SelectableValidators, SlashableValidators};
/// Mosaic's very own staking pallet
/// Note: functions might have an (immediate) or a (staged) qualifier to signify when the change is going to occur.
pub use pallet::*;
pub use types::PermissionType;
pub use weights::WeightInfo;

#[cfg(feature = "runtime-benchmarks")]
pub use benchmarking::BenchmarkHelper;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: sdk::frame_system::Config + SessionConfig + pallet_offences::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;

		type RuntimeHoldReason: From<HoldReason>;

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

		type ItemId: Parameter + MaxEncodedLen;

		type Fungible: Inspect<Self::AccountId, Balance = Self::Balance>
			+ InspectHold<Self::AccountId, Reason = Self::RuntimeHoldReason>
			+ Balanced<Self::AccountId>
			+ MutateHold<Self::AccountId>
			+ BalancedHold<Self::AccountId>;

		type NftStakingHandler: NftPermission<
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
		/// Not an enforced maximum, but we use it for weight estimation at the end of session
		type MaximumBoundValidators: Get<u32>;

		// Amount of **Tiles** to be rewarded in a given session.
		type SessionReward: Get<u128>;
		type Hooks: StakingHooks<Self::AccountId, Self::Balance, Self::ItemId>;

		/// A percent of the distributed session reward that goes somewhere other than the stakers
		type ContributionPercentage: Get<Perbill>;

		/// Where the contribution part of distributed reward goes
		type ContributionDestination: OnUnbalanced<Credit<Self::AccountId, Self::Fungible>>;

		/// Type representing the weights of calls in this pallet
		type WeightInfo: WeightInfo;

		#[cfg(feature = "runtime-benchmarks")]
		type BenchmarkHelper: BenchmarkHelper<Self>;
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		Staking,
	}

	#[pallet::storage]
	pub type TotalValidatorStakes<T: Config> = StorageNMap<
		Key = (NMapKey<Identity, StorageLayer>, NMapKey<Twox64Concat, T::AccountId>),
		Value = TotalValidatorStake<T::Balance>,
		QueryKind = OptionQuery,
	>;

	#[pallet::storage]
	pub type Validators<T: Config> =
		CountedStorageMap<_, Twox64Concat, T::AccountId, ValidatorDetails, OptionQuery>;

	#[pallet::storage]
	pub type Contracts<T: Config> = StorageNMap<
		Key = (
			NMapKey<Identity, StorageLayer>,
			NMapKey<Twox64Concat, T::AccountId>,
			NMapKey<Twox64Concat, T::AccountId>,
		),
		Value = Contract<T::Balance, T::ItemId>,
		QueryKind = OptionQuery,
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

	#[pallet::storage]
	pub type CurrentStagingLayer<T: Config> = StorageValue<_, StagingLayer, ValueQuery>;

	#[pallet::storage]
	#[pallet::unbounded]
	pub type SessionEndings<T: Config> = StorageValue<_, SpVec<SessionEnding<T>>, ValueQuery>;

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_idle(_block_number: BlockNumberFor<T>, remaining_weight: Weight) -> Weight {
			let mut meter = WeightMeter::with_limit(
				remaining_weight.saturating_sub(<T as Config>::WeightInfo::on_idle()),
			);

			SessionEndings::<T>::mutate(|endings| {
				loop {
					if endings.is_empty() {
						break;
					}

					// `SessionEnding` is not `Clone` and must be consumed.
					let SessionEnding { state, mut context } = endings.remove(0);

					match state.try_complete(&mut context, &mut meter) {
						// `SessionEnding` completed, try progressing the next one
						Ok(()) => {
							Self::deposit_event(Event::<T>::SessionEndingCompleted {
								session: context.session_index,
							});
						},

						// `SessionEnding` made progress, continue next time
						Err(state) => {
							// Place new state into the previous's place
							endings.insert(0, SessionEnding { state, context });
							break;
						},
					}
				}
			});

			meter.consumed()
		}
	}

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

		SessionEndingCompleted {
			session: SessionIndex,
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
		NftNotBound,
		ValidatorAlreadySelected,
		BindingContractExists,
		MoreContractsExist,
		UncommittedState,
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
		WouldDust,
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

				Contracts::<T>::insert(
					(StorageLayer::Committed, validator_id, validator_id),
					self_contract,
				);

				TotalValidatorStakes::<T>::insert(
					(StorageLayer::Committed, validator_id),
					TotalValidatorStake { total_stake: *nominal_value, contract_count: 1 },
				);
			}
		}
	}

	impl<T: Config> Pallet<T> {
		pub fn current_staging_layer() -> StagingLayer {
			CurrentStagingLayer::<T>::get()
		}

		pub fn rotate_staging_layer() {
			CurrentStagingLayer::<T>::mutate(|current| *current = current.other());
		}

		fn get_current<V>(f: impl Fn(StorageLayer) -> Option<V>) -> Option<V> {
			Self::get_current_staged(&f).or_else(|| f(StorageLayer::Committed))
		}

		fn get_current_staged<V>(f: impl Fn(StorageLayer) -> Option<V>) -> Option<V> {
			let staging = Self::current_staging_layer();
			f(StorageLayer::Staged(staging)).or_else(|| f(StorageLayer::Staged(staging.other())))
		}

		pub fn current_contract(
			validator: &T::AccountId,
			staker: &T::AccountId,
		) -> Option<Contract<T::Balance, T::ItemId>> {
			Self::get_current(|layer| Contracts::<T>::get((layer, validator, staker)))
		}

		pub fn current_staged_contract(
			validator: &T::AccountId,
			staker: &T::AccountId,
		) -> Option<Contract<T::Balance, T::ItemId>> {
			Self::get_current_staged(|layer| Contracts::<T>::get((layer, validator, staker)))
		}

		pub fn stage_contract(
			validator: &T::AccountId,
			staker: &T::AccountId,
			contract: Contract<T::Balance, T::ItemId>,
		) {
			Contracts::<T>::insert(
				(StorageLayer::Staged(Self::current_staging_layer()), validator, staker),
				contract,
			);
		}

		pub fn current_total_validator_stake(
			validator: &T::AccountId,
		) -> Option<TotalValidatorStake<T::Balance>> {
			Self::get_current(|layer| TotalValidatorStakes::<T>::get((layer, validator)))
		}

		pub fn current_staged_total_validator_stake(
			validator: &T::AccountId,
		) -> Option<TotalValidatorStake<T::Balance>> {
			Self::get_current_staged(|layer| TotalValidatorStakes::<T>::get((layer, validator)))
		}

		pub fn stage_total_validator_stake(
			validator: &T::AccountId,
			stake: TotalValidatorStake<T::Balance>,
		) {
			TotalValidatorStakes::<T>::insert(
				(StorageLayer::Staged(Self::current_staging_layer()), validator),
				stake,
			);
		}

		pub(crate) fn drafted_validators() -> impl Iterator<Item = <T as SessionConfig>::ValidatorId>
		{
			// Currently active + queued validators
			SessionPallet::<T>::queued_keys()
				.into_iter()
				.map(|(v, _)| v)
				.chain(SessionPallet::<T>::validators())
		}

		fn ensure_not_overdominant(validator: &T::AccountId) -> DispatchResult {
			let total_stake = Self::current_total_validator_stake(validator)
				.ok_or(Error::<T>::NotBound)?
				.total_stake;

			let ratio = Perbill::from_rational(
				total_stake,
				<T as pallet::Config>::Fungible::total_issuance(),
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
			T::Hooks::on_chill(&validator);
			Self::deposit_event(Event::<T>::ValidatorChilled { validator, reason });
			ValidatorState::Chilled(SessionPallet::<T>::current_index())
		}

		/// Grows total stake by amount
		/// (staged)
		pub(crate) fn grow_total_validator_stake_by(validator: &T::AccountId, value: T::Balance) {
			if value.is_zero() {
				return;
			}

			let stake = if let Some(mut stake) = Self::current_total_validator_stake(validator) {
				stake.total_stake.saturating_accrue(value);
				stake
			} else {
				TotalValidatorStake { total_stake: value, ..Default::default() }
			};

			Self::stage_total_validator_stake(validator, stake);
		}

		/// Shrinks total stake by amount
		/// (staged)
		pub(crate) fn shrink_total_validator_stake_by(validator: &T::AccountId, value: T::Balance) {
			if value.is_zero() {
				return;
			}

			let Some(mut stake) = Self::current_total_validator_stake(validator) else {
				return;
			};

			stake.total_stake.saturating_reduce(value);

			Self::stage_total_validator_stake(validator, stake);
		}

		// Requests the creation of a new contract for a given validator
		// Fails if contract limit is already reached for the validator
		fn request_new_contract(validator: &T::AccountId) -> DispatchResult {
			let stake = if let Some(mut stake) = Self::current_total_validator_stake(validator) {
				if stake.contract_count < T::MaximumContractsPerValidator::get() {
					stake.contract_count.saturating_inc();
					stake
				} else {
					return Err(Error::<T>::ContractLimitReached.into());
				}
			} else {
				TotalValidatorStake { contract_count: 1, ..Default::default() }
			};

			Self::stage_total_validator_stake(validator, stake);
			Ok(())
		}

		// Make a promise to destroy a contract for the given validator
		// This decreases the number of contracts associated with the validator
		pub(crate) fn relinquish_contract(validator: &T::AccountId) {
			let Some(mut stake) = Self::current_total_validator_stake(validator) else {
				// A bound validator always has at least a self-contract and nonzero stake,
				// so this branch should not be possible.
				return;
			};

			stake.contract_count.saturating_dec();
			Self::stage_total_validator_stake(validator, stake);
		}

		/// Adds the provided amount to the account's lock.
		/// (immediate)
		pub(crate) fn lock_currency(
			account_id: &T::AccountId,
			amount: T::Balance,
			precision: Precision,
		) -> Result<T::Balance, DispatchError> {
			let reducible = <T as Config>::Fungible::reducible_balance(
				account_id,
				Preservation::Preserve, // do not put ed on hold
				Fortitude::Force,       // do put frozen on hold
			);

			let amount = if amount > reducible {
				match precision {
					Precision::Exact => return Err(Error::<T>::InsufficientFunds.into()),
					Precision::BestEffort => reducible,
				}
			} else {
				amount
			};

			<T as Config>::Fungible::hold(&HoldReason::Staking.into(), account_id, amount)?;

			Ok(amount)
		}

		/// Removes the provided amount from the account's lock.
		/// (immediate)
		/// PANIC: if trying to unlock more than there is locked
		pub(crate) fn unlock_currency(account_id: &T::AccountId, amount: T::Balance) {
			<T as Config>::Fungible::release(
				&HoldReason::Staking.into(),
				account_id,
				amount,
				Precision::Exact,
			)
			.expect("no more unlocked than locked before");
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

			Self::lock_currency(staker, amount, Precision::Exact)?;
			Self::grow_total_validator_stake_by(validator, amount);
			Self::ensure_not_overdominant(validator)?;

			let min_staking_period_end =
				SessionPallet::<T>::current_index() + validator_details.min_staking_period::<T>();

			let stake = match Self::current_contract(validator, staker) {
				Some(Contract { stake: old, .. }) => {
					Stake { currency: old.currency.saturating_add(amount), ..old }
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

			Self::stage_contract(validator, staker, contract);

			T::Hooks::on_currency_stake(staker, validator, amount);

			Ok(())
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

			let stake = match Self::current_contract(validator, staker) {
				Some(Contract { mut stake, .. }) => {
					stake
						.delegated_nfts
						.try_push((item_id.clone(), nominal_value))
						.map_err(|_| Error::<T>::TooManyNftDelegatedToContract)?;

					stake
				},

				None => {
					Self::request_new_contract(validator)?;
					// `truncate_from` is safe, as (1) does not panic,
					// (2) we know the bound is non-zero
					let delegated_nfts =
						BoundedVec::truncate_from(vec![(item_id.clone(), nominal_value)]);

					Stake { delegated_nfts, ..Default::default() }
				},
			};

			let contract = Contract {
				stake,
				commission: validator_details.commission(),
				min_staking_period_end,
			};

			Self::stage_contract(validator, staker, contract);

			T::Hooks::on_nft_stake(staker, validator, item_id);

			Ok(())
		}

		fn do_unstake_currency(
			validator: &T::AccountId,
			staker: &T::AccountId,
			amount: T::Balance,
		) -> DispatchResult {
			let Some(mut contract) = Self::current_contract(validator, staker) else {
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

			contract.stake.currency.saturating_reduce(amount);

			ensure!(
				contract.stake.currency >= T::MinimumStakingAmount::get()
					|| contract.stake.currency.is_zero(),
				Error::<T>::WouldDust
			);

			Self::stage_contract(validator, staker, contract);

			Self::shrink_total_validator_stake_by(validator, amount);
			Self::stage_unlock_currency(staker, amount);

			T::Hooks::on_currency_unstake(staker, validator, amount);

			Ok(())
		}

		fn do_unstake_nft(
			validator: &T::AccountId,
			staker: &T::AccountId,
			item_id: &T::ItemId,
		) -> DispatchResult {
			let Some(mut contract) = Self::current_contract(validator, staker) else {
				return Err(Error::<T>::NoContract.into());
			};

			if let Some(index) =
				contract.stake.delegated_nfts.iter().position(|(x, _)| x == item_id)
			{
				contract.stake.delegated_nfts.swap_remove(index);
			} else {
				return Err(Error::<T>::NftNotBound.into());
			}

			let session = SessionPallet::<T>::current_index();
			ensure!(
				session >= contract.min_staking_period_end
					|| (validator != staker && Self::is_slacking(validator)),
				Error::<T>::EarlyUnstake
			);

			Self::stage_contract(validator, staker, contract);

			let nominal_value = T::NftDelegationHandler::nominal_value(item_id)?;
			UnlockingDelegatorNfts::<T>::insert(item_id.clone(), staker.clone());

			Self::shrink_total_validator_stake_by(validator, nominal_value);

			T::Hooks::on_nft_unstake(staker, validator, item_id);

			Ok(())
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(<T as Config>::WeightInfo::bind_validator())]
		pub fn bind_validator(origin: OriginFor<T>, item_id: T::ItemId) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(!Validators::<T>::contains_key(&caller), Error::<T>::AlreadyBound);

			let (permission, nominal_value) = T::NftStakingHandler::bind(&caller, &item_id)?;

			ensure!(
				T::NftStakingHandler::nominal_factor_of_bound(&caller)?
					> T::NominalValueThreshold::get(),
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

			Self::stage_contract(&caller, &caller, self_contract);

			T::Hooks::on_bound(&caller);
			Self::deposit_event(Event::<T>::ValidatorBound(caller));

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::unbind_validator(*contract_count))]
		pub fn unbind_validator(origin: OriginFor<T>, contract_count: u32) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Validators::<T>::contains_key(&caller), Error::<T>::NotBound);
			ensure!(Self::is_chilled(&caller), Error::<T>::CallerIsNotChilled);

			// NOTE: we use this as a proxy for all staged storages related to the caller (eg.: contracts)
			// match Self::current_staged_total_validator_stake(&caller) {
			// 	Some(_) => {
			// 		let current_layer = CurrentStagingLayer::<T>::get();
			// 		let write_layer = TotalValidatorStakes::<T>::get((
			// 			StorageLayer::Staged(current_layer),
			// 			&caller,
			// 		));
			// 		let drain_layer = TotalValidatorStakes::<T>::get((
			// 			StorageLayer::Staged(current_layer.other()),
			// 			&caller,
			// 		));
			// 		panic!("{current_layer:?} {write_layer:?} {drain_layer:?}");
			// 	},
			// 	None => (),
			// };

			ensure!(
				Self::current_staged_total_validator_stake(&caller).is_none(),
				Error::<T>::UncommittedState
			);

			let _ = T::NftStakingHandler::unbind(&caller)?;

			let validator_id = T::ValidatorIdOf::convert(caller.clone())
				.expect("caller address can always be converted to validator id");

			ensure!(
				Self::drafted_validators().all(|id| id != validator_id),
				Error::<T>::ValidatorAlreadySelected
			);

			let contracts = Contracts::<T>::drain_prefix((StorageLayer::Committed, &caller))
				.collect::<SpVec<_>>();

			ensure!(
				contracts.len()
					<= contract_count
						.try_into()
						.expect("Number of contracts always fits into a usize"),
				Error::<T>::MoreContractsExist
			);

			let session = SessionPallet::<T>::current_index();

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

				T::Hooks::on_kick(&contractee, &caller);

				Self::deposit_event(Event::<T>::StakerKicked {
					validator: caller.clone(),
					staker: contractee,
					reason: KickReason::Unbind,
				});
			}

			// FIXME: make sure this still holds in new setup
			// Note: we can remove immediately as no reward would be given.
			// Caveat: slashes in current session? We can forgive them, right?
			Validators::<T>::remove(&caller);
			ValidatorStates::<T>::remove(&caller);
			TotalValidatorStakes::<T>::remove((StorageLayer::Committed, &caller));
			InverseSlashes::<T>::remove(&caller);

			T::Hooks::on_unbound(&caller);
			Self::deposit_event(Event::<T>::ValidatorUnbound(caller));

			Ok(())
		}

		#[pallet::call_index(2)]
		#[pallet::weight(<T as Config>::WeightInfo::enable_delegations())]
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
		#[pallet::weight(<T as Config>::WeightInfo::disable_delegations())]
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
		#[pallet::weight(<T as Config>::WeightInfo::chill_validator())]
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
		#[pallet::weight(<T as Config>::WeightInfo::unchill_validator())]
		pub fn unchill_validator(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			ValidatorStates::<T>::mutate(&caller, |validator_state| {
				let validator_state = validator_state.as_mut().ok_or(Error::<T>::NotBound)?;
				match validator_state {
					ValidatorState::Chilled(_) => {
						*validator_state = ValidatorState::Normal;

						T::Hooks::on_unchill(&caller);
						Self::deposit_event(Event::<T>::ValidatorUnchilled(caller.clone()));
						Ok(())
					},
					_ => Err(Error::<T>::CallerIsNotChilled),
				}
			})?;

			ensure!(
				T::NftStakingHandler::nominal_factor_of_bound(&caller)?
					> T::NominalValueThreshold::get(),
				Error::<T>::ValidatorDisqualified
			);

			Ok(())
		}

		#[pallet::call_index(6)]
		#[pallet::weight(<T as Config>::WeightInfo::self_stake_currency())]
		pub fn self_stake_currency(origin: OriginFor<T>, amount: T::Balance) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let details = Validators::<T>::get(&caller).ok_or(Error::<T>::NotBound)?;
			ensure!(details.permission() == PermissionType::DPoS, Error::<T>::CallerNotDPoS);

			Self::do_stake_currency(&caller, details, &caller, amount)?;

			Self::deposit_event(Event::<T>::CurrencyStaked {
				validator: caller.clone(),
				staker: caller,
				amount,
			});

			Ok(())
		}

		#[pallet::call_index(7)]
		#[pallet::weight(<T as Config>::WeightInfo::self_stake_nft())]
		pub fn self_stake_nft(origin: OriginFor<T>, item_id: T::ItemId) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let details = Validators::<T>::get(&caller).ok_or(Error::<T>::NotBound)?;
			ensure!(details.permission() == PermissionType::DPoS, Error::<T>::CallerNotDPoS);

			Self::do_stake_nft(&caller, details, &caller, &item_id)?;

			Self::deposit_event(Event::<T>::NftDelegated {
				validator: caller.clone(),
				staker: caller,
				item_id,
			});

			Ok(())
		}

		#[pallet::call_index(8)]
		#[pallet::weight(<T as Config>::WeightInfo::self_unstake_currency())]
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
		#[pallet::weight(<T as Config>::WeightInfo::self_unstake_nft())]
		pub fn self_unstake_nft(origin: OriginFor<T>, item_id: T::ItemId) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let details = Validators::<T>::get(&caller).ok_or(Error::<T>::NotBound)?;
			ensure!(details.permission() == PermissionType::DPoS, Error::<T>::CallerNotDPoS);

			Self::do_unstake_nft(&caller, &caller, &item_id)?;

			Self::deposit_event(Event::<T>::NftUndelegated {
				validator: caller.clone(),
				staker: caller,
				item_id,
			});

			Ok(())
		}

		#[pallet::call_index(10)]
		#[pallet::weight(<T as Config>::WeightInfo::delegate_currency())]
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
		#[pallet::weight(<T as Config>::WeightInfo::delegate_nft())]
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
		#[pallet::weight(<T as Config>::WeightInfo::undelegate_currency())]
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
		#[pallet::weight(<T as Config>::WeightInfo::undelegate_nft())]
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
		#[pallet::weight(<T as Config>::WeightInfo::set_minimum_staking_period())]
		pub fn set_minimum_staking_period(
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
		#[pallet::weight(<T as Config>::WeightInfo::set_commission())]
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
		#[pallet::weight(<T as Config>::WeightInfo::kick())]
		pub fn kick(origin: OriginFor<T>, target: T::AccountId) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidCaller);

			let details = Validators::<T>::get(&caller).ok_or(Error::<T>::NotBound)?;

			ensure!(details.permission() == PermissionType::DPoS, Error::<T>::CallerNotDPoS);

			ensure!(!Self::is_chilled(&caller), Error::<T>::CallerIsChilled);

			let Some(mut contract) = Self::current_contract(&caller, &target) else {
				return Err(Error::<T>::NoContract.into());
			};

			if contract.min_staking_period_end > SessionPallet::<T>::current_index() {
				return Err(Error::<T>::EarlyKick.into());
			}

			// TODO: there might be events missing for the indexer
			Self::stage_unlock_currency(&target, contract.stake.currency);
			for (item_id, _) in &contract.stake.delegated_nfts {
				UnlockingDelegatorNfts::<T>::insert(item_id.clone(), target.clone());
			}

			// this is a staged action, so in the current session this delegator's stake is still included.
			Self::shrink_total_validator_stake_by(&caller, contract.stake.total());

			// Note: contracts with empty stake are removed in next session,
			// but the delegator can always just restake even in this session
			contract.stake = Stake::default();

			T::Hooks::on_kick(&target, &caller);

			Self::deposit_event(Event::<T>::StakerKicked {
				validator: caller.clone(),
				staker: target.clone(),
				reason: KickReason::Manual,
			});

			Self::stage_contract(&caller, &target, contract);

			Ok(())
		}

		#[pallet::call_index(17)]
		#[pallet::weight(<T as Config>::WeightInfo::topup())]
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

			if let Some(mut contract) = Self::current_contract(&caller, &caller) {
				contract.stake.permission_nft = Some(issued_nominal_value);

				Self::grow_total_validator_stake_by(&caller, imbalance);

				ValidatorStates::<T>::mutate_extant(&caller, |vstate| {
					if let ValidatorState::Faulted = vstate {
						*vstate = ValidatorState::Normal;
					}
				});

				Self::stage_contract(&caller, &caller, contract);
			}

			let withdrawn = <T as pallet::Config>::Fungible::withdraw(
				&caller,
				imbalance,
				Precision::Exact,
				Preservation::Preserve,
				Fortitude::Force,
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
