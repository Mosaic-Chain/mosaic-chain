use super::*;
use mock::*;

use frame_support::{assert_err, assert_ok, traits::fungible::InspectHold};
use frame_system::RawOrigin;

#[test]
#[ignore = "takes ~20-30s to run and unlikely to change"]
fn numerical_stability() {
	let stake1 = FixedU128::from_u32(1);
	let multiplier = FixedU128::from_float(1.000_000_140_19); // ~2x every year

	let mut score1 =
		Score { active: stake1, passive: Zero::zero(), base: stake1, last_update: 0u32 };

	let mut score2 = score1.clone();

	// about 8 years
	for block in 1..=8 * YEARS {
		score1.update_active(block, multiplier, SignedRounding::High); // round high as global score does
	}

	score2.update_active(8 * YEARS, multiplier, SignedRounding::Low); // round low as account's score does

	assert!((score2.active / score1.active) > FixedU128::from_rational(999_999, 1_000_000));
}

#[test]
fn staking_currency_is_handled() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(10), // 10
				passive: FixedU128::zero(),
				base: FixedU128::from_u32(10),
				last_update: 1
			}
		);

		stake(ALICE, 5);
		skip_blocks(1);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(30), // 10 * 2 + 5 * 2
				passive: FixedU128::zero(),
				base: FixedU128::from_u32(15),
				last_update: 2
			}
		);
	});
}

#[test]
fn unstaking_currency_is_handled() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(20), // 10 * 2
				passive: FixedU128::zero(),
				base: FixedU128::from_u32(10),
				last_update: 2
			}
		);

		unstake(ALICE, 5);
		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(10), // 10 * 2 - 5 (unstaked amount) - 5 (gained score passivated)
				// T := [10 (gained) / 2^(2-1) - 1 ] = 10
				// R := (T - (10 (base) - 5)) / T = 0.5
				// Passive := R * 10 (gained) = 5;
				passive: FixedU128::from_u32(5),
				base: FixedU128::from_u32(5),
				last_update: 2
			}
		);
	});
}

#[test]
fn slashing_currency_is_handled() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(20), // 10 * 2
				passive: FixedU128::zero(),
				base: FixedU128::from_u32(10),
				last_update: 2
			}
		);

		<StakingIncentive as StakingHooks<AccountId, Balance, ()>>::on_currency_slash(&ALICE, 2);
		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(16), // 10 * 2 - 2 (unstaked amount) - 2 (gained score passivated)
				// T := [10 (gained) / 2^(2-1) - 1 ] = 10
				// R := (T - (10 (base) - 2)) / T = 2/10
				// Passive := R * 10 (gained) = 2;
				passive: FixedU128::from_u32(2),
				base: FixedU128::from_u32(8),
				last_update: 2
			}
		);
	});
}

#[test]
fn reactivating_score_works() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(20), // 10 * 2
				passive: FixedU128::zero(),
				base: FixedU128::from_u32(10),
				last_update: 2
			}
		);

		unstake(ALICE, 9);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(2), // 20 - 9 (unstaked amount) - 9 (gained score passivated)
				// T := [10 (gained) / 2^(2-1) - 1 ] = 10
				// R := (T - (10 (base) - 9)) / T = 9/10
				// Passive := R * 10 (gained) = 9
				passive: FixedU128::from_u32(9),
				base: FixedU128::from_u32(1),
				last_update: 2
			}
		);

		skip_blocks(1);
		stake(ALICE, 1);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_float(9.5), // 2 * 2 + 1 (staked amount) + 4.5 (reactivated score)
				// T := [9 (passive) / 2^(3-1) - 1 ] = 3
				// Reactivated := [1 (staked) * 9 (passive)] / (T - 1 (base)) = 4.5
				passive: FixedU128::from_float(4.5), // 9 - 4.5
				base: FixedU128::from_u32(2),
				last_update: 3
			}
		);
	});
}

#[test]
fn unstake_action_before_stake_action_should_be_handled() {
	new_test_ext(Default::default()).execute_with(|| {
		// Let's say the validator only staked their permission NFT.
		// After being rewarded with 10 units, they decide to unstake.

		unstake(ALICE, 10);

		// Score stays default (except `last_update`)
		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::zero(),
				passive: FixedU128::zero(),
				base: FixedU128::zero(),
				last_update: 1
			}
		);

		// but `first_action` is not yet set...
		assert!(Accounts::<Test>::get(ALICE).first_action.is_none());
	});
}

#[test]
fn unstaking_more_than_base_leaves_consistent_state() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(20), // 10 * 2
				passive: FixedU128::zero(),
				base: FixedU128::from_u32(10),
				last_update: 2
			}
		);

		// Let's say the account was rewarded 10 units.
		// Since rewards are not counted towards the score
		// `base` remains 10, and `active` remains 20.
		//
		// The account now unstakes 15.

		unstake(ALICE, 15);

		// Saturating everywhere
		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(0), // 20 - 15 (unstaked) - 10 (passivated)
				passive: FixedU128::from_u32(10),
				base: FixedU128::from_u32(0),
				last_update: 2
			}
		);

		skip_blocks(1);

		// Since active is 0, no change
		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(0),
				passive: FixedU128::from_u32(10),
				base: FixedU128::from_u32(0),
				last_update: 3
			}
		);

		stake(ALICE, 1);
		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(4), // 1 (staked) + 3 (reactivated)
				// T := [10 (passive) / 2^(3-1) - 1 ] = 10/3
				// Reactivated := [1 (staked) * 10 (passive)] / (T - 0 (base)) = 3
				passive: FixedU128::from_u32(7),
				base: FixedU128::from_u32(1),
				last_update: 3
			}
		);
	});
}

#[test]
fn new_payout_should_work() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, Perbill::from_percent(10))
			.unwrap();
		System::assert_last_event(Event::<Test>::NewPayout { amount: 1000, index: 0 }.into());
	});
}

#[test]
fn new_payout_should_fail_if_pool_balance_is_insufficient() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		assert_err!(
			StakingIncentive::new_payout(
				RawOrigin::Root.into(),
				100_000,
				Perbill::from_percent(10)
			),
			Error::<Test>::InsufficientFundsInPool
		);
	});
}

#[test]
fn new_payout_should_fail_if_max_payouts_are_reached() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		for _ in 0..16 {
			StakingIncentive::new_payout(RawOrigin::Root.into(), 10, Perbill::from_percent(10))
				.unwrap();
		}

		assert_err!(
			StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, Perbill::from_percent(10)),
			Error::<Test>::MaxPayoutsReached
		);
	});
}

#[test]
fn new_payout_should_fail_if_total_effective_score_is_zero() {
	new_test_ext(Default::default()).execute_with(|| {
		assert_err!(
			StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, Perbill::from_percent(10)),
			Error::<Test>::ZeroTotalEffectiveScore
		);
	});
}

#[test]
fn claiming_payout_works() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);
		unstake(ALICE, 5); // for testing cutting passive score

		StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, Perbill::from_percent(10))
			.unwrap();

		StakingIncentive::update_and_claim(RawOrigin::Signed(ALICE).into()).unwrap();

		System::assert_has_event(
			Event::<Test>::Claim { account: ALICE, amount: 1000, payout_index: 0 }.into(),
		);

		// 1000 endowed + 1000 bonus from payout
		assert_eq!(Balances::total_balance(&ALICE), 2000);
		assert_eq!(Balances::total_balance_on_hold(&ALICE), 1000);
		assert!(matches!(
			HoldVesting::get_vesting_schedule(&ALICE, 0),
			Ok(Schedule { locked: 1000, per_block: 2, starting_block: Some(2) })
		));
		assert_eq!(Balances::total_balance(&StakingIncentive::pool_account()), 9001); // 10000 - 1000 payout + 1 ED
		assert_eq!(Balances::inactive_issuance(), 9001);

		// Cut is applied
		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_float(9.5), // 10 - 5 * 9/10 (gained score is cut by 10%)
				passive: FixedU128::from_float(4.5), // 5 * 9/10
				base: FixedU128::from_u32(5),
				last_update: 2
			}
		);
	});
}

#[test]
fn account_is_paid_out_on_staking_action() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, Perbill::from_percent(10))
			.unwrap();

		unstake(ALICE, 10);

		System::assert_has_event(
			Event::<Test>::Claim { account: ALICE, amount: 1000, payout_index: 0 }.into(),
		);
	});
}

#[test]
fn multiple_payouts_can_be_claimed() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);

		for _ in 0..3 {
			skip_blocks(1);
			StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, Perbill::from_percent(10))
				.unwrap();
		}

		StakingIncentive::update_and_claim(RawOrigin::Signed(ALICE).into()).unwrap();

		for payout_index in 0..3 {
			System::assert_has_event(
				Event::<Test>::Claim { account: ALICE, amount: 1000, payout_index }.into(),
			);
		}

		// 1000 endowed +  3 * 1000 bonus from payout
		assert_eq!(Balances::total_balance(&ALICE), 4000);
		assert_eq!(Balances::total_balance(&StakingIncentive::pool_account()), 7001); // 10000 - 3000 payout + 1 ED
		assert_eq!(Balances::inactive_issuance(), 7001);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				// After 1st payout: 10 + 10 * 9/10 = 19
				// After 2nd payout: 10 + (19 * 2 - 10) * 9/10 = 35.2
				// After 3rd payout: 10 + (35.2 * 2 - 10) * 9/10 = 64.36
				active: FixedU128::from_float(64.36),
				passive: FixedU128::from_u32(0),
				base: FixedU128::from_u32(10),
				last_update: 4
			}
		);
	});
}

#[test]
fn payouts_can_be_claimed_one_after_the_other() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		for i in 0..3 {
			StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, Perbill::from_percent(10))
				.unwrap();

			StakingIncentive::update_and_claim(RawOrigin::Signed(ALICE).into()).unwrap();

			System::assert_has_event(
				Event::<Test>::Claim { account: ALICE, amount: 1000, payout_index: i }.into(),
			);

			assert_eq!(Balances::total_balance(&ALICE), 1000 + (i + 1) as u128 * 1000);
		}
	});
}

#[test]
fn payout_is_distributed_according_to_account_score() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		stake(BOB, 5);
		stake(CHARLIE, 15);
		skip_blocks(1);

		unstake(CHARLIE, 5);

		skip_blocks(1);

		assert_eq!(
			StakingIncentive::current_score_of(&ALICE).effective(),
			30.into() // 10 * 2^2 - 10
		);

		assert_eq!(
			StakingIncentive::current_score_of(&BOB).effective(),
			15.into() // 5 * 2^2 - 5
		);

		assert!(
			// (15 * 2 - 10 (unstaked + passivated)) * 2 - 10 (base) + 5 (passive)
			fixed_point_eq(StakingIncentive::current_score_of(&CHARLIE).effective(), 35.into(), 15)
		);

		StakingIncentive::new_payout(RawOrigin::Root.into(), 10000, Perbill::from_percent(10))
			.unwrap();

		StakingIncentive::update_and_claim(RawOrigin::Signed(ALICE).into()).unwrap();
		StakingIncentive::update_and_claim(RawOrigin::Signed(BOB).into()).unwrap();
		StakingIncentive::update_and_claim(RawOrigin::Signed(CHARLIE).into()).unwrap();

		System::assert_has_event(
			Event::<Test>::Claim { account: ALICE, amount: 3750, payout_index: 0 }.into(),
		);

		System::assert_has_event(
			Event::<Test>::Claim { account: BOB, amount: 1875, payout_index: 0 }.into(),
		);

		System::assert_has_event(
			Event::<Test>::Claim { account: CHARLIE, amount: 4375, payout_index: 0 }.into(),
		);

		assert_eq!(Balances::total_balance(&StakingIncentive::pool_account()), 1); // 10000 - 10000 payout + 1 ED
		assert_eq!(Balances::inactive_issuance(), 1);
	});
}

#[test]
fn fuse_should_be_tripped_if_account_cant_be_paid_out() {
	new_test_ext(Default::default()).execute_with(|| {
		stake(ALICE, 10);
		skip_blocks(1);

		StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, Perbill::from_percent(10))
			.unwrap();

		// Let's say we are in an invalid state: not enough balance for payout
		Balances::set_balance(&StakingIncentive::pool_account(), 999);

		StakingIncentive::update_and_claim(RawOrigin::Signed(ALICE).into()).unwrap();

		System::assert_last_event(Event::<Test>::FuseTripped.into());
		assert_eq!(Balances::total_balance(&ALICE), 1000); // endowed amount only
		assert_eq!(Balances::total_balance(&StakingIncentive::pool_account()), 999);
		assert_eq!(Accounts::<Test>::get(ALICE).claimed_payouts, 0);
	});
}

#[test]
fn fuse_should_prevent_calls_and_staking_actions() {
	new_test_ext(Default::default()).execute_with(|| {
		// Stake and wait to ensure unstake also does nothing
		stake(ALICE, 10);
		skip_blocks(10);

		let score_before_fuse = StakingIncentive::current_score_of(&ALICE);

		StakingIncentive::trip_fuse();
		System::assert_last_event(Event::<Test>::FuseTripped.into());

		assert_err!(
			StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, Perbill::from_percent(10)),
			Error::<Test>::FuseIsTripped
		);

		assert_err!(
			StakingIncentive::update_and_claim(RawOrigin::Signed(ALICE).into()),
			Error::<Test>::FuseIsTripped
		);

		stake(ALICE, 10);
		unstake(ALICE, 10);
		<StakingIncentive as StakingHooks<AccountId, Balance, ()>>::on_currency_slash(&ALICE, 2);

		// Score does not change
		assert_eq!(StakingIncentive::current_score_of(&ALICE), score_before_fuse);
	});
}

#[test]
fn fuse_resetting_works() {
	new_test_ext(Default::default()).execute_with(|| {
		StakingIncentive::trip_fuse();
		System::assert_last_event(Event::<Test>::FuseTripped.into());

		assert_ok!(StakingIncentive::reset_fuse(RawOrigin::Root.into()));
		System::assert_last_event(Event::<Test>::FuseReset.into());

		// Can stake again
		stake(ALICE, 10);
		assert_eq!(
			StakingIncentive::current_score_of(&ALICE),
			Score {
				active: FixedU128::from_u32(10),
				passive: FixedU128::from_u32(0),
				base: FixedU128::from_u32(10),
				last_update: 1
			}
		);
	});
}

const ACC_STAKES: [[Balance; 9]; 15] = [
	[80, 100, 120, 140, 160, 180, 200, 220, 220],
	[50, 50, 100, 100, 150, 150, 200, 200, 200],
	[100, 100, 100, 200, 200, 100, 100, 100, 100],
	[100, 100, 100, 100, 100, 100, 100, 100, 100],
	[0, 0, 200, 200, 200, 200, 200, 200, 200],
	[0, 0, 150, 150, 150, 150, 150, 150, 150],
	[100, 100, 75, 75, 50, 50, 25, 25, 25],
	[50, 50, 50, 50, 50, 50, 100, 100, 100],
	[100, 20, 20, 30, 100, 100, 100, 100, 100],
	[200, 200, 200, 200, 0, 0, 0, 0, 0],
	[0, 0, 150, 150, 150, 150, 0, 0, 0],
	[100, 100, 0, 0, 100, 100, 0, 0, 0],
	[0, 0, 0, 0, 0, 0, 0, 800, 800],
	[50, 0, 0, 0, 0, 0, 800, 200, 100],
	[0, 0, 0, 0, 100, 100, 100, 100, 100],
];

#[test]
fn simulation_scenario_1() {
	// Period: 8 years
	// Payouts: false
	// 1 Simulated Year = 1 block
	// Time Factor: 0.32 => PerBlockMultiplier = 10^0.32
	new_test_ext(Some(FixedU128::from_float(2.08926))).execute_with(|| {
		let expected_weights = [
			0.164, 0.108, 0.169, 0.168, 0.076, 0.057, 0.054, 0.085, 0.057, 0.017, 0.013, 0.008,
			0.004, 0.011, 0.008,
		]
		.map(FixedU128::from_float);

		simulate_scenario(ACC_STAKES, expected_weights, None);
	});
}

#[test]
fn simulation_scenario_2() {
	// Period: 8 years
	// Payouts: true with 30% cut
	// 1 Simulated Year = 1 block
	// Time Factor: 0.32 => PerBlockMultiplier = 10^0.32
	new_test_ext(Some(FixedU128::from_float(2.08926))).execute_with(|| {
		let expected_weights = [
			0.155, 0.103, 0.168, 0.155, 0.075, 0.056, 0.079, 0.079, 0.055, 0.015, 0.012, 0.008,
			0.011, 0.019, 0.01,
		]
		.map(FixedU128::from_float);

		simulate_scenario(ACC_STAKES, expected_weights, Some(Perbill::from_percent(30)));
	});
}

fn simulate_scenario(
	acc_stakes: [[Balance; 9]; 15],
	expected_weights: [FixedU128; 15],
	cut_percent: Option<Perbill>,
) {
	for (acc, stk) in acc_stakes.iter().enumerate() {
		let acc = acc as AccountId;
		stake(acc, stk[0]);
	}

	for b in 0..8 {
		skip_blocks(1);

		if b >= 4 {
			if let Some(cut) = cut_percent {
				assert_ok!(StakingIncentive::new_payout(RawOrigin::Root.into(), 1000, cut));
			}
		}

		for (acc, stk) in acc_stakes.iter().enumerate() {
			let acc = acc as AccountId;

			let prev = stk[b];
			let curr = stk[b + 1];

			if prev < curr {
				stake(acc, curr - prev);
			} else if prev > curr {
				unstake(acc, prev - curr);
			}
		}
	}

	for (acc, expected_weight) in expected_weights.iter().enumerate() {
		assert!(fixed_point_eq(weight_of(acc as AccountId), *expected_weight, 3));
	}
}
