mod council;
mod vesting_freeze_staking;

use utils::run_until::{run_until, ToSession};

pub use crate::mock::*;

pub fn skip_min_staking_period() {
	run_until::<AllPalletsWithSystem, Test>(ToSession::current_plus::<Test>(
		MinimumStakingPeriod::get().get(),
	));
}

pub fn next_session() {
	run_until::<AllPalletsWithSystem, Test>(ToSession::current_plus::<Test>(1));
}
