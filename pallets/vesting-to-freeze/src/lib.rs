#![cfg_attr(not(feature = "std"), no_std)]
// Expect lints caused by procmacros
#![expect(clippy::manual_inspect)]

pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use sdk::{frame_support, frame_system, sp_core};

use frame_support::{
	pallet_prelude::{Hooks, StorageMap, ValueQuery},
	sp_runtime::{
		traits::{AtLeast32BitUnsigned, BlockNumberProvider, Convert, Saturating, Zero},
		DispatchError, DispatchResult,
	},
	traits::{
		fungible::{Inspect, InspectFreeze, MutateFreeze},
		IsType, VariantCount, VariantCountOf,
	},
	BoundedVec, Parameter, Twox64Concat,
};
use frame_system::pallet_prelude::*;
use sp_core::{Get, MaxEncodedLen};

use utils::traits::HoldVestingSchedule;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;
		type RuntimeFreezeReason: From<FreezeReason> + VariantCount;
		type Balance: Parameter
			+ Copy
			+ AtLeast32BitUnsigned
			+ TryInto<BlockNumberFor<Self>>
			+ MaxEncodedLen;

		type Fungible: Inspect<Self::AccountId, Balance = Self::Balance>
			+ InspectFreeze<Self::AccountId, Id = Self::RuntimeFreezeReason>
			+ MutateFreeze<Self::AccountId>;

		type VestingSchedule: HoldVestingSchedule<
			Self::AccountId,
			Fungible = Self::Fungible,
			BlockNumber = BlockNumberFor<Self>,
		>;

		type BlockNumberToBalance: Convert<BlockNumberFor<Self>, Self::Balance>;
		type BlockNumberProvider: BlockNumberProvider<BlockNumber = BlockNumberFor<Self>>;

		/// Maximum number of freezes, used for weight estimation.
		///
		/// Note: The de-facto fungible implementor `pallet_balances`
		/// updates freezes and locks together so it's recommended
		/// to provided a combined value.
		type MaxFreezes: Get<u32>;

		/// Maximum number of converted schedules.
		/// This value is enforced by the pallet.
		#[pallet::constant]
		type MaxFrozenSchedules: Get<u32>;

		/// Maximum number of vesting schedules that might be converted,
		/// used for weight calculation.
		type MaxVestingSchedules: Get<u32>;

		type WeightInfo: WeightInfo;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn integrity_test() {
			assert!(
				T::MaxFreezes::get() >= T::RuntimeFreezeReason::VARIANT_COUNT,
				"`MaxFreezes` must be at least the number of `RuntimeFreezeReason`s"
			);
			assert!(
				T::MaxFrozenSchedules::get() > 0,
				"`MaxFrozenSchedules` must be greater than 0"
			);
			assert!(
				T::MaxVestingSchedules::get() > 0,
				"`MaxVestingSchedules` must be greater than 0"
			);
		}
	}

	pub type MaxFreezesOf<T> = VariantCountOf<<T as Config>::RuntimeFreezeReason>;

	#[pallet::storage]
	pub type FrozenSchedules<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AccountId,
		BoundedVec<(BlockNumberFor<T>, T::Balance), T::MaxFrozenSchedules>,
		ValueQuery,
	>;

	#[pallet::composite_enum]
	pub enum FreezeReason {
		VestingToFreeze,
		#[cfg(feature = "runtime-benchmarks")]
		Test1,
		#[cfg(feature = "runtime-benchmarks")]
		Test2,
		#[cfg(feature = "runtime-benchmarks")]
		Test3,
		#[cfg(feature = "runtime-benchmarks")]
		Test4,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ConvertedSchedule {
			account_id: T::AccountId,
			schedule_index: u32,
			amount: T::Balance,
			thaws_on: BlockNumberFor<T>,
		},
		Thawed {
			account_id: T::AccountId,
			amount: T::Balance,
		},
	}

	#[pallet::error]
	pub enum Error<T> {
		NeverThaws,
		AlreadyExpired,
		NoFrozenSchedules,
		MaxFrozenSchedulesReached,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::convert_schedule(
			T::MaxVestingSchedules::get(),
			T::MaxFreezes::get(),
			T::MaxFrozenSchedules::get(),
		))]
		pub fn convert_schedule(origin: OriginFor<T>, schedule_index: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let schedule = T::VestingSchedule::get_vesting_schedule(&who, schedule_index)?;

			// A schedule might never unlock if it has no definite starting block or `locked / per_block` cannot be represented as a block number
			// in this case we don't want to lock the value forever, instead we disallow such schedules.
			let thaws_on: BlockNumberFor<T> = schedule
				.ending_block_as_balance::<T::BlockNumberToBalance>()
				.and_then(|b| b.try_into().ok())
				.ok_or(Error::<T>::NeverThaws)?;

			let now = T::BlockNumberProvider::current_block_number();
			let frozen_now = schedule.locked_at::<T::BlockNumberToBalance>(now);

			if thaws_on < now || frozen_now.is_zero() {
				return Err(Error::<T>::AlreadyExpired.into());
			}

			T::Fungible::increase_frozen(&FreezeReason::VestingToFreeze.into(), &who, frozen_now)?;

			FrozenSchedules::<T>::mutate(&who, |schedules| {
				schedules
					.try_push((thaws_on, frozen_now))
					.map_err(|_| Error::<T>::MaxFrozenSchedulesReached)
			})?;

			T::VestingSchedule::remove_vesting_schedule(&who, schedule_index)?;

			Self::deposit_event(Event::<T>::ConvertedSchedule {
				account_id: who,
				schedule_index,
				amount: frozen_now,
				thaws_on,
			});

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::thaw_expired(
			T::MaxFreezes::get(),
			T::MaxFrozenSchedules::get(),
		))]
		pub fn thaw_expired(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let thaw = FrozenSchedules::<T>::mutate_exists(&who, |maybe_schedules| {
				let schedules = maybe_schedules.as_mut().ok_or(Error::<T>::NoFrozenSchedules)?;

				let now = T::BlockNumberProvider::current_block_number();
				let mut thaw = T::Balance::zero();

				schedules.retain(|(unlocks_on, frozen)| {
					let keep = now < *unlocks_on;

					if !keep {
						thaw = thaw.saturating_add(*frozen);
					}

					keep
				});

				if schedules.is_empty() {
					*maybe_schedules = None;
				}

				Ok::<_, DispatchError>(thaw)
			})?;

			T::Fungible::decrease_frozen(&FreezeReason::VestingToFreeze.into(), &who, thaw)?;

			Self::deposit_event(Event::<T>::Thawed { account_id: who, amount: thaw });

			Ok(())
		}
	}
}
