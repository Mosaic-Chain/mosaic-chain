use crate::{Config, Contract};

pub struct Slash<Balance> {
	pub validator_reward: Balance,
	pub staker_reward: Balance,
}

pub fn calculate_slash<T: Config>(contract: &Contract<T::Balance>) -> Slash<T::Balance> {
	todo!()
}
