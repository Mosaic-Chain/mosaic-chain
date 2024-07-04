use frame_support::{
	dispatch::DispatchResult,
	sp_runtime::{DispatchError, Perbill},
};

pub trait OnDelegationNftExpire<AccountId, ItemId, Balance, BindMetadata> {
	fn on_expire(
		owner: &AccountId,
		bind_metadata: Option<BindMetadata>,
		item_id: &ItemId,
		nominal_value: &Balance,
	);
}

impl<AccountId, ItemId, Balance, BindMetadata>
	OnDelegationNftExpire<AccountId, ItemId, Balance, BindMetadata> for ()
{
	fn on_expire(
		_owner: &AccountId,
		_bind_metadata: Option<BindMetadata>,
		_item_id: &ItemId,
		_nominal_value: &Balance,
	) {
	}
}

pub trait NftStaking<AccountId, Balance, Variant, ItemId> {
	fn mint(
		account_id: &AccountId,
		permission: &Variant,
		nominal_value: &Balance,
	) -> Result<ItemId, DispatchError>;

	fn bind(account_id: &AccountId, item_id: &ItemId) -> Result<(Variant, Balance), DispatchError>;

	fn unbind(account_id: &AccountId) -> Result<Balance, DispatchError>;

	fn nominal_factor_of(account_id: &AccountId) -> Result<Perbill, DispatchError>;

	fn owner(item_id: &ItemId) -> Result<AccountId, DispatchError>;

	fn nominal_value(item_id: &ItemId) -> Result<Balance, DispatchError>;

	fn issued_nominal_value(item_id: &ItemId) -> Result<Balance, DispatchError>;

	fn set_nominal_value(item_id: &ItemId, new_value: Balance) -> DispatchResult;

	fn set_nominal_value_of_bound(account_id: &AccountId, new_value: Balance) -> DispatchResult;
}

// Some methods take the delegetor id to check ownership.
pub trait NftDelegation<AccountId, Balance, ItemId, BindMetadata> {
	fn bind(
		delegator_id: &AccountId,
		item_id: &ItemId,
		metadata: BindMetadata,
	) -> Result<(sp_staking::SessionIndex, Balance), DispatchError>;

	fn unbind(
		delegator_id: &AccountId,
		item_id: &ItemId,
	) -> Result<(Balance, BindMetadata), DispatchError>;

	fn metadata(item_id: &ItemId) -> Result<BindMetadata, DispatchError>;

	fn set_metadata(item_id: &ItemId, metadata: BindMetadata) -> DispatchResult;

	fn nominal_value(item_id: &ItemId) -> Result<Balance, DispatchError>;

	fn set_nominal_value(item_id: &ItemId, new_value: Balance) -> DispatchResult;
}
