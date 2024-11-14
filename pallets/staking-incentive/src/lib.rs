#![cfg_attr(not(feature = "std"), no_std)]
// Expect lints caused by procmacros
#![expect(clippy::manual_inspect)]

use sdk::{frame_support, frame_system, sp_arithmetic};

use frame_support::{
	pallet_prelude::*,
	sp_runtime::{
		traits::{
			AccountIdConversion, AtLeast32BitUnsigned, BlockNumberProvider, Bounded, CheckedDiv,
			Convert, One, Zero,
		},
		FixedU128, PerThing, Perbill, Saturating,
	},
	traits::{
		fungible::{Inspect, Mutate, Unbalanced},
		tokens::Preservation,
	},
	PalletId,
};
use frame_system::pallet_prelude::*;
use sp_arithmetic::SignedRounding;

use utils::{
	traits::{HoldVestingSchedule, StakingHooks},
	vesting::Schedule,
};

pub use pallet::*;

// TODO: Once the pallet is ready turn off dev_mode
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;

		type Balance: Member
			+ Parameter
			+ AtLeast32BitUnsigned
			+ MaybeSerializeDeserialize
			+ Default;

		type Fungible: Inspect<Self::AccountId, Balance = Self::Balance> + Mutate<Self::AccountId>;

		type VestingSchedule: HoldVestingSchedule<
			Self::AccountId,
			Fungible = Self::Fungible,
			BlockNumber = BlockNumberFor<Self>,
		>;

		/// The number of blocks that the claimed tokens should
		/// be vested for from when it's claimed.
		type ClaimVestingScheduleLength: Get<u32>;

		/// Active score is mutiplied by this value or every block.
		type PerBlockMultiplier: Get<FixedU128>;

		type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

		type BalanceToScore: Convert<Self::Balance, FixedU128>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;

		#[pallet::constant]
		type MaxPayouts: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		NewPayout { amount: T::Balance, index: u32 },
		Claim { account: T::AccountId, amount: T::Balance, payout_index: u32 },
		FuseTripped,
		FuseReset,
	}

	#[pallet::error]
	pub enum Error<T> {
		InsufficientFundsInPool,
		ZeroTotalEffectiveScore,
		MaxPayoutsReached,
		FuseIsTripped,
	}

	#[derive(Debug, Clone, Encode, Decode, TypeInfo)]
	pub struct Payout<BlockNumber, Balance> {
		pub block: BlockNumber,
		pub amount: Balance,
		pub score_cut_to: FixedU128, // 1 - `score_cut`
		pub total_effective_score: FixedU128,
	}

	pub type PayoutFor<T> = Payout<BlockNumberFor<T>, <T as Config>::Balance>;

	#[derive(Debug, Default, Clone, Encode, Decode, TypeInfo)]
	pub struct Score<BlockNumber> {
		pub active: FixedU128,
		pub passive: FixedU128,
		pub base: FixedU128,
		pub last_update: BlockNumber,
	}

	impl<BlockNumber: AtLeast32BitUnsigned + Copy> Score<BlockNumber> {
		/// Update the exponential growth of the active score
		/// Score left as is if `self.last_update` >= `now`
		pub fn update_active(
			&mut self,
			now: BlockNumber,
			multiplier: FixedU128,
			rounding: SignedRounding,
		) {
			match Self::time_delta_usize(self.last_update, now) {
				Some(delta) if !delta.is_zero() => {
					self.active = self
						.active
						.const_checked_mul_with_rounding(multiplier.saturating_pow(delta), rounding)
						.unwrap_or_else(FixedU128::max_value);
					self.last_update = now;
				},
				_ => {},
			}
		}

		/// Cut the gained and passive score after a payout by multiplying with `cut`
		pub fn cut(&mut self, cut_to: FixedU128) {
			self.passive = self.passive.saturating_mul(cut_to);
			self.active = self.base.saturating_add(self.gained().saturating_mul(cut_to));
		}

		/// Score accumulated by the exponential growth of the active score
		pub fn gained(&self) -> FixedU128 {
			self.active.saturating_sub(self.base)
		}

		/// Score according to which the final weights are calculated upon payout
		pub fn effective(&self) -> FixedU128 {
			self.gained().saturating_add(self.passive)
		}

		/// Calculate the threshold needed for score passivation
		///
		/// Returns `None` if `first_action` >= `now`
		pub fn passivation_threshold(
			&self,
			first_action: BlockNumber,
			now: BlockNumber,
			multiplier: FixedU128,
		) -> Option<FixedU128> {
			Self::threshold_base(first_action, now, multiplier, self.gained())
		}

		/// Calculate the threshold needed for score activation
		///
		/// Returns `None` if `first_action` >= `now`
		pub fn activation_threshold(
			&self,
			first_action: BlockNumber,
			now: BlockNumber,
			multiplier: FixedU128,
		) -> Option<FixedU128> {
			Self::threshold_base(first_action, now, multiplier, self.passive)
		}

		/// Base calculation for activation and passivation thresholds
		///
		/// Returns `None` if `first_action` >= `now`
		fn threshold_base(
			first_action: BlockNumber,
			now: BlockNumber,
			multiplier: FixedU128,
			score: FixedU128,
		) -> Option<FixedU128> {
			let delta = Self::time_delta_usize(first_action, now)?;
			let divisor = multiplier.saturating_pow(delta) - One::one();
			score.checked_div(&divisor)
		}

		/// Calculate the difference of two points in time
		///
		/// Returns `None` if `past` > `now` or the result is not representable by `usize`
		fn time_delta_usize(past: BlockNumber, now: BlockNumber) -> Option<usize> {
			now.checked_sub(&past).and_then(|v| v.try_into().ok())
		}
	}

	#[derive(Debug, Default, Clone, Encode, Decode, TypeInfo)]
	pub struct AccountData<BlockNumber> {
		/// Account's base, active and passive score
		pub score: Score<BlockNumber>,
		/// Number of claimed payouts
		pub claimed_payouts: u32,
		/// The block number of the first interaction with
		/// the staking mechanism
		pub first_action: Option<BlockNumber>,
	}

	pub type AccountDataFor<T> = AccountData<BlockNumberFor<T>>;

	#[derive(Debug, Default, Clone, Encode, Decode, TypeInfo)]
	pub struct GlobalData<BlockNumber, Balance> {
		/// The global base, active and passive score
		pub score: Score<BlockNumber>,
		/// The amount available for new payouts in the pool.
		/// The entire balance in the pool's account might not all be available
		/// as the not yet claimed payouts still sit there.
		pub available_pool_balance: Balance,
	}

	pub type GlobalDataFor<T> = GlobalData<BlockNumberFor<T>, <T as Config>::Balance>;

	#[pallet::storage]
	pub type Payouts<T: Config> =
		StorageValue<_, BoundedVec<PayoutFor<T>, T::MaxPayouts>, ValueQuery>;

	#[pallet::storage]
	pub type Global<T: Config> = StorageValue<_, GlobalDataFor<T>, ValueQuery>;

	#[pallet::storage]
	pub type Accounts<T: Config> =
		StorageMap<_, Twox64Concat, T::AccountId, AccountDataFor<T>, ValueQuery>;

	/// A fuse that can be tripped on a catastrophic error to stop
	/// the pallet from causing further damage. `true` means the fuse is
	/// tripped.
	#[pallet::storage]
	pub type Fuse<T: Config> = StorageValue<_, bool, ValueQuery>;

	macro_rules! ensure_fuse {
		() => {
			ensure_fuse!(Err(Error::<T>::FuseIsTripped.into()))
		};
		($x:expr) => {
			if Fuse::<T>::get() {
				return $x;
			}
		};
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub incentive_pool: T::Balance,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { incentive_pool: Zero::zero() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			let pool_account = Pallet::<T>::pool_account();

			let minted = T::Fungible::mint_into(
				&pool_account,
				self.incentive_pool.clone().saturating_add(T::Fungible::minimum_balance()),
			)
			.expect("could endow incentive pool");
			// Take the amount out of the circulating supply
			T::Fungible::deactivate(minted);

			Global::<T>::put(GlobalData {
				available_pool_balance: self.incentive_pool.clone(),
				..Default::default()
			});
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		pub fn new_payout(
			origin: OriginFor<T>,
			amount: T::Balance,
			score_cut: Perbill,
		) -> DispatchResult {
			ensure_fuse!();
			ensure_root(origin)?;

			Global::<T>::mutate(|global| {
				ensure!(
					amount <= global.available_pool_balance,
					Error::<T>::InsufficientFundsInPool
				);

				let block = T::BlockNumberProvider::current_block_number();
				global.score.update_active(
					block,
					T::PerBlockMultiplier::get(),
					SignedRounding::High,
				);

				let total_effective_score = global.score.effective();

				if total_effective_score.is_zero() {
					return Err(Error::<T>::ZeroTotalEffectiveScore.into());
				}

				let score_cut_to = score_cut.left_from_one().into();

				let payout =
					Payout { block, amount: amount.clone(), score_cut_to, total_effective_score };

				let index: u32 = Payouts::<T>::mutate(|ps| match ps.try_push(payout) {
					Ok(_) => Ok(ps.len() as u32),
					Err(_) => Err(Error::<T>::MaxPayoutsReached),
				})?;

				global.score.cut(score_cut_to);
				global.available_pool_balance -= amount.clone();

				Self::deposit_event(Event::<T>::NewPayout { amount, index });

				Ok(())
			})
		}

		#[pallet::call_index(1)]
		pub fn reset_fuse(origin: OriginFor<T>) -> DispatchResult {
			ensure_root(origin)?;
			Fuse::<T>::put(false);
			Self::deposit_event(Event::<T>::FuseReset);
			Ok(())
		}

		#[pallet::call_index(2)]
		pub fn update_and_claim(origin: OriginFor<T>) -> DispatchResult {
			ensure_fuse!();
			let who = ensure_signed(origin)?;

			Self::on_action(&who, |_, _, _| {});

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn pool_account() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}

		fn trip_fuse() {
			Fuse::<T>::put(true);
			Self::deposit_event(Event::<T>::FuseTripped);
		}

		fn do_payout_account(account_id: &T::AccountId, account_data: &mut AccountDataFor<T>) {
			let payouts = Payouts::<T>::get();
			let n_payouts = payouts
				.len()
				.try_into()
				.expect("length can be represented by u32 as the bound is u32");

			// Clone score so if fuse is tripped we can discard changes
			let mut score = account_data.score.clone();

			let total_payout: T::Balance =
				payouts.into_iter().skip(account_data.claimed_payouts as usize).fold(
					Zero::zero(),
					|total_payout,
					 Payout { block, amount, score_cut_to, total_effective_score }| {
						score.update_active(
							block,
							T::PerBlockMultiplier::get(),
							SignedRounding::Low,
						);

						let ratio = score
							.effective()
							.checked_rounding_div(total_effective_score, SignedRounding::Low)
							.expect("total_effective_score > 0");
						let payout = ratio.into_perbill() * amount; // multiplying by c in [0, 1] cannot overflow

						score.cut(score_cut_to);

						total_payout.saturating_add(payout)
					},
				);

			let per_block = total_payout
				.checked_div(&T::ClaimVestingScheduleLength::get().into())
				.filter(|v| !v.is_zero())
				.unwrap_or_else(One::one);

			let schedule = Schedule {
				locked: total_payout.clone(),
				per_block,
				starting_block: Some(T::BlockNumberProvider::current_block_number()),
			};

			if T::VestingSchedule::can_add_vesting_schedule(account_id, &schedule).is_ok() {
				// Transfer fails if it can't transfer the amount in full
				let Ok(_) = T::Fungible::transfer(
					&Self::pool_account(),
					account_id,
					total_payout.clone(),
					Preservation::Expendable,
				) else {
					// This should never happen as
					// upon creating a new payout we ensure the
					// pool still has enough funds.
					Self::trip_fuse();
					return;
				};

				T::VestingSchedule::add_vesting_schedule(account_id, schedule)
					.expect("all conditions are checked before");

				T::Fungible::reactivate(total_payout);
				account_data.score = score;
				account_data.claimed_payouts = n_payouts;
			}
		}

		fn on_action(
			account_id: &T::AccountId,
			update_score: impl Fn(&mut GlobalDataFor<T>, &mut AccountDataFor<T>, BlockNumberFor<T>),
		) {
			Global::<T>::mutate(|global| {
				let now = T::BlockNumberProvider::current_block_number();
				global
					.score
					.update_active(now, T::PerBlockMultiplier::get(), SignedRounding::High);

				Accounts::<T>::mutate(account_id, |account| {
					Self::do_payout_account(account_id, account);
					account.score.update_active(
						now,
						T::PerBlockMultiplier::get(),
						SignedRounding::Low,
					);

					update_score(global, account, now);
				});
			});
		}

		fn stake_action(account_id: &T::AccountId, amount: T::Balance) {
			let amount = T::BalanceToScore::convert(amount);
			Self::on_action(account_id, |global, account, now| {
				let activate_amount = if let Some(first_action) = account.first_action {
					Self::amount_to_activate(&account.score, amount, first_action, now)
						.unwrap_or_default()
				} else {
					account.first_action = Some(now);
					Zero::zero()
				};

				account.score.base = account.score.base.saturating_add(amount);
				global.score.base = global.score.base.saturating_add(amount);

				account.score.passive = account.score.passive.saturating_sub(activate_amount);
				global.score.passive = global.score.passive.saturating_sub(activate_amount);

				let active_extra = activate_amount.saturating_add(amount);

				account.score.active = account.score.active.saturating_add(active_extra);
				global.score.active = global.score.active.saturating_add(active_extra);
			});
		}

		fn unstake_action(account_id: &T::AccountId, amount: T::Balance) {
			let amount = T::BalanceToScore::convert(amount);
			Self::on_action(account_id, |global, account, now| {
				// The first action must be a stake so we disregard this event and refrain from
				// further updating the score
				let Some(first_action) = account.first_action else {
					frame_support::sp_runtime::print("ERROR: on_unstake was called before first on_stake in pallet_staking_incentive");
					return;
				};

				let passivate_amount =
					Self::amount_to_passivate(&account.score, amount, first_action, now)
						.unwrap_or_default();

				account.score.base = account.score.base.saturating_sub(amount);
				global.score.base = global.score.base.saturating_sub(amount);

				account.score.passive = account.score.passive.saturating_add(passivate_amount);
				global.score.passive = global.score.passive.saturating_add(passivate_amount);

				let passive_extra = passivate_amount.saturating_add(amount);

				account.score.active = account.score.active.saturating_sub(passive_extra);
				global.score.active = global.score.active.saturating_sub(passive_extra);
			});
		}

		fn amount_to_activate(
			score: &Score<BlockNumberFor<T>>,
			amount: FixedU128,
			first_action: BlockNumberFor<T>,
			now: BlockNumberFor<T>,
		) -> Option<FixedU128> {
			let threshold =
				score.activation_threshold(first_action, now, T::PerBlockMultiplier::get())?;

			if score.base.saturating_add(amount) > threshold {
				Some(score.passive)
			} else {
				amount
					.saturating_mul(score.passive)
					.checked_div(&threshold.saturating_sub(score.base))
			}
		}

		fn amount_to_passivate(
			score: &Score<BlockNumberFor<T>>,
			amount: FixedU128,
			first_action: BlockNumberFor<T>,
			now: BlockNumberFor<T>,
		) -> Option<FixedU128> {
			let threshold =
				score.passivation_threshold(first_action, now, T::PerBlockMultiplier::get())?;

			let base_remaining = score.base.saturating_sub(amount);
			let ratio = threshold.saturating_sub(base_remaining).checked_div(&threshold)?;

			Some(ratio * score.gained())
		}
	}

	impl<T: Config, ItemId> StakingHooks<T::AccountId, T::Balance, ItemId> for Pallet<T> {
		fn on_currency_stake(
			account_id: &T::AccountId,
			_validator: &T::AccountId,
			amount: T::Balance,
		) {
			ensure_fuse!({});
			Self::stake_action(account_id, amount);
		}

		fn on_currency_unstake(
			account_id: &T::AccountId,
			_validator: &T::AccountId,
			amount: T::Balance,
		) {
			ensure_fuse!({});
			Self::unstake_action(account_id, amount);
		}

		fn on_currency_slash(account_id: &T::AccountId, amount: T::Balance) {
			ensure_fuse!({});
			Self::unstake_action(account_id, amount);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn numerical_stability() {
		let stake1 = FixedU128::from_rational(1, 1);
		let multiplier = FixedU128::from_float(1.000_000_140_19); // ~2x every year

		let mut score1 =
			Score { active: stake1, passive: Zero::zero(), base: stake1, last_update: 0u32 };

		let mut score2 = score1.clone();

		// about 8 years
		for block in 1..=8 * 5_259_600 {
			score1.update_active(block, multiplier, SignedRounding::High); // round high as global score does
		}

		score2.update_active(8 * 5_259_600, multiplier, SignedRounding::Low); // round low as account's score does

		assert!((score2.active / score1.active) > FixedU128::from_rational(999_999, 1_000_000));
	}
}
