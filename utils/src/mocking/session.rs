use codec::Decode;
use sdk::{
	frame_support, pallet_session, sp_application_crypto::sr25519, sp_runtime, sp_staking, sp_std,
};

use frame_support::pallet_prelude::{StorageValue, ValueQuery};
use pallet_session::{SessionHandler, SessionManager, ShouldEndSession};
use sp_runtime::{
	impl_opaque_keys,
	traits::{Get, OpaqueKeys},
	RuntimeAppPublic,
};
use sp_std::vec::Vec;

use crate::traits::SessionHook;

use super::MockConfig;

impl_opaque_keys! {
	pub struct MockSessionKeys {
		pub dummy: sr25519::AppPublic,
	}
}

impl MockSessionKeys {
	pub fn from_index(idx: u64) -> Self {
		Self {
			dummy: sr25519::AppPublic::decode(&mut idx.to_le_bytes().repeat(32).as_slice())
				.expect("decodable bytes"),
		}
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

#[expect(
	type_alias_bounds,
	reason = "Although these bounds are not enforced they might be useful for the reader"
)]
pub type ValidatorSet<M: MockConfig> =
	StorageValue<ValidatorSetInstance, Vec<M::AccountId>, ValueQuery>;

// We end the session on each block
pub struct AlwaysEndSession;
impl<T> ShouldEndSession<T> for AlwaysEndSession {
	fn should_end_session(_now: T) -> bool {
		true
	}
}

pub struct DummySessionManager<M: MockConfig, Hook> {
	_phantom: sp_std::marker::PhantomData<(M::AccountId, Hook)>,
}

impl<M: MockConfig, Hook: SessionHook> SessionManager<M::AccountId>
	for DummySessionManager<M, Hook>
{
	fn new_session(new_index: sp_staking::SessionIndex) -> Option<Vec<M::AccountId>> {
		Hook::session_planned(new_index).unwrap();
		Some(ValidatorSet::<M>::get())
	}

	fn end_session(end_index: sp_staking::SessionIndex) {
		Hook::session_ended(end_index).unwrap();
	}

	fn start_session(start_index: sp_staking::SessionIndex) {
		Hook::session_started(start_index).unwrap();
	}

	fn new_session_genesis(idx: sp_staking::SessionIndex) -> Option<Vec<M::AccountId>> {
		Hook::session_genesis(idx).unwrap();
		None
	}
}

pub struct EmptySessionHandler<M: MockConfig>(sp_std::marker::PhantomData<M>);
impl<M: MockConfig> SessionHandler<M::AccountId> for EmptySessionHandler<M> {
	const KEY_TYPE_IDS: &'static [sp_runtime::KeyTypeId] = &[sr25519::AppPublic::ID];
	fn on_genesis_session<T: OpaqueKeys>(_validators: &[(M::AccountId, T)]) {}
	fn on_new_session<T: OpaqueKeys>(
		_changed: bool,
		_validators: &[(M::AccountId, T)],
		_queued_validators: &[(M::AccountId, T)],
	) {
	}
	fn on_disabled(_validator_index: u32) {}
	fn on_before_session_ending() {}
}
