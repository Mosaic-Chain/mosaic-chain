use super::*;
use sp_std::collections::btree_map::BTreeMap;
use utils::traits::{NftDelegation, OnDelegationNftExpire};

pub struct Instance;

impl frame_support::traits::StorageInstance for Instance {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "NftDelegationHandler";
}

type NftExpirationHandler = Staking;

#[derive(Encode, Decode)]
pub struct DelegatorNft {
	pub owner: AccountId,
	pub expiry: utils::SessionIndex,
	pub nominal_value: Balance,
}

#[derive(Default, Encode, Decode)]
pub struct State {
	next_item_id: ItemId,
	pub bound_tokens: BTreeMap<ItemId, AccountId>,
	pub tokens: BTreeMap<ItemId, DelegatorNft>,
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

pub type NftDelegationHandlerStore = StorageValue<Instance, State, ValueQuery>;
pub struct NftDelegationHandler;

impl NftDelegationHandler {
	pub fn mint(
		account_id: AccountId,
		expiry: utils::SessionIndex,
		nominal_value: Balance,
	) -> ItemId {
		NftDelegationHandlerStore::mutate(|state| {
			let id = state.next_item_id;
			state.next_item_id += 1;

			let nft = DelegatorNft { owner: account_id, expiry, nominal_value };

			state.tokens.insert(id, nft);
			id
		})
	}
}

impl NftDelegation<AccountId, Balance, ItemId, AccountId> for NftDelegationHandler {
	fn mint(
		account_id: &AccountId,
		expiration: utils::SessionIndex,
		nominal_value: &Balance,
	) -> Result<ItemId, DispatchError> {
		NftDelegationHandlerStore::mutate(|state| {
			let id = state.next_item_id;
			state.tokens.insert(
				id,
				DelegatorNft {
					owner: *account_id,
					expiry: expiration,
					nominal_value: *nominal_value,
				},
			);
			state.next_item_id += 1;
			Ok(id)
		})
	}

	fn bind(
		delegator_id: &AccountId,
		item_id: &ItemId,
		metadata: AccountId,
	) -> Result<(sp_staking::SessionIndex, Balance), DispatchError> {
		NftDelegationHandlerStore::mutate(|state| {
			let item = state.tokens.get(item_id).ok_or(Error::TokenDoesNotExist)?;

			ensure!(item.owner == *delegator_id, Error::WrongOwner);
			ensure!(!state.bound_tokens.contains_key(item_id), Error::AlreadyBound);

			state.bound_tokens.insert(*item_id, metadata);

			Ok((item.expiry, item.nominal_value))
		})
	}

	fn unbind(
		delegator_id: &AccountId,
		item_id: &ItemId,
	) -> Result<(Balance, AccountId), DispatchError> {
		NftDelegationHandlerStore::mutate(|state| {
			let item = state.tokens.get(item_id).ok_or(Error::TokenDoesNotExist)?;

			ensure!(item.owner == *delegator_id, Error::WrongOwner);

			let meta = state.bound_tokens.remove(item_id).ok_or(Error::NotBound)?;

			Ok((item.nominal_value, meta))
		})
	}

	fn metadata(item_id: &ItemId) -> Result<AccountId, DispatchError> {
		let state = NftDelegationHandlerStore::get();
		state.bound_tokens.get(item_id).ok_or(Error::NotBound.into()).cloned()
	}

	fn set_metadata(item_id: &ItemId, metadata: AccountId) -> DispatchResult {
		NftDelegationHandlerStore::mutate(|state| {
			ensure!(state.bound_tokens.contains_key(item_id), Error::NotBound);
			state.bound_tokens.insert(*item_id, metadata);

			Ok(())
		})
	}

	fn nominal_value(item_id: &ItemId) -> Result<Balance, DispatchError> {
		let state = NftDelegationHandlerStore::get();
		let item = state.tokens.get(item_id).ok_or(Error::TokenDoesNotExist)?;
		Ok(item.nominal_value)
	}

	fn is_bound(item_id: &ItemId) -> bool {
		NftDelegationHandlerStore::get().bound_tokens.contains_key(item_id)
	}

	fn set_nominal_value(item_id: &ItemId, new_value: Balance) -> DispatchResult {
		NftDelegationHandlerStore::mutate(|state| {
			let item = state.tokens.get_mut(item_id).ok_or(Error::TokenDoesNotExist)?;

			item.nominal_value = new_value;
			Ok(())
		})
	}
}

impl utils::traits::SessionHook for NftDelegationHandler {
	fn session_started(idx: sp_staking::SessionIndex) -> DispatchResult {
		NftDelegationHandlerStore::mutate(|state| {
			for (id, item) in state.tokens.iter().filter(|(_, item)| item.expiry == idx) {
				let metadata = if state.bound_tokens.contains_key(id) {
					state.bound_tokens.remove(id)
				} else {
					None
				};

				NftExpirationHandler::on_expire(&item.owner, metadata, id, &item.nominal_value);
			}

			Ok(())
		})
	}
}
