use super::*;

pub trait OnDelegationNftExpire<AccountId, ItemId, Balance> {
	fn on_expire(
		owner: &AccountId,
		validator: Option<&AccountId>,
		item_id: &ItemId,
		nominal_value: &Balance,
	);
}

impl<AccountId, ItemId, Balance> OnDelegationNftExpire<AccountId, ItemId, Balance> for () {
	fn on_expire(
		_owner: &AccountId,
		_validator: Option<&AccountId>,
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

	fn slash(account_id: &AccountId, slash_proportion: Perbill) -> Result<Balance, DispatchError>;

	fn chill(account_id: &AccountId) -> DispatchResult;

	fn unchill(account_id: &AccountId) -> DispatchResult;

	fn is_chilled(account_id: &AccountId) -> Result<bool, DispatchError>;
}

pub trait NftDelegation<AccountId, Balance, ItemId> {
	fn bind(
		delegator_id: &AccountId,
		validator_id: &AccountId,
		item_id: &ItemId,
	) -> Result<(SessionIndex, Balance), DispatchError>;

	fn unbind(delegator_id: &AccountId, item_id: &ItemId) -> Result<Balance, DispatchError>;

	fn slash(
		delegator_id: &AccountId,
		validator_id: &AccountId,
		slash_proportion: Perbill,
	) -> Result<Balance, DispatchError>;

	fn kick(validator_id: &AccountId, delegator_id: &AccountId) -> Result<Balance, DispatchError>;
}
