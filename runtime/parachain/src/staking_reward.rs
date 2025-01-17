use sdk::{frame_support, sp_core, sp_runtime};

use frame_support::traits::Currency;
use sp_core::Get;
use sp_runtime::helpers_128bit::sqrt;

use super::{params, Balance, Balances, Runtime, Session};
use params::{currency::MOSAIC, dynamic::tokenomics::TokenGenerationFactor};

use pallet_validator_subset_selection::CurrentSessionLength;

pub struct SessionReward;

impl SessionReward {
	const MAX_SUPPLY: Balance = 2_000_000_000 * MOSAIC; // 91 bit
	const MIN_CIRCULATING: Balance = 100_000_000 * MOSAIC;
	const DENOM: u128 = sqrt(Self::MAX_SUPPLY - Self::MIN_CIRCULATING); // 91 bit / 2 ~ 46 bit

	fn calculate(
		multiplier: u64,        // 64 bit
		circulating: Balance,   // 91 bit
		bound_validators: u128, // 12 bit
		active_set_size: u128,  // 9 bit
		session_length: u128,   // 10 bit
	) -> Balance {
		let n = Self::MAX_SUPPLY.saturating_sub(circulating.max(Self::MIN_CIRCULATING)); // 91 bit
		let factor = multiplier as u128 * sqrt(n) / Self::DENOM; // 110 bit - 46 bit = 64 bit

		(session_length * bound_validators * factor) / active_set_size // 10 bit + 12 bit + 64 bit (nominator: 86 bit) - 9 bit = 77 bit
	}
}

impl Get<Balance> for SessionReward {
	fn get() -> Balance {
		let multiplier = TokenGenerationFactor::get();
		let circulating = Balances::active_issuance();
		let bound_validators = pallet_nft_staking::Validators::<Runtime>::count() as u128;
		let active_set_size = Session::validators().len() as u128;
		let session_length = u128::from(CurrentSessionLength::<Runtime>::get());

		Self::calculate(multiplier, circulating, bound_validators, active_set_size, session_length)
	}
}

// TODO: more tests
#[cfg(test)]
mod tests {
	use super::*;
	const MULTIPLIER: u64 = 125 * (MOSAIC / 10) as u64;

	#[test]
	fn circulating_below_threshold() {
		let res = SessionReward::calculate(MULTIPLIER, 0, 2, 1, 10);

		// Avg block reward equals MULTIPLIER, but only half of the bound validators are rewarded
		// for 10 blocks
		assert_eq!(res, MULTIPLIER as u128 * 2 * 10);
	}

	#[test]
	fn circulating_equals_max() {
		let res = SessionReward::calculate(MULTIPLIER, SessionReward::MAX_SUPPLY, 2, 1, 10);
		assert_eq!(res, 0);
	}

	#[test]
	/// https://www.wolframalpha.com/input?i=sqrt%28%282000000000+-+x%29+%2F+%282000000000+-+100000000%29+%29+%3D+0.5
	fn factor_midpoint() {
		let res = SessionReward::calculate(MULTIPLIER, 1_525_000_000 * MOSAIC, 2, 1, 10);
		assert_eq!(res, MULTIPLIER as u128 / 2 * 2 * 10);
	}
}
