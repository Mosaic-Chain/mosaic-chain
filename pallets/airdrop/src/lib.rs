#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use sdk::{frame_support, frame_system, sp_core, sp_runtime, sp_std};

use codec::{Decode, DecodeWithMemTracking, Encode};
use frame_support::{
	pallet_prelude::{
		BuildGenesisConfig, DispatchResultWithPostInfo, Member, Pays, StorageValue, StorageVersion,
	},
	storage::bounded_vec::BoundedVec,
	traits::fungible::{Inspect, Mutate},
	PalletId, Parameter,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::{sr25519, Get};
use sp_runtime::{
	traits::{AccountIdConversion, AtLeast32BitUnsigned, Saturating, Zero},
	DispatchResult,
};
use sp_std::{marker::PhantomData, vec::Vec};

use utils::{
	traits::{HoldVestingSchedule, NftDelegation, NftPermission},
	vesting::Schedule as VestingSchedule,
	SessionIndex,
};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

#[expect(
	clippy::useless_conversion,
	reason = "pallet macro tries to convert PostDispatchInfo to itself"
)]
#[frame_support::pallet]
pub mod pallet {
	use super::*;

	/// The in-code storage version.
	const STORAGE_VERSION: StorageVersion = StorageVersion::new(2);

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		type Balance: Member + Parameter + AtLeast32BitUnsigned;
		type PermissionType: Member + Parameter;
		type ItemId: Clone + core::fmt::Debug;
		type DelegatorNftBindMetadata;
		type NftPermission: NftPermission<
			Self::AccountId,
			Self::Balance,
			Self::PermissionType,
			Self::ItemId,
		>;
		type NftDelegation: NftDelegation<
			Self::AccountId,
			Self::Balance,
			Self::ItemId,
			Self::DelegatorNftBindMetadata,
		>;
		type VestingSchedule: HoldVestingSchedule<
			Self::AccountId,
			Fungible = Self::Fungible,
			BlockNumber = BlockNumberFor<Self>,
		>;
		type Fungible: Inspect<Self::AccountId, Balance = Self::Balance> + Mutate<Self::AccountId>;

		type MaxAirdropsInPool: Get<u64>;

		const MAX_DELEGATOR_NFTS: u32;

		/// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::extra_constants]
	impl<T: Config> Pallet<T> {
		#[pallet::constant_name(MaxDelegatorNfts)]
		fn max_delegator_nfts() -> u32 {
			T::MAX_DELEGATOR_NFTS
		}
	}

	#[derive(TypeInfo, PartialEq, Clone)]
	#[scale_info(skip_type_params(T))]
	pub struct MaxDelegatorNftsGet<T>(PhantomData<T>);

	impl<T: Config> Get<u32> for MaxDelegatorNftsGet<T> {
		fn get() -> u32 {
			T::MAX_DELEGATOR_NFTS
		}
	}

	impl<T: Config> core::fmt::Debug for MaxDelegatorNftsGet<T> {
		fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
			f.write_fmt(core::format_args!("MaxDelegatorNfts({})", Self::get()))
		}
	}

	#[pallet::storage]
	pub type MintingAuthorityId<T: Config> = StorageValue<_, T::AccountId>;

	#[pallet::pallet]
	#[pallet::storage_version(STORAGE_VERSION)]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MintedPackage { account_id: T::AccountId },
		KeyRotated { new_account_id: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		InvalidSignature,
		ParseError,
		BadNonce,
	}

	pub type PackageOf<T> = Package<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
		<T as Config>::PermissionType,
		BlockNumberFor<T>,
		MaxDelegatorNftsGet<T>,
	>;

	#[derive(TypeInfo, Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq)]
	pub struct PermissionNft<PermissionType, Balance> {
		pub permission: PermissionType,
		pub nominal_value: Balance,
		pub metadata: Option<Vec<u8>>,
	}

	#[derive(TypeInfo, Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq)]
	pub struct DelegatorNft<Balance> {
		pub expiration: SessionIndex,
		pub nominal_value: Balance,
		pub metadata: Option<Vec<u8>>,
	}

	#[derive(TypeInfo, Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq)]
	pub struct VestingInfo<Balance, BlockNumber> {
		pub amount: Balance,
		pub unlock_per_block: Balance,
		pub start_block: Option<BlockNumber>,
	}

	impl<Balance, BlockNumber> From<VestingInfo<Balance, BlockNumber>>
		for VestingSchedule<Balance, BlockNumber>
	{
		fn from(v: VestingInfo<Balance, BlockNumber>) -> Self {
			Self { locked: v.amount, per_block: v.unlock_per_block, starting_block: v.start_block }
		}
	}

	#[derive(TypeInfo, Encode, Decode, DecodeWithMemTracking, Clone, Debug, PartialEq, Eq)]
	pub struct Package<AccountId, Balance, PermissionType, BlockNumber, MaxDelegatorNfts: Get<u32>> {
		pub account_id: AccountId,
		pub balance: Option<Balance>,
		pub vesting: Option<VestingInfo<Balance, BlockNumber>>,
		pub permission_nft: Option<PermissionNft<PermissionType, Balance>>,
		pub delegator_nfts: BoundedVec<DelegatorNft<Balance>, MaxDelegatorNfts>,
	}

	/// Identifies the pallet.
	/// Mostly used to derive default minting authority key.
	pub const PALLET_ID: PalletId = PalletId(*b"mairdrop");

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub minting_authority_id: T::AccountId,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { minting_authority_id: PALLET_ID.into_account_truncating() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MintingAuthorityId::<T>::put(self.minting_authority_id.clone());
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T>
	where
		T::AccountId: From<sr25519::Public>,
	{
		#[pallet::call_index(0)]
		#[pallet::weight(
			<T as Config>::WeightInfo::airdrop(package.delegator_nfts.len().try_into().unwrap_or(T::MAX_DELEGATOR_NFTS))
		)]
		pub fn airdrop(origin: OriginFor<T>, package: PackageOf<T>) -> DispatchResultWithPostInfo {
			let authority = ensure_signed(origin)?;

			if authority != Self::minting_authority_id() {
				return Err(Error::<T>::InvalidSignature.into());
			}

			// Ensure the account has at least enough balance to not be dusted
			let minimum_balance = T::Fungible::minimum_balance();
			let current_balance = T::Fungible::balance(&package.account_id);
			let endowed_balance = package.balance.unwrap_or_else(Zero::zero);

			let total_to_mint = endowed_balance.clone()
				+ (minimum_balance.saturating_sub(current_balance.saturating_add(endowed_balance)));

			if !total_to_mint.is_zero() {
				T::Fungible::mint_into(&package.account_id, total_to_mint)?;
			}

			if let Some(permission_nft) = &package.permission_nft {
				let item = T::NftPermission::mint(
					&package.account_id,
					&permission_nft.permission,
					&permission_nft.nominal_value,
				)?;

				if let Some(metadata) = &permission_nft.metadata {
					T::NftPermission::set_item_metadata(&item, metadata.as_slice())?;
				}
			}

			for nft in &package.delegator_nfts {
				let item = T::NftDelegation::mint(
					&package.account_id,
					nft.expiration,
					&nft.nominal_value,
				)?;

				if let Some(metadata) = &nft.metadata {
					T::NftDelegation::set_item_metadata(&item, metadata.as_slice())?;
				}
			}

			if let Some(vesting) = package.vesting.clone() {
				T::Fungible::mint_into(&package.account_id, vesting.amount.clone())?;

				let schedule = VestingSchedule::from(vesting);

				T::VestingSchedule::can_add_vesting_schedule(&package.account_id, &schedule)?;

				T::VestingSchedule::add_vesting_schedule(&package.account_id, schedule)?;
			}

			Self::deposit_event(Event::<T>::MintedPackage { account_id: package.account_id });

			Ok(Pays::No.into())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::rotate_key())]
		pub fn rotate_key(origin: OriginFor<T>, account_id: T::AccountId) -> DispatchResult {
			ensure_root(origin)?;

			MintingAuthorityId::<T>::put(account_id.clone());
			Self::deposit_event(Event::<T>::KeyRotated { new_account_id: account_id });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn minting_authority_id() -> T::AccountId {
			MintingAuthorityId::<T>::get().expect("pallet initialized properly")
		}
	}
}
