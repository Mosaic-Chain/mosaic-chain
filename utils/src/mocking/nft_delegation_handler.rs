use sdk::{frame_support, sp_runtime, sp_std};

use codec::{Decode, Encode};
use frame_support::{
	ensure,
	pallet_prelude::{StorageValue, ValueQuery},
	traits::Incrementable,
};
use sp_runtime::{DispatchError, DispatchResult};
use sp_std::collections::btree_map::BTreeMap;

use crate::{
	traits::{NftDelegation, OnDelegationNftExpire},
	SessionIndex,
};

use super::MockConfig;

pub struct Instance;

impl frame_support::traits::StorageInstance for Instance {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "NftDelegationHandler";
}

#[derive(Encode, Decode)]
pub struct DelegatorNft<M: MockConfig> {
	pub owner: M::AccountId,
	pub expiry: SessionIndex,
	pub nominal_value: M::Balance,
}

#[derive(Encode, Decode)]
pub struct State<M: MockConfig> {
	next_item_id: M::ItemId,
	pub bound_tokens: BTreeMap<M::ItemId, M::AccountId>,
	pub tokens: BTreeMap<M::ItemId, DelegatorNft<M>>,
}

impl<M: MockConfig> Default for State<M> {
	fn default() -> Self {
		Self {
			next_item_id: M::ItemId::initial_value().expect("has initial value"),
			bound_tokens: BTreeMap::default(),
			tokens: BTreeMap::default(),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
	TokenDoesNotExist,
	WrongOwner,
	AlreadyBound,
	NotBound,
}

impl From<Error> for DispatchError {
	fn from(err: Error) -> Self {
		let msg = match err {
			Error::TokenDoesNotExist => "token does not exist",
			Error::WrongOwner => "wrong owner",
			Error::AlreadyBound => "already bound",
			Error::NotBound => "not bound",
		};

		Self::Other(msg)
	}
}

#[expect(
	type_alias_bounds,
	reason = "Although these bounds are not enforced they might be useful for the reader"
)]
pub type NftDelegationHandlerStore<M: MockConfig> = StorageValue<Instance, State<M>, ValueQuery>;
pub struct NftDelegationHandler<M>(sp_std::marker::PhantomData<M>);

impl<M: MockConfig> NftDelegationHandler<M> {
	pub fn mint(
		account_id: M::AccountId,
		expiry: SessionIndex,
		nominal_value: M::Balance,
	) -> M::ItemId {
		NftDelegationHandlerStore::<M>::mutate(|state| {
			let id = state.next_item_id.clone();
			state.next_item_id =
				state.next_item_id.increment().expect("not running out of ItemIds in tests");

			let nft = DelegatorNft { owner: account_id, expiry, nominal_value };

			state.tokens.insert(id.clone(), nft);
			id
		})
	}
}

impl<M: MockConfig> NftDelegation<M::AccountId, M::Balance, M::ItemId, M::AccountId>
	for NftDelegationHandler<M>
{
	fn mint(
		account_id: &M::AccountId,
		expiration: SessionIndex,
		nominal_value: &M::Balance,
	) -> Result<M::ItemId, DispatchError> {
		NftDelegationHandlerStore::<M>::mutate(|state| {
			let id = state.next_item_id.clone();
			state.tokens.insert(
				id.clone(),
				DelegatorNft {
					owner: account_id.clone(),
					expiry: expiration,
					nominal_value: nominal_value.clone(),
				},
			);
			state.next_item_id =
				state.next_item_id.increment().expect("not running out of ItemIds in tests");

			Ok(id)
		})
	}

	fn bind(
		delegator_id: &M::AccountId,
		item_id: &M::ItemId,
		metadata: M::AccountId,
	) -> Result<(SessionIndex, M::Balance), DispatchError> {
		NftDelegationHandlerStore::<M>::mutate(|state| {
			let item = state.tokens.get(item_id).ok_or(Error::TokenDoesNotExist)?;

			ensure!(item.owner == *delegator_id, Error::WrongOwner);
			ensure!(!state.bound_tokens.contains_key(item_id), Error::AlreadyBound);

			state.bound_tokens.insert(item_id.clone(), metadata);

			Ok((item.expiry, item.nominal_value.clone()))
		})
	}

	fn unbind(
		delegator_id: &M::AccountId,
		item_id: &M::ItemId,
	) -> Result<(M::Balance, M::AccountId), DispatchError> {
		NftDelegationHandlerStore::<M>::mutate(|state| {
			let item = state.tokens.get(item_id).ok_or(Error::TokenDoesNotExist)?;

			ensure!(item.owner == *delegator_id, Error::WrongOwner);

			let meta = state.bound_tokens.remove(item_id).ok_or(Error::NotBound)?;

			Ok((item.nominal_value.clone(), meta))
		})
	}

	fn bind_metadata(item_id: &M::ItemId) -> Result<M::AccountId, DispatchError> {
		let state = NftDelegationHandlerStore::<M>::get();
		state.bound_tokens.get(item_id).ok_or(Error::NotBound.into()).cloned()
	}

	fn set_bind_metadata(item_id: &M::ItemId, metadata: M::AccountId) -> DispatchResult {
		NftDelegationHandlerStore::<M>::mutate(|state| {
			ensure!(state.bound_tokens.contains_key(item_id), Error::NotBound);
			state.bound_tokens.insert(item_id.clone(), metadata);

			Ok(())
		})
	}

	fn nominal_value(item_id: &M::ItemId) -> Result<M::Balance, DispatchError> {
		let state = NftDelegationHandlerStore::<M>::get();
		let item = state.tokens.get(item_id).ok_or(Error::TokenDoesNotExist)?;
		Ok(item.nominal_value.clone())
	}

	fn is_bound(item_id: &M::ItemId) -> bool {
		NftDelegationHandlerStore::<M>::get().bound_tokens.contains_key(item_id)
	}

	fn set_nominal_value(item_id: &M::ItemId, new_value: M::Balance) -> DispatchResult {
		NftDelegationHandlerStore::<M>::mutate(|state| {
			let item = state.tokens.get_mut(item_id).ok_or(Error::TokenDoesNotExist)?;

			item.nominal_value = new_value;
			Ok(())
		})
	}

	fn set_item_metadata(_item_id: &M::ItemId, _metadata: &[u8]) -> DispatchResult {
		unimplemented!()
	}
}

pub struct NftDelegationExpiry<M, OnExpire>(sp_std::marker::PhantomData<(M, OnExpire)>);

impl<
		M: MockConfig,
		OnExpire: OnDelegationNftExpire<M::AccountId, M::ItemId, M::Balance, M::AccountId>,
	> crate::traits::SessionHook for NftDelegationExpiry<M, OnExpire>
{
	fn session_started(idx: SessionIndex) -> DispatchResult {
		NftDelegationHandlerStore::<M>::mutate(|state| {
			for (id, item) in state.tokens.iter().filter(|(_, item)| item.expiry == idx) {
				let metadata = if state.bound_tokens.contains_key(id) {
					state.bound_tokens.remove(id)
				} else {
					None
				};

				OnExpire::on_expire(&item.owner, metadata, id, &item.nominal_value);
			}

			Ok(())
		})
	}
}
