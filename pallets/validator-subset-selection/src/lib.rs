//! Validator subset selection pallet
//! This module makes it possible to select a subset of the validator
//! NFT holders to create blocks in the next session.
//! The purpose is to maximize uniformity of weekly timeframe
//! returns(selection) in the population, while maximizing
//! unpredictability.
//! The module also manages the sessions.
//!
//! The Validator subset selection pallet provides the following features:
//!
//! - **Select subset**
//!
//! - **Change subset size**
//!
//! - **SessionManager implementation**
//!
//! - **ShouldEndSession implementation**
//!

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
pub mod benchmarking;

use core::marker::PhantomData;

use frame_support::{pallet_prelude::*, traits::ValidatorSet};
pub use pallet::*;
use sp_application_crypto::Ss58Codec;
use sp_runtime::PerThing;
use sp_runtime::{FixedI64, FixedPointNumber};
use sp_std::prelude::*;

pub trait RandomU128 {
	fn random(subject: &[u8]) -> u128;
}

///Convert a FixedI64 to a float for logging
#[allow(dead_code)]
fn to_float(input: FixedI64) -> f64 {
	input.into_inner() as f64 / <FixedI64 as FixedPointNumber>::DIV as f64
}

#[frame_support::pallet(dev_mode)] //TODO: remove dev mode
pub mod pallet {
	use super::*;
	use frame_support::traits::BuildGenesisConfig;
	use frame_system::pallet_prelude::*;
	use pallet_session::ShouldEndSession;
	use utils::traits::SessionHook;

	//If a validator's bucket is full, then the bucket value is decreased with decrease_ratio
	//and the other disperse_ratio=(1-decrease_ratio) is divided among all buckets
	const DECREASE_RATIO: FixedI64 = FixedI64::from_rational(1, 2);
	const DISPERSE_RATIO: FixedI64 = FixedI64::from_u32(1).sub(DECREASE_RATIO);

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type ValidatorId: Member + Parameter + Ss58Codec;
		type RandomGenerator: RandomU128;
		type ValidatorSuperset: ValidatorSet<Self::ValidatorId, ValidatorId = Self::ValidatorId>;

		#[pallet::constant]
		type MinSessionLength: Get<BlockNumberFor<Self>>;

		type SessionHook: SessionHook;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FewerValidatorsThanSubset,
		EmptySubset,
		SubsetSelected {
			validator_subset: Vec<T::ValidatorId>,
			session_start: BlockNumberFor<T>,
			session_end: BlockNumberFor<T>,
			session_index: sp_staking::SessionIndex,
		},
		SubsetSizeChanged(u64),
	}

	///Helper value for generating unique random numbers
	#[pallet::storage]
	pub type Nonce<T: Config> = StorageValue<_, u64>;

	#[pallet::storage]
	#[pallet::getter(fn subset_size)]
	type SubsetSize<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	type DoubleBucketMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ValidatorId, (FixedI64, FixedI64)>;

	#[pallet::storage]
	type SessionEnd<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	type NextSessionEnd<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	type CurrentSessionLength<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	type AvgSessionLenght<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T> {
		pub initial_subset_size: u64,
		pub _phantom: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_subset_size: 3, _phantom: PhantomData }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			SubsetSize::<T>::put(self.initial_subset_size);
		}
	}

	impl<T: Config> Pallet<T> {
		/// Select a subset of the validators for the next session with the two buckets algorithm
		pub fn select_subset(validators: Vec<T::ValidatorId>) -> Vec<T::ValidatorId> {
			let subset_size = Self::subset_size();

			if (validators.len() as u64) < subset_size {
				Self::deposit_event(Event::FewerValidatorsThanSubset);
				return validators;
			}

			let mean = FixedI64::from_rational(subset_size as u128, 2 * validators.len() as u128);

			let mut selected_subset = Vec::<T::ValidatorId>::with_capacity(subset_size as usize);
			for v in &validators {
				let mut buckets =
					DoubleBucketMap::<T>::get(v).unwrap_or((Self::random(), Self::random()));

				buckets.0 = buckets.0 + mean;
				buckets.1 = buckets.1 + mean;

				if let Some(new_buckets) = Self::select(buckets) {
					buckets = new_buckets;
					selected_subset.push(v.clone());
				}

				DoubleBucketMap::<T>::insert(v, buckets);
			}

			if selected_subset.is_empty() {
				Self::deposit_event(Event::EmptySubset);
				//If the subset is empty we redo the process
				//With enough validators the probability of this is negligible
				Self::select_subset(validators)
			} else {
				selected_subset
			}
		}

		fn disperse(buckets: (FixedI64, FixedI64)) -> (FixedI64, FixedI64) {
			let random_number = Self::random();
			let first_decrease = random_number * DISPERSE_RATIO;
			let second_decrease = FixedI64::from_u32(1).sub(random_number) * DISPERSE_RATIO;

			(buckets.0.sub(first_decrease), buckets.1.sub(second_decrease))
		}

		///Helper function for the two bucket algorithm
		///Determine if a validator is selected and return new bucket values
		fn select(buckets: (FixedI64, FixedI64)) -> Option<(FixedI64, FixedI64)> {
			if buckets.0 > 1.into() {
				Some(Self::disperse((buckets.0.sub(DECREASE_RATIO), buckets.1)))
			} else if buckets.1 > 1.into() {
				Some(Self::disperse((buckets.0, buckets.1.sub(DECREASE_RATIO))))
			} else {
				None
			}
		}

		///Helper function to generate more unique random numbers in a block
		fn next_nonce() -> Vec<u8> {
			let nonce = Nonce::<T>::get().unwrap_or_default();
			Nonce::<T>::put(nonce.wrapping_add(1));
			nonce.encode()
		}

		///Generate a random FixedI64 number between 0 and 1
		fn random() -> FixedI64 {
			let nonce = Self::next_nonce();
			FixedI64::from_rational(T::RandomGenerator::random(&nonce), u128::MAX)
		}

		fn session_length(subset_size: BlockNumberFor<T>) -> BlockNumberFor<T> {
			let min_session_length = T::MinSessionLength::get();
			if subset_size >= min_session_length {
				subset_size
			} else {
				let remainder = min_session_length % subset_size;
				if sp_runtime::traits::Zero::is_zero(&remainder) {
					min_session_length
				} else {
					min_session_length + (subset_size - remainder)
				}
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		///Change the subset size to a new value
		#[pallet::call_index(0)]
		pub fn change_subset_size(origin: OriginFor<T>, new_subset_size: u64) -> DispatchResult {
			ensure_root(origin)?;
			SubsetSize::<T>::put(new_subset_size);
			Self::deposit_event(Event::SubsetSizeChanged(new_subset_size));
			Ok(())
		}
	}

	//TODO(vismate): Handle errors more gracefully
	impl<T: Config> pallet_session::SessionManager<T::ValidatorId> for Pallet<T> {
		fn new_session_genesis(
			session_index: sp_staking::SessionIndex,
		) -> Option<Vec<T::ValidatorId>> {
			if session_index == 0 {
				T::SessionHook::session_genesis(session_index)
					.expect("session hook ran successfully");

				let superset = T::ValidatorSuperset::validators();
				let subset_size = Self::subset_size() as usize;

				let selected_subset = if superset.len() > subset_size {
					superset[0..subset_size].to_owned()
				} else {
					superset
				};

				let current_subset_size: BlockNumberFor<T> = (selected_subset.len() as u32).into();
				let new_session_start = 0_u32.into();
				let new_session_end = Self::session_length(current_subset_size);

				CurrentSessionLength::<T>::put(new_session_end);
				SessionEnd::<T>::put(new_session_end);

				Self::deposit_event(Event::SubsetSelected {
					validator_subset: selected_subset.clone(),
					session_start: new_session_start,
					session_end: new_session_end,
					session_index,
				});

				Some(selected_subset)
			} else {
				assert!(session_index == 1);
				Self::new_session(session_index)
			}
		}

		fn end_session(session_index: sp_staking::SessionIndex) {
			T::SessionHook::session_ended(session_index).expect("session hook ran successfully");
			SessionEnd::<T>::mutate(|session_end| {
				let next_session_end = NextSessionEnd::<T>::get();
				CurrentSessionLength::<T>::put(next_session_end - *session_end);
				*session_end = next_session_end;
			});
		}

		fn start_session(session_index: sp_staking::SessionIndex) {
			T::SessionHook::session_started(session_index).expect("session hook ran successfully");
		}

		fn new_session(session_index: sp_staking::SessionIndex) -> Option<Vec<T::ValidatorId>> {
			log::info!("new session {session_index}");
			T::SessionHook::session_planned(session_index).expect("session hook ran successfully");

			let selected_subset = Self::select_subset(T::ValidatorSuperset::validators());

			let new_subset_size: BlockNumberFor<T> = (selected_subset.len() as u32).into();
			let new_session_length = Self::session_length(new_subset_size);
			let current_session_end: BlockNumberFor<T> = SessionEnd::<T>::get();
			let new_session_end: BlockNumberFor<T> = current_session_end + new_session_length;

			NextSessionEnd::<T>::put(new_session_end);
			// TODO(vismate): This way of abusing types is ugly (conflicting block numbers with session indices)
			AvgSessionLenght::<T>::mutate(|v| {
				*v = (BlockNumberFor::<T>::from(session_index) * *v + new_session_length)
					/ (BlockNumberFor::<T>::from(session_index) + 1u32.into())
			});

			Self::deposit_event(Event::SubsetSelected {
				validator_subset: selected_subset.clone(),
				session_start: current_session_end + 1_u32.into(),
				session_end: new_session_end,
				session_index,
			});

			Some(selected_subset)
		}
	}

	impl<T: Config> ShouldEndSession<BlockNumberFor<T>> for Pallet<T> {
		fn should_end_session(now: BlockNumberFor<T>) -> bool {
			SessionEnd::<T>::get() == now
		}
	}

	// TODO: define weights
	impl<T: Config> frame_support::traits::EstimateNextSessionRotation<BlockNumberFor<T>>
		for Pallet<T>
	{
		fn average_session_length() -> BlockNumberFor<T> {
			AvgSessionLenght::<T>::get()
		}

		fn estimate_current_session_progress(
			now: BlockNumberFor<T>,
		) -> (Option<sp_runtime::Permill>, frame_support::dispatch::Weight) {
			let end = SessionEnd::<T>::get();
			let progress =
				sp_runtime::Permill::from_rational(end - now, CurrentSessionLength::<T>::get())
					.left_from_one();
			(Some(progress), sp_runtime::traits::Zero::zero())
		}

		fn estimate_next_session_rotation(
			_now: BlockNumberFor<T>,
		) -> (Option<BlockNumberFor<T>>, frame_support::dispatch::Weight) {
			(Some(SessionEnd::<T>::get()), sp_runtime::traits::Zero::zero())
		}
	}
}
