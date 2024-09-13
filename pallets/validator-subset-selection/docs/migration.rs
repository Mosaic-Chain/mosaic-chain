pub mod v101 {
	use crate::{
		opaque::SessionKeys, AccountId, Aura, CouncilMembership, Grandpa, ImOnlineId, Runtime,
		Signature, System, VERSION,
	};

	use frame_support::{
		pallet_prelude::BuildGenesisConfig, traits::OnRuntimeUpgrade, weights::Weight,
	};
	use frame_system::LastRuntimeUpgradeInfo;
	use pallet_nft_staking::PermissionType;
	use sp_core::{crypto::Ss58Codec, Pair};
	use sp_runtime::{
		impl_opaque_keys,
		traits::{IdentifyAccount, Verify},
	};
	use sp_std::{vec, vec::Vec};

	fn account(seed: &str) -> AccountId {
		<Signature as Verify>::Signer::from(
			sp_core::sr25519::Pair::from_string(seed, None)
				.expect("static values are valid")
				.public(),
		)
		.into_account()
	}

	/// Migrate from template runtime to solochain
	pub struct MigrateV100ToV101;

	impl OnRuntimeUpgrade for MigrateV100ToV101 {
		fn on_runtime_upgrade() -> Weight {
			let Some(LastRuntimeUpgradeInfo {
				spec_version: codec::Compact(old_spec_version), ..
			}) = frame_system::LastRuntimeUpgrade::<Runtime>::get()
			else {
				log::error!("Runtime version info not available: skipping runtime state migration");
				return Weight::zero();
			};

			if old_spec_version != 100 || VERSION.spec_version != 101 {
				log::error!(
					"Improper runtime state migration: {} -> {}",
					old_spec_version,
					VERSION.spec_version
				);
				return Weight::zero();
			}

			let alice = account("//Alice");
			let bob = account("//Bob");
			let charlie = account("//Charlie");
			let dave = account("//Dave");
			let eve = account("//Eve");
			let ferdie = account("//Ferdie");

			let im_online_ids = [
				(
					alice.clone(),
					ImOnlineId::from_ss58check("5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY")
						.unwrap(),
				),
				(
					bob.clone(),
					ImOnlineId::from_ss58check("5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty")
						.unwrap(),
				),
				(
					charlie.clone(),
					ImOnlineId::from_ss58check("5FLSigC9HGRKVhB9FiEo4Y3koPsNmBmLJbpXg2mp1hXcS59Y")
						.unwrap(),
				),
				(
					dave.clone(),
					ImOnlineId::from_ss58check("5DAAnrj7VHTznn2AWBemMuyBwZWs6FNFjdyVXUeYum3PTXFy")
						.unwrap(),
				),
				(
					eve.clone(),
					ImOnlineId::from_ss58check("5HGjWAeFDfFCWPsjFQdVV2Msvz2XtMktvgocEZcCj68kUMaw")
						.unwrap(),
				),
				(
					ferdie.clone(),
					ImOnlineId::from_ss58check("5CiPPseXPECbkjWCa6MnjNokrgYjMqmKndv2rSnekmSK2DjL")
						.unwrap(),
				),
			];

			// Run nft-delegation genesis
			pallet_nft_delegation::GenesisConfig::<Runtime>::default().build();

			// Run nft-permission genesis
			pallet_nft_permission::GenesisConfig::<Runtime>::default().build();

			// Run nft-staking genesis
			pallet_nft_staking::GenesisConfig::<Runtime> {
				initial_staking_validators: [&alice, &bob, &charlie, &dave, &eve, &ferdie]
					.into_iter()
					.map(|acc| (acc.clone(), PermissionType::PoS, 0))
					.collect(), // SpVec<(T::AccountId, PermissionType, T::Balance)>
			}
			.build();

			// Run subset selection genesis
			pallet_validator_subset_selection::GenesisConfig::<Runtime> {
				initial_subset_size: 3,
				..Default::default()
			}
			.build();

			// From template config
			let period = 6;
			let current_block = System::block_number();
			let current_session_end = ((current_block + period - 1) / period) * period;

			pallet_validator_subset_selection::CurrentSessionEnd::<Runtime>::put(
				current_session_end,
			);

			pallet_validator_subset_selection::NextSessionEnd::<Runtime>::put(
				current_session_end + period,
			);

			pallet_validator_subset_selection::CurrentSessionLength::<Runtime>::put(period);
			pallet_validator_subset_selection::AvgSessionLength::<Runtime>::put(period);

			pallet_membership::GenesisConfig::<Runtime, CouncilMembership> {
				members: vec![alice, bob, charlie].try_into().unwrap(),
				..Default::default()
			}
			.build();

			impl_opaque_keys! {
				pub struct OldSessionKeys {
					pub aura: Aura,
					pub grandpa: Grandpa,
				}
			}

			pallet_session::Pallet::<Runtime>::upgrade_keys::<OldSessionKeys, _>(
				|validator_id, old| SessionKeys {
					aura: old.aura,
					grandpa: old.grandpa,
					im_online: im_online_ids
						.iter()
						.find_map(|(a, k)| (*a == validator_id).then_some(k))
						.cloned()
						.unwrap(),
				},
			);

			// A runtime upgrade clears the transaction pool, and consumes the entire block
			// to ensure only one version of the runtime used per block.
			// Megrations run before any extrinsic or `on_initialize` so in a simple case like this
			// we can skip benchmarking.
			Weight::zero()
		}
	}
}
