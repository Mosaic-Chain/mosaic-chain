use sdk::frame_support::{
	dispatch::DispatchResult,
	sp_runtime::{DispatchError, Perbill},
};

use crate::SessionIndex;

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

pub trait NftPermission<AccountId, Balance, Variant, ItemId> {
	fn mint(
		account_id: &AccountId,
		permission: &Variant,
		nominal_value: &Balance,
	) -> Result<ItemId, DispatchError>;

	fn bind(account_id: &AccountId, item_id: &ItemId) -> Result<(Variant, Balance), DispatchError>;

	fn unbind(account_id: &AccountId) -> Result<Balance, DispatchError>;

	fn nominal_factor_of_bound(account_id: &AccountId) -> Result<Perbill, DispatchError>;

	fn owner(item_id: &ItemId) -> Result<AccountId, DispatchError>;

	fn nominal_value(item_id: &ItemId) -> Result<Balance, DispatchError>;

	fn issued_nominal_value(item_id: &ItemId) -> Result<Balance, DispatchError>;

	fn set_nominal_value(item_id: &ItemId, new_value: Balance) -> DispatchResult;

	fn set_nominal_value_of_bound(account_id: &AccountId, new_value: Balance) -> DispatchResult;
}

// Some methods take the delegetor id to check ownership.
pub trait NftDelegation<AccountId, Balance, ItemId, BindMetadata> {
	fn mint(
		account_id: &AccountId,
		expiration: crate::SessionIndex,
		nominal_value: &Balance,
	) -> Result<ItemId, DispatchError>;

	fn bind(
		delegator_id: &AccountId,
		item_id: &ItemId,
		metadata: BindMetadata,
	) -> Result<(SessionIndex, Balance), DispatchError>;

	fn unbind(
		delegator_id: &AccountId,
		item_id: &ItemId,
	) -> Result<(Balance, BindMetadata), DispatchError>;

	fn metadata_of_bound(item_id: &ItemId) -> Result<BindMetadata, DispatchError>;

	fn set_metadata_of_bound(item_id: &ItemId, metadata: BindMetadata) -> DispatchResult;

	fn nominal_value(item_id: &ItemId) -> Result<Balance, DispatchError>;

	fn is_bound(item_id: &ItemId) -> bool;

	fn set_nominal_value(item_id: &ItemId, new_value: Balance) -> DispatchResult;
}

// TODO: add more events and detailed parameters (eg.: chill reason)
pub trait StakingHooks<AccountId, Balance, ItemId> {
	/// A validator is bound
	fn on_bound(_validator: &AccountId) {}
	/// A validator is unbound
	fn on_unbound(_validator: &AccountId) {}
	/// Currency has been staked
	fn on_currency_stake(_delegator: &AccountId, _validator: &AccountId, _amount: Balance) {}
	/// Currency has been unstaked
	fn on_currency_unstake(_delegator: &AccountId, _validator: &AccountId, _amount: Balance) {}
	/// A delegator NFT has been staked
	fn on_nft_stake(_delegator: &AccountId, _validator: &AccountId, _item_id: &ItemId) {}
	/// A delegator NFT has been staked
	fn on_nft_unstake(_delegator: &AccountId, _validator: &AccountId, _item_id: &ItemId) {}
	/// Account's currency has been slashed by given amount
	fn on_currency_slash(_account_id: &AccountId, _amount: Balance) {}
	/// A delegator nft has been slashed
	fn on_nft_slash(_account_id: &AccountId, _item_id: &ItemId, _amount: Balance) {}
	/// A validator's permission nft has been slashed
	fn on_permission_nft_slash(_validator: &AccountId, _amount: Balance) {}
	/// A validator got chilled
	fn on_chill(_validator: &AccountId) {}
	/// A validator got unchilled
	fn on_unchill(_validator: &AccountId) {}
	/// Account has been rewarded with the given amount
	fn on_reward(_account_id: &AccountId, _amount: Balance) {}
	/// A delegator has been kicked by the validator
	fn on_kick(_delegator: &AccountId, _validator: &AccountId) {}
}

impl<AccountId, Balance, ItemId> StakingHooks<AccountId, Balance, ItemId> for () {}
