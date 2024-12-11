use core::ops::Add;

use sdk::{frame_support, sp_runtime, sp_staking::SessionIndex};

use codec::{Codec, MaxEncodedLen};
use frame_support::{
	pallet_prelude::{Decode, Encode, TypeInfo},
	sp_runtime::RuntimeDebug,
	traits::Get,
};

use sp_runtime::{
	traits::{ConstU32, Zero},
	BoundedVec, Perbill,
};

use super::Config;

pub const MAX_NFTS_PER_CONTRACT: u32 = 5;

#[derive(
	Copy,
	Clone,
	PartialEq,
	Eq,
	Encode,
	Decode,
	RuntimeDebug,
	TypeInfo,
	MaxEncodedLen,
	serde::Serialize,
	serde::Deserialize,
)]
pub enum PermissionType {
	PoS,
	DPoS,
}

#[derive(
	PartialEq, Eq, PartialOrd, Ord, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen,
)]
pub struct Stake<Balance, ItemId> {
	pub currency: Balance,
	pub delegated_nfts: BoundedVec<(ItemId, Balance), ConstU32<MAX_NFTS_PER_CONTRACT>>,
	pub permission_nft: Option<Balance>,
}

impl<Balance, ItemId> Stake<Balance, ItemId>
where
	Balance: Add<Balance, Output = Balance> + Copy + Zero,
{
	// An empty stake is a stake that can safely be removed from any context
	// WARNING: empty => total() = 0, BUT total() = 0 !=> empty
	// For example: total() = 0, but an nft with zero nominal value is still present
	pub fn is_empty(&self) -> bool {
		self.currency.is_zero() && self.permission_nft.is_none() && self.delegated_nfts.is_empty()
	}

	pub fn total(&self) -> Balance {
		self.currency + self.permission_value() + self.delegated_nft()
	}

	pub fn permission_value(&self) -> Balance {
		self.permission_nft.unwrap_or(Zero::zero())
	}

	pub fn delegated_nft(&self) -> Balance {
		self.delegated_nfts.iter().fold(Zero::zero(), |acc, &(_, b)| acc + b)
	}
}

impl<Balance: Default + Codec, ItemId: Codec> Default for Stake<Balance, ItemId> {
	fn default() -> Self {
		Self {
			currency: Default::default(),
			delegated_nfts: BoundedVec::default(),
			permission_nft: None,
		}
	}
}

#[derive(Default, Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct TotalValidatorStake<Balance> {
	pub total_stake: Balance,
	pub contract_count: u32,
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ValidatorDetails {
	PoS,
	DPoS { commission: Perbill, min_staking_period: u32, accept_delegations: bool },
}

impl ValidatorDetails {
	pub fn permission(&self) -> PermissionType {
		match self {
			Self::PoS => PermissionType::PoS,
			Self::DPoS { .. } => PermissionType::DPoS,
		}
	}

	/// Helper function to return commission regardless the permission type
	pub fn commission(&self) -> Perbill {
		match self {
			Self::PoS => Perbill::from_percent(100),
			Self::DPoS { commission, .. } => *commission,
		}
	}

	/// Helper function to return `min_staking_period` regardless the permission type
	pub fn min_staking_period<T: Config>(&self) -> u32 {
		match self {
			Self::PoS => T::MinimumStakingPeriod::get().into(),
			Self::DPoS { min_staking_period, .. } => *min_staking_period,
		}
	}
}

#[derive(Clone, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub struct Contract<Balance, ItemId> {
	pub stake: Stake<Balance, ItemId>,
	pub commission: Perbill,
	/// The last session where the contract is binding
	/// If using staged operations, we can now unstake
	/// otherwise we must wait for the session after.
	pub min_staking_period_end: SessionIndex,
}

impl<Balance: Default + Codec, ItemId: Codec> Default for Contract<Balance, ItemId> {
	fn default() -> Self {
		Self {
			stake: Stake::default(),
			commission: Perbill::default(),
			min_staking_period_end: Default::default(),
		}
	}
}

#[derive(Clone, Copy, Encode, Decode, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum ValidatorState {
	/// No issue with the validator
	Normal,
	/// Validator will be or has been slashed
	Faulted,
	/// Stores the SessionIndex so we can check if it is slacking
	Chilled(SessionIndex),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum ChillReason {
	/// The validator chose to chill a bit
	Manual,
	/// The validator has misbehaved in two consecutive sessions
	DoubleFault,
	/// The validator's permission nft has been slashed below a certain value
	Disqualified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Encode, Decode, TypeInfo, MaxEncodedLen)]
pub enum KickReason {
	/// The validator chose to kick the staker
	Manual,
	/// The validator ceased to be active thus the contract must be terminated
	Unbind,
}

#[derive(
	Clone, Copy, Default, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo,
)]
pub enum StagingLayer {
	#[default]
	A,
	B,
}

impl StagingLayer {
	pub fn other(self) -> Self {
		match self {
			Self::A => Self::B,
			Self::B => Self::A,
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Encode, Decode, RuntimeDebug, MaxEncodedLen, TypeInfo)]
pub enum StorageLayer {
	Staged(StagingLayer),
	Committed,
}
