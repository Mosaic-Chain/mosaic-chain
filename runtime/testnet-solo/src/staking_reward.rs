use sp_core::Get;
use sp_runtime::helpers_128bit::sqrt;

use super::{Balance, Balances, Runtime, Session, ValidatorSubsetSelection, MOSAIC};

pub struct SessionReward;

impl SessionReward {
	const MULTIPLIER: u128 = 125 * (MOSAIC / 10); // 12.5 MOS, 64 bit
	const MAX_SUPPLY: Balance = 2_000_000_000 * MOSAIC; // 91 bit
	const MIN_CIRCULATING: Balance = 100_000_000 * MOSAIC;
	const DENOM: u128 = sqrt(Self::MAX_SUPPLY - Self::MIN_CIRCULATING); // 91 bit / 2 ~ 46 bit

	fn calculate(
		circulating: Balance,   // 91 bit
		bound_validators: u128, // 12 bit
		active_set_size: u128,  // 9 bit
		session_length: u128,   // 10 bit
	) -> Balance {
		let n = Self::MAX_SUPPLY.saturating_sub(circulating.max(Self::MIN_CIRCULATING)); // 91 bit
		let factor = Self::MULTIPLIER * sqrt(n) / Self::DENOM; // 110 bit - 46 bit = 64 bit

		(session_length * bound_validators * factor) / active_set_size // 10 bit + 12 bit + 64 bit (nominator: 86 bit) - 9 bit = 77 bit
	}
}

impl Get<Balance> for SessionReward {
	fn get() -> Balance {
		let circulating = Balances::total_issuance(); // TODO: replace this
		let bound_validators =
			pallet_nft_staking::Validators::<Runtime>::iter_keys().count() as u128;
		let active_set_size = Session::validators().len() as u128;
		let session_length = u128::from(ValidatorSubsetSelection::current_session_length());

		Self::calculate(circulating, bound_validators, active_set_size, session_length)
	}
}

// TODO: more tests
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn circulating_below_threshold() {
		let res = SessionReward::calculate(0, 2, 1, 10);
		// Avg block reward equals MULTIPLIER, but only half of the bound validators are rewarded
		// for 10 blocks
		assert_eq!(res, SessionReward::MULTIPLIER * 2 * 10);
	}

	#[test]
	fn circulating_equals_max() {
		let res = SessionReward::calculate(SessionReward::MAX_SUPPLY, 2, 1, 10);
		assert_eq!(res, 0);
	}

	#[test]
	/// https://www.wolframalpha.com/input?i=sqrt%28%282000000000+-+x%29+%2F+%282000000000+-+100000000%29+%29+%3D+0.5
	fn factor_midpoint() {
		let res = SessionReward::calculate(1_525_000_000 * MOSAIC, 2, 1, 10);
		assert_eq!(res, SessionReward::MULTIPLIER / 2 * 2 * 10);
	}
}
