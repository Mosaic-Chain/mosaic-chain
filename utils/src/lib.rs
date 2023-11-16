#![no_std]
pub mod session_hook;
pub mod staking;
pub mod traits {
	pub use super::{session_hook::*, staking::*};
}

pub use sp_staking::SessionIndex;

use frame_support::{
	pallet_prelude::DispatchResult,
	sp_runtime::{DispatchError, Perbill},
};
