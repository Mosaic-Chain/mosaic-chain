// construct_runtime! macro creates some non-camel-case type names.
#![allow(non_camel_case_types)]

use core::marker::PhantomData;

use frame_support::traits::{ConstU16, ConstU64, Randomness, ValidatorSet};
use sp_application_crypto::RuntimeAppPublic;
use sp_core::{Hasher, H256};
use sp_runtime::{
	traits::{BlakeTwo256, ConvertInto, IdentifyAccount, IdentityLookup, Verify, Zero},
	BuildStorage, MultiSignature,
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

pub struct MockSessionHandler;
impl pallet_session::SessionHandler<AccountId> for MockSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] =
		&[sp_runtime::testing::UintAuthorityId::ID];
	fn on_genesis_session<T: sp_runtime::traits::OpaqueKeys>(
		_validators: &[(sp_runtime::AccountId32, T)],
	) {
	}
	fn on_new_session<T: sp_runtime::traits::OpaqueKeys>(
		_changed: bool,
		_validators: &[(sp_runtime::AccountId32, T)],
		_queued_validators: &[(sp_runtime::AccountId32, T)],
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

pub struct MockRandomGenerator;

impl<BlockNumber: Zero> Randomness<sp_core::H256, BlockNumber> for MockRandomGenerator {
	fn random(subject: &[u8]) -> (sp_core::H256, BlockNumber) {
		(BlakeTwo256::hash(subject), Zero::zero())
	}
}

pub fn account(id: u64) -> AccountId {
	let id_as_bytes = id.to_ne_bytes();
	let zeros: [u8; 24] = [0; 24];
	let ret: [u8; 32] = [&id_as_bytes[..], &zeros[..]].concat().try_into().unwrap();

	ret.into()
}

const SUPERSET_SIZE: u64 = 1000;

impl ValidatorSet<AccountId> for Test {
	type ValidatorId = AccountId;
	type ValidatorIdOf = ConvertInto;

	fn session_index() -> SessionIndex {
		Session::current_index()
	}

	fn validators() -> Vec<Self::ValidatorId> {
		(0..SUPERSET_SIZE).map(account).collect()
	}
}

impl pallet_validator_subset_selection::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type ValidatorSuperset = Self;
	type SessionHook = ();
	type Randomness = MockRandomGenerator;
	type MinSessionLength = ConstU64<1>;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
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
	});

	ext
}
