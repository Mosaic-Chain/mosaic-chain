use super::*;

use mock::*;
use utils::{
	run_until::{run_until, ToBlock},
	vesting::Schedule,
};

use frame_support::{
	assert_noop,
	traits::{
		fungible::{Balanced, InspectFreeze, InspectHold},
		tokens::{Fortitude, Precision, Preservation},
	},
};

fn add_schedule(who: AccountId, s: Schedule<Balance, BlockNumberFor<Test>>) {
	// Mint and put balance on hold (+1 for existential deposit if needed)
	let amount = if Balances::total_balance(&who).is_zero() { s.locked + 1 } else { s.locked };
	let _imbalance = Balances::deposit(&who, amount, Precision::Exact).expect("can endow");
	MockVesting::add_vesting_schedule(&who, s).expect("could add schedule");
}

fn run_to_block(index: BlockNumberFor<Test>) {
	run_until::<AllPalletsWithSystem, Test>(ToBlock(index));
}

fn origin(who: AccountId) -> RuntimeOrigin {
	RuntimeOrigin::signed(who)
}

#[test]
fn convert_schedule_works() {
	new_test_ext().execute_with(|| {
		let owner = 1;
		let schedule = Schedule { locked: 100, per_block: 1, starting_block: Some(20) };

		add_schedule(owner, schedule);
		run_to_block(20 + 50);

		// No amount can be put on hold right now (eg.: for staking)
		assert_eq!(
			Balances::reducible_balance(&owner, Preservation::Preserve, Fortitude::Force),
			0
		);

		VestingToFreeze::convert_schedule(origin(owner), 0).expect("could convert schedule");

		System::assert_last_event(
			Event::<Test>::ConvertedSchedule {
				account_id: owner,
				schedule_index: 0,
				amount: 50,
				thaws_on: 120,
			}
			.into(),
		);

		// The entire amount - ed can be potentionally put on hold
		assert_eq!(
			Balances::reducible_balance(&owner, Preservation::Preserve, Fortitude::Force),
			100
		);

		assert_eq!(Balances::balance_on_hold(&HOLD_REASON, &owner), 0);
		assert_eq!(Balances::balance_frozen(&FreezeReason::VestingToFreeze.into(), &owner), 50);
		assert_eq!(FrozenSchedules::<Test>::get(owner).as_slice(), &[(120, 50)]);
	});
}

#[test]
fn convert_schedule_no_schedule() {
	new_test_ext().execute_with(|| {
		let owner = 1;

		// Not vesting at all
		assert_noop!(
			VestingToFreeze::convert_schedule(origin(owner), 0),
			vesting::Error::NotVesting
		);

		add_schedule(owner, Schedule { locked: 100, per_block: 1, starting_block: Some(0) });

		// Wrong index
		assert_noop!(
			VestingToFreeze::convert_schedule(origin(owner), 1),
			vesting::Error::NoSuchSchedule
		);
	});
}

#[test]
fn convert_schedule_ending_block_not_representable() {
	new_test_ext().execute_with(|| {
		let owner = 1;

		// Never unlocks as ending block cannot be represented by block number
		add_schedule(
			owner,
			Schedule {
				locked: BlockNumberFor::<Test>::MAX as Balance + 1,
				per_block: 1,
				starting_block: Some(0),
			},
		);
		assert_noop!(
			VestingToFreeze::convert_schedule(origin(owner), 0),
			Error::<Test>::NeverThaws
		);
	});
}

#[test]
fn convert_schedule_not_started_yet() {
	new_test_ext().execute_with(|| {
		let owner = 1;

		// Never unlocks as we don't know when/if it starts
		add_schedule(owner, Schedule { locked: 100, per_block: 1, starting_block: None });
		assert_noop!(
			VestingToFreeze::convert_schedule(origin(owner), 0),
			Error::<Test>::NeverThaws
		);
	});
}

#[test]
fn convert_schedule_already_expired() {
	new_test_ext().execute_with(|| {
		let owner = 1;

		add_schedule(owner, Schedule { locked: 100, per_block: 1, starting_block: Some(0) });

		run_to_block(100);

		assert_noop!(
			VestingToFreeze::convert_schedule(origin(owner), 0),
			Error::<Test>::AlreadyExpired
		);
	});
}

#[test]
fn convert_schedule_max_schedules_reached() {
	new_test_ext().execute_with(|| {
		let owner = 1;

		for i in 0..MAX_VESTING_SCHEDULES {
			add_schedule(
				owner,
				Schedule { locked: 100, per_block: 1, starting_block: Some(u64::from(i)) },
			);
		}

		let max = <Test as vesting_to_freeze::Config>::MaxFrozenSchedules::get();
		for i in 0..max {
			VestingToFreeze::convert_schedule(origin(owner), i).expect("could convert schedule");
		}

		assert_noop!(
			VestingToFreeze::convert_schedule(origin(owner), max),
			Error::<Test>::MaxFrozenSchedulesReached
		);
	});
}

#[test]
fn thaw_expired_works() {
	new_test_ext().execute_with(|| {
		let owner = 1;

		for i in 0..3u32 {
			add_schedule(
				owner,
				Schedule {
					locked: 100,
					per_block: 1,
					starting_block: Some(u64::from(i) * 100 + 1),
				},
			);

			// NOTE: every iteration the newly created schedule will get index 0 as the previous was already removed
			VestingToFreeze::convert_schedule(origin(owner), 0).expect("could convert schedule");
		}

		// Thaw first schedule
		run_to_block(101);
		VestingToFreeze::thaw_expired(origin(owner)).expect("could thaw");
		System::assert_last_event(Event::<Test>::Thawed { account_id: owner, amount: 100 }.into());

		assert_eq!(Balances::balance_frozen(&FreezeReason::VestingToFreeze.into(), &owner), 200);
		assert_eq!(FrozenSchedules::<Test>::get(owner).as_slice(), &[(201, 100), (301, 100)]);

		// Thaw second and third schedule at once
		run_to_block(301);
		VestingToFreeze::thaw_expired(origin(owner)).expect("could thaw");

		assert_eq!(Balances::balance_frozen(&FreezeReason::VestingToFreeze.into(), &owner), 0);
		System::assert_last_event(Event::<Test>::Thawed { account_id: owner, amount: 200 }.into());

		assert!(FrozenSchedules::<Test>::get(owner).is_empty());
	});
}

#[test]
fn thaw_expired_no_schedules() {
	new_test_ext().execute_with(|| {
		assert_noop!(VestingToFreeze::thaw_expired(origin(1)), Error::<Test>::NoFrozenSchedules);
	});
}

#[test]
fn force_thaw_works() {
	new_test_ext().execute_with(|| {
		let owner = 1;

		for i in 0..3u32 {
			add_schedule(
				owner,
				Schedule {
					locked: 100,
					per_block: 1,
					starting_block: Some(u64::from(i) * 100 + 1),
				},
			);

			// NOTE: every iteration the newly created schedule will get index 0 as the previous was already removed
			VestingToFreeze::convert_schedule(origin(owner), 0).expect("could convert schedule");
		}

		// Let's say the first two schedule expired
		run_to_block(201);

		// We forcefully thaw the third (that hasn't expired)
		VestingToFreeze::force_thaw(RuntimeOrigin::root(), owner, 2).unwrap();

		// It thaws
		System::assert_last_event(Event::<Test>::Thawed { account_id: owner, amount: 100 }.into());

		// But the other remain frozen despite they **could** be thawed by the user.
		assert_eq!(Balances::balance_frozen(&FreezeReason::VestingToFreeze.into(), &owner), 200);
		assert_eq!(FrozenSchedules::<Test>::get(owner).as_slice(), &[(101, 100), (201, 100)]);
	});
}

#[test]
fn force_thaw_no_frozen_schedules() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			VestingToFreeze::force_thaw(RuntimeOrigin::root(), 1, 0),
			Error::<Test>::NoFrozenSchedules
		);
	});
}

#[test]
fn force_thaw_invalid_index() {
	new_test_ext().execute_with(|| {
		let owner = 1;

		for _ in 0..3u32 {
			add_schedule(owner, Schedule { locked: 100, per_block: 1, starting_block: Some(1) });

			// NOTE: every iteration the newly created schedule will get index 0 as the previous was already removed
			VestingToFreeze::convert_schedule(origin(owner), 0).expect("could convert schedule");
		}

		assert_noop!(
			VestingToFreeze::force_thaw(RuntimeOrigin::root(), owner, 3),
			Error::<Test>::InvalidFrozenScheduleIndex
		);
	});
}
