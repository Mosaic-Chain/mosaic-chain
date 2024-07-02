pub mod v101 {
	use core::marker::PhantomData;

	use crate::{AccountId, CouncilMembership, Runtime};

	use frame_support::{
		pallet_prelude::BuildGenesisConfig, traits::OnRuntimeUpgrade, weights::Weight,
	};

	use pallet_balances::Instance1;
	use sp_runtime::{codec::Decode, BoundedVec};

	use sp_std::{vec, vec::Vec};

	fn account_from_hex(hex: &str) -> AccountId {
		let data: [u8; 32] = sp_core::bytes::from_hex(hex)
			.expect("static values are valid hex")
			.try_into()
			.expect("static value represents a 32 byte long key");

		AccountId::from(data)
	}

	pub struct MigrateV100ToV101<Runtime>(PhantomData<Runtime>);

	impl MigrateV100ToV101<Runtime> {
		fn council_members(
		) -> BoundedVec<AccountId, <Runtime as pallet_membership::Config<Instance1>>::MaxMembers> {
			if cfg!(feature = "local") {
				let alice_bytes: [u8; 32] = [
					212, 53, 147, 199, 21, 253, 211, 28, 97, 20, 26, 189, 4, 169, 159, 214, 130,
					44, 133, 88, 133, 76, 205, 227, 154, 86, 132, 231, 165, 109, 162, 125,
				];
				let bob_bytes: [u8; 32] = [
					142, 175, 4, 21, 22, 135, 115, 99, 38, 201, 254, 161, 126, 37, 252, 82, 135,
					97, 54, 147, 201, 18, 144, 156, 178, 38, 170, 71, 148, 242, 106, 72,
				];
				let charlie_bytes: [u8; 32] = [
					144, 181, 171, 32, 92, 105, 116, 201, 234, 132, 27, 230, 136, 134, 70, 51, 220,
					156, 168, 163, 87, 132, 62, 234, 207, 35, 20, 100, 153, 101, 254, 34,
				];
				let dave_bytes: [u8; 32] = [
					48, 103, 33, 33, 29, 84, 4, 189, 157, 168, 142, 2, 4, 54, 10, 26, 154, 184,
					184, 124, 102, 193, 188, 47, 205, 211, 127, 60, 34, 34, 204, 32,
				];
				let eve_bytes: [u8; 32] = [
					230, 89, 167, 161, 98, 140, 221, 147, 254, 188, 4, 164, 224, 100, 110, 162, 14,
					159, 95, 12, 224, 151, 217, 160, 82, 144, 212, 169, 224, 84, 223, 78,
				];
				let ferdie_bytes: [u8; 32] = [
					28, 189, 45, 67, 83, 10, 68, 112, 90, 208, 136, 175, 49, 62, 24, 248, 11, 83,
					239, 22, 179, 97, 119, 205, 75, 119, 184, 70, 242, 165, 240, 124,
				];

				let alice =
					AccountId::decode(&mut &alice_bytes[..]).expect("could decode AccountId");
				let bob = AccountId::decode(&mut &bob_bytes[..]).expect("could decode AccountId");
				let charlie =
					AccountId::decode(&mut &charlie_bytes[..]).expect("could decode AccountId");
				let dave = AccountId::decode(&mut &dave_bytes[..]).expect("could decode AccountId");
				let eve = AccountId::decode(&mut &eve_bytes[..]).expect("could decode AccountId");
				let ferdie =
					AccountId::decode(&mut &ferdie_bytes[..]).expect("could decode AccountId");

				vec![alice, bob, charlie, dave, eve, ferdie]
					.try_into()
					.expect("no more accounts than MaxMembers")
			} else {
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
				.collect::<Vec<_>>()
				.try_into()
				.expect("no more accounts than MaxMembers")
			}
		}
	}

	impl OnRuntimeUpgrade for MigrateV100ToV101<Runtime> {
		fn on_runtime_upgrade() -> Weight {
			pallet_membership::GenesisConfig::<Runtime, CouncilMembership> {
				members: Self::council_members(),
				..Default::default()
			}
			.build();

			log::debug!("Runtime upgrade done");
			Weight::zero()
		}
	}
}
