use core::marker::PhantomData;

use crate as pallet_validator_subset_selection;
use crate::Random128;

use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU16, ConstU64},
	PalletId,
};
use sp_core::{ConstU32, Hasher, H256};
use sp_runtime::{
	traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, Verify},
	BuildStorage, MultiSignature,
};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test {
		System: frame_system,
		Balances: pallet_balances,
		NftPermission: pallet_nft_permission,
		Nfts: pallet_nfts,
		ValidatorSubsetSelection: pallet_validator_subset_selection,
	}
);

pub type Signature = MultiSignature;
pub type AccountPublic = <Signature as Verify>::Signer;
pub type AccountId = <AccountPublic as IdentifyAccount>::AccountId;
pub type ValidatorId = AccountId;

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

impl pallet_balances::Config for Test {
	type Balance = u64;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ConstU64<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = ConstU32<50>;
	type ReserveIdentifier = [u8; 8];
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type MaxHolds = ();
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
}

parameter_types! {
	pub const NftPermissionPalletId: PalletId = PalletId(*b"nft_perm");
}

impl utils::traits::Successor<u32> for Test {
	fn initial() -> u32 {
		0
	}

	fn successor(val: &u32) -> u32 {
		val + 1
	}
}

impl pallet_nft_permission::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type WeightInfo = pallet_nft_permission::weights::SubstrateWeight<Test>;
	type PalletId = NftPermissionPalletId;
	type PrivilegedOrigin = frame_system::EnsureRoot<AccountId>;
	type ItemIdSuccession = Self;
	type Permission = ();
	type Balance = u64;
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

impl pallet_validator_subset_selection::ValidatorSuperset<AccountId> for Test {
	fn get_superset() -> Vec<AccountId> {
		NftPermission::accounts_with_bound_permission()
			.expect("pallet is not initialized properly")
			.into_iter()
			.collect()
	}
}

impl pallet_validator_subset_selection::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = AccountId;
	type RandomGenerator = MockRandomGenerator;
	type ValidatorSuperset = Self;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	pallet_nft_permission::GenesisConfig::<Test> { initial_permission_holders: vec![] }
		.assimilate_storage(&mut t)
		.unwrap();
	pallet_validator_subset_selection::GenesisConfig::<Test> {
		initial_subset_size: 3,
		_phantom: PhantomData,
	}
	.assimilate_storage(&mut t)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}
