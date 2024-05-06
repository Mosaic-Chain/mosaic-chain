use sp_std::collections::btree_map::BTreeMap;

use frame_support::{
	parameter_types,
	traits::{AsEnsureOriginWithArg, ConstU16, ConstU64},
	PalletId,
};
use pallet_session::{SessionHandler, SessionManager, ShouldEndSession};
use sp_core::{ConstU32, H256};
use sp_runtime::{
	impl_opaque_keys,
	testing::UintAuthorityId,
	traits::{BlakeTwo256, IdentifyAccount, IdentityLookup, OpaqueKeys, Verify},
	BuildStorage, MultiSignature, RuntimeAppPublic,
};

use crate::{self as nft_delegation, OnDelegationNftExpire};
use utils::{traits::SessionHook, SessionIndex};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Nfts: pallet_nfts,
		NftDelegation: nft_delegation,
		Session: pallet_session,
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
	type RuntimeTask = ();
	type SingleBlockMigrations = ();
	type MultiBlockMigrator = ();
	type PreInherents = ();
	type PostInherents = ();
	type PostTransactions = ();
}

impl pallet_balances::Config for Test {
	type MaxLocks = ConstU32<50>;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];

	/// The type for recording an account's balance.
	type Balance = u64;

	/// The ubiquitous event type.
	type RuntimeEvent = RuntimeEvent;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU64<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type FreezeIdentifier = ();
	type MaxFreezes = ();
	type RuntimeHoldReason = ();
	type RuntimeFreezeReason = ();
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

// We end the session on each block
pub struct AlwaysEndSession;
impl<T> ShouldEndSession<T> for AlwaysEndSession {
	fn should_end_session(_now: T) -> bool {
		true
	}
}

pub struct OtherSessionHandler;
impl SessionHandler<AccountId> for OtherSessionHandler {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[UintAuthorityId::ID];
	fn on_genesis_session<T: OpaqueKeys>(_validators: &[(sp_runtime::AccountId32, T)]) {}
	fn on_new_session<T: OpaqueKeys>(
		_changed: bool,
		_validators: &[(sp_runtime::AccountId32, T)],
		_queued_validators: &[(sp_runtime::AccountId32, T)],
	) {
	}
	fn on_disabled(_validator_index: u32) {}
	fn on_before_session_ending() {}
}

impl sp_runtime::BoundToRuntimeAppPublic for OtherSessionHandler {
	type Public = UintAuthorityId;
}

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: UintAuthorityId,
	}
}

pub struct DummySessionManager<AccountId, Hook> {
	_phantom: sp_std::marker::PhantomData<(AccountId, Hook)>,
}

impl<AccountId, Hook: SessionHook> SessionManager<AccountId>
	for DummySessionManager<AccountId, Hook>
{
	fn new_session(new_index: SessionIndex) -> Option<Vec<AccountId>> {
		Hook::session_planned(new_index).unwrap();
		None
	}

	fn end_session(end_index: SessionIndex) {
		Hook::session_ended(end_index).unwrap();
	}

	fn start_session(start_index: SessionIndex) {
		Hook::session_started(start_index).unwrap();
	}
}

impl pallet_session::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type ValidatorId = Self::AccountId;
	type ValidatorIdOf = ();
	type ShouldEndSession = AlwaysEndSession;
	type NextSessionRotation = ();
	type SessionManager = DummySessionManager<AccountId, NftDelegation>;
	type SessionHandler = OtherSessionHandler;
	type Keys = MockSessionKeys;
	type WeightInfo = ();
}

parameter_types! {
	pub const NftPermissionPalletId: PalletId = PalletId(*b"nft_dlgt");
	pub static ExpiredTokens: BTreeMap<SessionIndex, Vec<u32>> = BTreeMap::default();
}

pub struct ExpirationHandler;

impl<AccountId, Balance> OnDelegationNftExpire<AccountId, u32, Balance, AccountId>
	for ExpirationHandler
{
	fn on_expire(
		_owner: &AccountId,
		_validator: Option<AccountId>,
		item_id: &u32,
		_nominal_value: &Balance,
	) {
		let session = Session::current_index();

		ExpiredTokens::mutate(|exp| exp.entry(session).or_default().push(*item_id));
	}
}

impl ExpirationHandler {
	pub fn expired_this_session() -> Option<Vec<u32>> {
		let session = Session::current_index();

		Self::expired_on(session)
	}

	pub fn expired_on(session: SessionIndex) -> Option<Vec<u32>> {
		ExpiredTokens::get().get(&session).cloned()
	}

	pub fn reset() {
		ExpiredTokens::take();
	}
}

impl nft_delegation::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = NftPermissionPalletId;
	type PrivilegedOrigin = frame_system::EnsureRoot<AccountId>;
	type NftExpirationHandler = ExpirationHandler;
	type Balance = u64;
	type BindMetadata = AccountId;
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	nft_delegation::GenesisConfig::<Test> { initial_token_holders: vec![] }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);

	ext.execute_with(|| System::set_block_number(1));

	ext
}
