#![cfg_attr(not(feature = "std"), no_std)]
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

use sdk::{frame_support, frame_system, sp_core, sp_runtime, sp_std};

use codec::{Decode, Encode};
use frame_support::{
	pallet_prelude::{BuildGenesisConfig, Member, StorageValue},
	storage::bounded_vec::BoundedVec,
	traits::{
		fungible::{Inspect, Mutate},
		IsType,
	},
	PalletId, Parameter,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::{sr25519, Get};
use sp_runtime::{
	traits::{AccountIdConversion, AtLeast32BitUnsigned, Saturating, Zero},
	DispatchResult,
};
use sp_std::marker::PhantomData;

use utils::{
	traits::{HoldVestingSchedule, NftDelegation, NftPermission},
	vesting::Schedule as VestingSchedule,
	SessionIndex,
};

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::WeightInfo;

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;
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
	pub type MintingAuthority<T: Config> = StorageValue<_, sr25519::Public>;

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MintedPackage { account_id: T::AccountId },
		KeyRotated { new_key: sr25519::Public },
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

	#[derive(TypeInfo, Encode, Decode, Clone, Debug, PartialEq, Eq)]
	pub struct PermissionNft<PermissionType, Balance> {
		pub permission: PermissionType,
		pub nominal_value: Balance,
	}

	#[derive(TypeInfo, Encode, Decode, Clone, Debug, PartialEq, Eq)]
	pub struct DelegatorNft<Balance> {
		pub expiration: SessionIndex,
		pub nominal_value: Balance,
	}

	#[derive(TypeInfo, Encode, Decode, Clone, Debug, PartialEq, Eq)]
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

	#[derive(TypeInfo, Encode, Decode, Clone, Debug, PartialEq, Eq)]
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
		pub minting_authority: sr25519::Public,
		pub _phantom: PhantomData<T>,
	}

	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			Self { minting_authority: PALLET_ID.into_account_truncating(), _phantom: PhantomData }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> BuildGenesisConfig for GenesisConfig<T> {
		fn build(&self) {
			MintingAuthority::<T>::put(self.minting_authority);
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
		#[pallet::feeless_if(|_: &OriginFor<T>,_: &PackageOf<T>| -> bool { true })]
		pub fn airdrop(origin: OriginFor<T>, package: PackageOf<T>) -> DispatchResult {
			let authority = ensure_signed(origin)?;

			if authority != Self::minting_authority_account_id() {
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
				T::NftPermission::mint(
					&package.account_id,
					&permission_nft.permission,
					&permission_nft.nominal_value,
				)?;
			}

			for nft in &package.delegator_nfts {
				T::NftDelegation::mint(&package.account_id, nft.expiration, &nft.nominal_value)?;
			}

			if let Some(vesting) = package.vesting.clone() {
				T::Fungible::mint_into(&package.account_id, vesting.amount.clone())?;

				let schedule = VestingSchedule::from(vesting);

				T::VestingSchedule::can_add_vesting_schedule(&package.account_id, &schedule)?;

				T::VestingSchedule::add_vesting_schedule(&package.account_id, schedule)?;
			}

			Self::deposit_event(Event::<T>::MintedPackage { account_id: package.account_id });

			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(<T as Config>::WeightInfo::rotate_key())]
		pub fn rotate_key(origin: OriginFor<T>, key: sr25519::Public) -> DispatchResult {
			ensure_root(origin)?;

			MintingAuthority::<T>::put(key);
			Self::deposit_event(Event::<T>::KeyRotated { new_key: key });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T>
	where
		T::AccountId: From<sr25519::Public>,
	{
		fn minting_authority_account_id() -> T::AccountId {
			let public_key = MintingAuthority::<T>::get().expect("pallet initalized properly");
			public_key.into()
		}
	}
}
