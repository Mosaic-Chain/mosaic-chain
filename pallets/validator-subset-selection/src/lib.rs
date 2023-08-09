#![allow(dead_code)]
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;

#[frame_support::pallet(dev_mode)] //TODO: remove dev mode
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::Randomness;
	use frame_system::pallet_prelude::BlockNumberFor;
	use sp_runtime::FixedI64;
	use sp_std::vec::Vec;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		type ValidatorId: Member + Parameter;
		type RandomGenerator: Randomness<u128, BlockNumberFor<Self>>;
		type SubsetSize: Get<u64>;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		FewerValidatorsThenSubset,
		EmptySubset,
		SubsetSelected(Vec<T::ValidatorId>),
	}

	#[pallet::storage]
	pub type Nonce<T: Config> = StorageValue<_, u64>;

	#[pallet::storage]
	pub type BucketMap<T: Config> = StorageMap<_, _, T::ValidatorId, FixedI64>;

	impl<T: Config> Pallet<T> {
		/// Select a subset of the validators for the next session
		pub fn select_subset(validators: &Vec<T::ValidatorId>) -> Vec<T::ValidatorId> {
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
				let mut v = Vec::new();
				v.push(validators[0].clone());
				v
			} else {
				selected_subset
			}
		}

		fn get_and_increment_nonce() -> Vec<u8> {
			let nonce = Nonce::<T>::get().unwrap_or_default();
			Nonce::<T>::put(nonce.wrapping_add(1));
			nonce.encode()
		}

		fn generate_random() -> FixedI64 {
			let nonce = Self::get_and_increment_nonce();
			let (random_number, _block_number) = T::RandomGenerator::random(&nonce);
			FixedI64::from_rational(random_number, u128::MAX)
		}

		/// Delete all buckets from the storage map
		pub fn garbage_collector() {
			BucketMap::<T>::drain();
		}
	}
}
