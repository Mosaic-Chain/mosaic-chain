#![no_std]
pub mod session_hook;
pub mod staking;
pub mod storage;
pub mod vesting;
pub mod traits {
	pub use super::{session_hook::*, staking::*, vesting::HoldVestingSchedule};
}

pub use sdk::sp_staking::SessionIndex;

#[macro_export]
macro_rules! prod_or_fast {
	($prod:expr, $test:expr) => {
		if cfg!(feature = "fast-runtime") {
			$test
		} else {
			$prod
		}
	};
}
