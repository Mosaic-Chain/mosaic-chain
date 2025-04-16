use sp_runtime::{ArithmeticError, TokenError};

use super::*;

#[derive(TypeInfo, Encode, Decode, Copy, Clone, Debug, PartialEq, Eq)]
pub enum Permission {
	PoS,
	DPoS,
}

#[derive(TypeInfo, Encode, Decode, Clone, Debug, PartialEq, Eq)]
pub enum Entry {
	TokensMinted {
		account: AccountId,
		amount: Balance,
	},
	VestingScheduleAdded {
		account: AccountId,
		schedule: VestingSchedule<Balance, BlockNumberFor<Test>>,
	},
	PermissionNftMinted {
		account: AccountId,
		nominal_value: Balance,
		permission: Permission,
	},
	DelegatorNftMinted {
		account: AccountId,
		nominal_value: Balance,
		expiration: SessionIndex,
	},
	NftMetadataSet {
		item_id: ItemId,
		metadata: Vec<u8>,
	},
}

pub struct MintLog;

impl frame_support::traits::StorageInstance for MintLog {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "Assets";
}

pub type EventLogStorage = StorageValue<MintLog, Vec<Entry>, ValueQuery>;

impl MintLog {
	pub fn deposit(e: Entry) {
		EventLogStorage::mutate(|events| {
			events.push(e);
		});
	}

	pub fn assert_has_entry(e: &Entry) {
		let events = EventLogStorage::get();
		assert!(events.contains(e), "Event not found:\n{e:#?}\nCurrent events:\n{events:#?}")
	}

	pub fn assert_last_entry(e: &Entry) {
		assert!(EventLogStorage::get().last().is_some_and(|x| x == e), "last entry is not {e:?}")
	}

	pub fn assert_height(height: usize) {
		assert_eq!(Self::height(), height)
	}

	pub fn height() -> usize {
		EventLogStorage::get().len()
	}
}

impl frame_support::traits::fungible::Inspect<AccountId> for MintLog {
	type Balance = Balance;

	fn total_issuance() -> Self::Balance {
		unimplemented!()
	}

	fn minimum_balance() -> Self::Balance {
		1
	}

	fn total_balance(who: &AccountId) -> Self::Balance {
		EventLogStorage::get()
			.into_iter()
			.filter_map(|entry| match entry {
				Entry::TokensMinted { account, amount } if account == *who => Some(amount),
				_ => None,
			})
			.sum()
	}

	fn balance(who: &AccountId) -> Self::Balance {
		let total = Self::total_balance(who);
		let vesting: Balance = EventLogStorage::get()
			.into_iter()
			.filter_map(|entry| match entry {
				Entry::VestingScheduleAdded { account, schedule } if account == *who => {
					Some(schedule.locked)
				},
				_ => None,
			})
			.sum();

		total - vesting
	}

	fn reducible_balance(
		_who: &AccountId,
		_preservation: frame_support::traits::tokens::Preservation,
		_force: frame_support::traits::tokens::Fortitude,
	) -> Self::Balance {
		unimplemented!()
	}

	fn can_deposit(
		_who: &AccountId,
		_amount: Self::Balance,
		_provenance: frame_support::traits::tokens::Provenance,
	) -> frame_support::traits::tokens::DepositConsequence {
		unimplemented!()
	}

	fn can_withdraw(
		_who: &AccountId,
		_amount: Self::Balance,
	) -> frame_support::traits::tokens::WithdrawConsequence<Self::Balance> {
		unimplemented!()
	}
}

impl frame_support::traits::fungible::Unbalanced<AccountId> for MintLog {
	fn write_balance(
		_who: &AccountId,
		_amount: Self::Balance,
	) -> Result<Option<Self::Balance>, sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn handle_dust(_dust: frame_support::traits::fungible::Dust<AccountId, Self>) {
		unimplemented!()
	}

	fn set_total_issuance(_amount: Self::Balance) {
		unimplemented!()
	}
}

impl frame_support::traits::fungible::Mutate<AccountId> for MintLog {
	fn mint_into(who: &AccountId, amount: Self::Balance) -> Result<Self::Balance, DispatchError> {
		let old_balance = Self::balance(who);
		let new_balance = old_balance.checked_add(amount).ok_or(ArithmeticError::Overflow)?;

		if new_balance < Self::minimum_balance() {
			return Err(TokenError::BelowMinimum.into());
		}

		let actual = new_balance.saturating_sub(old_balance);

		MintLog::deposit(Entry::TokensMinted { account: who.clone(), amount: actual });
		Ok(actual)
	}
}

impl utils::traits::NftPermission<AccountId, Balance, Permission, ItemId> for MintLog {
	fn mint(
		account_id: &AccountId,
		permission: &Permission,
		nominal_value: &Balance,
	) -> Result<ItemId, sp_runtime::DispatchError> {
		MintLog::deposit(Entry::PermissionNftMinted {
			account: account_id.clone(),
			nominal_value: *nominal_value,
			permission: *permission,
		});

		Ok(0) // we dont care about the id
	}

	fn bind(
		_account_id: &AccountId,
		_item_id: &ItemId,
	) -> Result<(Permission, Balance), sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn unbind(_account_id: &AccountId) -> Result<Balance, sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn nominal_factor_of_bound(
		_account_id: &AccountId,
	) -> Result<sp_runtime::Perbill, sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn owner(_item_id: &ItemId) -> Result<AccountId, sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn nominal_value(_item_id: &ItemId) -> Result<Balance, sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn issued_nominal_value(_item_id: &ItemId) -> Result<Balance, sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn set_nominal_value(_item_id: &ItemId, _new_value: Balance) -> DispatchResult {
		unimplemented!()
	}

	fn set_nominal_value_of_bound(_account_id: &AccountId, _new_value: Balance) -> DispatchResult {
		unimplemented!()
	}

	fn set_item_metadata(item_id: &ItemId, metadata: &[u8]) -> DispatchResult {
		MintLog::deposit(Entry::NftMetadataSet { item_id: *item_id, metadata: metadata.to_vec() });
		Ok(())
	}
}

impl utils::traits::NftDelegation<AccountId, Balance, ItemId, AccountId> for MintLog {
	fn mint(
		account_id: &AccountId,
		expiration: utils::SessionIndex,
		nominal_value: &Balance,
	) -> Result<ItemId, sp_runtime::DispatchError> {
		MintLog::deposit(Entry::DelegatorNftMinted {
			account: account_id.clone(),
			nominal_value: *nominal_value,
			expiration,
		});

		Ok(0) // we dont care about the item id
	}

	fn bind(
		_delegator_id: &AccountId,
		_item_id: &ItemId,
		_metadata: AccountId,
	) -> Result<(SessionIndex, Balance), sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn unbind(
		_delegator_id: &AccountId,
		_item_id: &ItemId,
	) -> Result<(Balance, AccountId), sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn bind_metadata(_item_id: &ItemId) -> Result<AccountId, sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn set_bind_metadata(_item_id: &ItemId, _metadata: AccountId) -> DispatchResult {
		unimplemented!()
	}

	fn nominal_value(_item_id: &ItemId) -> Result<Balance, sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn is_bound(_item_id: &ItemId) -> bool {
		unimplemented!()
	}

	fn set_nominal_value(_item_id: &ItemId, _new_value: Balance) -> DispatchResult {
		unimplemented!()
	}

	fn set_item_metadata(item_id: &ItemId, metadata: &[u8]) -> DispatchResult {
		MintLog::deposit(Entry::NftMetadataSet { item_id: *item_id, metadata: metadata.to_vec() });
		Ok(())
	}
}

pub const INVALID_SCHEDULE_ERROR: DispatchError = DispatchError::Other("invalid vesting schedule");

impl utils::traits::HoldVestingSchedule<AccountId> for MintLog {
	type BlockNumber = BlockNumberFor<Test>;
	type Fungible = Self;

	fn add_vesting_schedule(
		who: &AccountId,
		schedule: VestingSchedule<Balance, Self::BlockNumber>,
	) -> DispatchResult {
		MintLog::deposit(Entry::VestingScheduleAdded { account: who.clone(), schedule });
		Ok(())
	}

	fn can_add_vesting_schedule(
		_who: &AccountId,
		schedule: &VestingSchedule<Balance, Self::BlockNumber>,
	) -> DispatchResult {
		if !schedule.is_valid() {
			return Err(INVALID_SCHEDULE_ERROR);
		}

		Ok(())
	}

	fn vesting_balance(_who: &AccountId) -> Option<Balance> {
		unimplemented!()
	}

	fn get_vesting_schedule(
		_who: &AccountId,
		_schedule_index: u32,
	) -> Result<VestingSchedule<Balance, Self::BlockNumber>, sp_runtime::DispatchError> {
		unimplemented!()
	}

	fn remove_vesting_schedule(_who: &AccountId, _schedule_index: u32) -> DispatchResult {
		unimplemented!()
	}
}
