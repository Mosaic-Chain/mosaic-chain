// construct_runtime! macro creates some non-camel-case type names.
#![allow(non_camel_case_types)]

use sdk::{frame_support, frame_system, pallet_session, sp_io, sp_runtime};

use frame_support::{
	derive_impl,
	pallet_prelude::ValueQuery,
	storage::types::StorageValue,
	traits::{ConstU64, ValidatorSet},
};
use sp_runtime::{
	traits::{ConvertInto, Get},
	BuildStorage, RuntimeAppPublic,
};

use utils::SessionIndex;

use crate::{self as pallet_validator_subset_selection};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Session: pallet_session,
		ValidatorSubsetSelection: pallet_validator_subset_selection,
	}
);

pub type AccountId = <Test as frame_system::Config>::AccountId;

pub struct Superset;
impl frame_support::traits::StorageInstance for Superset {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "Superset";
}

pub type SupersetStorage = StorageValue<Superset, Vec<AccountId>, ValueQuery>;

impl ValidatorSet<AccountId> for Superset {
	type ValidatorId = AccountId;
	type ValidatorIdOf = ConvertInto;

	fn session_index() -> SessionIndex {
		Session::current_index()
	}

	fn validators() -> Vec<Self::ValidatorId> {
		SupersetStorage::get()
	}
}

pub struct SubsetSize;
impl frame_support::traits::StorageInstance for SubsetSize {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "SubsetSize";
}

pub type SubsetSizeStorage = StorageValue<SubsetSize, u32, ValueQuery>;

impl Get<u32> for SubsetSize {
	fn get() -> u32 {
		SubsetSizeStorage::get()
	}
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
}

pub struct MockSessionHandler;
impl pallet_session::SessionHandler<AccountId> for MockSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] =
		&[sp_runtime::testing::UintAuthorityId::ID];
	fn on_genesis_session<T: sp_runtime::traits::OpaqueKeys>(_validators: &[(AccountId, T)]) {}
	fn on_new_session<T: sp_runtime::traits::OpaqueKeys>(
		_changed: bool,
		_validators: &[(AccountId, T)],
		_queued_validators: &[(AccountId, T)],
	) {
	}
	fn on_disabled(_validator_index: u32) {}
	fn on_before_session_ending() {}
}

impl sp_runtime::BoundToRuntimeAppPublic for MockSessionHandler {
	type Public = sp_runtime::testing::UintAuthorityId;
}

sp_runtime::impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: sp_runtime::testing::UintAuthorityId,
	}
}

impl pallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorIdOf = ConvertInto;
	type ShouldEndSession = ValidatorSubsetSelection;
	type NextSessionRotation = ValidatorSubsetSelection;
	type SessionManager = ValidatorSubsetSelection;
	type SessionHandler = MockSessionHandler;
	type Keys = MockSessionKeys;
	type WeightInfo = ();
}

impl pallet_validator_subset_selection::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorSuperset = Superset;
	type SubsetSize = SubsetSize;
	type SessionHook = ();
	type MinSessionLength = ConstU64<450>; // 45 minutes
}

pub fn new_test_ext(superset_size: u64, subset_size: u32) -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
	let mut ext = sp_io::TestExternalities::new(t);

	ext.execute_with(|| {
		System::set_block_number(1);
		SupersetStorage::put((0..superset_size).collect::<Vec<_>>());
		SubsetSizeStorage::put(subset_size);
	});

	ext
}
