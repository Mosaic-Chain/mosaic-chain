#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;
	use frame_support::dispatch::Codec;
	use frame_support::pallet_prelude::*;
	use frame_support::traits::Currency;
	use frame_system::pallet_prelude::*;
	use pallet_nfts::Config as NftsConfig;
	use sp_runtime::FixedPointOperand;
	use sp_runtime::Perbill;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	pub trait NftStaking<AccountId, Balance, Metadata, ItemId> {
		fn bind(
			account_id: &AccountId,
			item_id: &ItemId,
		) -> Result<(Metadata, Balance), DispatchError>;
		fn unbind(account_id: &AccountId, item_id: &ItemId) -> DispatchResult;

		fn chill(account_id: &AccountId) -> DispatchResult;
		fn unchill(account_id: &AccountId) -> DispatchResult;

		fn slash(
			validator_id: &AccountId,
			account_id: &AccountId,
			slash_proportion: Perbill,
		) -> DispatchResult;
	}

	impl<AccountId, Balance, Metadata, ItemId> NftStaking<AccountId, Balance, Metadata, ItemId> for () {
		fn bind(
			account_id: &AccountId,
			item_id: &ItemId,
		) -> Result<(Metadata, Balance), DispatchError> {
			todo!()
		}
		fn unbind(account_id: &AccountId, item_id: &ItemId) -> DispatchResult {
			Ok(())
		}

		fn chill(account_id: &AccountId) -> DispatchResult {
			Ok(())
		}
		fn unchill(account_id: &AccountId) -> DispatchResult {
			Ok(())
		}

		fn slash(
			validator_id: &AccountId,
			account_id: &AccountId,
			slash_proportion: Perbill,
		) -> DispatchResult {
			Ok(())
		}
	}

	pub enum ValidatorVariants {
		PoS,
		DPos,
	}

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + NftsConfig {
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
			+ FixedPointOperand;

		type Currency: frame_support::traits::LockableCurrency<
			Self::AccountId,
			Moment = BlockNumberFor<Self>,
			Balance = Self::Balance,
		>;

		type NftStakingHandler: NftStaking<
			Self::AccountId,
			Self::Balance,
			ValidatorVariants,
			Self::ItemId,
		>;

		type NftDelegatingHandler: NftStaking<
			Self::AccountId,
			Self::Balance,
			ValidatorVariants,
			Self::ItemId,
		>;

		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
	}

	#[pallet::storage]
	#[pallet::getter(fn total_stake)]
	pub type TotalStake<T: Config> = StorageValue<_, T::Balance>;

	#[pallet::storage]
	#[pallet::getter(fn account_exposure)]
	pub type AccountExposure<T: Config> = StorageMap<_, Twox64Concat, T::AccountId, T::Balance>;

	#[pallet::storage]
	#[pallet::getter(fn currency_exposure)]
	pub type CurrencyExposure<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId, // ValidatorId
		Twox64Concat,
		T::AccountId, // DelegatorId
		T::Balance,
	>;

	#[pallet::storage]
	#[pallet::getter(fn nft_exposure)]
	pub type NftExposure<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId, // ValidatorId
		Twox64Concat,
		T::AccountId, // DelegatorId
		T::Balance,
	>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		SomethingStored { who: T::AccountId },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		BadState,
		/// Errors should have helpful documentation associated with them.
		AlreadyBound,
	}

	impl<T: Config> Pallet<T> {
		fn do_stake(who: &T::AccountId, nominal_value: T::Balance) -> DispatchResult {
			let total = TotalStake::<T>::get().ok_or(Error::<T>::BadState)?;
			TotalStake::<T>::put(total + nominal_value);

			AccountExposure::<T>::insert(&who, nominal_value);
			NftExposure::<T>::insert(&who, &who, nominal_value);
			Ok(())
		}

		fn do_unstake() {}
		fn do_bind(
			who: &T::AccountId,
			item_id: &T::ItemId,
		) -> Result<(ValidatorVariants, T::Balance), DispatchError> {
			T::NftStakingHandler::bind(&who, &item_id)
		}
		fn do_unbind(who: &T::AccountId, item_id: &T::ItemId) -> DispatchResult {
			T::NftStakingHandler::unbind(&who, item_id)
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		pub fn stake_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			ensure!(AccountExposure::<T>::get(&who).is_none(), Error::<T>::AlreadyBound);

			Ok(())
		}

		/// An example dispatchable that may throw a custom error.
		#[pallet::call_index(1)]
		pub fn unstake_nft(
			origin: OriginFor<T>,
			item_id: <T as NftsConfig>::ItemId,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;

			let unstaked_value: u32 = 0;
			Ok(())
		}

		#[pallet::call_index(3)]
		pub fn stake_currency(
			origin: OriginFor<T>,
			#[pallet::compact] value: T::Balance,
		) -> DispatchResult {
			let stash = ensure_signed(origin)?;

			frame_system::Pallet::<T>::inc_consumers(&stash).map_err(|_| Error::<T>::BadState)?;

			let stash_balance = <T as pallet::Config>::Currency::free_balance(&stash);
			let value = value.min(stash_balance);
			Ok(())
		}

		#[pallet::call_index(4)]
		pub fn unstake_currency(origin: OriginFor<T>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let _who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::call_index(5)]
		pub fn stake_deletated_nft(origin: OriginFor<T>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let _who = ensure_signed(origin)?;

			Ok(())
		}

		#[pallet::call_index(6)]
		pub fn unstake_deletated_nft(origin: OriginFor<T>) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			// https://docs.substrate.io/main-docs/build/origins/
			let _who = ensure_signed(origin)?;
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}
	}
}
