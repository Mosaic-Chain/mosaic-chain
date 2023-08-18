//! Validator subset selection pallet
//! This module makes it possible to select a subset of the validator
//! NFT holders to create blocks in the next session.
//! The purpose is to maximize uniformity of weekly timeframe
//! returns(selection) in the population, while maximizing
//! unpredictability.
//!
//! The Validator subset selection pallet provides the following features:
//!
//! - **Select subset:**
//!

#![allow(dead_code)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use sp_runtime::{FixedI64, FixedPointNumber};

pub trait Random128 {
	fn random(subject: &[u8]) -> u128;
}

///Convert a FixedI64 to a float for logging
fn to_float(input: FixedI64) -> f64 {
	input.into_inner() as f64 / <FixedI64 as FixedPointNumber>::DIV as f64
}

#[frame_support::pallet(dev_mode)] //TODO: remove dev mode
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use sp_application_crypto::Ss58Codec;
	use sp_std::prelude::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type ValidatorId: Member + Parameter + Ss58Codec;
		type RandomGenerator: Random128;
		type SubsetSize: Get<u64>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FewerValidatorsThenSubset,
		EmptySubset,
		SubsetSelected(Vec<T::ValidatorId>),
	}

	///Helper value for generating unique random numbers
	#[pallet::storage]
	pub type Nonce<T: Config> = StorageValue<_, u64>;

	#[pallet::storage]
	pub type BucketMap<T: Config> = StorageMap<_, _, T::ValidatorId, FixedI64>;

	#[pallet::storage]
	pub type DoubleBucketMap<T: Config> = StorageMap<_, _, T::ValidatorId, (FixedI64, FixedI64)>;

	impl<T: Config> Pallet<T> {
		/// Select a subset of the validators for the next session with the one bucket algorithm
		pub fn select_subset_basic(validators: &Vec<T::ValidatorId>) -> Vec<T::ValidatorId> {
			let subset_size = T::SubsetSize::get();
			if (validators.len() as u64) < subset_size {
				Self::deposit_event(Event::FewerValidatorsThenSubset);
			}

			let mean = FixedI64::from_rational(subset_size as u128, validators.len() as u128);
			let mut selected_subset = Vec::<T::ValidatorId>::new();
			for v in validators {
				let mut bucket_value =
					BucketMap::<T>::get(v).unwrap_or_else(|| Self::generate_random()) + mean;
				if bucket_value > 1.into() {
					bucket_value = bucket_value.sub(1.into());
					selected_subset.push(v.clone());
				}
				BucketMap::<T>::insert(v, bucket_value);
			}
			Self::deposit_event(Event::SubsetSelected(selected_subset.clone()));
			if selected_subset.is_empty() {
				Self::deposit_event(Event::EmptySubset);
				Self::select_subset_basic(validators)
			} else {
				selected_subset
			}
		}

		/// Select a subset of the validators for the next session with the two buckets algorithm
		pub fn select_subset(validators: &Vec<T::ValidatorId>) -> Vec<T::ValidatorId> {
			if (validators.len() as u64) < T::SubsetSize::get() {
				Self::deposit_event(Event::FewerValidatorsThenSubset);
			}
			let mean =
				FixedI64::from_rational(T::SubsetSize::get() as u128, 2 * validators.len() as u128);
			let mut selected_subset = Vec::<T::ValidatorId>::new();
			for v in validators {
				let mut bucket_value_pair = DoubleBucketMap::<T>::get(v)
					.unwrap_or_else(|| (Self::generate_random(), Self::generate_random()));
				bucket_value_pair.0 = bucket_value_pair.0 + mean;
				bucket_value_pair.1 = bucket_value_pair.1 + mean;
				log::info!(
					"Bucket value {}: {:?}",
					v.to_ss58check(),
					(to_float(bucket_value_pair.0), to_float(bucket_value_pair.1))
				);
				let (is_selected, new_bucket_value_pair) =
					Self::select_if_bucket_full(bucket_value_pair);
				if is_selected {
					selected_subset.push(v.clone());
					log::info!("Validator {} selected.", v.to_ss58check());
					log::info!(
						"Updated bucket value {}: {:?}",
						v.to_ss58check(),
						(to_float(new_bucket_value_pair.0), to_float(new_bucket_value_pair.1))
					);
				}
				DoubleBucketMap::<T>::insert(v, new_bucket_value_pair);
			}
			Self::deposit_event(Event::SubsetSelected(selected_subset.clone()));
			log::info!(
				"Subset selected: {:?}",
				selected_subset.iter().map(|v| v.to_ss58check()).collect::<Vec<_>>()
			);
			if selected_subset.is_empty() {
				Self::deposit_event(Event::EmptySubset);
				//If the subset is empty we redo the process
				//With 1000 validators the probability of this is negligible
				Self::select_subset(validators)
			} else {
				selected_subset
			}
		}

		///Helper function for the two bucket algorithm
		///Determine if a validator is selected and return new bucket values
		fn select_if_bucket_full(
			mut bucket_value_pair: (FixedI64, FixedI64),
		) -> (bool, (FixedI64, FixedI64)) {
			let decrease_ratio = FixedI64::from_rational(1, 2);
			let mut is_selected = false;
			if bucket_value_pair.0 > 1.into() {
				bucket_value_pair.0 = bucket_value_pair.0 - decrease_ratio;
				is_selected = true;
			} else if bucket_value_pair.1 > 1.into() {
				is_selected = true;
				bucket_value_pair.1 = bucket_value_pair.1 - decrease_ratio;
			}
			if is_selected {
				let random_number = Self::generate_random();
				let first_decrease = random_number * FixedI64::from_u32(1).sub(decrease_ratio);
				let second_decrease = FixedI64::from_u32(1).sub(random_number)
					* FixedI64::from_u32(1).sub(decrease_ratio);
				bucket_value_pair.0 = bucket_value_pair.0.sub(first_decrease);
				bucket_value_pair.1 = bucket_value_pair.1.sub(second_decrease);
			}
			(is_selected, bucket_value_pair)
		}

		fn get_and_increment_nonce() -> Vec<u8> {
			let nonce = Nonce::<T>::get().unwrap_or_default();
			Nonce::<T>::put(nonce.wrapping_add(1));
			nonce.encode()
		}

		///Generate a random FixedI64 number between 0 and 1
		fn generate_random() -> FixedI64 {
			let nonce = Self::get_and_increment_nonce();
			let random_number = T::RandomGenerator::random(&nonce);
			FixedI64::from_rational(random_number, u128::MAX)
		}

		/// Delete all buckets from the storage map
		pub fn garbage_collector_basic() {
			BucketMap::<T>::drain();
		}

		/// Delete all bucket pairs from the storage map
		pub fn garbage_collector() {
			DoubleBucketMap::<T>::drain();
		}
	}
}
