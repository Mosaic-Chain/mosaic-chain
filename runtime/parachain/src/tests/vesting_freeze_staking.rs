use super::*;

use crate::{
	frame_support::{
		assert_ok,
		traits::{fungible::InspectHold, Currency},
	},
	frame_system::RawOrigin,
};
use sdk::{
	frame_support::traits::fungible::InspectFreeze, sp_runtime::Perbill,
	sp_staking::offence::ReportOffence,
};
use utils::{
	mocking::nft_staking_handler::NftStakingHandler,
	staking::NftPermission,
	vesting::{HoldVestingSchedule, Schedule},
};

#[test]
fn vesting_schedule_converted_to_freeze_stays_frozen_after_unstake() {
	new_test_ext().execute_with(|| {
		let _ = Balances::deposit_creating(&100, 100000); // avoid overdominant stake
		let _ = Balances::deposit_creating(&10, 1000);
		assert_ok!(HoldVesting::add_vesting_schedule(&10, Schedule::new(500, 10, Some(1))));

		assert_eq!(Balances::free_balance(10), 500);
		assert_eq!(HoldVesting::vesting_balance(&10), Some(500));

		assert_ok!(VestingToFreeze::convert_schedule(RawOrigin::Signed(10).into(), 0));

		assert_eq!(Balances::free_balance(10), 1000);
		assert_eq!(Balances::usable_balance(10), 500);
		assert_eq!(HoldVesting::vesting_balance(&10), None);

		let nft =
			NftStakingHandler::<Test>::mint(&10, &pallet_nft_staking::PermissionType::DPoS, &0)
				.expect("could mint permission nft");

		assert_ok!(NftStaking::bind_validator(RawOrigin::Signed(10).into(), nft));
		assert_ok!(NftStaking::self_stake_currency(RawOrigin::Signed(10).into(), 999));

		assert_eq!(Balances::free_balance(10), 1);
		assert_eq!(Balances::total_balance_on_hold(&10), 999);
		assert_eq!(Balances::usable_balance(10), 0); // The ED is 1

		skip_min_staking_period();
		assert_ok!(NftStaking::self_unstake_currency(RawOrigin::Signed(10).into(), 999));
		next_session();

		assert_eq!(Balances::free_balance(10), 1000);
		assert_eq!(Balances::total_balance_on_hold(&10), 0);
		assert_eq!(Balances::usable_balance(10), 500);

		// Enough time has passed to thaw schedule
		assert_ok!(VestingToFreeze::thaw_expired(RawOrigin::Signed(10).into()));

		assert_eq!(Balances::free_balance(10), 1000);
		assert_eq!(Balances::usable_balance(10), 1000);
	})
}

#[test]
fn slashing_frozen_schedules() {
	new_test_ext().execute_with(|| {
		let _ = Balances::deposit_creating(&100, 100000); // avoid overdominant stake
		let _ = Balances::deposit_creating(&10, 1000);
		assert_ok!(HoldVesting::add_vesting_schedule(&10, Schedule::new(900, 10, Some(1))));
		assert_ok!(VestingToFreeze::convert_schedule(RawOrigin::Signed(10).into(), 0));
		let nft =
			NftStakingHandler::<Test>::mint(&10, &pallet_nft_staking::PermissionType::DPoS, &0)
				.expect("could mint permission nft");

		assert_ok!(NftStaking::bind_validator(RawOrigin::Signed(10).into(), nft));
		assert_ok!(NftStaking::self_stake_currency(RawOrigin::Signed(10).into(), 900));

		next_session();

		// Slash delegated stake by 50% (450)
		let offence = Offence {
			offenders: vec![10],
			session: Session::current_index(),
			slash_fraction: Perbill::from_percent(50),
		};

		Offences::report_offence(vec![42], offence).expect("Could report offence");

		next_session();

		assert_eq!(Balances::total_balance(&10), 550);
		assert_eq!(Balances::free_balance(10), 100);
		assert_eq!(Balances::total_balance_on_hold(&10), 450);
		assert_eq!(Balances::usable_balance(10), 0);

		skip_min_staking_period();

		assert_ok!(NftStaking::self_unstake_currency(RawOrigin::Signed(10).into(), 450));
		next_session();

		assert_eq!(
			Balances::balance_frozen(
				&pallet_vesting_to_freeze::FreezeReason::VestingToFreeze.into(),
				&10
			),
			900
		);
		assert_eq!(Balances::free_balance(10), 550);
		assert_eq!(Balances::total_balance_on_hold(&10), 0);
		assert_eq!(Balances::usable_balance(10), 0);

		// Enough time has passed to thaw schedule
		assert_ok!(VestingToFreeze::thaw_expired(RawOrigin::Signed(10).into()));

		assert_eq!(Balances::free_balance(10), 550);
		assert_eq!(Balances::usable_balance(10), 550);
	})
}
