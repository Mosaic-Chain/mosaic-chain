use sdk::{frame_support, frame_system, pallet_balances, pallet_nfts, sp_core, sp_io, sp_runtime};

use frame_support::{derive_impl, parameter_types, traits::AsEnsureOriginWithArg, PalletId};
use sp_core::ConstU32;
use sp_runtime::{
	traits::{IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};

use crate as nft_permission;

// type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Nfts: pallet_nfts,
		NftPermission: nft_permission,
	}
);

pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId;
	type AccountData = pallet_balances::AccountData<<Self as pallet_balances::Config>::Balance>;
	type Block = Block;
	type Lookup = IdentityLookup<AccountId>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
}

impl pallet_nfts::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type CollectionId = u32;
	type ItemId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<AccountId>;
	type CreateOrigin = AsEnsureOriginWithArg<frame_system::EnsureSigned<Self::AccountId>>;
	type Locker = ();
	type CollectionDeposit = ();
	type ItemDeposit = ();
	type MetadataDepositBase = ();
	type AttributeDepositBase = ();
	type DepositPerByte = ();
	type StringLimit = ConstU32<256>;
	type KeyLimit = ConstU32<256>;
	type ValueLimit = ConstU32<256>;
	type ApprovalsLimit = ();
	type ItemAttributesApprovalsLimit = ();
	type MaxTips = ();
	type MaxDeadlineDuration = ();
	type MaxAttributesPerCall = ();
	type Features = ();
	type OffchainSignature = Signature;
	type OffchainPublic = AccountPublic;
	type WeightInfo = pallet_nfts::weights::SubstrateWeight<Test>;
	#[cfg(feature = "runtime-benchmarks")]
	type Helper = ();
}

parameter_types! {
	pub const NftPermissionPalletId: PalletId = PalletId(*b"nft_perm");
}

impl nft_permission::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = nft_permission::weights::SubstrateWeight<Test>;
	type PalletId = NftPermissionPalletId;
	type PrivilegedOrigin = frame_system::EnsureRoot<AccountId>;
	type Permission = String;
	type Balance = u64;
	const COLLECTION_DESCRIPTION: &str = "permission nfts";
}

pub fn account(id: u8) -> AccountId {
	[id; 32].into()
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	nft_permission::GenesisConfig::<Test> { unstaked_permission_holders: vec![] }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);

	ext.execute_with(|| System::set_block_number(1));

	ext
}
