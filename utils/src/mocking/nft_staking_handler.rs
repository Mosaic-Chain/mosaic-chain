use sdk::{frame_support, sp_runtime, sp_std};

use codec::{Decode, Encode};
use frame_support::{
	ensure,
	pallet_prelude::{StorageValue, ValueQuery},
	traits::Incrementable,
};
use sp_runtime::{DispatchError, DispatchResult, Perbill};
use sp_std::collections::btree_map::BTreeMap;

use super::MockConfig;

pub struct Instance;

impl frame_support::traits::StorageInstance for Instance {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "NftStakingHandler";
}

#[derive(Encode, Decode)]
pub struct PermissionNft<M: MockConfig> {
	pub owner: M::AccountId,
	pub permission: M::PermissionType,
	pub nominal_value: M::Balance,
	pub issued_nominal_value: M::Balance,
}

#[derive(Encode, Decode)]
pub struct State<M: MockConfig> {
	next_item_id: M::ItemId,
	pub bound_tokens: BTreeMap<M::AccountId, M::ItemId>,
	pub tokens: BTreeMap<M::ItemId, PermissionNft<M>>,
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
	InvalidNominalValue,
}

impl From<Error> for DispatchError {
	fn from(err: Error) -> Self {
		let msg = match err {
			Error::TokenDoesNotExist => "token does not exist",
			Error::WrongOwner => "wrong owner",
			Error::AlreadyBound => "already bound",
			Error::NotBound => "not bound",
			Error::InvalidNominalValue => "invalid nominal value",
		};

		Self::Other(msg)
	}
}

#[expect(
	type_alias_bounds,
	reason = "Although these bounds are not enforced they might be useful for the reader"
)]
pub type NftStakingHandler<M: MockConfig> = StorageValue<Instance, State<M>, ValueQuery>;

impl<M: MockConfig>
	crate::traits::NftPermission<M::AccountId, M::Balance, M::PermissionType, M::ItemId>
	for NftStakingHandler<M>
{
	fn mint(
		account_id: &M::AccountId,
		permission: &M::PermissionType,
		nominal_value: &M::Balance,
	) -> Result<M::ItemId, DispatchError> {
		Self::mutate(|state| {
			let id = state.next_item_id.clone();
			state.next_item_id =
				state.next_item_id.increment().expect("not running out of ItemIds in tests");

			let nft = PermissionNft {
				owner: account_id.clone(),
				permission: permission.clone(),
				nominal_value: nominal_value.clone(),
				issued_nominal_value: nominal_value.clone(),
			};

			state.tokens.insert(id.clone(), nft);
			Ok(id)
		})
	}

	fn bind(
		account_id: &M::AccountId,
		item_id: &M::ItemId,
	) -> Result<(M::PermissionType, M::Balance), DispatchError> {
		Self::mutate(|state| {
			let item = state.tokens.get(item_id).ok_or(Error::TokenDoesNotExist)?;

			ensure!(item.owner == *account_id, Error::WrongOwner);
			ensure!(!state.bound_tokens.contains_key(account_id), Error::AlreadyBound);

			state.bound_tokens.insert(account_id.clone(), item_id.clone());

			Ok((item.permission.clone(), item.nominal_value.clone()))
		})
	}

	fn unbind(account_id: &M::AccountId) -> Result<M::Balance, DispatchError> {
		Self::mutate(|state| {
			let item_id =
				state.bound_tokens.remove(account_id).ok_or(DispatchError::Other("not bound"))?;

			let item =
				state.tokens.get(&item_id).ok_or(DispatchError::Other("token does not exist"))?;

			Ok(item.nominal_value.clone())
		})
	}

	fn nominal_factor_of_bound(account_id: &M::AccountId) -> Result<Perbill, DispatchError> {
		let state = Self::get();
		let item_id =
			state.bound_tokens.get(account_id).ok_or(DispatchError::Other("not bound"))?;
		let item = state.tokens.get(item_id).ok_or(DispatchError::Other("token does not exist"))?;

		Ok(Perbill::from_rational(item.nominal_value.clone(), item.issued_nominal_value.clone()))
	}

	fn owner(item_id: &M::ItemId) -> Result<M::AccountId, DispatchError> {
		let state = Self::get();
		let item = state.tokens.get(item_id).ok_or(DispatchError::Other("token does not exist"))?;

		Ok(item.owner.clone())
	}

	fn nominal_value(item_id: &M::ItemId) -> Result<M::Balance, DispatchError> {
		let state = Self::get();
		let item = state.tokens.get(item_id).ok_or(DispatchError::Other("token does not exist"))?;

		Ok(item.nominal_value.clone())
	}

	fn issued_nominal_value(item_id: &M::ItemId) -> Result<M::Balance, DispatchError> {
		let state = Self::get();
		let item = state.tokens.get(item_id).ok_or(DispatchError::Other("token does not exist"))?;

		Ok(item.issued_nominal_value.clone())
	}

	fn set_nominal_value(item_id: &M::ItemId, new_value: M::Balance) -> DispatchResult {
		Self::mutate(|state| {
			let item = state
				.tokens
				.get_mut(item_id)
				.ok_or(DispatchError::Other("token does not exist"))?;

			ensure!(
				item.issued_nominal_value >= new_value,
				DispatchError::Other("invalid nominal value")
			);

			item.nominal_value = new_value;

			Ok(())
		})
	}

	fn set_nominal_value_of_bound(
		account_id: &M::AccountId,
		new_value: M::Balance,
	) -> DispatchResult {
		let state = Self::get();
		let item_id =
			state.bound_tokens.get(account_id).ok_or(DispatchError::Other("not bound"))?;

		Self::set_nominal_value(item_id, new_value)
	}

	fn set_item_metadata(_item_id: &M::ItemId, _metadata: &[u8]) -> DispatchResult {
		unimplemented!()
	}
}
