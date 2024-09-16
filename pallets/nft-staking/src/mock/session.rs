use pallet_session::{SessionHandler, ShouldEndSession};
use sp_runtime::{impl_opaque_keys, traits::OpaqueKeys, RuntimeAppPublic};

use super::*;

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: UintAuthorityId,
	}
}

pub struct SessionRewardInstance;
impl frame_support::traits::StorageInstance for SessionRewardInstance {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "NftStakingSessionReward";
}

pub type SessionReward = StorageValue<SessionRewardInstance, u128, ValueQuery>;

impl Get<u128> for SessionRewardInstance {
	fn get() -> u128 {
		SessionReward::get()
	}
}

pub struct ValidatorSetInstance;

impl frame_support::traits::StorageInstance for ValidatorSetInstance {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "MockSessionValidatorSet";
}

pub type ValidatorSet = StorageValue<ValidatorSetInstance, Vec<AccountId>, ValueQuery>;

// We end the session on each block
pub struct AlwaysEndSession;
impl<T> ShouldEndSession<T> for AlwaysEndSession {
	fn should_end_session(_now: T) -> bool {
		true
	}
}

pub struct DummySessionManager<AccountId, Hook> {
	_phantom: sp_std::marker::PhantomData<(AccountId, Hook)>,
}

impl<Hook: SessionHook> SessionManager<AccountId> for DummySessionManager<AccountId, Hook> {
	fn new_session(new_index: sp_staking::SessionIndex) -> Option<Vec<AccountId>> {
		Hook::session_planned(new_index).unwrap();
		Some(ValidatorSet::get())
	}

	fn end_session(end_index: sp_staking::SessionIndex) {
		Hook::session_ended(end_index).unwrap();
	}

	fn start_session(start_index: sp_staking::SessionIndex) {
		Hook::session_started(start_index).unwrap();
	}

	fn new_session_genesis(idx: sp_staking::SessionIndex) -> Option<Vec<AccountId>> {
		Hook::session_genesis(idx).unwrap();
		None
	}
}

pub struct EmptySessionHandler;
impl SessionHandler<AccountId> for EmptySessionHandler {
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

impl sp_runtime::BoundToRuntimeAppPublic for EmptySessionHandler {
	type Public = UintAuthorityId;
}

// Testing block production, for reference see:
// https://web.archive.org/web/20230129131011/https://docs.substrate.io/test/unit-testing/#block-production
// In case of using `AlwaysEndSession` one block = one session;
pub fn run_to_block(n: u64, on_new: impl Fn(u32)) {
	let mut block_number = System::block_number();

	assert!(
		block_number < n,
		"Fix your test! It does not know that block {n} has already been created."
	);

	loop {
		block_number = System::block_number();

		if block_number >= n {
			break;
		}

		if block_number > 0 {
			Session::on_finalize(block_number);
			System::on_finalize(block_number);
		}

		System::reset_events();
		System::set_block_number(block_number + 1);

		block_number = System::block_number();

		System::on_initialize(block_number);
		Session::on_initialize(block_number);

		on_new(Session::current_index());
	}
}
