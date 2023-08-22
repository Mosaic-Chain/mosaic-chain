//! Validator subset selection pallet
//! This module makes it possible to select a subset of the validator
//! NFT holders to create blocks in the next session.
//! The purpose is to maximize uniformity of weekly timeframe
//! returns(selection) in the population, while maximizing
//! unpredictability.
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

#![allow(dead_code)]
#![cfg_attr(not(feature = "std"), no_std)]

use core::marker::PhantomData;

use frame_support::pallet_prelude::*;
pub use pallet::*;
use sp_application_crypto::Ss58Codec;
use sp_runtime::{FixedI64, FixedPointNumber};
use sp_std::prelude::*;

pub trait Random128 {
	fn random(subject: &[u8]) -> u128;
}

///Convert a FixedI64 to a float for logging
fn to_float(input: FixedI64) -> f64 {
	input.into_inner() as f64 / <FixedI64 as FixedPointNumber>::DIV as f64
}

pub trait ValidatorSuperset<ValidatorId: Member + Parameter + Ss58Codec> {
	fn get_superset() -> Vec<ValidatorId>;
}

#[frame_support::pallet(dev_mode)] //TODO: remove dev mode
pub mod pallet {
	use super::*;
	use frame_support::traits::BuildGenesisConfig;
	use frame_system::{
		ensure_root,
		pallet_prelude::{BlockNumberFor, OriginFor},
	};
	use pallet_session::ShouldEndSession;

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
		type RandomGenerator: Random128;
		type ValidatorSuperset: ValidatorSuperset<Self::ValidatorId>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FewerValidatorsThenSubset,
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
	pub type SubsetSize<T: Config> = StorageValue<_, u64, ValueQuery>;

	#[pallet::storage]
	pub type DoubleBucketMap<T: Config> =
		StorageMap<_, Blake2_128Concat, T::ValidatorId, (FixedI64, FixedI64)>;

	#[pallet::storage]
	pub type SessionEnd<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

	#[pallet::storage]
	pub type NextSessionEnd<T: Config> = StorageValue<_, BlockNumberFor<T>, ValueQuery>;

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

			//TODO: solve 0. and 1. sessions problem without this lines
			let one: BlockNumberFor<T> = 6_u32.into();
			SessionEnd::<T>::put(one);
			let two: BlockNumberFor<T> = 12_u32.into();
			NextSessionEnd::<T>::put(two);
		}
	}

	impl<T: Config> Pallet<T> {
		/// Select a subset of the validators for the next session with the two buckets algorithm
		pub fn select_subset(validators: Vec<T::ValidatorId>) -> Vec<T::ValidatorId> {
			let subset_size = Self::subset_size();
			if (validators.len() as u64) < subset_size {
				Self::deposit_event(Event::FewerValidatorsThenSubset);
				return validators;
			}
			let mean = FixedI64::from_rational(subset_size as u128, 2 * validators.len() as u128);
			log::info!(
				"Subset size: {}, validators len: {}, mean: {}",
				subset_size,
				validators.len(),
				mean
			);
			let mut selected_subset = Vec::<T::ValidatorId>::new();
			for v in &validators {
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
			log::info!(
				"Subset selected: {:?}",
				selected_subset.iter().map(|v| v.to_ss58check()).collect::<Vec<_>>()
			);
			if selected_subset.is_empty() {
				Self::deposit_event(Event::EmptySubset);
				//If the subset is empty we redo the process
				//With enough validators the probability of this is negligible
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
			let mut is_selected = false;
			if bucket_value_pair.0 > 1.into() {
				bucket_value_pair.0 = bucket_value_pair.0 - DECREASE_RATIO;
				is_selected = true;
			} else if bucket_value_pair.1 > 1.into() {
				is_selected = true;
				bucket_value_pair.1 = bucket_value_pair.1 - DECREASE_RATIO;
			}
			if is_selected {
				let random_number = Self::generate_random();
				let first_decrease = random_number * DISPERSE_RATIO;
				let second_decrease = FixedI64::from_u32(1).sub(random_number) * DISPERSE_RATIO;
				log::info!("first and second decrease {} {}", first_decrease, second_decrease);
				bucket_value_pair.0 = bucket_value_pair.0.sub(first_decrease);
				bucket_value_pair.1 = bucket_value_pair.1.sub(second_decrease);
			}
			(is_selected, bucket_value_pair)
		}

		///Helper function to generate more unique random numbers in a block
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

		/// Delete all bucket pairs from the storage map
		pub fn garbage_collector() {
			DoubleBucketMap::<T>::drain();
		}

		fn session_length(subset_size: BlockNumberFor<T>) -> BlockNumberFor<T> {
			//TODO: update this function
			subset_size
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

	impl<T: Config> pallet_session::SessionManager<T::ValidatorId> for Pallet<T> {
		fn new_session_genesis(
			session_index: sp_staking::SessionIndex,
		) -> Option<Vec<T::ValidatorId>> {
			log::info!("New session genesis {}", session_index);
			log::info!("Subset size: {}", SubsetSize::<T>::get());
			None
		}

		fn end_session(_: sp_staking::SessionIndex) {
			SessionEnd::<T>::put(NextSessionEnd::<T>::get());
		}

		fn start_session(_: sp_staking::SessionIndex) {}

		fn new_session(session_index: sp_staking::SessionIndex) -> Option<Vec<T::ValidatorId>> {
			log::info!("new session {}", session_index);
			let selected_subset =
				Self::select_subset(<T::ValidatorSuperset as ValidatorSuperset<_>>::get_superset());
			let current_subset_size: BlockNumberFor<T> = (selected_subset.len() as u32).into();
			let session_end: BlockNumberFor<T> = SessionEnd::<T>::get();
			let next_session_end: BlockNumberFor<T> =
				session_end + Self::session_length(current_subset_size);
			NextSessionEnd::<T>::put(next_session_end);
			Self::deposit_event(Event::SubsetSelected {
				validator_subset: selected_subset.clone(),
				//The subset is selected for the next session, which starts
				//at blocknumber = this session end plus one
				session_start: session_end + 1_u32.into(),
				session_end: next_session_end,
				session_index,
			});
			Some(selected_subset)
		}
	}

	impl<T: Config> ShouldEndSession<BlockNumberFor<T>> for Pallet<T> {
		fn should_end_session(now: BlockNumberFor<T>) -> bool {
			let session_end = SessionEnd::<T>::get();
			session_end == now
		}
	}
}
