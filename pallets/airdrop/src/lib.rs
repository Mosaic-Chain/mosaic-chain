#![cfg_attr(not(feature = "std"), no_std)]
// Expect lints caused by procmacros
#![expect(clippy::manual_inspect)]

use sdk::{frame_support, frame_system, sp_core, sp_std};

use codec::{Decode, Encode};
use frame_support::{
	pallet_prelude::{
		BuildGenesisConfig, InvalidTransaction, Member, StorageValue, TransactionSource,
		TransactionValidity, ValidTransaction, ValueQuery,
	},
	sp_runtime::{traits::AccountIdConversion, DispatchResult},
	storage::bounded_vec::BoundedVec,
	traits::{
		fungible::{Inspect, Mutate},
		IsType,
	},
	unsigned::ValidateUnsigned,
	PalletId, Parameter,
};
use frame_system::pallet_prelude::*;
use scale_info::TypeInfo;
use sp_core::{crypto::Pair, sr25519, Get};
use sp_std::marker::PhantomData;

use utils::{
	traits::{HoldVestingSchedule, NftDelegation, NftStaking},
	vesting::Schedule as VestingSchedule,
	SessionIndex,
};

pub use pallet::*;

// TODO: Once the pallet is ready turn off dev_mode
#[frame_support::pallet(dev_mode)]
pub mod pallet {
	use super::*;

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;
		type Balance: Member + Parameter;
		type PermissionType: Member + Parameter;
		type ItemId: Clone + core::fmt::Debug;
		type DelegatorNftBindMetadata;
		type NftStaking: NftStaking<
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

		/// NOTE: this should have the maximum value of u64::MAX - `Self::MaxAirdropsPerBlock`
		type BaseTransactionPriority: Get<u64>;
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

	#[pallet::storage]
	pub type Nonce<T: Config> = StorageValue<_, u64, ValueQuery>;

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

	type Signature = <sr25519::Pair as Pair>::Signature;

	type PackageOf<T> = Package<
		<T as frame_system::Config>::AccountId,
		<T as Config>::Balance,
		<T as Config>::PermissionType,
		BlockNumberFor<T>,
		MaxDelegatorNftsGet<T>,
	>;

	#[derive(TypeInfo, Encode, Decode, Clone, Debug, PartialEq, Eq)]
	struct PermissionNft<PermissionType, Balance> {
		permission: PermissionType,
		nominal_value: Balance,
	}

	#[derive(TypeInfo, Encode, Decode, Clone, Debug, PartialEq, Eq)]
	struct DelegatorNft<Balance> {
		expiration: SessionIndex,
		nominal_value: Balance,
	}

	#[derive(TypeInfo, Encode, Decode, Clone, Debug, PartialEq, Eq)]
	struct VestingInfo<Balance, BlockNumber> {
		amount: Balance,
		unlock_per_block: Balance,
		start_block: Option<BlockNumber>,
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
		nonce: u64,
		account_id: AccountId,
		balance: Option<Balance>,
		vesting: Option<VestingInfo<Balance, BlockNumber>>,
		permission_nft: Option<PermissionNft<PermissionType, Balance>>,
		delegator_nfts: Option<BoundedVec<DelegatorNft<Balance>, MaxDelegatorNfts>>,
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
		pub fn airdrop(
			origin: OriginFor<T>,
			package: PackageOf<T>,
			signature: Signature,
		) -> DispatchResult {
			ensure_none(origin)?;

			if !Self::check_signature(&package.encode(), &signature) {
				return Err(Error::<T>::InvalidSignature.into());
			}

			if package.nonce != Self::post_increment_nonce() {
				return Err(Error::<T>::BadNonce.into());
			}

			if let Some(balance) = &package.balance {
				T::Fungible::mint_into(&package.account_id, balance.clone())?;
			}

			if let Some(permission_nft) = &package.permission_nft {
				T::NftStaking::mint(
					&package.account_id,
					&permission_nft.permission,
					&permission_nft.nominal_value,
				)?;
			}

			if let Some(delegator_nfts) = &package.delegator_nfts {
				for nft in delegator_nfts {
					T::NftDelegation::mint(
						&package.account_id,
						nft.expiration,
						&nft.nominal_value,
					)?;
				}
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
		pub fn rotate_key(origin: OriginFor<T>, key: sr25519::Public) -> DispatchResult {
			ensure_root(origin)?;

			MintingAuthority::<T>::put(key);
			Self::deposit_event(Event::<T>::KeyRotated { new_key: key });

			Ok(())
		}
	}

	impl<T: Config> Pallet<T> {
		fn minting_authority() -> sr25519::Public {
			MintingAuthority::<T>::get().expect("pallet initalized properly")
		}

		fn post_increment_nonce() -> u64 {
			let current = Nonce::<T>::get();
			Nonce::<T>::put(current + 1);
			current
		}

		fn check_signature(data: &[u8], signature: &Signature) -> bool {
			sr25519::Pair::verify(signature, data, &Self::minting_authority())
		}
	}

	#[pallet::validate_unsigned]
	impl<T: Config> ValidateUnsigned for Pallet<T>
	where
		T::AccountId: From<sr25519::Public>,
	{
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			match call {
				Call::airdrop { package, signature } => {
					if !Self::check_signature(&package.encode(), signature) {
						return InvalidTransaction::BadSigner.into();
					}

					let current_nonce = Nonce::<T>::get();
					let nonce_limit = current_nonce.saturating_add(T::MaxAirdropsInPool::get());

					if package.nonce < current_nonce {
						return InvalidTransaction::Stale.into();
					}

					if package.nonce >= nonce_limit {
						return InvalidTransaction::Future.into();
					}

					let valid = ValidTransaction::with_tag_prefix("Airdrop")
						.priority(T::BaseTransactionPriority::get() + (nonce_limit - package.nonce))
						.longevity(1) // Ensure priority is updated after every block
						.and_provides(package.nonce); // Disallow transactions with equal nonces in the tx pool

					if package.nonce == current_nonce {
						valid.build()
					} else {
						// If the tx is not the first to be included in the next block
						// require the tx-pool to already include the tx with the previous nonce
						valid.and_requires(package.nonce - 1).build()
					}
				},

				_ => InvalidTransaction::Call.into(),
			}
		}
	}
}
