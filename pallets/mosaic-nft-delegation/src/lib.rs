// clippy::pedantic warnings coming from code generated by macros that annoy the hell out of me
#![allow(
	clippy::must_use_candidate,
	clippy::cast_possible_truncation,
	dead_code,
	clippy::used_underscore_binding,
	clippy::wildcard_imports
)]
#![cfg_attr(not(feature = "std"), no_std)]

/// # pallet-nft-delegation
///
/// The pallet is a runtime module for managing non-fungible tokens (NFTs)
/// representing delegation rights in a staking system. It allows for minting, binding,
/// unbinding, and slashing these delegation NFTs. Additionally, it handles expiration checks
/// for bound tokens and provides an extensible `OnNftExpire` trait for custom logic
/// when an NFT expires.
///
/// ## Module Features
///
/// - Minting delegation NFTs with specified expiration and nominal value.
/// - Binding and unbinding delegation NFTs to validators.
/// - Slashing bound delegation NFTs based on a proportion.
/// - Expiration checks for bound delegation NFTs.
///
/// ## Usage
///
/// Users can mint delegation NFTs, bind them to validators, unbind them when necessary,
/// and slash them if needed. The module automatically checks for NFT expiration during
/// each session change.
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use sp_std::vec::Vec as SpVec;

use codec::Codec;
use frame_support::{
	pallet_prelude::*,
	traits::{
		tokens::nonfungibles_v2::{Create, Inspect, Mutate, Transfer},
		BuildGenesisConfig, Incrementable,
	},
	PalletError, PalletId,
};
use frame_system::pallet_prelude::OriginFor;
use pallet_nfts::{
	CollectionConfig, CollectionSettings, Config as NftsConfig, ItemConfig, ItemSettings,
	MintSettings, MintType, Pallet as NftsPallet,
};
use sp_runtime::{traits::AccountIdConversion, DispatchError, FixedPointOperand};

use utils::traits::{NftDelegation, OnDelegationNftExpire};

// TODO: Once the pallet is ready turn off dev_mode
#[frame_support::pallet(dev_mode)]
pub mod pallet {

	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config + NftsConfig {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// The id of the pallet from witch the collection owner's address is derived.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// The origin that is authorized to mint new tokens and modify existing ones.
		type PrivilegedOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// Used for the nominal value of delegator tokens
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

		type NftExpirationHandler: OnDelegationNftExpire<
			Self::AccountId,
			<Self as NftsConfig>::ItemId,
			Self::Balance,
			Self::BindMetadata,
		>;

		/// Arbitrary data stored during when an item is bound
		type BindMetadata: Parameter + Member + Codec + TypeInfo + MaxEncodedLen;
	}

	/// The id of the collection that's managed by this pallet.
	#[pallet::storage]
	#[pallet::getter(fn collection_id)]
	pub type CollectionId<T: Config> = StorageValue<_, <T as NftsConfig>::CollectionId>;

	/// The account who owns the managed collection. Derived from the `PalletId`.
	#[pallet::storage]
	#[pallet::getter(fn pallet_account_id)]
	pub type PalletAccountId<T: Config> = StorageValue<_, T::AccountId>;

	#[pallet::storage]
	#[pallet::getter(fn next_item_id)]
	pub type NextItemId<T: Config> = StorageValue<_, <T as NftsConfig>::ItemId>;

	// Maps item ids to validators if bound
	#[pallet::storage]
	pub type BoundItems<T: Config> =
		StorageMap<_, Twox64Concat, <T as NftsConfig>::ItemId, T::BindMetadata>;

	#[pallet::storage]
	pub type ExpiryCache<T: Config> =
		StorageMap<_, Twox64Concat, sp_staking::SessionIndex, SpVec<<T as NftsConfig>::ItemId>>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A token has been minted to the specified account.
		TokenCreated { account: T::AccountId, item_id: <T as NftsConfig>::ItemId },

		/// A token has been successfully bound
		TokenBound { item_id: <T as NftsConfig>::ItemId },

		/// A token has been successfully unbound
		TokenUnbound { item_id: <T as NftsConfig>::ItemId },

		/// A set of token has been expired
		TokensExpired { items: SpVec<<T as NftsConfig>::ItemId> },
	}

	#[derive(
		Debug, PartialEq, Eq, Copy, Clone, TypeInfo, Encode, Decode, PalletError, MaxEncodedLen,
	)]
	#[repr(u8)]
	pub enum AttributeKey {
		Expiration = 0,
		NominalValue = 1,
	}

	impl From<AttributeKey> for &[u8] {
		fn from(value: AttributeKey) -> Self {
			match value {
				AttributeKey::Expiration => b"EXPR",
				AttributeKey::NominalValue => b"NOMV",
			}
		}
	}

	#[pallet::error]
	pub enum Error<T> {
		CollectionNotInitialized,
		ItemNotInitialized,
		InvalidAttribute {
			/// Key of NFT attribute that could not be decoded
			attribute_key: AttributeKey,
		},
		WrongOwner,
		WrongDelegate,
		AlreadyBound,
		NotBound,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub initial_token_holders: SpVec<(T::AccountId, sp_staking::SessionIndex, T::Balance)>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { initial_token_holders: SpVec::new() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T>
	where
		T::ItemId: Incrementable,
	{
		fn build(&self) {
			let admin: T::AccountId = T::PalletId::get().into_account_truncating();
			PalletAccountId::<T>::put(admin.clone());

			let collection_id = NftsPallet::<T>::create_collection(
				&admin,
				&admin,
				&CollectionConfig {
					settings: CollectionSettings::default(),
					max_supply: None,
					mint_settings: MintSettings {
						mint_type: MintType::Issuer,
						price: None,
						start_block: None,
						end_block: None,
						default_item_settings: ItemSettings::default(),
					},
				},
			)
			.expect("could create collection");

			CollectionId::<T>::put(collection_id);

			let mut item_id = T::ItemId::initial_value().expect("ItemId has an initial value");

			for (account_id, expiration, nominal_value) in &self.initial_token_holders {
				NftsPallet::<T>::mint_into(
					&collection_id,
					&item_id,
					account_id,
					&ItemConfig::default(),
					true,
				)
				.expect("could mint new permission nft");

				nominal_value
					.using_encoded(|nominal_value| {
						expiration.using_encoded(|expiration| {
							Pallet::<T>::init_attributes(
								&collection_id,
								&item_id,
								expiration,
								nominal_value,
							)
						})
					})
					.expect("could initialize attributes");

				Pallet::<T>::expiry_cache(&item_id, *expiration);

				item_id = item_id.increment().expect("could increment item_id");
			}

			NextItemId::<T>::put(item_id);
		}
	}

	impl<T: Config> Pallet<T>
	where
		T::ItemId: Incrementable,
	{
		/// Mint a new delegator NFT with provided expiration session index and nominal value.
		///
		/// # Parameters
		///
		/// - `account_id`: Account to assign the NFT.
		/// - `expiration`: Session Index until the token is valid.
		/// - `nominal_value`: Nominal value of the NFT.
		///
		/// # Errors
		///
		/// - Pallet is not initialized.
		/// - Error during minting process.
		pub fn do_mint_delegator_token(
			account_id: &T::AccountId,
			expiration: sp_staking::SessionIndex,
			nominal_value: &T::Balance,
		) -> Result<<T as NftsConfig>::ItemId, DispatchError> {
			nominal_value.using_encoded(|nominal_value| {
				expiration.using_encoded(|expiration_bytes| {
					let item_id =
						Self::next_item_id().ok_or(Error::<T>::CollectionNotInitialized)?;
					let collection_id =
						Self::collection_id().ok_or(Error::<T>::CollectionNotInitialized)?;

					NftsPallet::<T>::mint_into(
						&collection_id,
						&item_id,
						account_id,
						&ItemConfig::default(),
						true,
					)?;

					Self::init_attributes(
						&collection_id,
						&item_id,
						expiration_bytes,
						nominal_value,
					)?;

					Pallet::<T>::deposit_event(Event::<T>::TokenCreated {
						account: account_id.clone(),
						item_id,
					});

					let next_item_id = item_id.increment().ok_or(Error::<T>::ItemNotInitialized)?;
					NextItemId::<T>::put(next_item_id);

					Self::expiry_cache(&item_id, expiration);

					Ok(item_id)
				})
			})
		}

		/// Returns the nominal value of the provided item
		///
		/// # Errors
		///  - Pallet is not initialized
		///  - NFT is not initialized
		///  - Failed to decode data
		pub fn nominal_value_of(
			item_id: &<T as NftsConfig>::ItemId,
		) -> Result<T::Balance, DispatchError> {
			let collection_id =
				Self::collection_id().ok_or(Error::<T>::CollectionNotInitialized)?;

			Self::decode_nominal_value(&collection_id, item_id)
		}

		/// Returns the expiration of the provided item
		///
		/// # Errors
		///  - Pallet is not initialized
		///  - NFT is not initialized
		///  - Failed to decode data
		pub fn expiration_of(
			item_id: &<T as NftsConfig>::ItemId,
		) -> Result<sp_staking::SessionIndex, DispatchError> {
			let collection_id =
				Self::collection_id().ok_or(Error::<T>::CollectionNotInitialized)?;

			Self::decode_expiration(&collection_id, item_id)
		}

		pub fn is_bound(item_id: &<T as NftsConfig>::ItemId) -> bool {
			BoundItems::<T>::get(item_id).is_some()
		}

		fn expiry_cache(item_id: &<T as NftsConfig>::ItemId, expiration: sp_staking::SessionIndex) {
			ExpiryCache::<T>::mutate(expiration, |itms| match itms {
				Some(v) => v.push(*item_id),
				None => {
					*itms = Some([*item_id].to_vec());
				},
			});
		}

		fn encode_nominal_value(
			collection_id: &<T as NftsConfig>::CollectionId,
			item_id: &<T as NftsConfig>::ItemId,
			nominal_value: &T::Balance,
		) -> DispatchResult {
			nominal_value.using_encoded(|nominal_value| {
				<NftsPallet<T> as Mutate<_, _>>::set_attribute(
					collection_id,
					item_id,
					AttributeKey::NominalValue.into(),
					nominal_value,
				)
			})
		}

		fn encode_expiration(
			collection_id: &<T as NftsConfig>::CollectionId,
			item_id: &<T as NftsConfig>::ItemId,
			expiration: sp_staking::SessionIndex,
		) -> DispatchResult {
			expiration.using_encoded(|expiration| {
				<NftsPallet<T> as Mutate<_, _>>::set_attribute(
					collection_id,
					item_id,
					AttributeKey::Expiration.into(),
					expiration,
				)
			})
		}

		fn decode_nominal_value(
			collection_id: &<T as NftsConfig>::CollectionId,
			item_id: &<T as NftsConfig>::ItemId,
		) -> Result<T::Balance, DispatchError> {
			T::Balance::decode(
				&mut NftsPallet::<T>::system_attribute(
					collection_id,
					Some(item_id),
					AttributeKey::NominalValue.into(),
				)
				.ok_or(Error::<T>::ItemNotInitialized)?
				.as_slice(),
			)
			.map_err(|_| {
				Error::<T>::InvalidAttribute { attribute_key: AttributeKey::NominalValue }.into()
			})
		}

		fn decode_expiration(
			collection_id: &<T as NftsConfig>::CollectionId,
			item_id: &<T as NftsConfig>::ItemId,
		) -> Result<sp_staking::SessionIndex, DispatchError> {
			sp_staking::SessionIndex::decode(
				&mut NftsPallet::<T>::system_attribute(
					collection_id,
					Some(item_id),
					AttributeKey::Expiration.into(),
				)
				.ok_or(Error::<T>::ItemNotInitialized)?
				.as_slice(),
			)
			.map_err(|_| {
				Error::<T>::InvalidAttribute { attribute_key: AttributeKey::Expiration }.into()
			})
		}

		fn init_attributes(
			collection_id: &<T as NftsConfig>::CollectionId,
			item_id: &<T as NftsConfig>::ItemId,
			expiration: &[u8],
			nominal_value: &[u8],
		) -> DispatchResult {
			<NftsPallet<T> as Mutate<_, _>>::set_attribute(
				collection_id,
				item_id,
				AttributeKey::Expiration.into(),
				expiration,
			)?;

			<NftsPallet<T> as Mutate<_, _>>::set_attribute(
				collection_id,
				item_id,
				AttributeKey::NominalValue.into(),
				nominal_value,
			)
		}
	}

	// TODO: calculate weights
	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::ItemId: Incrementable,
	{
		/// Mint a new delegator NFT with provided expiration session index and nominal value.
		///
		/// # Parameters
		///
		/// - `origin`: Caller's origin.
		/// - `account_id`: Account to assign the NFT.
		/// - `expiration`: Session Index until the token is valid.
		/// - `nominal_value`: Nominal value of the NFT.
		///
		/// # Errors
		///
		/// - Pallet is not initialized.
		/// - Origin is not authorized.
		/// - Error during minting process.
		#[pallet::call_index(0)]
		pub fn mint_delegator_token(
			origin: OriginFor<T>,
			account_id: T::AccountId,
			expiration: sp_staking::SessionIndex,
			nominal_value: T::Balance,
		) -> DispatchResult {
			T::PrivilegedOrigin::ensure_origin(origin)?;

			Self::do_mint_delegator_token(&account_id, expiration, &nominal_value).map(|_| ())
		}
	}

	impl<T: Config>
		NftDelegation<T::AccountId, T::Balance, <T as NftsConfig>::ItemId, T::BindMetadata> for Pallet<T>
	where
		T::ItemId: Incrementable,
	{
		fn bind(
			delegator_id: &T::AccountId,
			item_id: &<T as NftsConfig>::ItemId,
			metadata: T::BindMetadata,
		) -> Result<(sp_staking::SessionIndex, T::Balance), DispatchError> {
			let collection_id =
				Self::collection_id().ok_or(Error::<T>::CollectionNotInitialized)?;

			ensure!(
				NftsPallet::<T>::owner(collection_id, *item_id)
					.is_some_and(|owner| owner == *delegator_id),
				Error::<T>::WrongOwner
			);

			ensure!(!Self::is_bound(item_id), Error::<T>::AlreadyBound);

			BoundItems::<T>::insert(item_id, metadata);

			<NftsPallet<T> as Transfer<_>>::disable_transfer(&collection_id, item_id)?;

			let expiration = Self::decode_expiration(&collection_id, item_id)?;
			let nominal_value = Self::decode_nominal_value(&collection_id, item_id)?;

			Self::deposit_event(Event::<T>::TokenBound { item_id: *item_id });

			Ok((expiration, nominal_value))
		}

		fn unbind(
			delegator_id: &T::AccountId,
			item_id: &<T as NftsConfig>::ItemId,
		) -> Result<(T::Balance, T::BindMetadata), DispatchError> {
			let collection_id =
				Self::collection_id().ok_or(Error::<T>::CollectionNotInitialized)?;
			ensure!(
				NftsPallet::<T>::owner(collection_id, *item_id)
					.is_some_and(|owner| owner == *delegator_id),
				Error::<T>::WrongOwner
			);

			let metadata = BoundItems::<T>::take(item_id).ok_or(Error::<T>::NotBound)?;

			<NftsPallet<T> as Transfer<_>>::enable_transfer(&collection_id, item_id)?;

			let nominal_value = Self::decode_nominal_value(&collection_id, item_id)?;

			Self::deposit_event(Event::<T>::TokenUnbound { item_id: *item_id });

			Ok((nominal_value, metadata))
		}

		fn metadata(item_id: &<T as NftsConfig>::ItemId) -> Result<T::BindMetadata, DispatchError> {
			BoundItems::<T>::get(item_id).ok_or(Error::<T>::NotBound.into())
		}

		fn set_metadata(
			item_id: &<T as NftsConfig>::ItemId,
			metadata: T::BindMetadata,
		) -> DispatchResult {
			ensure!(Self::is_bound(item_id), Error::<T>::NotBound);
			BoundItems::<T>::insert(item_id, metadata);
			Ok(())
		}

		fn nominal_value(item_id: &T::ItemId) -> Result<T::Balance, DispatchError> {
			Self::nominal_value_of(item_id)
		}

		fn set_nominal_value(item_id: &T::ItemId, new_value: T::Balance) -> DispatchResult {
			let collection_id =
				Self::collection_id().ok_or(Error::<T>::CollectionNotInitialized)?;
			ensure!(Self::is_bound(item_id), Error::<T>::NotBound);
			Self::encode_nominal_value(&collection_id, item_id, &new_value)
		}
	}

	impl<T: Config> utils::traits::SessionHook for Pallet<T>
	where
		T::ItemId: Incrementable,
	{
		fn session_started(index: utils::session_hook::SessionIndex) -> DispatchResult {
			let collection_id =
				Pallet::<T>::collection_id().ok_or(Error::<T>::CollectionNotInitialized)?;

			if let Some(tokens_expiring) = ExpiryCache::<T>::take(index) {
				for item in &tokens_expiring {
					let owner = NftsPallet::<T>::owner(collection_id, *item)
						.ok_or(Error::<T>::ItemNotInitialized)?;

					let (nominal_value, metadata) = if BoundItems::<T>::contains_key(item) {
						Pallet::<T>::unbind(&owner, item).map(|(nom, met)| (nom, Some(met)))?
					} else {
						(Pallet::<T>::decode_nominal_value(&collection_id, item)?, None)
					};

					T::NftExpirationHandler::on_expire(&owner, metadata, item, &nominal_value);
				}

				Pallet::<T>::deposit_event(Event::<T>::TokensExpired { items: tokens_expiring });
			}

			Ok(())
		}
	}
}