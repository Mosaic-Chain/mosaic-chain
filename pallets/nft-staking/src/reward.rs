use sp_runtime::{helpers_128bit::multiply_by_rational_with_rounding, Rounding};

use crate::{Config, Contract};

#[inline]
fn rmul(a: u128, b: u128, c: u128) -> u128 {
	multiply_by_rational_with_rounding(a, b, c, Rounding::NearestPrefDown)
		.expect("no arithmical overflow")
}

pub struct Reward<Balance> {
	pub validator_reward: Balance,
	pub staker_reward: Balance,
}

pub fn calculate_reward<T: Config>(
	total_stake: u128,
	session_reward: u128,
	contract: &Contract<T::Balance>,
) -> Reward<T::Balance> {
	let contract_reward = rmul(session_reward, contract.stake.total().into(), total_stake);

	let commission: u128 = contract.commission.deconstruct().into();

	// 10^9 Tile is a Mosaic
	let validator_reward = rmul(contract_reward, commission, u128::pow(10, 9));
	let staker_reward = contract_reward.saturating_sub(validator_reward);

	Reward { validator_reward: validator_reward.into(), staker_reward: staker_reward.into() }
}
