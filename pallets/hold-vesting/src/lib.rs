// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # Vesting Pallet
//!
//! - [`Config`]
//! - [`Call`]
//!
//! ## Overview
//!
//! A simple pallet providing a means of placing a linear curve on some of an account's balance. This
//! pallet ensures that there is a hold in place preventing the balance to drop below the *unvested*
//! amount for any reason.
//!
//! As the amount vested increases over time, the amount unvested reduces. However, holds remain in
//! place and explicit action is needed on behalf of the user to ensure that the amount held is
//! equivalent to the amount remaining to be vested. This is done through a dispatchable function,
//! either `vest` (in typical case where the sender is calling on their own behalf) or `vest_other`
//! in case the sender is calling on another account's behalf.
//!
//! ## Interface
//!
//! This pallet implements the `VestingSchedule` trait.
//!
//! ### Dispatchable Functions
//!
//! - `vest` - Update the hold, reducing it in line with the amount "vested" so far.
//! - `vest_other` - Update the hold of another account, reducing it in line with the amount
//!   "vested" so far.

#![cfg_attr(not(feature = "std"), no_std)]
// Expect lints caused by procmacros
#![expect(clippy::type_complexity)] // TODO: make it so this is not needed to be allowed

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

// #[cfg(test)]
// mod mock;
// #[cfg(test)]
// mod tests;

//pub mod migrations;
pub mod weights;

use sdk::{frame_support, frame_system, sp_runtime, sp_std};

use codec::MaxEncodedLen;
use frame_support::{
	dispatch::DispatchResult,
	ensure,
	pallet_prelude::{
		Blake2_128Concat, BuildGenesisConfig, DispatchResultWithPostInfo, Hooks, IsType,
		StorageMap, TypeInfo,
	},
	storage::bounded_vec::BoundedVec,
	traits::{
		fungible::{Inspect, InspectHold, Mutate, MutateHold},
		tokens::{Precision, Preservation},
		Get, VariantCount, VariantCountOf,
	},
	Parameter,
};
use frame_system::pallet_prelude::{ensure_root, ensure_signed, BlockNumberFor, OriginFor};

use sp_runtime::{
	traits::{
		AtLeast32BitUnsigned, BlockNumberProvider, Convert, MaybeSerializeDeserialize, One,
		Saturating, StaticLookup, Zero,
	},
	DispatchError,
};
use sp_std::{fmt::Debug, marker::PhantomData, prelude::*};

use utils::vesting::{HoldVestingSchedule, Schedule};

pub use pallet::*;
pub use weights::WeightInfo;

pub type MaxHoldsOf<T> = VariantCountOf<<T as Config>::RuntimeHoldReason>;
pub type AccountIdLookupOf<T> = <<T as frame_system::Config>::Lookup as StaticLookup>::Source;
pub type ScheduleOf<T> = Schedule<<T as Config>::Balance, BlockNumberFor<T>>;

/// Actions to take against a user's `Vesting` storage entry.
#[derive(Clone, Copy)]
enum VestingAction {
	/// Do not actively remove any schedules.
	Passive,
	/// Remove the schedule specified by the index.
	Remove { index: usize },
	/// Remove the two schedules, specified by index, so they can be merged.
	Merge { index1: usize, index2: usize },
}

impl VestingAction {
	/// Whether or not the filter says the schedule index should be removed.
	fn should_remove(&self, index: usize) -> bool {
		match self {
			Self::Passive => false,
			Self::Remove { index: index1 } => *index1 == index,
			Self::Merge { index1, index2 } => *index1 == index || *index2 == index,
		}
	}

	/// Pick the schedules that this action dictates should continue vesting undisturbed.
	fn pick_schedules<T: Config>(
		&self,
		schedules: Vec<ScheduleOf<T>>,
	) -> impl Iterator<Item = ScheduleOf<T>> + '_ {
		schedules.into_iter().enumerate().filter_map(move |(index, schedule)| {
			if self.should_remove(index) {
				None
			} else {
				Some(schedule)
			}
		})
	}
}

// Wrapper for `T::MAX_VESTING_SCHEDULES` to satisfy `trait Get`.
pub struct MaxVestingSchedulesGet<T>(PhantomData<T>);
impl<T: Config> Get<u32> for MaxVestingSchedulesGet<T> {
	fn get() -> u32 {
		T::MAX_VESTING_SCHEDULES
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;

		type RuntimeHoldReason: From<HoldReason> + VariantCount;

		type Balance: Parameter
			+ MaybeSerializeDeserialize
			+ Zero
			+ Saturating
			+ Copy
			+ AtLeast32BitUnsigned
			+ MaxEncodedLen
			+ TypeInfo;

		/// The currency trait.
		type Fungible: Inspect<Self::AccountId, Balance = Self::Balance>
			+ Mutate<Self::AccountId>
			+ MutateHold<Self::AccountId>
			+ InspectHold<Self::AccountId, Reason = Self::RuntimeHoldReason, Balance = Self::Balance>;

		/// Convert the block number into a balance.
		type BlockNumberToBalance: Convert<BlockNumberFor<Self>, Self::Balance>;

		/// The minimum amount transferred to call `vested_transfer`.
		#[pallet::constant]
		type MinVestedTransfer: Get<Self::Balance>;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;

		/// Provider for the block number.
		type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

		/// Maximum number of vesting schedules an account may have at a given moment.
		const MAX_VESTING_SCHEDULES: u32;
	}

	#[pallet::extra_constants]
	impl<T: Config> Pallet<T> {
		#[pallet::constant_name(MaxVestingSchedules)]
		fn max_vesting_schedules() -> u32 {
			T::MAX_VESTING_SCHEDULES
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn integrity_test() {
			assert!(T::MAX_VESTING_SCHEDULES > 0, "`MaxVestingSchedules` must be greater than 0");
		}
	}

	#[pallet::composite_enum]
	pub enum HoldReason {
		Vesting,
		#[cfg(feature = "runtime-benchmarks")]
		Test1,
		#[cfg(feature = "runtime-benchmarks")]
		Test2,
		#[cfg(feature = "runtime-benchmarks")]
		Test3,
		#[cfg(feature = "runtime-benchmarks")]
		Test4,
	}

	/// Information regarding the vesting of a given account.
	#[pallet::storage]
	#[pallet::getter(fn vesting)]
	pub type Vesting<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		BoundedVec<ScheduleOf<T>, MaxVestingSchedulesGet<T>>,
	>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::genesis_config]
	#[derive(frame_support::DefaultNoBound)]
	pub struct GenesisConfig<T: Config> {
		pub vesting: Vec<(T::AccountId, Option<BlockNumberFor<T>>, BlockNumberFor<T>, T::Balance)>,
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			// Generate initial vesting configuration
			// * who - Account which we are generating vesting configuration for
			// * begin - Block when the account will start to vest
			// * length - Number of blocks from `begin` until fully vested
			// * liquid - Number of units which can be spent before vesting begins
			for &(ref who, begin, length, liquid) in self.vesting.iter() {
				let balance = T::Fungible::balance(who);
				assert!(!balance.is_zero(), "Currencies must be init'd before vesting");
				// Total genesis `balance` minus `liquid` equals funds locked for vesting
				let locked = balance.saturating_sub(liquid + T::Fungible::minimum_balance());
				let length_as_balance = T::BlockNumberToBalance::convert(length).max(One::one());

				// div_ceil
				let per_block = (locked + length_as_balance - One::one()) / length_as_balance;

				let vesting_info = Schedule::new(locked, per_block, begin);
				if !vesting_info.is_valid() {
					panic!("Invalid Schedule params at genesis")
				};

				Vesting::<T>::try_append(who, vesting_info)
					.expect("Too many vesting schedules at genesis.");

				T::Fungible::set_on_hold(&HoldReason::Vesting.into(), who, locked)
					.expect("Can put calculated amount on hold");
			}
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The amount vested has been updated. This could indicate a change in funds available.
		/// The balance given is the amount which is left unvested (and thus locked).
		VestingUpdated { account: T::AccountId, unvested: T::Balance },
		/// An \[account\] has become fully vested.
		VestingCompleted { account: T::AccountId },
	}

	/// Error for the vesting pallet.
	#[pallet::error]
	pub enum Error<T> {
		/// The account given is not vesting.
		NotVesting,
		/// The account already has `MaxVestingSchedules` count of schedules and thus
		/// cannot add another one. Consider merging existing schedules in order to add another.
		AtMaxVestingSchedules,
		/// Amount being transferred is too low to create a vesting schedule.
		AmountLow,
		/// An index was out of bounds of the vesting schedules.
		ScheduleIndexOutOfBounds,
		/// Failed to create a new schedule because some parameter was invalid.
		InvalidScheduleParams,
		/// Tried to merge schedules where at least one has no definite starting block.
		NoDefiniteStartingBlock,
		/// The starting block of a vesting schedule has already been set.
		StartingBlockAlreadySet,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Unlock any vested funds of the sender account.
		///
		/// The dispatch origin for this call must be _Signed_ and the sender must have funds still
		/// locked under this pallet.
		///
		/// Emits either `VestingCompleted` or `VestingUpdated`.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::vest_locked(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES)
			.max(T::WeightInfo::vest_unlocked(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES))
		)]
		pub fn vest(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Self::do_vest(who)
		}

		/// Unlock any vested funds of a `target` account.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `target`: The account whose vested funds should be unlocked. Must have funds still
		/// locked under this pallet.
		///
		/// Emits either `VestingCompleted` or `VestingUpdated`.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::vest_other_locked(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES)
			.max(T::WeightInfo::vest_other_unlocked(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES))
		)]
		pub fn vest_other(origin: OriginFor<T>, target: AccountIdLookupOf<T>) -> DispatchResult {
			ensure_signed(origin)?;
			let who = T::Lookup::lookup(target)?;
			Self::do_vest(who)
		}

		/// Create a vested transfer.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `target`: The account receiving the vested funds.
		/// - `schedule`: The vesting schedule attached to the transfer.
		///
		/// Emits `VestingCreated`.
		///
		/// NOTE: This will unlock all schedules through the current block.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(2)]
		#[pallet::weight(
			T::WeightInfo::vested_transfer(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES)
		)]
		pub fn vested_transfer(
			origin: OriginFor<T>,
			target: AccountIdLookupOf<T>,
			schedule: ScheduleOf<T>,
		) -> DispatchResult {
			let transactor = ensure_signed(origin)?;
			let transactor = <T::Lookup as StaticLookup>::unlookup(transactor);
			Self::do_vested_transfer(transactor, target, schedule)
		}

		/// Force a vested transfer.
		///
		/// The dispatch origin for this call must be _Root_.
		///
		/// - `source`: The account whose funds should be transferred.
		/// - `target`: The account that should be transferred the vested funds.
		/// - `schedule`: The vesting schedule attached to the transfer.
		///
		/// Emits `VestingCreated`.
		///
		/// NOTE: This will unlock all schedules through the current block.
		///
		/// ## Complexity
		/// - `O(1)`.
		#[pallet::call_index(3)]
		#[pallet::weight(
			T::WeightInfo::force_vested_transfer(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES)
		)]
		pub fn force_vested_transfer(
			origin: OriginFor<T>,
			source: AccountIdLookupOf<T>,
			target: AccountIdLookupOf<T>,
			schedule: ScheduleOf<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			Self::do_vested_transfer(source, target, schedule)
		}

		/// Merge two vesting schedules together, creating a new vesting schedule that unlocks over
		/// the highest possible start and end blocks. If both schedules have already started the
		/// current block will be used as the schedule start; with the caveat that if one schedule
		/// is finished by the current block, the other will be treated as the new merged schedule,
		/// unmodified.
		///
		/// NOTE: If `schedule1_index == schedule2_index` this is a no-op.
		/// NOTE: This will unlock all schedules through the current block prior to merging.
		/// NOTE: If both schedules have ended by the current block, no new schedule will be created
		/// and both will be removed.
		///
		/// Merged schedule attributes:
		/// - `starting_block`: `MAX(schedule1.starting_block, scheduled2.starting_block,
		///   current_block)`.
		/// - `ending_block`: `MAX(schedule1.ending_block, schedule2.ending_block)`.
		/// - `locked`: `schedule1.locked_at(current_block) + schedule2.locked_at(current_block)`.
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `schedule1_index`: index of the first schedule to merge.
		/// - `schedule2_index`: index of the second schedule to merge.
		#[pallet::call_index(4)]
		#[pallet::weight(
			T::WeightInfo::not_unlocking_merge_schedules(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES)
			.max(T::WeightInfo::unlocking_merge_schedules(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES))
		)]
		pub fn merge_schedules(
			origin: OriginFor<T>,
			schedule1_index: u32,
			schedule2_index: u32,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			if schedule1_index == schedule2_index {
				return Ok(());
			};
			let schedule1_index = schedule1_index as usize;
			let schedule2_index = schedule2_index as usize;

			let schedules = Self::vesting(&who).ok_or(Error::<T>::NotVesting)?;
			let merge_action =
				VestingAction::Merge { index1: schedule1_index, index2: schedule2_index };

			let (schedules, locked_now) = Self::exec_action(schedules.to_vec(), merge_action)?;

			Self::write_vesting(&who, schedules)?;
			Self::write_lock(&who, locked_now);

			Ok(())
		}

		/// Force remove a vesting schedule
		///
		/// The dispatch origin for this call must be _Root_.
		///
		/// - `target`: An account that has a vesting schedule
		/// - `schedule_index`: The vesting schedule index that should be removed
		#[pallet::call_index(5)]
		#[pallet::weight(
			T::WeightInfo::force_remove_vesting_schedule(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES)
		)]
		pub fn force_remove_vesting_schedule(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			schedule_index: u32,
		) -> DispatchResultWithPostInfo {
			ensure_root(origin)?;
			let who = T::Lookup::lookup(target)?;

			let schedules_count = Vesting::<T>::decode_len(&who).unwrap_or_default();
			ensure!(schedule_index < schedules_count as u32, Error::<T>::InvalidScheduleParams);

			Self::remove_vesting_schedule(&who, schedule_index)?;

			Ok(Some(T::WeightInfo::force_remove_vesting_schedule(
				MaxHoldsOf::<T>::get(),
				schedules_count as u32,
			))
			.into())
		}

		/// Sets the starting block of a schedule if it's not already set.
		///
		/// The dispatch origin for this call must be _Root_.
		///
		/// - `target`: An account that has a vesting schedule
		/// - `schedule_index`: The vesting schedule index to be updated
		/// - `starting_block`: The block on which the schedule starts
		///
		/// If the starting block is already set
		#[pallet::call_index(6)]
		#[pallet::weight(
			T::WeightInfo::start_vesting_schedule_locked(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES)
			.max(T::WeightInfo::start_vesting_schedule_unlocked(MaxHoldsOf::<T>::get(), T::MAX_VESTING_SCHEDULES))
		)]
		pub fn start_vesting_schedule(
			origin: OriginFor<T>,
			target: <T::Lookup as StaticLookup>::Source,
			schedule_index: u32,
			starting_block: BlockNumberFor<T>,
		) -> DispatchResult {
			ensure_root(origin)?;
			let who = T::Lookup::lookup(target)?;

			let mut schedules = Self::vesting(&who).ok_or(Error::<T>::NotVesting)?;
			let schedule = schedules
				.get_mut(schedule_index as usize)
				.ok_or(Error::<T>::InvalidScheduleParams)?;

			if schedule.starting_block.is_none() {
				schedule.starting_block = Some(starting_block);
			} else {
				return Err(Error::<T>::StartingBlockAlreadySet.into());
			}

			let (schedules, locked_now) =
				Self::exec_action(schedules.to_vec(), VestingAction::Passive)?;

			Self::write_vesting(&who, schedules)?;
			Self::write_lock(&who, locked_now);

			Ok(())
		}
	}
}

impl<T: Config> Pallet<T> {
	// Create a new `Schedule`, based off of two other `Schedule`s.
	// NOTE: We assume both schedules have had funds unlocked up through the current block.
	fn merge_vesting_info(
		now: BlockNumberFor<T>,
		schedule1: ScheduleOf<T>,
		schedule2: ScheduleOf<T>,
	) -> Result<Option<ScheduleOf<T>>, DispatchError> {
		let (s1_start, s2_start) = schedule1
			.starting_block()
			.zip(schedule2.starting_block())
			.ok_or(Error::<T>::NoDefiniteStartingBlock)?;

		// We know that if a schedule has a definite starting block then it must also have a definite ending block as well.
		let (s1_end, s2_end) = schedule1
			.ending_block_as_balance::<T::BlockNumberToBalance>()
			.zip(schedule2.ending_block_as_balance::<T::BlockNumberToBalance>())
			.expect("has definite ending block");

		let now_as_balance = T::BlockNumberToBalance::convert(now);

		// Check if one or both schedules have ended.
		match (s1_end <= now_as_balance, s2_end <= now_as_balance) {
			// If both schedules have ended, we don't merge and exit early.
			(true, true) => return Ok(None),
			// If one schedule has ended, we treat the one that has not ended as the new
			// merged schedule.
			(true, false) => return Ok(Some(schedule2)),
			(false, true) => return Ok(Some(schedule1)),
			// If neither schedule has ended don't exit early.
			_ => {},
		}

		let locked = schedule1
			.locked_at::<T::BlockNumberToBalance>(now)
			.saturating_add(schedule2.locked_at::<T::BlockNumberToBalance>(now));
		// This shouldn't happen because we know at least one ending block is greater than now,
		// thus at least a schedule a some locked balance.
		debug_assert!(
			!locked.is_zero(),
			"merge_vesting_info validation checks failed to catch a locked of 0"
		);

		let ending_block = s1_end.max(s2_end);
		let starting_block = now.max(s1_start.max(s2_start));

		let per_block = {
			let duration = ending_block
				.saturating_sub(T::BlockNumberToBalance::convert(starting_block))
				.max(One::one());
			(locked / duration).max(One::one())
		};

		let schedule = Schedule::new(locked, per_block, Some(starting_block));
		debug_assert!(schedule.is_valid(), "merge_vesting_info schedule validation check failed");

		Ok(Some(schedule))
	}

	// Execute a vested transfer from `source` to `target` with the given `schedule`.
	fn do_vested_transfer(
		source: AccountIdLookupOf<T>,
		target: AccountIdLookupOf<T>,
		schedule: ScheduleOf<T>,
	) -> DispatchResult {
		// Validate user inputs.
		ensure!(schedule.locked() >= T::MinVestedTransfer::get(), Error::<T>::AmountLow);
		if !schedule.is_valid() {
			return Err(Error::<T>::InvalidScheduleParams.into());
		};
		let target = T::Lookup::lookup(target)?;
		let source = T::Lookup::lookup(source)?;

		// Check we can add to this account prior to any storage writes.
		Self::can_add_vesting_schedule(&target, &schedule)?;

		T::Fungible::transfer(&source, &target, schedule.locked(), Preservation::Expendable)?;

		// We can't let this fail because the currency transfer has already happened.
		let res = Self::add_vesting_schedule(&target, schedule);
		debug_assert!(res.is_ok(), "Failed to add a schedule when we had to succeed.");

		Ok(())
	}

	/// Iterate through the schedules to track the current locked amount and
	/// filter out completed and specified schedules.
	///
	/// Returns a tuple that consists of:
	/// - Vec of vesting schedules, where completed schedules and those specified
	///   by filter are removed. (Note the vec is not checked for respecting
	///   bounded length.)
	/// - The amount locked at the current block number based on the given schedules.
	///
	/// NOTE: the amount locked does not include any schedules that are filtered out via `action`.
	fn report_schedule_updates(
		schedules: Vec<ScheduleOf<T>>,
		action: VestingAction,
	) -> (Vec<ScheduleOf<T>>, T::Balance) {
		let now = T::BlockNumberProvider::current_block_number();

		let mut total_locked_now: T::Balance = Zero::zero();
		let filtered_schedules = action
			.pick_schedules::<T>(schedules)
			.filter(|schedule| {
				let locked_now = schedule.locked_at::<T::BlockNumberToBalance>(now);
				let keep = !locked_now.is_zero();
				if keep {
					total_locked_now = total_locked_now.saturating_add(locked_now);
				}
				keep
			})
			.collect::<Vec<_>>();

		(filtered_schedules, total_locked_now)
	}

	/// Write an accounts updated vesting lock to storage.
	fn write_lock(who: &T::AccountId, total_locked_now: T::Balance) {
		if total_locked_now.is_zero() {
			T::Fungible::release_all(&HoldReason::Vesting.into(), who, Precision::BestEffort)
				.expect("release_all with BestEffort should never fail");
			Self::deposit_event(Event::<T>::VestingCompleted { account: who.clone() });
		} else {
			T::Fungible::set_on_hold(&HoldReason::Vesting.into(), who, total_locked_now)
				.expect("total_locked_now should be valid at this point");
			Self::deposit_event(Event::<T>::VestingUpdated {
				account: who.clone(),
				unvested: total_locked_now,
			});
		};
	}

	/// Write an accounts updated vesting schedules to storage.
	fn write_vesting(
		who: &T::AccountId,
		schedules: Vec<ScheduleOf<T>>,
	) -> Result<(), DispatchError> {
		let schedules: BoundedVec<ScheduleOf<T>, MaxVestingSchedulesGet<T>> =
			schedules.try_into().map_err(|_| Error::<T>::AtMaxVestingSchedules)?;

		if schedules.len() == 0 {
			Vesting::<T>::remove(who);
		} else {
			Vesting::<T>::insert(who, schedules);
		}

		Ok(())
	}

	/// Unlock any vested funds of `who`.
	fn do_vest(who: T::AccountId) -> DispatchResult {
		let schedules = Self::vesting(&who).ok_or(Error::<T>::NotVesting)?;

		let (schedules, locked_now) =
			Self::exec_action(schedules.to_vec(), VestingAction::Passive)?;

		Self::write_vesting(&who, schedules)?;
		Self::write_lock(&who, locked_now);

		Ok(())
	}

	/// Execute a `VestingAction` against the given `schedules`. Returns the updated schedules
	/// and locked amount.
	fn exec_action(
		schedules: Vec<ScheduleOf<T>>,
		action: VestingAction,
	) -> Result<(Vec<ScheduleOf<T>>, T::Balance), DispatchError> {
		let (schedules, locked_now) = match action {
			VestingAction::Merge { index1: idx1, index2: idx2 } => {
				// The schedule index is based off of the schedule ordering prior to filtering out
				// any schedules that may be ending at this block.
				let schedule1 = *schedules.get(idx1).ok_or(Error::<T>::ScheduleIndexOutOfBounds)?;
				let schedule2 = *schedules.get(idx2).ok_or(Error::<T>::ScheduleIndexOutOfBounds)?;

				// The length of `schedules` decreases by 2 here since we filter out 2 schedules.
				// Thus we know below that we can push the new merged schedule without error
				// (assuming initial state was valid).
				let (mut schedules, mut locked_now) =
					Self::report_schedule_updates(schedules.to_vec(), action);

				let now = T::BlockNumberProvider::current_block_number();
				if let Some(new_schedule) = Self::merge_vesting_info(now, schedule1, schedule2)? {
					// Merging created a new schedule so we:
					// 1) need to add it to the accounts vesting schedule collection,
					schedules.push(new_schedule);
					// (we use `locked_at` in case this is a schedule that started in the past)
					let new_schedule_locked =
						new_schedule.locked_at::<T::BlockNumberToBalance>(now);
					// and 2) update the locked amount to reflect the schedule we just added.
					locked_now = locked_now.saturating_add(new_schedule_locked);
				} // In the None case there was no new schedule to account for.

				(schedules, locked_now)
			},
			_ => Self::report_schedule_updates(schedules.clone(), action),
		};

		debug_assert!(
			locked_now > Zero::zero() && !schedules.is_empty()
				|| locked_now == Zero::zero() && schedules.is_empty()
		);

		Ok((schedules, locked_now))
	}
}

impl<T: Config> HoldVestingSchedule<T::AccountId> for Pallet<T>
where
	T::Balance: MaybeSerializeDeserialize + Debug,
{
	type Fungible = T::Fungible;
	type BlockNumber = BlockNumberFor<T>;

	/// Get the amount that is currently being vested and cannot be transferred out of this account.
	fn vesting_balance(who: &T::AccountId) -> Option<T::Balance> {
		if let Some(v) = Self::vesting(who) {
			let now = T::BlockNumberProvider::current_block_number();
			let total_locked_now = v.iter().fold(Zero::zero(), |total, schedule| {
				schedule.locked_at::<T::BlockNumberToBalance>(now).saturating_add(total)
			});
			Some(T::Fungible::total_balance(who).min(total_locked_now))
		} else {
			None
		}
	}

	/// Adds a vesting schedule to a given account.
	///
	/// If the account has `MaxVestingSchedules`, an Error is returned and nothing
	/// is updated.
	///
	/// On success, a linearly reducing amount of funds will be locked. In order to realise any
	/// reduction of the lock over time as it diminishes, the account owner must use `vest` or
	/// `vest_other`.
	///
	/// Is a no-op if the amount to be vested is zero.
	fn add_vesting_schedule(who: &T::AccountId, schedule: ScheduleOf<T>) -> DispatchResult {
		if schedule.locked.is_zero() {
			return Ok(());
		}

		// Check for `per_block` or `locked` of 0.
		if !schedule.is_valid() {
			return Err(Error::<T>::InvalidScheduleParams.into());
		};

		let mut schedules = Self::vesting(who).unwrap_or_default();

		// NOTE: we must push the new schedule so that `exec_action`
		// will give the correct new locked amount.
		ensure!(schedules.try_push(schedule).is_ok(), Error::<T>::AtMaxVestingSchedules);

		let (schedules, locked_now) =
			Self::exec_action(schedules.to_vec(), VestingAction::Passive)?;

		Self::write_vesting(who, schedules)?;
		Self::write_lock(who, locked_now);

		Ok(())
	}

	// Ensure we can call `add_vesting_schedule` without error. This should always
	// be called prior to `add_vesting_schedule`.
	fn can_add_vesting_schedule(who: &T::AccountId, schedule: &ScheduleOf<T>) -> DispatchResult {
		// Check for `per_block` or `locked` of 0.
		if !schedule.is_valid() {
			return Err(Error::<T>::InvalidScheduleParams.into());
		}

		ensure!(
			(Vesting::<T>::decode_len(who).unwrap_or_default() as u32) < T::MAX_VESTING_SCHEDULES,
			Error::<T>::AtMaxVestingSchedules
		);

		Ok(())
	}

	fn get_vesting_schedule(
		who: &T::AccountId,
		schedule_index: u32,
	) -> Result<ScheduleOf<T>, DispatchError> {
		let schedules = Self::vesting(who).ok_or(Error::<T>::NotVesting)?;
		schedules
			.get(schedule_index as usize)
			.copied()
			.ok_or(Error::<T>::ScheduleIndexOutOfBounds.into())
	}

	/// Remove a vesting schedule for a given account.
	fn remove_vesting_schedule(who: &T::AccountId, schedule_index: u32) -> DispatchResult {
		let schedules = Self::vesting(who).ok_or(Error::<T>::NotVesting)?;
		let remove_action = VestingAction::Remove { index: schedule_index as usize };

		let (schedules, locked_now) = Self::exec_action(schedules.to_vec(), remove_action)?;

		Self::write_vesting(who, schedules)?;
		Self::write_lock(who, locked_now);
		Ok(())
	}
}
