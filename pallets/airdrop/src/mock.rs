#![allow(non_camel_case_types)]

use super::*;
use sdk::{
	frame_support::{
		self, derive_impl, dispatch::DispatchResult, pallet_prelude::ValueQuery, parameter_types,
		traits::fungible::Inspect,
	},
	frame_system,
	sp_application_crypto::Pair,
	sp_io,
	sp_runtime::{self, traits::AccountIdLookup, AccountId32, BuildStorage, DispatchError},
};

pub use crate as airdrop;

pub mod mint_log;
pub use mint_log::{Entry, MintLog, Permission};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Airdrop: airdrop
	}
);

pub type AccountId = AccountId32;
pub type Balance = u32;
pub type ItemId = u32;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type Lookup = AccountIdLookup<AccountId, ()>;
}

parameter_types! {
	pub const MaxAirdropsInPool: u64 = 2;
}

pub const MAX_DELEGATOR_NFTS: u32 = 2;

impl airdrop::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = Balance;
	type PermissionType = Permission;
	type ItemId = ItemId;
	type DelegatorNftBindMetadata = AccountId;
	type NftPermission = MintLog;
	type NftDelegation = MintLog;
	type VestingSchedule = MintLog;
	type Fungible = MintLog;
	type MaxAirdropsInPool = MaxAirdropsInPool;
	const MAX_DELEGATOR_NFTS: u32 = MAX_DELEGATOR_NFTS;

	type WeightInfo = ();
}

pub fn account(index: u8) -> AccountId {
	[index; 32].into()
}

pub fn minting_authority() -> sr25519::Pair {
	sr25519::Pair::from_string("//MintingAuthority", None).expect("seed is valid")
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	airdrop::GenesisConfig::<Test> { minting_authority_id: minting_authority().public().into() }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext: sp_io::TestExternalities = t.into();

	ext.execute_with(|| System::set_block_number(1));
	ext
}
