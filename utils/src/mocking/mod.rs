//! Module for mocking parts of the Mosaic Chain runtime for integration tests.
//! The goal isn't to provide fully generic primitives, but rather to reduce the
//! number of pallets to be configured in mock runtimes and to ease creating testing fixtures.

use core::cmp::Ord;

use sdk::{frame_support, sp_runtime};

use codec::MaxEncodedLen;
use frame_support::{traits::Incrementable, Parameter};
use sp_runtime::traits::AtLeast32BitUnsigned;

pub mod nft_delegation_handler;
pub mod nft_staking_handler;
pub mod session;

pub use nft_delegation_handler::{
	Error as NftDelegationHandlerError, NftDelegationExpiry, NftDelegationHandler,
	NftDelegationHandlerStore,
};
pub use nft_staking_handler::{Error as NftStakingHandlerError, NftStakingHandler};
pub use session::{
	AlwaysEndSession, DummySessionManager, EmptySessionHandler, MockSessionKeys, SessionReward,
	SessionRewardInstance, ValidatorSet, ValidatorSetInstance,
};

pub trait MockConfig: 'static {
	type AccountId: Parameter + MaxEncodedLen + Ord;
	type ItemId: Parameter + MaxEncodedLen + Ord + Incrementable;
	type Balance: Parameter + Ord + AtLeast32BitUnsigned;
	type PermissionType: Parameter;
}
