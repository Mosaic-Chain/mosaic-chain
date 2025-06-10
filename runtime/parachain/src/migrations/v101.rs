use core::marker::PhantomData;

use sdk::{
	frame_support, frame_system, pallet_balances, pallet_membership, pallet_session, sp_core,
	sp_runtime, sp_std,
};

use frame_support::{
	pallet_prelude::BuildGenesisConfig,
	traits::{fungible::Mutate, OnRuntimeUpgrade},
	weights::Weight,
};

use frame_system::LastRuntimeUpgradeInfo;

use sp_core::{ByteArray, Pair, Public};
use sp_runtime::{impl_opaque_keys, BoundedVec};
use sp_std::vec::Vec;

use pallet_balances::Instance1;
use pallet_im_online::sr25519::AuthorityId as ImOnlineId;
use pallet_nft_staking::PermissionType;

use crate::{
	collectives, funds,
	opaque::SessionKeys,
	params::{currency::MOSAIC, time::MINUTES},
	AccountId, Aura, Balances, Runtime, System, VERSION,
};

pub fn public_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
	TPublic::Pair::from_string(seed, None)
		.expect("static values are valid; qed")
		.public()
}

pub fn public_from_hex<TPublic: Public>(hex: &str) -> <TPublic::Pair as Pair>::Public {
	sp_core::bytes::from_hex(hex)
		.expect("static values are valid hex")
		.as_slice()
		.try_into()
		.expect("static value represents the key")
}

fn account_from_seed(seed: &str) -> AccountId {
	public_from_seed::<sp_core::sr25519::Public>(seed).into()
}

fn account_from_hex(hex: &str) -> AccountId {
	public_from_hex::<sp_core::sr25519::Public>(hex).into()
}

fn dummy_im_online_key_from_aura(
	aura: &sdk::sp_consensus_aura::sr25519::AuthorityId,
) -> ImOnlineId {
	aura.as_slice().try_into().expect("aura id can be cast to ImOnlineId")
}

pub struct MigrateFromV100<Runtime>(PhantomData<Runtime>);

impl MigrateFromV100<Runtime> {
	fn council_members(
	) -> BoundedVec<AccountId, <Runtime as pallet_membership::Config<Instance1>>::MaxMembers> {
		if cfg!(feature = "local") {
			Self::accounts()
		} else {
			// NOTE: these are the addresses collected from the members of the polkadot
			// admin multisig.
			[
				"0x56635c6ed226344060eae9313b47472ef7bdc4ca9e4d3eb8ccee73badfe1ec42",
				"0xa04ed7f7003af5b1f2cdc596c58ef31a58d2a12899f8e2b0ebe0ea10b5dcfa72",
				"0xaa3c8082e71c64e4652e2e5b11d0f1b8aa0dcab0e48bfbc6aa395978630acf48",
				"0xb4dd5b7a774828c2a267175a43347d0b06c28cdb9fd4443954ca167e07a1af2b",
				"0x6efe1c45d2ad830ff71c16c26e005b6d0f6f40feadb72cfa6e010a9541444b77",
				"0x269230f6b7087977655840a851769f109af8a28a7c2900604805141025b7d123",
			]
			.into_iter()
			.map(account_from_hex)
			.collect()
		}
		.try_into()
		.expect("no more accounts than MaxMembers")
	}

	fn accounts() -> Vec<AccountId> {
		if cfg!(feature = "local") {
			["//Alice", "//Bob", "//Charlie", "//Dave", "//Eve", "//Ferdie"]
				.into_iter()
				.map(account_from_seed)
				.collect()
		} else if cfg!(feature = "mainnet") {
			[
				"0x1ee256f0b5b975c51b62e199ae1796b342d9337aa2b1dbc9777d44107446ae1d",
				"0x5e4c345149989cfdba0f452e5e2e132901d85ad513269fdc06a77bf205d0cf67",
				"0xdc6b9379f2f366ea7a60dae93db47b479dfa4def30962c40428167b225e9285c",
				"0x6ebbc72a185b1b4ebc38ca63ce142a667b816a54dcc94ed25dcdb022cecce13a",
			]
			.into_iter()
			.map(account_from_hex)
			.collect()
		} else {
			[
				"0x40dfa76970d4764caf433a9fab6d182e1e4d930e708e1c4f2037745126f4c368",
				"0xecf63d434c813fead2bd9f609505a4f4c2458353038b1c207d71ef4247457316",
				"0x94ceb7088b35c85435a29912a8a5e6245fe3b8b8dd448f2fefcdd3688f86f55a",
				"0x720122b86ea8eded8598ed8daf51ccd02bfd437cc5a521d9fa54f4e21e38ea39",
				"0xfedf595419ad6922807543d56ed10b1ef306558b27e562b770392b7c25e12132",
				"0xc87afa66b5d934ffa2d486953ce9da7dc3648317855821b462072628506e3e0e",
			]
			.into_iter()
			.map(account_from_hex)
			.collect()
		}
	}

	fn minting_authority_id() -> AccountId {
		let public_key = if cfg!(feature = "local") {
			public_from_seed::<sp_core::sr25519::Public>("//MintingAuthority")
		} else if cfg!(feature = "mainnet") {
			public_from_hex::<sp_core::sr25519::Public>(
				"0xca494ec6c3f000a1ad37c0fd8ef176007a54480c5328cdd1003f4ec1324d1927",
			)
		} else {
			public_from_hex::<sp_core::sr25519::Public>(
				"0x724057d84b455a2fef18d9d1ddf6a0b524d69954a0bf997a902a64dce6d22d35",
			)
		};
		public_key.into()
	}
}

impl OnRuntimeUpgrade for MigrateFromV100<Runtime> {
	fn on_runtime_upgrade() -> Weight {
		let Some(LastRuntimeUpgradeInfo { spec_version: codec::Compact(old_spec_version), .. }) =
			frame_system::LastRuntimeUpgrade::<Runtime>::get()
		else {
			log::error!("Runtime version info not available: skipping runtime state migration");
			return Weight::zero();
		};

		if old_spec_version > 100 || VERSION.spec_version != 103 {
			log::error!(
				"MigrateFromV100 ignored migration: {} -> {}",
				old_spec_version,
				VERSION.spec_version
			);
			return Weight::zero();
		}

		// Endow funds
		for (fund, amount) in [
			(funds::treasury::Account::get(), 10_000_000 * MOSAIC),
			(funds::development_fund::Account::get(), 24_000_000 * MOSAIC),
			(funds::financial_fund::Account::get(), 20_000_000 * MOSAIC),
			(funds::community_fund::Account::get(), 20_000_000 * MOSAIC),
			(funds::team_and_advisors_fund::Account::get(), 8_000_000 * MOSAIC),
			(funds::security_fund::Account::get(), 4_000_000 * MOSAIC),
			(funds::education_fund::Account::get(), 2_400_000 * MOSAIC),
		] {
			Balances::mint_into(&fund, amount).unwrap();
		}

		// Airdrop: set minting authority
		pallet_airdrop::GenesisConfig::<Runtime> {
			minting_authority_id: Self::minting_authority_id(),
		}
		.build();

		// Staking incentive: endow and deactivate pool
		pallet_staking_incentive::GenesisConfig::<Runtime> { incentive_pool: 500_000_000 * MOSAIC }
			.build();

		// nft-delegation: setup collection
		pallet_nft_delegation::GenesisConfig::<Runtime>::default().build();

		// nft-permission genesis: setup collection
		pallet_nft_permission::GenesisConfig::<Runtime>::default().build();

		// Run nft-staking genesis
		pallet_nft_staking::GenesisConfig::<Runtime> {
			initial_staking_validators: Self::accounts()
				.into_iter()
				.map(|acc| (acc, PermissionType::PoS, 0))
				.collect(),
		}
		.build();

		let period = 25 * MINUTES; // from v100
		let current_block = System::block_number();
		let current_session_end = ((current_block + period - 1) / period) * period;

		pallet_validator_subset_selection::CurrentSessionEnd::<Runtime>::put(current_session_end);

		pallet_validator_subset_selection::NextSessionEnd::<Runtime>::put(
			current_session_end + period,
		);

		pallet_validator_subset_selection::CurrentSessionLength::<Runtime>::put(period);
		pallet_validator_subset_selection::AvgSessionLength::<Runtime>::put(period);

		pallet_membership::GenesisConfig::<Runtime, collectives::council::MembershipInstance> {
			members: Self::council_members(),
			..Default::default()
		}
		.build();

		impl_opaque_keys! {
			pub struct OldSessionKeys {
				pub aura: Aura,
			}
		}

		// As the actual values of the keys are not known at runtime upgrade,
		// we initialize the `im_online` key to a (unique) dummy value with the expectation
		// that all validators should invoke `set_keys` before those keys are actually
		// required.
		pallet_session::Pallet::<Runtime>::upgrade_keys::<OldSessionKeys, _>(|_, old| {
			SessionKeys { im_online: dummy_im_online_key_from_aura(&old.aura), aura: old.aura }
		});

		log::info!("MigrateFromV100 done");

		// A runtime upgrade clears the transaction pool, and consumes the entire block
		// to ensure only one version of the runtime used per block.
		// Megrations run before any extrinsic or `on_initialize` so in a simple case like this
		// we can skip benchmarking.
		Weight::zero()
	}
}
