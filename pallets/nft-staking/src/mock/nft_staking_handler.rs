use super::*;
use sp_std::collections::btree_map::BTreeMap;

pub struct Instance;

impl frame_support::traits::StorageInstance for Instance {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "NftStakingHandler";
}

#[derive(Encode, Decode)]
pub struct PermissionNft {
	pub owner: AccountId,
	pub permission: pallet_nft_staking::PermissionType,
	pub nominal_value: Balance,
	pub issued_nominal_value: Balance,
}

#[derive(Default, Encode, Decode)]
pub struct State {
	next_item_id: ItemId,
	pub bound_tokens: BTreeMap<AccountId, ItemId>,
	pub tokens: BTreeMap<ItemId, PermissionNft>,
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

pub type NftStakingHandler = StorageValue<Instance, State, ValueQuery>;

impl utils::traits::NftPermission<AccountId, Balance, pallet_nft_staking::PermissionType, ItemId>
	for NftStakingHandler
{
	fn mint(
		account_id: &AccountId,
		permission: &pallet_nft_staking::PermissionType,
		nominal_value: &Balance,
	) -> Result<ItemId, DispatchError> {
		Self::mutate(|state| {
			let id = state.next_item_id;
			state.next_item_id += 1;

			let nft = PermissionNft {
				owner: *account_id,
				permission: *permission,
				nominal_value: *nominal_value,
				issued_nominal_value: *nominal_value,
			};

			state.tokens.insert(id, nft);
			Ok(id)
		})
	}

	fn bind(
		account_id: &AccountId,
		item_id: &ItemId,
	) -> Result<(pallet_nft_staking::PermissionType, Balance), DispatchError> {
		Self::mutate(|state| {
			let item = state.tokens.get(item_id).ok_or(Error::TokenDoesNotExist)?;

			ensure!(item.owner == *account_id, Error::WrongOwner);
			ensure!(!state.bound_tokens.contains_key(account_id), Error::AlreadyBound);

			state.bound_tokens.insert(*account_id, *item_id);

			Ok((item.permission, item.nominal_value))
		})
	}

	fn unbind(account_id: &AccountId) -> Result<Balance, DispatchError> {
		Self::mutate(|state| {
			let item_id =
				state.bound_tokens.remove(account_id).ok_or(DispatchError::Other("not bound"))?;

			let item =
				state.tokens.get(&item_id).ok_or(DispatchError::Other("token does not exist"))?;

			Ok(item.nominal_value)
		})
	}

	fn nominal_factor_of_bound(account_id: &AccountId) -> Result<Perbill, DispatchError> {
		let state = Self::get();
		let item_id =
			state.bound_tokens.get(account_id).ok_or(DispatchError::Other("not bound"))?;
		let item = state.tokens.get(item_id).ok_or(DispatchError::Other("token does not exist"))?;

		Ok(Perbill::from_rational(item.nominal_value, item.issued_nominal_value))
	}

	fn owner(item_id: &ItemId) -> Result<AccountId, DispatchError> {
		let state = Self::get();
		let item = state.tokens.get(item_id).ok_or(DispatchError::Other("token does not exist"))?;

		Ok(item.owner)
	}

	fn nominal_value(item_id: &ItemId) -> Result<Balance, DispatchError> {
		let state = Self::get();
		let item = state.tokens.get(item_id).ok_or(DispatchError::Other("token does not exist"))?;

		Ok(item.nominal_value)
	}

	fn issued_nominal_value(item_id: &ItemId) -> Result<Balance, DispatchError> {
		let state = Self::get();
		let item = state.tokens.get(item_id).ok_or(DispatchError::Other("token does not exist"))?;

		Ok(item.issued_nominal_value)
	}

	fn set_nominal_value(item_id: &ItemId, new_value: Balance) -> DispatchResult {
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

	fn set_nominal_value_of_bound(account_id: &AccountId, new_value: Balance) -> DispatchResult {
		let state = Self::get();
		let item_id =
			state.bound_tokens.get(account_id).ok_or(DispatchError::Other("not bound"))?;

		Self::set_nominal_value(item_id, new_value)
	}

	fn set_item_metadata(_item_id: &ItemId, _metadata: &[u8]) -> DispatchResult {
		unimplemented!()
	}
}
