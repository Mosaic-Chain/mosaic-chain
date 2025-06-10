use core::marker::PhantomData;

use sdk::{frame_support, frame_system, sp_application_crypto};

use frame_support::{
	pallet_prelude::ValueQuery,
	traits::{GetStorageVersion, OnRuntimeUpgrade, StorageVersion},
	weights::Weight,
};
use frame_system::LastRuntimeUpgradeInfo;
use sp_application_crypto::sr25519;

use crate::{AccountId, Runtime, VERSION};

pub mod v101 {
	use super::*;
	pub mod pallet_airdrop {
		use super::*;
		use frame_support::{storage::types::StorageValue, traits::StorageInstance};

		const AIRDROP_PALLET_PREFIX: &str = "Airdrop";

		pub struct NonceInstance;

		impl StorageInstance for NonceInstance {
			fn pallet_prefix() -> &'static str {
				AIRDROP_PALLET_PREFIX
			}

			const STORAGE_PREFIX: &'static str = "Nonce";
		}

		pub type Nonce = StorageValue<NonceInstance, u64, ValueQuery>;

		pub struct MintingAuthorityInstance;

		impl StorageInstance for MintingAuthorityInstance {
			fn pallet_prefix() -> &'static str {
				AIRDROP_PALLET_PREFIX
			}

			const STORAGE_PREFIX: &'static str = "MintingAuthority";
		}

		pub type MintingAuthority = StorageValue<MintingAuthorityInstance, sr25519::Public>;
	}
}

pub struct MigrateFromV101<Runtime>(PhantomData<Runtime>);

impl MigrateFromV101<Runtime> {}

impl OnRuntimeUpgrade for MigrateFromV101<Runtime> {
	fn on_runtime_upgrade() -> Weight {
		let Some(LastRuntimeUpgradeInfo { spec_version: codec::Compact(old_spec_version), .. }) =
			frame_system::LastRuntimeUpgrade::<Runtime>::get()
		else {
			log::error!("Runtime version info not available: skipping runtime state migration");
			return Weight::zero();
		};

		if old_spec_version > 102 || VERSION.spec_version != 103 {
			log::error!(
				"MigrateFromV101 ignored migration: {} -> {}",
				old_spec_version,
				VERSION.spec_version
			);
			return Weight::zero();
		}

		let airdrop_storage_version =
			<pallet_airdrop::Pallet<Runtime> as GetStorageVersion>::on_chain_storage_version();
		log::info!("airdrop pallet storage version was {airdrop_storage_version:?}");
		let nonce = v101::pallet_airdrop::Nonce::take();
		log::info!("Last nonce used in airdrop pallet was {nonce}");
		if let Some(minting_authority) = v101::pallet_airdrop::MintingAuthority::take() {
			let minting_authority_id = AccountId::from(minting_authority);
			log::info!("Migrating airdrop minting authority from pubkey {minting_authority:?} to account {minting_authority_id:?}");
			pallet_airdrop::MintingAuthorityId::<Runtime>::put(minting_authority_id);
		}
		StorageVersion::put::<pallet_airdrop::Pallet<Runtime>>(
			&<pallet_airdrop::Pallet<Runtime> as GetStorageVersion>::in_code_storage_version(),
		);

		log::info!("MigrateFromV101 done");

		Weight::zero()
	}
}
