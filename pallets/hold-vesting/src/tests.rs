use sdk::{
	frame_support::{assert_noop, assert_ok, assert_storage_noop},
	frame_system::RawOrigin,
	sp_runtime::{
		traits::{BadOrigin, Identity},
		TokenError,
	},
};

use super::{Vesting as VestingStorage, *};
use crate::mock::{vesting_events_since_last_call, Balances, ExtBuilder, System, Test, Vesting};
use codec::EncodeLike;

/// A default existential deposit.
const ED: u64 = 256;

/// Calls vest, and asserts that there is no entry for `account`
/// in the `Vesting` storage item.
fn vest_and_assert_no_vesting<T>(account: u64)
where
	u64: EncodeLike<<T as frame_system::Config>::AccountId>,
	T: pallet::Config,
{
	// Its ok for this to fail because the user may already have no schedules.
	let _result = Vesting::vest(Some(account).into());
	assert!(!<VestingStorage<T>>::contains_key(account));
}

#[test]
fn check_vesting_status() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user1_free_balance = Balances::free_balance(1);
		let user2_free_balance = Balances::free_balance(2);
		assert_eq!(user1_free_balance, ED * 5); // Account 1 has free balance
		assert_eq!(user2_free_balance, ED); // Account 2 has free balance
		let user1_vesting_schedule = Schedule::new(
			ED * 5,
			128, // Vesting over 10 blocks
			Some(0),
		);
		let user2_vesting_schedule = Schedule::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(1).unwrap(), vec![user1_vesting_schedule]); // Account 1 has a vesting schedule
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![user2_vesting_schedule]); // Account 2 has a vesting schedule

		// Account 1 has only 128 units vested from their illiquid ED * 5 units at block 1
		assert_eq!(Vesting::vesting_balance(&1), Some(128 * 9));
		// Account 2 has their full vested balance locked still
		assert_eq!(Vesting::vesting_balance(&2), Some(20 * ED));

		System::set_block_number(10);
		assert_eq!(System::block_number(), 10);

		// Account 1 has fully vested by block 10
		assert_eq!(Vesting::vesting_balance(&1), Some(0));
		// Account 2 has started vesting by block 10
		assert_eq!(Vesting::vesting_balance(&2), Some(20 * ED));

		System::set_block_number(30);
		assert_eq!(System::block_number(), 30);

		assert_eq!(Vesting::vesting_balance(&1), Some(0)); // Account 1 is still fully vested, and not negative
		assert_eq!(Vesting::vesting_balance(&2), Some(0)); // Account 2 has fully vested by block 30

		// Once we unlock the funds, they are removed from storage.
		vest_and_assert_no_vesting::<Test>(1);
		vest_and_assert_no_vesting::<Test>(2);
	});
}

#[test]
fn check_vesting_status_for_multi_schedule_account() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(System::block_number(), 1);
		let sched0 = Schedule::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			Some(10),
		);

		// Account 2 already has a vesting schedule.
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0]);

		// Account 2's held balance is from sched0.
		let held_balance = Balances::total_balance_on_hold(&2);
		assert_eq!(held_balance, ED * 20);
		assert_eq!(Vesting::vesting_balance(&2), Some(held_balance));

		// Add a 2nd schedule that is already unlocking by block #1.
		let sched1 = Schedule::new(
			ED * 10,
			ED, // Vesting over 10 blocks
			Some(0),
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1));

		// Held balance is equal to the fully locked `sched0` and the remaining 9/10 part of `sched1`
		let held_balance = Balances::total_balance_on_hold(&2);
		assert_eq!(held_balance, ED * (9 + 20));
		// The most recently added schedule exists.
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0, sched1]);

		// Add a 3rd schedule.
		let sched2 = Schedule::new(
			ED * 30,
			ED, // Vesting over 30 blocks
			Some(5),
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched2));

		System::set_block_number(9);

		// sched1 and sched2 are freeing funds at block #9.
		assert_eq!(
			Vesting::vesting_balance(&2),
			Some(sched0.locked() + sched1.per_block() + sched2.per_block() * 26)
		);

		System::set_block_number(20);
		// At block #20 sched1 is fully unlocked while sched2 and sched0 are partially unlocked.
		assert_eq!(
			Vesting::vesting_balance(&2),
			Some(sched2.per_block() * 15 + sched0.per_block() * 10)
		);

		System::set_block_number(30);
		// At block #30 sched0 and sched1 are fully unlocked while sched2 is partially unlocked.
		assert_eq!(Vesting::vesting_balance(&2), Some(sched2.per_block() * 5));

		// At block #35 sched2 fully unlocks and thus all schedules funds are unlocked.
		System::set_block_number(35);
		assert_eq!(Vesting::vesting_balance(&2), Some(0));
		// Since we have not called any extrinsics that would unlock funds the schedules
		// are still in storage,
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0, sched1, sched2]);
		// but once we unlock the funds, they are removed from storage.
		vest_and_assert_no_vesting::<Test>(2);
	});
}

#[test]
fn unvested_balance_should_not_transfer() {
	ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
		let user1_free_balance = Balances::free_balance(1);
		assert_eq!(user1_free_balance, 50); // Account 1 has free balance

		// Account 1 has only 5 units vested at block 1 (plus 50 unvested)
		assert_eq!(Vesting::vesting_balance(&1), Some(45));

		assert_noop!(
			Balances::transfer_allow_death(Some(1).into(), 2, 51),
			TokenError::FundsUnavailable
		);
	});
}

#[test]
fn vested_balance_should_transfer() {
	ExtBuilder::default().existential_deposit(10).build().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 50);

		// Account 1 has only 5 units vested at block 1 (plus 50 unvested)
		assert_eq!(Vesting::vesting_balance(&1), Some(45));

		assert_ok!(Vesting::vest(Some(1).into()));
		assert_eq!(Balances::free_balance(1), 55);

		// Aaagh... transfer_allow_death does not actually allows death if the account has
		// holds or freezes, yet holds and freezes do not contribute to the existential deposit.
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 2, 55 - 10)); // free - ed
	});
}

#[test]
fn vested_balance_should_transfer_with_multi_sched() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let sched0 = Schedule::new(5 * ED, 128, Some(0));
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 1, sched0));
		// Total 10*ED locked for all the schedules.
		assert_eq!(VestingStorage::<Test>::get(1).unwrap(), vec![sched0, sched0]);

		assert_eq!(Balances::free_balance(1), 6 * ED);
		assert_eq!(Balances::total_balance(&1), 3 * 5 * ED);

		// Account 1 has only 256 units unlocking at block 1 (plus 1280 already fee).
		assert_eq!(Vesting::vesting_balance(&1), Some(2304));
		assert_ok!(Vesting::vest(Some(1).into()));
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 2, 1536 - ED));
	});
}

#[test]
fn non_vested_cannot_vest() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert!(!<VestingStorage<Test>>::contains_key(4));
		assert_noop!(Vesting::vest(Some(4).into()), Error::<Test>::NotVesting);
	});
}

#[test]
fn vested_balance_should_transfer_using_vest_other() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 5 * ED);

		assert_eq!(Vesting::vesting_balance(&1), Some(5 * ED - ED / 2));
		assert_ok!(Vesting::vest_other(Some(2).into(), 1));
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 2, 5 * ED + ED / 2 - ED));
	});
}

#[test]
fn vested_balance_should_transfer_using_vest_other_with_multi_sched() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let sched0 = Schedule::new(5 * ED, 128, Some(0));
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 1, sched0));
		// Total of 10*ED of locked for all the schedules.
		assert_eq!(VestingStorage::<Test>::get(1).unwrap(), vec![sched0, sched0]);

		assert_eq!(Balances::free_balance(1), 6 * ED);
		assert_eq!(Balances::total_balance(&1), 3 * 5 * ED);

		// Account 1 has only 256 units unlocking at block 1 (plus 1280 already free).
		assert_eq!(Vesting::vesting_balance(&1), Some(2304));
		assert_ok!(Vesting::vest_other(Some(2).into(), 1));
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 2, 1536 - ED));
	});
}

#[test]
fn non_vested_cannot_vest_other() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert!(!<VestingStorage<Test>>::contains_key(4));
		assert_noop!(Vesting::vest_other(Some(3).into(), 4), Error::<Test>::NotVesting);
	});
}

#[test]
fn extra_balance_should_transfer() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_ok!(Balances::transfer_allow_death(Some(3).into(), 1, ED * 10));
		assert_ok!(Balances::transfer_allow_death(Some(3).into(), 2, ED * 10));

		let user1_free_balance = Balances::free_balance(1);
		assert_eq!(user1_free_balance, ED * 15);
		let user2_free_balance = Balances::free_balance(2);
		assert_eq!(user2_free_balance, ED * 11);

		// Account 1 has only ED / 2 units vested at block 1 (plus 15 * ED unvested)
		assert_eq!(Vesting::vesting_balance(&1), Some(ED * 5 - ED / 2));
		assert_ok!(Vesting::vest(Some(1).into()));
		// Account 1 can send extra units gained
		assert_ok!(Balances::transfer_allow_death(Some(1).into(), 3, 15 * ED + ED / 2 - ED));

		// Account 2 has no units vested at block 1, but gained 10 * ED
		assert_eq!(Vesting::vesting_balance(&2), Some(20 * ED));
		assert_ok!(Vesting::vest(Some(2).into()));
		// Account 2 can send extra units gained
		assert_ok!(Balances::transfer_allow_death(Some(2).into(), 3, 10 * ED));
	});
}

#[test]
fn liquid_funds_should_transfer_with_delayed_vesting() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user12_free_balance = Balances::free_balance(12);

		// Account 12 has free balance
		assert_eq!(user12_free_balance, ED * 5);
		// Account 12 has liquid funds
		assert_eq!(Vesting::vesting_balance(&12), Some(ED * 5));

		// Account 12 has delayed vesting
		let user12_vesting_schedule = Schedule::new(
			ED * 5,
			// Vesting over 20 blocks
			ED / 4,
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(12).unwrap(), vec![user12_vesting_schedule]);

		// Account 12 can still send liquid funds
		assert_ok!(Balances::transfer_allow_death(Some(12).into(), 3, ED * 5 - ED));
	});
}

#[test]
fn liquid_funds_should_transfer_with_unknown_starting_block_schedule() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 3 has schedule not yet started
		let user3_vesting_schedule = Schedule::new(
			ED * 5,
			// Vesting over 20 blocks
			ED / 4,
			None,
		);
		assert_ok!(Vesting::vested_transfer(
			RawOrigin::Signed(13).into(),
			3,
			user3_vesting_schedule
		));
		assert_eq!(VestingStorage::<Test>::get(3).unwrap(), vec![user3_vesting_schedule]);

		let user3_free_balance = Balances::free_balance(3);

		// Account 3 has free balance
		assert_eq!(user3_free_balance, ED * 30);
		// Account 3 has liquid funds
		assert_eq!(Vesting::vesting_balance(&3), Some(ED * 5));

		// Account 3 can still send liquid funds
		assert_ok!(Balances::transfer_allow_death(Some(3).into(), 12, ED * 30 - ED));
	});
}

#[test]
fn vested_transfer_works() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user3_free_balance = Balances::free_balance(3);
		let user4_free_balance = Balances::free_balance(4);
		assert_eq!(user3_free_balance, ED * 30);
		assert_eq!(user4_free_balance, ED * 40);
		// Account 4 should not have any vesting yet.
		assert_eq!(VestingStorage::<Test>::get(4), None);
		// Make the schedule for the new transfer.
		let new_vesting_schedule = Schedule::new(
			ED * 5,
			64, // Vesting over 20 blocks
			Some(10),
		);
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 4, new_vesting_schedule));
		// Verify that the last event is `VestingUpdated`.
		assert_eq!(
			vesting_events_since_last_call(),
			vec![Event::VestingUpdated { account: 4, unvested: 5 * ED },]
		);
		// Now account 4 should have vesting.
		assert_eq!(VestingStorage::<Test>::get(4).unwrap(), vec![new_vesting_schedule]);
		// Ensure the transfer happened correctly.
		assert_eq!(Balances::free_balance(3), ED * 25);
		assert_eq!(Balances::free_balance(4), ED * 40);
		assert_eq!(Balances::total_balance_on_hold(&4), ED * 5);
		// Account 4 has 5 * 256 locked.
		assert_eq!(Vesting::vesting_balance(&4), Some(ED * 5));

		System::set_block_number(20);
		assert_eq!(System::block_number(), 20);

		// Account 4 has 5 * 64 units vested by block 20.
		assert_eq!(Vesting::vesting_balance(&4), Some(10 * 64));

		System::set_block_number(30);
		assert_eq!(System::block_number(), 30);

		// Account 4 has fully vested,
		assert_eq!(Vesting::vesting_balance(&4), Some(0));
		// and after unlocking its schedules are removed from storage.
		vest_and_assert_no_vesting::<Test>(4);
	});
}

#[test]
fn vested_transfer_correctly_fails() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(Balances::total_balance(&2), ED * 21);
		assert_eq!(Balances::total_balance(&4), ED * 40);

		// Account 2 should already have a vesting schedule.
		let user2_vesting_schedule = Schedule::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![user2_vesting_schedule]);

		// Fails due to too low transfer amount.
		let new_vesting_schedule_too_low =
			Schedule::new(<Test as Config>::MinVestedTransfer::get() - 1, 64, Some(10));
		assert_noop!(
			Vesting::vested_transfer(Some(3).into(), 4, new_vesting_schedule_too_low),
			Error::<Test>::AmountLow,
		);

		// `per_block` is 0, which would result in a schedule with infinite duration.
		let schedule_per_block_0 =
			Schedule::new(<Test as Config>::MinVestedTransfer::get(), 0, Some(10));
		assert_noop!(
			Vesting::vested_transfer(Some(13).into(), 4, schedule_per_block_0),
			Error::<Test>::InvalidScheduleParams,
		);

		// `locked` is 0.
		let schedule_locked_0 = Schedule::new(0, 1, Some(10));
		assert_noop!(
			Vesting::vested_transfer(Some(3).into(), 4, schedule_locked_0),
			Error::<Test>::AmountLow,
		);

		assert_eq!(Balances::total_balance(&2), ED * 21);
		assert_eq!(Balances::total_balance(&4), ED * 40);
		assert_eq!(Balances::total_balance_on_hold(&2), ED * 20);
		assert_eq!(Balances::total_balance_on_hold(&4), 0);

		// Account 4 has no schedules.
		vest_and_assert_no_vesting::<Test>(4);
	});
}

#[test]
fn vested_transfer_allows_max_schedules() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let max_schedules = <Test as Config>::MAX_VESTING_SCHEDULES;
		let sched = Schedule::new(
			<Test as Config>::MinVestedTransfer::get(),
			1, // Vest over 2 * 256 blocks.
			Some(10),
		);

		// Add max amount schedules to user 4.
		for _ in 0..max_schedules {
			assert_ok!(Vesting::vested_transfer(Some(13).into(), 4, sched));
		}

		// The schedules count towards vesting balance
		let transferred_amount = <Test as Config>::MinVestedTransfer::get() * max_schedules as u64;
		assert_eq!(Vesting::vesting_balance(&4), Some(transferred_amount));
		assert_eq!(Balances::total_balance_on_hold(&4), transferred_amount);

		let total_balance = Balances::total_balance(&4);
		// Cannot insert a 4th vesting schedule when `MaxVestingSchedules` === 3
		assert_noop!(
			Vesting::force_vested_transfer(RawOrigin::Root.into(), 3, 4, sched),
			Error::<Test>::AtMaxVestingSchedules,
		);
		// so the held and total balance does not change.
		assert_eq!(Vesting::vesting_balance(&4), Some(transferred_amount));
		assert_eq!(Balances::total_balance_on_hold(&4), transferred_amount);
		assert_eq!(Balances::total_balance(&4), total_balance);

		// Account 4 has fully vested when all the schedules end,
		System::set_block_number(
			<Test as Config>::MinVestedTransfer::get() + sched.starting_block().unwrap_or_default(),
		);
		assert_eq!(Vesting::vesting_balance(&4), Some(0));
		// and after unlocking its schedules are removed from storage.
		vest_and_assert_no_vesting::<Test>(4);
	});
}

#[test]
fn force_vested_transfer_works() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let user3_free_balance = Balances::free_balance(3);
		let user4_free_balance = Balances::free_balance(4);
		assert_eq!(user3_free_balance, ED * 30);
		assert_eq!(user4_free_balance, ED * 40);
		// Account 4 should not have any vesting yet.
		assert_eq!(VestingStorage::<Test>::get(4), None);
		// Make the schedule for the new transfer.
		let new_vesting_schedule = Schedule::new(
			ED * 5,
			64, // Vesting over 20 blocks
			Some(10),
		);

		assert_noop!(
			Vesting::force_vested_transfer(Some(4).into(), 3, 4, new_vesting_schedule),
			BadOrigin
		);
		assert_ok!(Vesting::force_vested_transfer(
			RawOrigin::Root.into(),
			3,
			4,
			new_vesting_schedule
		));

		// Verify that the last event is `VestingUpdated`.
		assert_eq!(
			vesting_events_since_last_call(),
			vec![Event::VestingUpdated { account: 4, unvested: 1280 },]
		);
		// Now account 4 should have vesting.
		assert_eq!(VestingStorage::<Test>::get(4).unwrap()[0], new_vesting_schedule);
		assert_eq!(VestingStorage::<Test>::get(4).unwrap().len(), 1);
		// Ensure the transfer happened correctly.
		let user3_free_balance_updated = Balances::free_balance(3);
		assert_eq!(user3_free_balance_updated, ED * 25);
		let user4_held_balance = Balances::total_balance_on_hold(&4);
		assert_eq!(user4_held_balance, 1280);
		// Account 4 has 5 * ED locked.
		assert_eq!(Vesting::vesting_balance(&4), Some(ED * 5));

		System::set_block_number(20);
		assert_eq!(System::block_number(), 20);

		// Account 4 has 5 * 64 units vested by block 20.
		assert_eq!(Vesting::vesting_balance(&4), Some(10 * 64));

		System::set_block_number(30);
		assert_eq!(System::block_number(), 30);

		// Account 4 has fully vested,
		assert_eq!(Vesting::vesting_balance(&4), Some(0));
		// and after unlocking its schedules are removed from storage.
		vest_and_assert_no_vesting::<Test>(4);
	});
}

#[test]
fn force_vested_transfer_correctly_fails() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(Balances::total_balance(&2), ED * 21);
		assert_eq!(Balances::total_balance(&4), ED * 40);
		// Account 2 should already have a vesting schedule.
		let user2_vesting_schedule = Schedule::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![user2_vesting_schedule]);

		// Too low transfer amount.
		let new_vesting_schedule_too_low =
			Schedule::new(<Test as Config>::MinVestedTransfer::get() - 1, 64, Some(10));
		assert_noop!(
			Vesting::force_vested_transfer(
				RawOrigin::Root.into(),
				3,
				4,
				new_vesting_schedule_too_low
			),
			Error::<Test>::AmountLow,
		);

		// `per_block` is 0.
		let schedule_per_block_0 =
			Schedule::new(<Test as Config>::MinVestedTransfer::get(), 0, Some(10));
		assert_noop!(
			Vesting::force_vested_transfer(RawOrigin::Root.into(), 13, 4, schedule_per_block_0),
			Error::<Test>::InvalidScheduleParams,
		);

		// `locked` is 0.
		let schedule_locked_0 = Schedule::new(0, 1, Some(10));
		assert_noop!(
			Vesting::force_vested_transfer(RawOrigin::Root.into(), 3, 4, schedule_locked_0),
			Error::<Test>::AmountLow,
		);

		// Verify no currency transfer happened.
		assert_eq!(Balances::total_balance(&2), ED * 21);
		assert_eq!(Balances::total_balance(&4), ED * 40);
		assert_eq!(Balances::total_balance_on_hold(&2), ED * 20);
		assert_eq!(Balances::total_balance_on_hold(&4), 0);

		// Account 4 has no schedules.
		vest_and_assert_no_vesting::<Test>(4);
	});
}

#[test]
fn force_vested_transfer_allows_max_schedules() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let max_schedules = <Test as Config>::MAX_VESTING_SCHEDULES;
		let sched = Schedule::new(
			<Test as Config>::MinVestedTransfer::get(),
			1, // Vest over 2 * 256 blocks.
			Some(10),
		);

		// Add max amount schedules to user 4.
		for _ in 0..max_schedules {
			assert_ok!(Vesting::force_vested_transfer(RawOrigin::Root.into(), 13, 4, sched));
		}

		// The schedules count towards vesting balance.
		let transferred_amount = <Test as Config>::MinVestedTransfer::get() * max_schedules as u64;
		assert_eq!(Vesting::vesting_balance(&4), Some(transferred_amount));
		assert_eq!(Balances::total_balance_on_hold(&4), transferred_amount);

		let total_balance = Balances::total_balance(&4);
		// Cannot insert a 4th vesting schedule when `MaxVestingSchedules` === 3
		assert_noop!(
			Vesting::force_vested_transfer(RawOrigin::Root.into(), 3, 4, sched),
			Error::<Test>::AtMaxVestingSchedules,
		);
		// so the held and total balance does not change.
		assert_eq!(Vesting::vesting_balance(&4), Some(transferred_amount));
		assert_eq!(Balances::total_balance_on_hold(&4), transferred_amount);
		assert_eq!(Balances::total_balance(&4), total_balance);

		// Account 4 has fully vested when all the schedules end,
		System::set_block_number(<Test as Config>::MinVestedTransfer::get() + 10);
		assert_eq!(Vesting::vesting_balance(&4), Some(0));
		// and after unlocking its schedules are removed from storage.
		vest_and_assert_no_vesting::<Test>(4);
	});
}

#[test]
fn merge_schedules_that_have_not_started() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = Schedule::new(
			ED * 20,
			ED, // Vest over 20 blocks.
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0]);
		assert_eq!(Balances::usable_balance(2), 0);

		// Add a schedule that is identical to the one that already exists.
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 2, sched0));
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0, sched0]);
		assert_eq!(Balances::usable_balance(2), 0);
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));

		// Since we merged identical schedules, the new schedule finishes at the same
		// time as the original, just with double the amount.
		let sched1 = Schedule::new(
			sched0.locked() * 2,
			sched0.per_block() * 2,
			Some(10), // Starts at the block the schedules are merged/
		);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched1]);

		assert_eq!(Balances::usable_balance(2), 0);
	});
}

#[test]
fn merge_ongoing_schedules() {
	// Merging two schedules that have started will vest both before merging.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = Schedule::new(
			ED * 20,
			ED, // Vest over 20 blocks.
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0]);

		let sched1 = Schedule::new(
			ED * 10,
			// Vest over 10 blocks.
			ED,
			// Start at block 15.
			Some(sched0.starting_block().unwrap_or_default() + 5),
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1));
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0, sched1]);

		// Got to half way through the second schedule where both schedules are actively vesting.
		let cur_block = 20;
		System::set_block_number(cur_block);

		// Account 2 has no usable balances prior to the merge because they have not unlocked
		// with `vest` yet.
		assert_eq!(Balances::usable_balance(2), 0);

		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));

		// Merging schedules un-vests all pre-existing schedules prior to merging, which is
		// reflected in account 2's updated usable balance.
		let sched0_vested_now =
			sched0.per_block() * (cur_block - sched0.starting_block().unwrap_or_default());
		let sched1_vested_now =
			sched1.per_block() * (cur_block - sched1.starting_block().unwrap_or_default());
		assert_eq!(Balances::usable_balance(2), sched0_vested_now + sched1_vested_now);

		// The locked amount is the sum of what both schedules have locked at the current block.
		let sched2_locked = sched1
			.locked_at::<Identity>(cur_block)
			.saturating_add(sched0.locked_at::<Identity>(cur_block));
		// End block of the new schedule is the greater of either merged schedule.
		let sched2_end = sched1
			.ending_block_as_balance::<Identity>()
			.max(sched0.ending_block_as_balance::<Identity>());
		let sched2_duration = sched2_end.unwrap_or_default() - cur_block;
		// Based off the new schedules total locked and its duration, we can calculate the
		// amount to unlock per block.
		let sched2_per_block = sched2_locked / sched2_duration;

		let sched2 = Schedule::new(sched2_locked, sched2_per_block, Some(cur_block));
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched2]);

		// And just to double check, we assert the new merged schedule we be cleaned up as expected.
		System::set_block_number(30);
		vest_and_assert_no_vesting::<Test>(2);
	});
}

#[test]
fn merging_shifts_other_schedules_index() {
	// Schedules being merged are filtered out, schedules to the right of any merged
	// schedule shift left and the merged schedule is always last.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let sched0 = Schedule::new(
			ED * 10,
			ED, // Vesting over 10 blocks.
			Some(10),
		);
		let sched1 = Schedule::new(
			ED * 11,
			ED, // Vesting over 11 blocks.
			Some(11),
		);
		let sched2 = Schedule::new(
			ED * 12,
			ED, // Vesting over 12 blocks.
			Some(12),
		);

		// Account 3 starts out with no schedules,
		assert_eq!(VestingStorage::<Test>::get(3), None);
		// and some usable balance.
		let usable_balance = Balances::usable_balance(3);
		assert_eq!(usable_balance, 30 * ED);

		let cur_block = 1;
		assert_eq!(System::block_number(), cur_block);

		// Transfer the above 3 schedules to account 3.
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, sched0));
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, sched1));
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 3, sched2));

		// With no schedules vested or merged they are in the order they are created
		assert_eq!(VestingStorage::<Test>::get(3).unwrap(), vec![sched0, sched1, sched2]);
		// and the usable balance has not changed except now ED is not usable.
		assert_eq!(usable_balance - ED, Balances::usable_balance(3));
		assert_eq!(usable_balance, Balances::free_balance(3));

		assert_ok!(Vesting::merge_schedules(Some(3).into(), 0, 2));

		// Create the merged schedule of sched0 & sched2.
		// The merged schedule will have the max possible starting block,
		let sched3_start = sched1.starting_block().max(sched2.starting_block());
		// `locked` equal to the sum of the two schedules locked through the current block,
		let sched3_locked =
			sched2.locked_at::<Identity>(cur_block) + sched0.locked_at::<Identity>(cur_block);
		// and will end at the max possible block.
		let sched3_end = sched2
			.ending_block_as_balance::<Identity>()
			.max(sched0.ending_block_as_balance::<Identity>());
		let sched3_duration = sched3_end.unwrap_or_default() - sched3_start.unwrap_or_default();
		let sched3_per_block = sched3_locked / sched3_duration;
		let sched3 = Schedule::new(sched3_locked, sched3_per_block, sched3_start);

		// The not touched schedule moves left and the new merged schedule is appended.
		assert_eq!(VestingStorage::<Test>::get(3).unwrap(), vec![sched1, sched3]);
		// The usable balance hasn't changed since none of the schedules have started.
		assert_eq!(Balances::usable_balance(3), usable_balance - ED);
	});
}

#[test]
fn merge_ongoing_and_yet_to_be_started_schedules() {
	// Merge an ongoing schedule that has had `vest` called and a schedule that has not already
	// started.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = Schedule::new(
			ED * 20,
			ED, // Vesting over 20 blocks
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0]);

		// Fast forward to half way through the life of sched1.
		let mut cur_block = (sched0.starting_block().unwrap()
			+ sched0.ending_block_as_balance::<Identity>().unwrap())
			/ 2;
		assert_eq!(cur_block, 20);
		System::set_block_number(cur_block);

		// Prior to vesting there is no usable balance.
		let mut usable_balance = 0;
		assert_eq!(Balances::usable_balance(2), usable_balance);
		// Vest the current schedules (which is just sched0 now).
		Vesting::vest(Some(2).into()).unwrap();

		// After vesting the usable balance increases by the unlocked amount.
		let sched0_vested_now = sched0.locked() - sched0.locked_at::<Identity>(cur_block);
		usable_balance += sched0_vested_now;
		assert_eq!(Balances::usable_balance(2), usable_balance);

		// Go forward a block.
		cur_block += 1;
		System::set_block_number(cur_block);

		// And add a schedule that starts after this block, but before sched0 finishes.
		let sched1 = Schedule::new(
			ED * 10,
			1, // Vesting over 256 * 10 (2560) blocks
			Some(cur_block + 1),
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1));

		// Merge the schedules before sched1 starts.
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));
		// After merging, the usable balance only changes by the amount sched0 vested since we
		// last called `vest` (which is just 1 block). The usable balance is not affected by
		// sched1 because it has not started yet.
		usable_balance += sched0.per_block();
		assert_eq!(Balances::usable_balance(2), usable_balance);

		// The resulting schedule will have the later starting block of the two,
		let sched2_start = sched1.starting_block();
		// `locked` equal to the sum of the two schedules locked through the current block,
		let sched2_locked =
			sched0.locked_at::<Identity>(cur_block) + sched1.locked_at::<Identity>(cur_block);
		// and will end at the max possible block.
		let sched2_end = sched0
			.ending_block_as_balance::<Identity>()
			.max(sched1.ending_block_as_balance::<Identity>());
		let sched2_duration = sched2_end.unwrap() - sched2_start.unwrap();
		let sched2_per_block = sched2_locked / sched2_duration;

		let sched2 = Schedule::new(sched2_locked, sched2_per_block, sched2_start);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched2]);
	});
}

#[test]
fn merge_finished_and_ongoing_schedules() {
	// If a schedule finishes by the current block we treat the ongoing schedule,
	// without any alterations, as the merged one.
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = Schedule::new(
			ED * 20,
			ED, // Vesting over 20 blocks.
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0]);

		let sched1 = Schedule::new(
			ED * 40,
			ED, // Vesting over 40 blocks.
			Some(10),
		);
		assert_ok!(Vesting::vested_transfer(Some(4).into(), 2, sched1));

		// Transfer a 3rd schedule, so we can demonstrate how schedule indices change.
		// (We are not merging this schedule.)
		let sched2 = Schedule::new(
			ED * 30,
			ED, // Vesting over 30 blocks.
			Some(10),
		);
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 2, sched2));

		// The schedules are in expected order prior to merging.
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0, sched1, sched2]);

		// Fast forward to sched0's end block.
		let cur_block = sched0.ending_block_as_balance::<Identity>().unwrap();
		System::set_block_number(cur_block);
		assert_eq!(System::block_number(), 30);

		// Prior to `merge_schedules` and with no vest/vest_other called the user has no usable
		// balance.
		assert_eq!(Balances::usable_balance(2), 0);
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));

		// sched2 is now the first, since sched0 & sched1 get filtered out while "merging".
		// sched1 gets treated like the new merged schedule by getting pushed onto back
		// of the vesting schedules vec. Note: sched0 finished at the current block.
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched2, sched1]);

		// sched0 has finished, so its funds are fully unlocked.
		let sched0_unlocked_now = sched0.locked();
		// The remaining schedules are ongoing, so their funds are partially unlocked.
		let sched1_unlocked_now = sched1.locked() - sched1.locked_at::<Identity>(cur_block);
		let sched2_unlocked_now = sched2.locked() - sched2.locked_at::<Identity>(cur_block);

		// Since merging also vests all the schedules, the users usable balance after merging
		// includes all pre-existing schedules unlocked through the current block, including
		// schedules not merged.
		assert_eq!(
			Balances::usable_balance(2),
			sched0_unlocked_now + sched1_unlocked_now + sched2_unlocked_now
		);
	});
}

#[test]
fn merge_finishing_schedules_does_not_create_a_new_one() {
	// If both schedules finish by the current block we don't create new one
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 12 should already have a vesting schedule.
		let sched0 = Schedule::new(
			ED * 5,
			ED / 4, // 20 block duration.
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(12).unwrap(), vec![sched0]);

		// Create sched1 and transfer it to account 12.
		let sched1 = Schedule::new(
			ED * 30,
			ED, // 30 block duration.
			Some(10),
		);
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 12, sched1));
		assert_eq!(VestingStorage::<Test>::get(12).unwrap(), vec![sched0, sched1]);

		let all_scheds_end = sched0
			.ending_block_as_balance::<Identity>()
			.max(sched1.ending_block_as_balance::<Identity>())
			.unwrap();

		assert_eq!(all_scheds_end, 40);
		System::set_block_number(all_scheds_end);

		// Prior to merge_schedules and with no vest/vest_other called the user has onle 10 * ED usable
		// balance.
		assert_eq!(Balances::usable_balance(12), 5 * ED - ED);

		// Merge schedule 0 and 1.
		assert_ok!(Vesting::merge_schedules(Some(12).into(), 0, 1));
		// The user no longer has any more vesting schedules because they both ended at the
		// block they where merged,
		assert!(!<VestingStorage<Test>>::contains_key(12));
		// and their usable balance has increased by the total amount locked in the merged
		// schedules.
		assert_eq!(Balances::usable_balance(12), 5 * ED + sched0.locked() + sched1.locked());
	});
}

#[test]
fn merge_finished_and_yet_to_be_started_schedules() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = Schedule::new(
			ED * 20,
			ED,       // 20 block duration.
			Some(10), // Ends at block 30
		);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0]);

		let sched1 = Schedule::new(
			ED * 30,
			ED * 2, // 30 block duration.
			Some(35),
		);
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 2, sched1));
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0, sched1]);

		let sched2 = Schedule::new(
			ED * 40,
			ED, // 40 block duration.
			Some(30),
		);
		// Add a 3rd schedule to demonstrate how sched1 shifts.
		assert_ok!(Vesting::vested_transfer(Some(13).into(), 2, sched2));
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0, sched1, sched2]);

		System::set_block_number(30);

		// At block 30, sched0 has finished unlocking while sched1 and sched2 are still fully
		// locked,
		assert_eq!(Vesting::vesting_balance(&2), Some(sched1.locked() + sched2.locked()));
		// but since we have not vested usable balance is still 0.
		assert_eq!(Balances::usable_balance(2), 0);

		// Merge schedule 0 and 1.
		assert_ok!(Vesting::merge_schedules(Some(2).into(), 0, 1));

		// sched0 is removed since it finished, and sched1 is removed and then pushed on the back
		// because it is treated as the merged schedule
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched2, sched1]);

		// The usable balance is updated because merging fully unlocked sched0.
		assert_eq!(Balances::usable_balance(2), sched0.locked());
	});
}

#[test]
fn merge_schedules_throws_proper_errors() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// Account 2 should already have a vesting schedule.
		let sched0 = Schedule::new(
			ED * 20,
			ED, // 20 block duration.
			Some(10),
		);
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0]);

		// Account 2 only has 1 vesting schedule.
		assert_noop!(
			Vesting::merge_schedules(Some(2).into(), 0, 1),
			Error::<Test>::ScheduleIndexOutOfBounds
		);

		// Account 4 has 0 vesting schedules.
		assert_eq!(VestingStorage::<Test>::get(4), None);
		assert_noop!(Vesting::merge_schedules(Some(4).into(), 0, 1), Error::<Test>::NotVesting);

		// There are enough schedules to merge but an index is non-existent.
		Vesting::vested_transfer(Some(3).into(), 2, sched0).unwrap();
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0, sched0]);
		assert_noop!(
			Vesting::merge_schedules(Some(2).into(), 0, 2),
			Error::<Test>::ScheduleIndexOutOfBounds
		);

		// It is a storage noop with no errors if the indexes are the same.
		assert_storage_noop!(Vesting::merge_schedules(Some(2).into(), 0, 0).unwrap());

		// Can't merge schedules where the starting block is unknown for any of the schedules
		let sched1 = Schedule::new(
			ED * 20,
			ED, // 20 block duration.
			None,
		);
		Vesting::vested_transfer(Some(13).into(), 2, sched1).unwrap();
		assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![sched0, sched0, sched1]);
		assert_noop!(
			Vesting::merge_schedules(Some(2).into(), 1, 2),
			Error::<Test>::NoDefiniteStartingBlock
		);
	});
}

#[test]
fn generates_multiple_schedules_from_genesis_config() {
	let vesting_config = vec![
		// 5 * existential deposit locked.
		(1, Some(0), 10, 5 * ED),
		// 1 * existential deposit locked.
		(2, Some(10), 20, ED),
		// 2 * existential deposit locked.
		(2, Some(10), 20, 2 * ED),
		// 1 * existential deposit locked.
		(12, Some(10), 20, ED),
		// 2 * existential deposit locked.
		(12, Some(10), 20, 2 * ED),
		// 3 * existential deposit locked.
		(12, Some(10), 20, 3 * ED),
	];
	ExtBuilder::default()
		.existential_deposit(ED)
		.vesting_genesis_config(vesting_config)
		.build()
		.execute_with(|| {
			let user1_sched1 = Schedule::new(5 * ED, 128, Some(0u64));
			assert_eq!(VestingStorage::<Test>::get(1).unwrap(), vec![user1_sched1]);

			let user2_sched1 = Schedule::new(ED, 13, Some(10u64));
			let user2_sched2 = Schedule::new(2 * ED, 26, Some(10u64));
			assert_eq!(VestingStorage::<Test>::get(2).unwrap(), vec![user2_sched1, user2_sched2]);

			let user12_sched1 = Schedule::new(ED, 13, Some(10u64));
			let user12_sched2 = Schedule::new(2 * ED, 26, Some(10u64));
			let user12_sched3 = Schedule::new(3 * ED, 39, Some(10u64));
			assert_eq!(
				VestingStorage::<Test>::get(12).unwrap(),
				vec![user12_sched1, user12_sched2, user12_sched3]
			);
		});
}

#[test]
#[should_panic]
fn multiple_schedules_from_genesis_config_errors() {
	// MaxVestingSchedules is 3, but this config has 4 for account 12 so we panic when building
	// from genesis.
	let vesting_config = vec![
		(12, Some(10), 20, ED),
		(12, Some(10), 20, ED),
		(12, Some(10), 20, ED),
		(12, Some(10), 20, ED),
	];
	ExtBuilder::default()
		.existential_deposit(ED)
		.vesting_genesis_config(vesting_config)
		.build();
}

#[test]
fn merge_vesting_handles_per_block_0() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		let sched0 = Schedule::new(
			ED,
			0, // Vesting over 256 blocks.
			Some(1),
		);
		assert_eq!(sched0.ending_block_as_balance::<Identity>(), Some(257));
		let sched1 = Schedule::new(
			ED * 2,
			0, // Vesting over 512 blocks.
			Some(10),
		);
		assert_eq!(sched1.ending_block_as_balance::<Identity>(), Some(512u64 + 10));

		let merged = Schedule::new(764, 1, Some(10));
		assert_eq!(Vesting::merge_vesting_info(5, sched0, sched1), Ok(Some(merged)));
	});
}

#[test]
fn vesting_info_validate_works() {
	let min_transfer = <Test as Config>::MinVestedTransfer::get();
	// Does not check for min transfer.
	assert!(Schedule::new(min_transfer - 1, 1u64, Some(10u64)).is_valid());

	// `locked` cannot be 0.
	assert!(!Schedule::new(0, 1u64, Some(10u64)).is_valid());

	// `per_block` cannot be 0.
	assert!(!Schedule::new(min_transfer + 1, 0u64, Some(10u64)).is_valid());

	// With valid inputs it does not error.
	assert!(Schedule::new(min_transfer, 1u64, Some(10u64)).is_valid());
}

#[test]
fn vesting_info_ending_block_as_balance_works() {
	// Treats `per_block` 0 as 1.
	let per_block_0 = Schedule::new(256u32, 0u32, Some(10u32));
	assert_eq!(per_block_0.ending_block_as_balance::<Identity>(), Some(256 + 10));

	// `per_block >= locked` always results in a schedule ending the block after it starts
	let per_block_gt_locked = Schedule::new(256u32, 256 * 2u32, Some(10u32));
	assert_eq!(
		per_block_gt_locked.ending_block_as_balance::<Identity>(),
		per_block_gt_locked.starting_block().map(|v| v + 1)
	);
	let per_block_eq_locked = Schedule::new(256u32, 256u32, Some(10u32));
	assert_eq!(
		per_block_gt_locked.ending_block_as_balance::<Identity>(),
		per_block_eq_locked.ending_block_as_balance::<Identity>()
	);

	// Correctly calcs end if `locked % per_block != 0`. (We need a block to unlock the remainder).
	let imperfect_per_block = Schedule::new(256u32, 250u32, Some(10u32));
	assert_eq!(
		imperfect_per_block.ending_block_as_balance::<Identity>(),
		imperfect_per_block.starting_block().map(|v| v + 2),
	);
	assert_eq!(
		imperfect_per_block.locked_at::<Identity>(
			imperfect_per_block.ending_block_as_balance::<Identity>().unwrap()
		),
		0
	);

	// Return `None` if starting block is unknown
	let unknown_start_block = Schedule::new(256u32, 1u32, None);
	assert_eq!(unknown_start_block.ending_block_as_balance::<Identity>(), None);
}

#[test]
fn per_block_works() {
	let per_block_0 = Schedule::new(256u32, 0u32, Some(10u32));
	assert_eq!(per_block_0.per_block(), 1u32);
	assert_eq!(per_block_0.raw_per_block(), 0u32);

	let per_block_1 = Schedule::new(256u32, 1u32, Some(10u32));
	assert_eq!(per_block_1.per_block(), 1u32);
	assert_eq!(per_block_1.raw_per_block(), 1u32);
}

// When an accounts free balance + schedule.locked is less than ED, the vested transfer will fail.
#[test]
fn vested_transfer_less_than_existential_deposit_fails() {
	ExtBuilder::default().existential_deposit(4 * ED).build().execute_with(|| {
		// MinVestedTransfer is less the ED.
		assert!(
			<Test as Config>::Fungible::minimum_balance()
				> <Test as Config>::MinVestedTransfer::get()
		);

		let sched = Schedule::new(<Test as Config>::MinVestedTransfer::get(), 1u64, Some(10u64));
		// The new account balance with the schedule's locked amount would be less than ED.
		assert!(
			Balances::free_balance(99) + sched.locked()
				< <Test as Config>::Fungible::minimum_balance()
		);

		// vested_transfer fails.
		assert_noop!(Vesting::vested_transfer(Some(3).into(), 99, sched), TokenError::BelowMinimum,);
		// force_vested_transfer fails.
		assert_noop!(
			Vesting::force_vested_transfer(RawOrigin::Root.into(), 3, 99, sched),
			TokenError::BelowMinimum,
		);
	});
}

#[test]
fn remove_vesting_schedule() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(Balances::free_balance(3), 256 * 30);
		assert_eq!(Balances::free_balance(4), 256 * 40);
		// Account 4 should not have any vesting yet.
		assert_eq!(VestingStorage::<Test>::get(4), None);
		// Make the schedule for the new transfer.
		let new_vesting_schedule = Schedule::new(
			ED * 5,
			(ED * 5) / 20, // Vesting over 20 blocks
			Some(10),
		);
		assert_ok!(Vesting::vested_transfer(Some(3).into(), 4, new_vesting_schedule));
		// Verify that the last event is `VestingUpdated`.
		assert_eq!(
			vesting_events_since_last_call(),
			vec![Event::VestingUpdated { account: 4, unvested: 1280 },]
		);

		// Now account 4 should have vesting.
		assert_eq!(VestingStorage::<Test>::get(4).unwrap(), vec![new_vesting_schedule]);
		// Account 4 has 5 * 256 locked.
		assert_eq!(Vesting::vesting_balance(&4), Some(256 * 5));
		// Verify only root can call.
		assert_noop!(Vesting::force_remove_vesting_schedule(Some(4).into(), 4, 0), BadOrigin);
		// Verify that root can remove the schedule.
		assert_ok!(Vesting::force_remove_vesting_schedule(RawOrigin::Root.into(), 4, 0));
		// Verify that last event is VestingCompleted.
		System::assert_last_event(Event::VestingCompleted { account: 4 }.into());
		// Appropriate storage is cleaned up.
		assert!(!<VestingStorage<Test>>::contains_key(4));
		// Check the vesting balance is zero.
		assert_eq!(VestingStorage::<Test>::get(4), None);
		// Verifies that trying to remove a schedule when it doesnt exist throws error.
		assert_noop!(
			Vesting::force_remove_vesting_schedule(RawOrigin::Root.into(), 4, 0),
			Error::<Test>::InvalidScheduleParams
		);
	});
}

#[test]
fn vested_transfer_impl_works() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		assert_eq!(Balances::free_balance(3), 256 * 30);
		assert_eq!(Balances::free_balance(4), 256 * 40);
		// Account 4 should not have any vesting yet.
		assert_eq!(VestingStorage::<Test>::get(4), None);

		// Now account 4 should have vesting.
		let new_vesting_schedule = Schedule::new(
			ED * 5,
			(ED * 5) / 20, // Vesting over 20 blocks
			Some(10),
		);

		// Basic working scenario
		assert_ok!(Vesting::vested_transfer(RawOrigin::Signed(3).into(), 4, new_vesting_schedule));
		assert_eq!(VestingStorage::<Test>::get(4).unwrap(), vec![new_vesting_schedule]);
		// Account 4 has 5 * 256 locked.
		assert_eq!(Vesting::vesting_balance(&4), Some(256 * 5));

		// If the transfer fails (because they don't have enough balance), no storage is changed.
		assert_noop!(
			Vesting::vested_transfer(
				RawOrigin::Signed(3).into(),
				4,
				Schedule::new(ED * 9999, ED * 5 / 20, Some(10))
			),
			TokenError::FundsUnavailable
		);

		// If applying the vesting schedule fails (per block is 0), no storage is changed.
		assert_noop!(
			Vesting::vested_transfer(
				RawOrigin::Signed(3).into(),
				4,
				Schedule::new(ED * 5, 0, Some(10))
			),
			Error::<Test>::InvalidScheduleParams
		);
	});
}

#[test]
fn set_starting_block_works() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// account 3 has schedule not yet started
		let sched = Schedule::new(
			ED * 5,
			ED, // Vesting over 5 blocks
			None,
		);

		assert_ok!(Vesting::vested_transfer(RawOrigin::Signed(13).into(), 3, sched));
		assert_eq!(VestingStorage::<Test>::get(3).unwrap(), vec![sched]);

		// successfully set start to 10
		assert_ok!(Vesting::start_vesting_schedule(RawOrigin::Root.into(), 3, 0, 10));
		assert_eq!(
			VestingStorage::<Test>::get(3).unwrap(),
			vec![Schedule { starting_block: Some(10), ..sched }]
		);

		// on block 10 the schedule is started
		System::set_block_number(10);
		assert_eq!(Vesting::vesting_balance(&3).unwrap(), ED * 5);

		// on block 11 some amount unlocks
		System::set_block_number(11);
		assert_eq!(Vesting::vesting_balance(&3).unwrap(), ED * 4);

		// on block 15 all unlocks
		System::set_block_number(15);
		assert_eq!(Vesting::vesting_balance(&3).unwrap(), 0);
		vest_and_assert_no_vesting::<Test>(3);
	})
}

#[test]
fn set_starting_block_works_retroactively() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// account 3 has schedule not yet started
		let sched = Schedule::new(
			ED * 10,
			ED / 2, // Vesting over 20 blocks
			None,
		);

		assert_ok!(Vesting::vested_transfer(RawOrigin::Signed(13).into(), 3, sched));
		assert_eq!(VestingStorage::<Test>::get(3).unwrap(), vec![sched]);

		// on block 10 nothing is unlocked
		System::set_block_number(10);
		assert_eq!(Vesting::vesting_balance(&3).unwrap(), ED * 10);

		// successfully set start to 0
		assert_ok!(Vesting::start_vesting_schedule(RawOrigin::Root.into(), 3, 0, 0));
		assert_eq!(
			VestingStorage::<Test>::get(3).unwrap(),
			vec![Schedule { starting_block: Some(0), ..sched }]
		);

		// on block 10 some unlocks
		assert_eq!(Vesting::vesting_balance(&3).unwrap(), ED * 5);

		// on block 20 all unlocks
		System::set_block_number(20);
		assert_eq!(Vesting::vesting_balance(&3).unwrap(), 0);
		vest_and_assert_no_vesting::<Test>(3);
	})
}

#[test]
fn set_starting_block_fails_if_already_started() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// account 1 has schedule already started
		let sched = Schedule::new(
			ED * 5,
			ED / 2, // Vesting over 10 blocks
			Some(0),
		);
		assert_eq!(VestingStorage::<Test>::get(1).unwrap(), vec![sched]);

		// failed to set start to 10
		assert_noop!(
			Vesting::start_vesting_schedule(RawOrigin::Root.into(), 1, 0, 10),
			Error::<Test>::StartingBlockAlreadySet
		);
	})
}

#[test]
fn set_starting_block_fails_if_schedule_index_is_invalid() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// account 1 has one schedule
		let sched = Schedule::new(
			ED * 5,
			ED / 2, // Vesting over 10 blocks
			Some(0),
		);
		assert_eq!(VestingStorage::<Test>::get(1).unwrap(), vec![sched]);

		// failed to set start of schedule 1
		assert_noop!(
			Vesting::start_vesting_schedule(RawOrigin::Root.into(), 1, 1, 10),
			Error::<Test>::InvalidScheduleParams
		);

		// account 3 has no schedule
		assert_eq!(VestingStorage::<Test>::get(3), None);

		// failed to set start of non-existing schedule
		assert_noop!(
			Vesting::start_vesting_schedule(RawOrigin::Root.into(), 3, 0, 10),
			Error::<Test>::NotVesting
		);
	})
}

#[test]
fn set_starting_block_fails_if_caller_is_not_root() {
	ExtBuilder::default().existential_deposit(ED).build().execute_with(|| {
		// account 1 has one schedule
		let sched = Schedule::new(
			ED * 5,
			ED / 2, // Vesting over 10 blocks
			Some(0),
		);
		assert_eq!(VestingStorage::<Test>::get(1).unwrap(), vec![sched]);

		// failed to set start of schedule 0 with signed origin
		assert_noop!(
			Vesting::start_vesting_schedule(RawOrigin::Signed(1).into(), 1, 0, 10),
			DispatchError::BadOrigin
		);
	})
}
