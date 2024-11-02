#![cfg_attr(not(feature = "std"), no_std)]
// Expect lints caused by procmacros
#![expect(clippy::manual_inspect, clippy::must_use_candidate, clippy::cast_possible_truncation)]

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
//!    - Allowing for session hooks
//!
//! - **ShouldEndSession implementation**
//!
//! - **EstimateNextSessionRotation implementation**

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use sdk::{frame_support, frame_system, pallet_session, sp_runtime, sp_std};

use sp_std::prelude::*;

use frame_support::{
	pallet_prelude::*,
	traits::{Randomness, ValidatorSet},
};
use frame_system::pallet_prelude::BlockNumberFor;
use sp_runtime::{
	traits::{Hash, One, Zero},
	FixedI64, PerThing,
};
use utils::{traits::SessionHook, SessionIndex};

pub use pallet::*;

#[frame_support::pallet(dev_mode)] //TODO: remove dev mode
pub mod pallet {
	use super::*;
	// If a validator's bucket is full, then the bucket value is decreased with decrease_ratio
	// and the other disperse_ratio=(1-decrease_ratio) is divided among all buckets
	const DECREASE_RATIO: FixedI64 = FixedI64::from_rational(1, 2);
	const DISPERSE_RATIO: FixedI64 = FixedI64::from_u32(1).sub(DECREASE_RATIO);

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;
		type ValidatorId: Member + Parameter;
		type Randomness: Randomness<Self::Hash, BlockNumberFor<Self>>;
		type ValidatorSuperset: ValidatorSet<Self::ValidatorId, ValidatorId = Self::ValidatorId>;
		type SubsetSize: Get<u64>;

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
			session_index: SessionIndex,
		},
	}

	///Helper value for generating unique random numbers
	#[pallet::storage]
	pub type Nonce<T: Config> = StorageValue<_, u64>;

	#[pallet::storage]
	#[pallet::getter(fn buckets)]
	pub type DoubleBucketMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ValidatorId, (FixedI64, FixedI64)>;

	#[pallet::storage]
	#[pallet::getter(fn current_session_end)]
	pub type CurrentSessionEnd<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn next_session_end)]
	pub type NextSessionEnd<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn current_session_length)]
	pub type CurrentSessionLength<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn avg_session_length)]
	pub type AvgSessionLength<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	impl<T: Config> Pallet<T> {
		/// Select a subset of the validators for the next session with the two buckets algorithm
		pub(super) fn select_subset(validators: Vec<T::ValidatorId>) -> Vec<T::ValidatorId> {
			let subset_size = T::SubsetSize::get().max(1);

			if (validators.len() as u64) < subset_size {
				Self::deposit_event(Event::FewerValidatorsThanSubset);

				return validators;
			}

			// We divide the mean by two, because we are distributing it between two buckets
			let mean =
				FixedI64::from_rational(u128::from(subset_size), 2 * validators.len() as u128);
			let mut selected_subset = Vec::<T::ValidatorId>::with_capacity(subset_size as usize);

			for v in &validators {
				let mut buckets =
					Self::buckets(v).unwrap_or_else(|| (Self::random(), Self::random()));
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

				// If the subset is empty we redo the process
				// With enough validators the probability of this is negligible
				Self::select_subset(validators)
			} else {
				selected_subset
			}
		}

		fn disperse(buckets: (FixedI64, FixedI64)) -> (FixedI64, FixedI64) {
			let random_number = Self::random();
			let first_decrease = random_number * DISPERSE_RATIO;
			let second_decrease = FixedI64::one().sub(random_number) * DISPERSE_RATIO;

			(buckets.0.sub(first_decrease), buckets.1.sub(second_decrease))
		}

		/// Calculates the new average session length based on new session details and the current average.
		/// This function is used for updating the rolling average.
		pub(crate) fn new_avg_session_length(
			session_index: SessionIndex,
			session_length: BlockNumberFor<T>,
			avg: BlockNumberFor<T>,
		) -> BlockNumberFor<T> {
			let session_index = BlockNumberFor::<T>::from(session_index);
			(avg * session_index + session_length) / (session_index + One::one())
		}

		///Helper function for the two bucket algorithm
		///Determine if a validator is selected and return new bucket values
		fn select(buckets: (FixedI64, FixedI64)) -> Option<(FixedI64, FixedI64)> {
			if buckets.0 > One::one() {
				Some(Self::disperse((buckets.0.sub(DECREASE_RATIO), buckets.1)))
			} else if buckets.1 > One::one() {
				Some(Self::disperse((buckets.0, buckets.1.sub(DECREASE_RATIO))))
			} else {
				None
			}
		}

		///Helper function to generate more unique random numbers in a block
		fn next_nonce() -> u64 {
			let nonce = Nonce::<T>::get().unwrap_or_default();

			Nonce::<T>::put(nonce.wrapping_add(1));

			nonce
		}

		///Generate a random FixedI64 number between 0 and 1
		fn random() -> FixedI64 {
			let nonce = Self::next_nonce().to_le_bytes();
			let hash = if T::ValidatorSuperset::session_index() == 0 {
				Self::bootstrap_randomness(&nonce)
			} else {
				T::Randomness::random(&nonce).0
			};

			let p = u128::from_le_bytes(
				hash.as_ref()[0..16]
					.try_into()
					.expect("Can't convert first part of random hash to u128!"),
			);

			FixedI64::from_rational(p, u128::MAX)
		}

		/// Initial "randomness" expected to be used in the 0th session
		fn bootstrap_randomness(nonce: &[u8]) -> T::Hash {
			let s = [b"Mosaic", nonce, b"Chain"].concat();

			T::Hashing::hash(&s)
		}

		pub(crate) fn session_length(subset_size: BlockNumberFor<T>) -> BlockNumberFor<T> {
			let min_session_length = T::MinSessionLength::get();

			if subset_size >= min_session_length {
				subset_size
			} else {
				let remainder = min_session_length % subset_size;

				if remainder.is_zero() {
					min_session_length
				} else {
					min_session_length + (subset_size - remainder)
				}
			}
		}
	}

	impl<T: Config> pallet_session::SessionManager<T::ValidatorId> for Pallet<T> {
		fn new_session_genesis(session_index: SessionIndex) -> Option<Vec<T::ValidatorId>> {
			T::SessionHook::session_genesis(session_index).expect("session hook ran successfully");

			if session_index == 0 {
				let subset = Self::select_subset(T::ValidatorSuperset::validators());

				// NOTE: this should usually not happen outside of tests
				// as it means the runtime is misconfigured.
				// eg.: wrong pallet initialization order or bad genesis.
				if subset.is_empty() {
					return None;
				}

				let end = Self::session_length((subset.len() as u32).into());

				// We include the genesis block in session 0 as well although it's not associated with one particular validator.
				let len = end + One::one();

				CurrentSessionLength::<T>::put(len);
				AvgSessionLength::<T>::put(len);
				CurrentSessionEnd::<T>::put(end);

				Self::deposit_event(Event::SubsetSelected {
					validator_subset: subset.clone(),
					session_start: Zero::zero(),
					session_end: end,
					session_index,
				});

				Some(subset)
			} else {
				Self::new_session(session_index)
			}
		}

		fn end_session(session_index: SessionIndex) {
			T::SessionHook::session_ended(session_index).expect("session hook ran successfully");

			CurrentSessionEnd::<T>::mutate(|session_end| {
				let next_session_end = NextSessionEnd::<T>::get();

				CurrentSessionLength::<T>::put(next_session_end - *session_end);

				*session_end = next_session_end;
			});
		}

		fn start_session(session_index: SessionIndex) {
			T::SessionHook::session_started(session_index).expect("session hook ran successfully");
		}

		fn new_session(session_index: SessionIndex) -> Option<Vec<T::ValidatorId>> {
			T::SessionHook::session_planned(session_index).expect("session hook ran successfully");

			let selected_subset = Self::select_subset(T::ValidatorSuperset::validators());

			// TODO: a better strategy to handle this case
			// problem: the runtime might assume that a validator is only selected
			// iff it's present in the superset.
			// This error however occurs when the superset is empty.
			// Returning `None` here violates the assumption.
			if selected_subset.is_empty() {
				return None;
			}

			let new_subset_size: BlockNumberFor<T> = (selected_subset.len() as u32).into();
			let new_session_length = Self::session_length(new_subset_size);
			let new_session_end: BlockNumberFor<T> =
				Self::current_session_end() + new_session_length;

			NextSessionEnd::<T>::put(new_session_end);

			AvgSessionLength::<T>::mutate(|v| {
				*v = Self::new_avg_session_length(session_index, new_session_length, *v);
			});

			Self::deposit_event(Event::SubsetSelected {
				validator_subset: selected_subset.clone(),
				session_start: Self::current_session_end() + One::one(),
				session_end: Self::next_session_end(),
				session_index,
			});

			Some(selected_subset)
		}
	}

	impl<T: Config> pallet_session::ShouldEndSession<BlockNumberFor<T>> for Pallet<T> {
		fn should_end_session(now: BlockNumberFor<T>) -> bool {
			Self::current_session_end() == now
		}
	}

	impl<T: Config> frame_support::traits::EstimateNextSessionRotation<BlockNumberFor<T>>
		for Pallet<T>
	{
		fn average_session_length() -> BlockNumberFor<T> {
			Self::avg_session_length()
		}

		fn estimate_current_session_progress(
			now: BlockNumberFor<T>,
		) -> (Option<sp_runtime::Permill>, Weight) {
			let end = Self::current_session_end();
			let progress =
				sp_runtime::Permill::from_rational(end - now, Self::current_session_length())
					.left_from_one();

			(Some(progress), T::DbWeight::get().reads(2))
		}

		fn estimate_next_session_rotation(
			_now: BlockNumberFor<T>,
		) -> (Option<BlockNumberFor<T>>, Weight) {
			(Some(Self::current_session_end()), T::DbWeight::get().reads(1))
		}
	}
}
