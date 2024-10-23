#![cfg_attr(not(feature = "std"), no_std)]
// Expect lints caused by procmacros
#![expect(clippy::manual_inspect)]

use sdk::{frame_support, frame_system, sp_core};

use frame_support::{
	pallet_prelude::{StorageMap, ValueQuery},
	sp_runtime::{
		traits::{AtLeast32BitUnsigned, BlockNumberProvider, Convert, Saturating, Zero},
		DispatchError, DispatchResult,
	},
	traits::{
		fungible::{Inspect, InspectFreeze, MutateFreeze},
		IsType,
	},
	BoundedVec, Parameter, Twox64Concat,
};
use frame_system::pallet_prelude::*;
use sp_core::Get;

use utils::traits::HoldVestingSchedule;

pub use pallet::*;

// TODO: Once the pallet is ready turn off dev_mode
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;
		type RuntimeFreezeReason: From<FreezeReason>;
		type Balance: Parameter + Copy + AtLeast32BitUnsigned + TryInto<BlockNumberFor<Self>>;

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

		#[pallet::constant]
		type MaxFrozenSchedules: Get<u32>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

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
		NoFrozenSchedules,
		MaxFrozenSchedulesReached,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		pub fn convert_schedule(origin: OriginFor<T>, schedule_index: u32) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let schedule = T::VestingSchedule::get_vesting_schedule(&who, schedule_index)?;

			// A schedule might never unlock if it has no definite starting block or `locked / per_block` cannot be represented as a block number
			// in this case we don't want to lock the value forever, instead we disallow such schedules.
			let thaws_on: BlockNumberFor<T> = schedule
				.ending_block_as_balance::<T::BlockNumberToBalance>()
				.and_then(|b| b.try_into().ok())
				.ok_or(Error::<T>::NeverThaws)?;

			let frozen_now = schedule.locked_at::<T::BlockNumberToBalance>(
				T::BlockNumberProvider::current_block_number(),
			);

			T::Fungible::increase_frozen(&FreezeReason::VestingToFreeze.into(), &who, frozen_now)?;

			FrozenSchedules::<T>::get(&who)
				.try_push((thaws_on, frozen_now))
				.map_err(|_| Error::<T>::MaxFrozenSchedulesReached)?;

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
