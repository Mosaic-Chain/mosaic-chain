#![cfg_attr(not(feature = "std"), no_std)]
/// Mosaic's very own staking pallet (TODO: DOCS)
/// Note: functions might have an (immediate) or a (staged) qualifier to signify when the change is going to occur.
/// # Missing features:
///   - Lesser punishment (we currently use the recommended slash proportions)
///   - Slippage
///   - Events
///   - Tests and benchmarks
pub use pallet::*;
mod reward;
mod staging;

use core::{num::NonZeroU32, ops::Add};

use codec::Codec;
use frame_support::{
	pallet_prelude::*,
	traits::{
		Currency, ExistenceRequirement, Imbalance, LockableCurrency, OnUnbalanced, ValidatorSet,
		ValidatorSetWithIdentification, WithdrawReasons,
	},
	Twox64Concat,
};
use frame_system::pallet_prelude::*;
use pallet_nfts::Config as NftsConfig;
use pallet_session::{Config as SessionConfig, Pallet as SessionPallet};
use sp_std::vec::Vec as SpVec;

use sp_runtime::{
	traits::{Convert, ConvertInto, Zero},
	FixedPointOperand, PerThing, Perbill, Saturating,
};

use utils::{
	traits::{NftDelegation, NftStaking},
	SessionIndex,
};

use staging::Staging;
use utils::traits::{NftDelegation, NftStaking};

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

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
			+ Saturating;

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

		/// Period after which a chilled validator is considered slacking
		type SlackingPeriod: Get<u32>;

		/// A percent under which a validator is disqualified
		type NominalValueThreshold: Get<Perbill>;

		type NftDelegationHandler: NftDelegation<
			Self::AccountId,
			Self::Balance,
			Self::ItemId,
			Self::AccountId,
		>;

		type MinimumStakingPeriod: Get<NonZeroU32>;
		type MinimumCommissionRate: Get<Perbill>;
		type MinimumStakingAmount: Get<Self::Balance>;
		type MaximumStakePercentage: Get<Perbill>;

		type OnReward: OnUnbalanced<PositiveImbalanceOf<Self>>;

		#[pallet::constant]
		type PalletId: Get<frame_support::PalletId>;
	}

	type PositiveImbalanceOf<T> = <<T as Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::PositiveImbalance;

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

	#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Stake<Balance, ItemId> {
		pub currency: Balance,
		pub delegated_nfts: SpVec<(ItemId, Balance)>,
		pub permission_nft: Option<Balance>,
	}

	impl<Balance, ItemId> Stake<Balance, ItemId>
	where
		Balance: Add<Balance, Output = Balance> + Copy + Zero,
	{
		pub fn total(&self) -> Balance {
			self.currency + self.permission_value() + self.delegated_nft()
		}

		pub fn permission_value(&self) -> Balance {
			self.permission_nft.unwrap_or(Zero::zero())
		}

		pub fn delegated_nft(&self) -> Balance {
			self.delegated_nfts.iter().fold(Zero::zero(), |acc, v| acc + v.1)
		}
	}

	impl<Balance: Default + Codec, ItemId: Codec> Default for Stake<Balance, ItemId> {
		fn default() -> Self {
			Self {
				currency: Default::default(),
				delegated_nfts: Default::default(),
				permission_nft: Default::default(),
			}
		}
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum ValidatorDetails {
		PoS,
		DPoS { commission: Perbill, min_staking_period: u32, accept_delegations: bool },
	}

	impl ValidatorDetails {
		fn permission(&self) -> PermissionType {
			match self {
				Self::PoS => PermissionType::PoS,
				Self::DPoS { .. } => PermissionType::DPoS,
			}
		}

		/// Helper function to return commission regardless the permission type
		fn commission(&self) -> Perbill {
			match self {
				Self::PoS => Perbill::from_percent(100),
				Self::DPoS { commission, .. } => *commission,
			}
		}

		/// Helper function to return min_staking_period regardless the permission type
		fn min_staking_period<T: Config>(&self) -> u32 {
			match self {
				Self::PoS => T::MinimumStakingPeriod::get().into(),
				Self::DPoS { min_staking_period, .. } => *min_staking_period,
			}
		}
	}

	#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub struct Contract<Balance, ItemId> {
		pub stake: Stake<Balance, ItemId>,
		pub commission: Perbill,
		/// The last session where the contract is binding
		/// If using staged operations, we can now unstake
		/// otherwise we must wait for the session after.
		pub min_staking_period_end: SessionIndex,
	}

	#[pallet::storage]
	pub type TotalStake<T: Config> = StorageValue<_, Staging<T::Balance>, ValueQuery>;

	// NODES
	// TODO: after implementing slippage this does not need to be Staging.
	#[pallet::storage]
	pub type Validators<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, Staging<ValidatorDetails>, ValueQuery>;

	#[pallet::storage]
	pub type Contracts<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		T::AccountId,
		Staging<Contract<T::Balance, <T as NftsConfig>::ItemId>>,
		ValueQuery,
	>;

	#[pallet::storage]
	pub type LockedCurrency<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::Balance>;

	#[pallet::storage]
	pub type UnlockingCurrency<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::Balance>;

	#[pallet::storage]
	pub type UnlockingDelegatorNfts<T: Config> =
		StorageMap<_, Twox64Concat, <T as NftsConfig>::ItemId, T::AccountId>;

	#[pallet::storage]
	pub type InverseSlashes<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, Perbill>;

	#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo)]
	pub enum ValidatorState {
		/// No issue with the validator
		Normal,
		/// Validator will be or has been slashed
		Faulted,
		/// Stores the SessionIndex so we can check if it is slacking
		Chilled(SessionIndex),
	}

	#[pallet::storage]
	pub type ValidatorStates<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, ValidatorState>;

	// TODO(vismate): Useful events
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		Blank,
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		AlreadyBound,
		NotBound,
		BlockedUnbind,
		TargetNotDPoS,
		InvalidTarget,
		AlreadyAcceptsDelegations,
		AlreadyDeniesDelegations,
		TargetDeniesDelegations,
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

	// TODO: seperate nft minting from staking genesis
	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let mut total_stake = Zero::zero();
			for (validator_id, permission, nominal_value) in &self.initial_staking_validators {
				assert!(!Validators::<T>::get(validator_id).exists());

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

				Validators::<T>::insert(validator_id, Staging::new(validator));
				ValidatorStates::<T>::insert(validator_id, ValidatorState::Normal);

				let self_contract = Contract {
					stake: Stake { permission_nft: Some(*nominal_value), ..Default::default() },
					commission: Perbill::from_percent(100),
					min_staking_period_end: T::MinimumStakingPeriod::get().into(),
				};

				Contracts::<T>::insert(validator_id, validator_id, Staging::new(self_contract));

				total_stake += *nominal_value;
			}

			TotalStake::<T>::put(Staging::new(total_stake));
		}
	}

	impl<T: Config> Pallet<T> {
		fn ensure_stake_limit(
			stake: &Stake<T::Balance, <T as NftsConfig>::ItemId>,
		) -> DispatchResult {
			let ratio = Perbill::from_rational(
				stake.total(),
				<T as pallet::Config>::Currency::total_issuance(),
			);

			ensure!(ratio <= T::MaximumStakePercentage::get(), Error::<T>::OverdominantStake);
			Ok(())
		}

		fn is_chilled(account: &T::AccountId) -> bool {
			let state = ValidatorStates::<T>::get(account).unwrap();
			matches!(state, ValidatorState::Chilled(_))
		}

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

		/// Commits all storage values. Removes values where there is neither a staged nor a committed value.
		/// We also remove contracts where stake became zero
		fn commit_storage() {
			TotalStake::<T>::mutate(Staging::commit);

			Validators::<T>::translate_values(|mut s: Staging<ValidatorDetails>| {
				s.exists().then(|| {
					s.commit();
					s
				})
			});

			Contracts::<T>::translate_values(
				|mut s: Staging<Contract<T::Balance, <T as NftsConfig>::ItemId>>| {
					s.current().is_some_and(|contract| !contract.stake.total().is_zero()).then(
						|| {
							s.commit();
							s
						},
					)
				},
			);
		}

		/// Grows total stake by amount
		/// (staged)
		fn grow_total_stake_by(value: T::Balance) {
			if value.is_zero() {
				return;
			}

			TotalStake::<T>::mutate(|s| {
				let current = s.current().copied().unwrap_or_default();
				s.stage(current.saturating_add(value));
			});
		}

		/// Shrinks total stake by amount
		/// (staged)
		fn shrink_total_stake_by(value: T::Balance) {
			if value.is_zero() {
				return;
			}

			TotalStake::<T>::mutate(|s| {
				let current = s.current().copied().unwrap_or_default();
				s.stage(current.saturating_sub(value));
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
		/// (immediate)
		fn unlock_currency(account_id: &T::AccountId, amount: T::Balance) {
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

		// Removes the provided amount from the account's lock.
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
			let is_chilled = Self::is_chilled(validator);
			ensure!(!is_chilled, Error::<T>::TargetIsChilled);

			ensure!(amount >= T::MinimumStakingAmount::get(), Error::<T>::TooSmallStake);

			let balance = <T as pallet::Config>::Currency::free_balance(staker);
			ensure!(balance >= amount, Error::<T>::InsufficientFunds);

			Self::lock_currency(staker, amount);
			Self::grow_total_stake_by(amount);

			let min_staking_period_end =
				SessionPallet::<T>::current_index() + validator_details.min_staking_period::<T>();

			Contracts::<T>::mutate(validator, staker, |s| {
				let stake = match s.current() {
					Some(Contract { stake: old, .. }) => {
						Stake { currency: old.currency.saturating_add(amount), ..old.clone() }
					},

					None => Stake { currency: amount, ..Default::default() },
				};

				Self::ensure_stake_limit(&stake)?;

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
			item_id: &<T as NftsConfig>::ItemId,
		) -> DispatchResult {
			ensure!(!Self::is_chilled(validator), Error::<T>::TargetIsChilled);

			let (expiry, nominal_value) =
				T::NftDelegationHandler::bind(staker, item_id, validator.clone())?;

			ensure!(nominal_value >= T::MinimumStakingAmount::get(), Error::<T>::LowNominalValue);

			let min_staking_period_end =
				SessionPallet::<T>::current_index() + validator_details.min_staking_period::<T>();

			// Note: nft is still considered valid until the end of the session it expires on.
			ensure!(expiry >= min_staking_period_end, Error::<T>::NftExpiresEarly);

			Self::grow_total_stake_by(nominal_value);

			Contracts::<T>::mutate(validator, staker, |s| {
				let stake = match s.current() {
					Some(Contract { stake: old, .. }) => {
						let mut new = old.clone();
						new.delegated_nfts.push((*item_id, nominal_value));
						new
					},

					None => {
						let mut delegated_nfts = SpVec::default();
						delegated_nfts.push((*item_id, nominal_value));
						Stake { delegated_nfts, ..Default::default() }
					},
				};

				Self::ensure_stake_limit(&stake)?;

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
			let Some(mut contract) = Contracts::<T>::get(validator, staker).current().cloned()
			else {
				return Err(Error::<T>::NoContract.into());
			};

			let session = SessionPallet::<T>::current_index();
			ensure!(
				session >= contract.min_staking_period_end || Self::is_slacking(validator),
				Error::<T>::EarlyUnstake
			);
			ensure!(amount >= T::MinimumStakingAmount::get(), Error::<T>::TooSmallUnstake);
			ensure!(amount <= contract.stake.currency, Error::<T>::InsufficientFunds);

			contract.stake.currency = contract.stake.currency.saturating_sub(amount);

			ensure!(
				contract.stake.currency >= T::MinimumStakingAmount::get()
					|| contract.stake.currency.is_zero(),
				Error::<T>::TooSmallStake
			);

			Contracts::<T>::mutate(validator, staker, |s| s.stage(contract));

			Self::shrink_total_stake_by(amount);
			Self::stage_unlock_currency(staker, amount);

			Ok(())
		}

		fn do_unstake_nft(
			validator: &T::AccountId,
			staker: &T::AccountId,
			item_id: &<T as NftsConfig>::ItemId,
		) -> DispatchResult {
			let Some(mut contract) = Contracts::<T>::get(validator, staker).current().cloned()
			else {
				return Err(Error::<T>::NoContract.into());
			};

			let session = SessionPallet::<T>::current_index();
			ensure!(
				session >= contract.min_staking_period_end || Self::is_slacking(validator),
				Error::<T>::EarlyUnstake
			);

			let nominal_value = T::NftDelegationHandler::nominal_value(item_id)?;
			UnlockingDelegatorNfts::<T>::insert(*item_id, staker.clone());

			if let Some(index) =
				contract.stake.delegated_nfts.iter().position(|(x, _)| x == item_id)
			{
				contract.stake.delegated_nfts.remove(index);
			} else {
				return Err(Error::<T>::NotBound.into());
			}

			Contracts::<T>::mutate(validator, staker, |s| s.stage(contract));

			Self::shrink_total_stake_by(nominal_value);

			Ok(())
		}

		fn do_reward_participants(
			rewarded_validators: impl Iterator<Item = T::AccountId>,
			session_reward: u128,
		) {
			let committed_total_stake: u128 =
				TotalStake::<T>::get().committed().copied().unwrap_or_default().into();

			if committed_total_stake == 0 {
				return;
			}

			let mut total_rewarded = PositiveImbalanceOf::<T>::default();

			for validator in rewarded_validators {
				ValidatorStates::<T>::mutate_extant(&validator, |vstate| {
					if let ValidatorState::Faulted = *vstate {
						*vstate = ValidatorState::Normal;
					}
				});

				let mut total_v_imbalance = PositiveImbalanceOf::<T>::default();
				for (staker, contract) in Contracts::<T>::iter_prefix(&validator)
					.filter_map(|(staker, s)| s.committed().cloned().map(|c| (staker, c)))
				{
					let reward = reward::calculate_contract_reward::<T>(
						committed_total_stake,
						session_reward,
						&contract,
					);

					let v_imbalance = <T as Config>::Currency::deposit_creating(
						&validator,
						reward.validator_reward,
					);

					total_v_imbalance.subsume(v_imbalance);

					let s_imbalance =
						<T as Config>::Currency::deposit_creating(&staker, reward.staker_reward);

					Self::lock_currency(&staker, s_imbalance.peek());

					Contracts::<T>::mutate(&validator, &staker, |s| {
						if let Some(mut new) = s.current().cloned() {
							new.stake.currency =
								new.stake.currency.saturating_add(s_imbalance.peek());
							s.stage(new);
						}
					});
					total_rewarded.subsume(s_imbalance);
				}

				Self::lock_currency(&validator, total_v_imbalance.peek());

				Contracts::<T>::mutate(&validator, &validator, |s| {
					if let Some(mut new) = s.current().cloned() {
						new.stake.currency =
							new.stake.currency.saturating_add(total_v_imbalance.peek());
						s.stage(new);
					}
				});

				total_rewarded.subsume(total_v_imbalance);
			}

			Self::grow_total_stake_by(total_rewarded.peek());

			T::OnReward::on_unbalanced(total_rewarded);
		}

		fn chill_if_faulted(validator: &T::AccountId) {
			// Autochill after two consecutive slashes
			ValidatorStates::<T>::mutate_extant(validator, |vstate| {
				*vstate = match *vstate {
					ValidatorState::Normal => ValidatorState::Faulted,
					ValidatorState::Faulted => {
						ValidatorState::Chilled(SessionPallet::<T>::current_index())
					},
					s => s,
				}
			});
		}

		fn do_slash_participants() {
			let mut total_stake_slash = T::Balance::zero();

			for (validator, slash) in
				InverseSlashes::<T>::drain().map(|(v, is)| (v, is.left_from_one()))
			{
				Self::chill_if_faulted(&validator);

				for (delegator, contract) in Contracts::<T>::iter_prefix(&validator) {
					let Some(committed_contract) = contract.committed().cloned() else {
						// Slashed validators must have a committed self-contract
						return;
					};

					let Some(mut new_contract) = contract.current().cloned() else {
						// Theoretically impossible
						return;
					};

					// Currency slash
					let currency_slash = slash * committed_contract.stake.currency;
					let staged_value = new_contract.stake.currency;

					total_stake_slash = total_stake_slash.saturating_add(currency_slash);
					if currency_slash > staged_value {
						let rem = currency_slash - staged_value;
						// Prevent double-unlocking by this...
						UnlockingCurrency::<T>::mutate_extant(&delegator, |unlocking| {
							*unlocking = (*unlocking).saturating_sub(rem);
						});

						total_stake_slash = total_stake_slash.saturating_sub(rem);
					}

					Self::unlock_currency(&delegator, currency_slash);

					// TODO: trace if actual_slash != currency_slash
					let (actual_slash, _) =
						<T as pallet::Config>::Currency::slash(&delegator, currency_slash);

					new_contract.stake.currency =
						new_contract.stake.currency.saturating_sub(actual_slash.peek());

					// Slash delegator nfts
					// TODO: we iterate through delegated nfts quite a lot. Can we make it more efficient?
					let mut d_nft_slash = slash * committed_contract.stake.delegated_nft();

					for (nft, value) in committed_contract.stake.delegated_nfts.iter() {
						if d_nft_slash.is_zero() {
							break;
						}

						let slashed_from_this = d_nft_slash.min(*value);
						d_nft_slash -= slashed_from_this;
						let new_value = *value - slashed_from_this;

						// find nft in new_contract and deduct the slash both from it and the total stake.
						// if it's not found it means it just being unbound in which case it's whole value is already deducted.

						for (s_nft, s_value) in new_contract.stake.delegated_nfts.iter_mut() {
							if *s_nft == *nft {
								// TODO: unbind if value is lower then minimal staking amount?
								*s_value = new_value;
								total_stake_slash =
									total_stake_slash.saturating_add(slashed_from_this);
								break;
							}
						}

						// set nft nominal value
						// FIXME: dont panic
						T::NftDelegationHandler::set_nominal_value(nft, new_value)
							.expect("could set nominal value");
					}

					// If it's the self-contract let's slash the permission nft as well.
					if let Some(p_nominal_value) = committed_contract.stake.permission_nft {
						let p_slash = slash * p_nominal_value;
						total_stake_slash = total_stake_slash.saturating_add(p_slash);
						let new_value =
							new_contract.stake.permission_nft.unwrap().saturating_sub(p_slash);
						new_contract.stake.permission_nft = Some(new_value);
						T::NftStakingHandler::set_nominal_value_of_bound(&validator, new_value)
							.expect("could set nominal value");

						// If slashed below some% autochill
						if T::NftStakingHandler::nominal_factor_of(&validator)
							.expect("could get nominal factor")
							<= T::NominalValueThreshold::get()
						{
							ValidatorStates::<T>::mutate_extant(&validator, |vstate| {
								*vstate =
									ValidatorState::Chilled(SessionPallet::<T>::current_index())
							});
						}
					};

					// End
					Contracts::<T>::mutate(&validator, delegator, |s| s.stage(new_contract));
				}
			}

			Self::shrink_total_stake_by(total_stake_slash);
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		pub fn bind_validator(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(!Validators::<T>::get(&caller).exists(), Error::<T>::AlreadyBound);

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

			Validators::<T>::insert(&caller, Staging::new_staged(details));
			ValidatorStates::<T>::insert(&caller, ValidatorState::Normal);

			let self_contract = Contract {
				stake: Stake { permission_nft: Some(nominal_value), ..Default::default() },
				commission: Perbill::from_percent(100),
				min_staking_period_end: SessionPallet::<T>::current_index()
					+ u32::from(T::MinimumStakingPeriod::get()),
			};

			Contracts::<T>::insert(&caller, &caller, Staging::new_staged(self_contract));

			Ok(())
		}

		#[pallet::call_index(1)]
		pub fn unbind_validator(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Validators::<T>::get(&caller).exists(), Error::<T>::NotBound);
			ensure!(Self::is_chilled(&caller), Error::<T>::TargetIsNotChilled);

			// Note: we can unbind immediately, as the next bind could only take effect in the next session.
			let _ = T::NftStakingHandler::unbind(&caller)?;

			let validator_id = T::ValidatorIdOf::convert(caller.clone())
				.expect("caller address can be converted to validator id");

			// Currently active + queued validators
			// TODO(vismate): factor this out for testability
			let mut already_selected = SessionPallet::<T>::queued_keys()
				.into_iter()
				.map(|(v, _)| v)
				.chain(SessionPallet::<T>::validators().into_iter());

			ensure!(already_selected.all(|id| id != validator_id), Error::<T>::BlockedUnbind);

			let session = SessionPallet::<T>::current_index();

			let contracts =
				Contracts::<T>::drain_prefix(&caller).filter_map(|(contractee, contract)| {
					contract.current().map(|c| (contractee, c.clone()))
				});

			// Note: this includes the self-contract as well
			for (contractee, contract) in contracts {
				// Note: since we are terminating the contract immediately we have to check for equality too
				if contract.min_staking_period_end >= session {
					return Err(Error::<T>::BlockedUnbind.into());
				}

				// We immediately release currency and NFTs so delegators needn't wait until the end of session.
				// Note: they would not receive rewards for this contract this session.
				Self::unlock_currency(&contractee, contract.stake.currency);
				for (item_id, _) in contract.stake.delegated_nfts.iter() {
					T::NftDelegationHandler::unbind(&contractee, item_id)?;
				}

				// FIXME?: this is a staged action, so in the current session this validator's stake is still included.
				Self::shrink_total_stake_by(contract.stake.total());
			}

			// Note: we can remove immediately as no reward would be given.
			// Caveat: slashes in current session? We can forgive them, right?
			Validators::<T>::remove(&caller);
			ValidatorStates::<T>::remove(&caller);
			InverseSlashes::<T>::remove(&caller); // In theory this should already be empty

			Ok(())
		}

		#[pallet::call_index(2)]
		pub fn enable_delegations(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// Note: we check the current value, as enabling/disabling delegations must be immediate
			// After implementing slippage this can be cleaned up.

			let mut details =
				Validators::<T>::get(&caller).current().cloned().ok_or(Error::<T>::NotBound)?;

			if let ValidatorDetails::DPoS { accept_delegations, .. } = &mut details {
				ensure!(!*accept_delegations, Error::<T>::AlreadyAcceptsDelegations);

				*accept_delegations = true;

				Validators::<T>::mutate(&caller, |s| {
					s.stage(details);
				});
			} else {
				return Err(Error::<T>::TargetNotDPoS.into());
			};

			Ok(())
		}

		#[pallet::call_index(3)]
		pub fn disable_delegations(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			// Note: we check the current value, as enabling/disabling delegations must be immediate
			// After implementing slippage this can be cleaned up.

			let mut details =
				Validators::<T>::get(&caller).current().cloned().ok_or(Error::<T>::NotBound)?;

			if let ValidatorDetails::DPoS { accept_delegations, .. } = &mut details {
				ensure!(*accept_delegations, Error::<T>::AlreadyDeniesDelegations);

				*accept_delegations = false;

				Validators::<T>::mutate(&caller, |s| {
					s.stage(details);
				});
			} else {
				return Err(Error::<T>::TargetNotDPoS.into());
			};

			Ok(())
		}

		#[pallet::call_index(4)]
		pub fn chill_validator(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ValidatorStates::<T>::mutate(&caller, |validator_state| {
				let validator_state = validator_state.as_mut().ok_or(Error::<T>::NotBound)?;
				match validator_state {
					ValidatorState::Chilled(_) => Err(Error::<T>::TargetIsChilled.into()),
					_ => {
						*validator_state =
							ValidatorState::Chilled(SessionPallet::<T>::current_index());
						Ok(())
					},
				}
			})
		}

		#[pallet::call_index(5)]
		pub fn unchill_validator(origin: OriginFor<T>) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(
				T::NftStakingHandler::nominal_factor_of(&caller)? > T::NominalValueThreshold::get(),
				Error::<T>::ValidatorDisqualified
			);

			ValidatorStates::<T>::mutate(&caller, |validator_state| {
				let validator_state = validator_state.as_mut().ok_or(Error::<T>::NotBound)?;
				match validator_state {
					ValidatorState::Chilled(_) => {
						*validator_state = ValidatorState::Normal;
						Ok(())
					},
					_ => Err(Error::<T>::TargetIsNotChilled.into()),
				}
			})
		}

		#[pallet::call_index(6)]
		pub fn self_stake_currency(origin: OriginFor<T>, amount: T::Balance) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let details =
				Validators::<T>::get(&caller).current().cloned().ok_or(Error::<T>::NotBound)?;

			Self::do_stake_currency(&caller, details, &caller, amount)
		}

		#[pallet::call_index(7)]
		pub fn self_stake_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let details =
				Validators::<T>::get(&caller).current().cloned().ok_or(Error::<T>::NotBound)?;

			Self::do_stake_nft(&caller, details, &caller, &item_id)
		}

		#[pallet::call_index(8)]
		pub fn self_unstake_currency(origin: OriginFor<T>, amount: T::Balance) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Validators::<T>::get(&caller).exists(), Error::<T>::NotBound);

			Self::do_unstake_currency(&caller, &caller, amount)
		}

		#[pallet::call_index(9)]
		pub fn self_unstake_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(Validators::<T>::get(&caller).exists(), Error::<T>::NotBound);

			Self::do_unstake_nft(&caller, &caller, &item_id)
		}

		#[pallet::call_index(10)]
		pub fn delegate_currency(
			origin: OriginFor<T>,
			amount: T::Balance,
			target: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidTarget);

			let details =
				Validators::<T>::get(&target).current().cloned().ok_or(Error::<T>::NotBound)?;

			let ValidatorDetails::DPoS { accept_delegations, .. } = details else {
				return Err(Error::<T>::TargetNotDPoS.into());
			};

			ensure!(accept_delegations, Error::<T>::TargetDeniesDelegations);

			Self::do_stake_currency(&target, details, &caller, amount)
		}

		#[pallet::call_index(11)]
		pub fn delegate_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
			target: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidTarget);

			let details =
				Validators::<T>::get(&target).current().cloned().ok_or(Error::<T>::NotBound)?;

			let ValidatorDetails::DPoS { accept_delegations, .. } = details else {
				return Err(Error::<T>::TargetNotDPoS.into());
			};

			ensure!(accept_delegations, Error::<T>::TargetDeniesDelegations);

			Self::do_stake_nft(&target, details, &caller, &item_id)
		}

		#[pallet::call_index(12)]
		pub fn undelegate_currency(
			origin: OriginFor<T>,
			amount: T::Balance,
			target: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidTarget);

			let details =
				Validators::<T>::get(&target).current().cloned().ok_or(Error::<T>::NotBound)?;
			ensure!(details.permission() == PermissionType::DPoS, Error::<T>::TargetNotDPoS);

			Self::do_unstake_currency(&target, &caller, amount)
		}

		#[pallet::call_index(13)]
		pub fn undelegate_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
			target: T::AccountId,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(caller != target, Error::<T>::InvalidTarget);

			let permission = Validators::<T>::get(&target)
				.current()
				.map(|details| details.permission())
				.ok_or(Error::<T>::NotBound)?;

			ensure!(permission == PermissionType::DPoS, Error::<T>::TargetNotDPoS);

			Self::do_unstake_nft(&target, &caller, &item_id)
		}

		#[pallet::call_index(14)]
		pub fn set_minium_staking_period(
			origin: OriginFor<T>,
			new_min_period: u32,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let mut details =
				Validators::<T>::get(&caller).current().cloned().ok_or(Error::<T>::NotBound)?;

			let is_chilled = Self::is_chilled(&caller);
			ensure!(!is_chilled, Error::<T>::TargetIsChilled);

			let ValidatorDetails::DPoS { min_staking_period, .. } = &mut details else {
				return Err(Error::<T>::TargetNotDPoS.into());
			};

			ensure!(
				new_min_period >= T::MinimumStakingPeriod::get().into(),
				Error::<T>::InvalidStakingPeriod
			);

			//TODO: should there be an upper limit too?

			*min_staking_period = new_min_period;

			// TODO(vismate): after implementing slippage this should be immidiate
			Validators::<T>::mutate(&caller, |s| s.stage(details));

			Ok(())
		}

		#[pallet::call_index(15)]
		pub fn set_commission(origin: OriginFor<T>, new_commission: Perbill) -> DispatchResult {
			let caller = ensure_signed(origin)?;

			let mut details =
				Validators::<T>::get(&caller).current().cloned().ok_or(Error::<T>::NotBound)?;

			ensure!(!Self::is_chilled(&caller), Error::<T>::TargetIsChilled);

			let ValidatorDetails::DPoS { commission, .. } = &mut details else {
				return Err(Error::<T>::TargetNotDPoS.into());
			};

			ensure!(
				new_commission >= T::MinimumCommissionRate::get(),
				Error::<T>::InvalidCommission
			);

			*commission = new_commission;

			// TODO(vismate): after implementing slippage this should be immidiate
			Validators::<T>::mutate(&caller, |s| s.stage(details));

			Ok(())
		}

		#[pallet::call_index(16)]
		pub fn kick(origin: OriginFor<T>, target: T::AccountId) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			//FIXME: InvalidTarget and TargetNotDPos refer to the caller but can be misconstrued
			ensure!(caller != target, Error::<T>::InvalidTarget);

			let details =
				Validators::<T>::get(&caller).current().cloned().ok_or(Error::<T>::NotBound)?;

			ensure!(details.permission() == PermissionType::DPoS, Error::<T>::TargetNotDPoS);

			ensure!(!Self::is_chilled(&caller), Error::<T>::TargetIsChilled);

			let mut contract = Contracts::<T>::get(&caller, &target)
				.current()
				.cloned()
				.ok_or(Error::<T>::NoContract)?;

			if contract.min_staking_period_end > SessionPallet::<T>::current_index() {
				return Err(Error::<T>::EarlyKick.into());
			}

			Self::stage_unlock_currency(&target, contract.stake.currency);
			for (item_id, _) in contract.stake.delegated_nfts.iter() {
				T::NftDelegationHandler::unbind(&target, item_id)?;
			}

			// FIXME?: this is a staged action, so in the current session this delegator's stake is still included.
			Self::shrink_total_stake_by(contract.stake.total());

			// Note: contracts witth zero total stake are removed in next session,
			// but the delegator can always just restake even in this session
			contract.stake = Default::default();

			Contracts::<T>::mutate(&caller, &target, |s| s.stage(contract));

			Ok(())
		}

		#[pallet::call_index(17)]
		pub fn topup(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
			allowed_amount: T::Balance,
		) -> DispatchResult {
			let caller = ensure_signed(origin)?;
			ensure!(T::NftStakingHandler::owner(&item_id)? == caller, Error::<T>::TopupWrongOwner);
			let nominal_value = T::NftStakingHandler::nominal_value(&item_id)?;
			let issued_nominal_value = T::NftStakingHandler::issued_nominal_value(&item_id)?;

			let imbalance = issued_nominal_value - nominal_value;

			ensure!(imbalance <= allowed_amount, Error::<T>::SlippageExceeded);

			Contracts::<T>::mutate(&caller, &caller, |s| {
				let Some(mut new) = s.current().cloned() else {
					return;
				};

				new.stake.permission_nft = Some(issued_nominal_value);

				Self::grow_total_stake_by(imbalance);

				ValidatorStates::<T>::mutate_extant(&caller, |vstate| {
					*vstate = match *vstate {
						ValidatorState::Faulted => ValidatorState::Normal,
						s => s,
					}
				});

				s.stage(new);
			});

			<T as pallet::Config>::Currency::withdraw(
				&caller,
				imbalance,
				//TODO: Is `all` okay for us? In this entire pallet...
				WithdrawReasons::all(),
				ExistenceRequirement::KeepAlive,
			)?;

			T::NftStakingHandler::set_nominal_value(&item_id, issued_nominal_value)
		}
	}

	impl<T: Config> ValidatorSet<T::AccountId> for Pallet<T> {
		type ValidatorId = T::AccountId;
		type ValidatorIdOf = ConvertInto;

		fn session_index() -> SessionIndex {
			SessionPallet::<T>::current_index()
		}

		fn validators() -> SpVec<Self::ValidatorId> {
			ValidatorStates::<T>::iter()
				.filter_map(|(validator_id, vstate)| match vstate {
					ValidatorState::Normal | ValidatorState::Faulted => Some(validator_id),
					_ => None,
				})
				.collect()
		}
	}

	// TODO: Can we not do silly things like this?
	pub struct ValidatorOf<T>(PhantomData<T>);

	impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for ValidatorOf<T> {
		fn convert(account: T::AccountId) -> Option<T::AccountId> {
			Some(account)
		}
	}

	impl<T: Config> ValidatorSetWithIdentification<T::AccountId> for Pallet<T> {
		type Identification = T::AccountId;
		type IdentificationOf = ValidatorOf<T>;
	}

	impl<T: Config>
		utils::traits::OnDelegationNftExpire<
			T::AccountId,
			<T as NftsConfig>::ItemId,
			T::Balance,
			T::AccountId,
		> for Pallet<T>
	{
		fn on_expire(
			owner: &T::AccountId,
			validator: Option<T::AccountId>,
			item_id: &<T as NftsConfig>::ItemId,
			nominal_value: &T::Balance,
		) {
			let Some(validator) = validator else {
				return;
			};

			let Some(mut contract) = Contracts::<T>::get(&validator, owner).current().cloned()
			else {
				return;
			};

			if let Some(index) =
				contract.stake.delegated_nfts.iter().position(|(x, _)| x == item_id)
			{
				contract.stake.delegated_nfts.remove(index);
			} else {
				return;
			}

			// Note: the nft is still considered in the session it expires, the delegator may still receive reward
			// We have ensured that this nominal value was staked for at least the minimum staking period when staking.
			Contracts::<T>::mutate(&validator, owner, |s| s.stage(contract));
			Self::shrink_total_stake_by(*nominal_value);
		}
	}

	impl<T: Config> utils::traits::SessionHook for Pallet<T>
	where
		T::AccountId: From<<T as pallet_session::Config>::ValidatorId>,
	{
		fn session_ended(_: u32) -> DispatchResult {
			let active_validators = SessionPallet::<T>::validators();
			// FIXME: replace active validator len with total number of blocks created in session
			// TODO: set this to a more sensible value
			let session_reward = 100 * active_validators.len() as u128;

			let rewarded = active_validators.into_iter().filter_map(|v| {
				let v = T::AccountId::from(v);
				(!InverseSlashes::<T>::contains_key(&v)).then_some(v)
			});

			Self::do_reward_participants(rewarded, session_reward);
			Self::do_slash_participants();

			Self::commit_storage();

			for (staker, amount) in UnlockingCurrency::<T>::drain() {
				Self::unlock_currency(&staker, amount);
			}

			for (item_id, staker) in UnlockingDelegatorNfts::<T>::drain() {
				T::NftDelegationHandler::unbind(&staker, &item_id).expect("could unbind nft");
			}

			Ok(())
		}
	}

	// TODO: make id tuple more generic
	// TODO: define weights
	// TODO: This trait seems to not quite fit our use-case perfectly. What should we do?
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

			frame_support::weights::Weight::default()
		}
	}
}
