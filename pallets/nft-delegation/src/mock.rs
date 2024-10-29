use sdk::{
	frame_support, frame_system, pallet_balances, pallet_nfts, pallet_session, sp_core, sp_io,
	sp_runtime, sp_std,
};

use sp_std::collections::btree_map::BTreeMap;

use frame_support::{derive_impl, parameter_types, traits::AsEnsureOriginWithArg, PalletId};
use pallet_session::{SessionHandler, SessionManager, ShouldEndSession};
use sp_core::ConstU32;
use sp_runtime::{
	impl_opaque_keys,
	testing::UintAuthorityId,
	traits::{IdentifyAccount, IdentityLookup, OpaqueKeys, Verify},
	BuildStorage, MultiSignature, RuntimeAppPublic,
};

use crate::{self as nft_delegation, OnDelegationNftExpire};
use utils::{traits::SessionHook, SessionIndex};

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

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountId = AccountId;
	type Lookup = IdentityLookup<AccountId>;
	type Block = Block;
	type AccountData = pallet_balances::AccountData<u64>;
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

pub struct CurrentSession;
impl sp_core::Get<SessionIndex> for CurrentSession {
	fn get() -> SessionIndex {
		Session::current_index()
	}
}

impl nft_delegation::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type PalletId = NftPermissionPalletId;
	type CurrentSession = CurrentSession;
	type PrivilegedOrigin = frame_system::EnsureRoot<AccountId>;
	type NftExpirationHandler = ExpirationHandler;
	type Balance = u64;
	type BindMetadata = AccountId;
	type WeightInfo = nft_delegation::weights::SubstrateWeight<Test>;
	type MaxExpirationsPerSession = ConstU32<16>;
}

pub fn account(id: u8) -> AccountId {
	[id; 32].into()
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
