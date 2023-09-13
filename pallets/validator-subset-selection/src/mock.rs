use core::marker::PhantomData;

use crate as pallet_validator_subset_selection;
use crate::Random128;
use frame_support::pallet_prelude::*;

use frame_support::traits::{ConstU16, ConstU64};
use sp_core::{Hasher, H256};
use sp_runtime::{
	generic, impl_opaque_keys,
	traits::{BlakeTwo256, ConvertInto, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};

type Block = frame_system::mocking::MockBlock<Test>;
pub type BlockNumber = u32;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		ValidatorSubsetSelection: pallet_validator_subset_selection,
	}
);

pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Nonce = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type Block = Block;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

pub struct MockRandomGenerator;

impl Random128 for MockRandomGenerator {
	fn random(subject: &[u8]) -> u128 {
		let hash_of_nonce = BlakeTwo256::hash(subject);
		u128::from_le_bytes(
			hash_of_nonce.as_ref()[0..16]
				.try_into()
				.expect("Can't convert first part of random hash to u128!"),
		)
	}
}

fn account(id: u64) -> AccountId {
	let id_as_bytes = id.to_ne_bytes();
	let zeros: [u8; 24] = [0; 24];
	let ret: [u8; 32] = [&id_as_bytes[..], &zeros[..]].concat().try_into().unwrap();
	ret.into()
}

struct SupersetSizeStorageInstance;

impl frame_support::traits::StorageInstance for SupersetSizeStorageInstance {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}
	const STORAGE_PREFIX: &'static str = "SupersetSize";
}

type SupersetSize = StorageValue<SupersetSizeStorageInstance, u64, ValueQuery>;

impl pallet_validator_subset_selection::ValidatorSuperset<AccountId> for Test {
	fn get_superset() -> Vec<AccountId> {
		let superset_size = SupersetSize::get();
		(0..superset_size).map(|n| account(n)).collect()
	}
}

impl pallet_validator_subset_selection::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type RandomGenerator = MockRandomGenerator;
	type InitialRandomGenerator = MockRandomGenerator;
	type ValidatorSuperset = Self;
}

pub fn new_test_ext(superset_size: Option<u64>) -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	pallet_validator_subset_selection::GenesisConfig::<Test> {
		initial_subset_size: 250,
		_phantom: PhantomData,
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
		SupersetSize::set(superset_size.unwrap_or_else(|| 1000));
	});
	ext
}
