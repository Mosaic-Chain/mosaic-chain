//! Vesting pallet benchmarking.

use sdk::{
	frame_benchmarking::v2::*,
	frame_support::{
		assert_ok,
		pallet_prelude::*,
		traits::{
			fungible::{Inspect, Mutate, MutateHold},
			tokens::Preservation,
			VariantCount,
		},
	},
	frame_system::{pallet_prelude::*, Pallet as SystemPallet, RawOrigin},
	pallet_balances::{Config as BalancesConfig, Pallet as BalancesPallet},
	sp_runtime::traits::{
		BlockNumberProvider, CheckedAdd, CheckedDiv, CheckedMul, StaticLookup, Zero,
	},
	sp_std::prelude::*,
};

use crate::{
	AccountIdLookupOf, Call, Config, HoldReason, HoldVestingSchedule, Pallet, Schedule, ScheduleOf,
	Vesting as VestingStorage,
};

const SEED: u32 = 0;
const HELD: u32 = 256;

type BalanceOf<T> = <T as BalancesConfig>::Balance;

fn endow<T: BalancesConfig>(
	who: &T::AccountId,
	amount: BalanceOf<T>,
) -> Result<BalanceOf<T>, DispatchError> {
	<BalancesPallet<T> as Mutate<_>>::mint_into(who, amount)
}

fn add_holds<T: Config + BalancesConfig>(who: &T::AccountId, n: u8)
where
	<T as BalancesConfig>::RuntimeHoldReason: From<crate::HoldReason>,
{
	const ALL_TEST_REASONS: [HoldReason; 4] =
		[HoldReason::Test1, HoldReason::Test2, HoldReason::Test3, HoldReason::Test4];
	for id in 0..n {
		endow::<T>(who, HELD.into()).unwrap();
		let reason = (ALL_TEST_REASONS[id as usize]).into();
		<BalancesPallet<T> as MutateHold<_>>::hold(&reason, who, HELD.into()).unwrap();
	}
}

fn add_vesting_schedules<T>(
	target: AccountIdLookupOf<T>,
	n: u32,
) -> Result<BalanceOf<T>, &'static str>
where
	T: Config + BalancesConfig<Balance = <T as Config>::Balance>,
{
	let min_transfer = T::MinVestedTransfer::get();
	let held = min_transfer.checked_mul(&20u32.into()).unwrap();
	// Schedule has a duration of 20.
	let per_block = min_transfer;
	let starting_block = 1u32;

	let source: T::AccountId = account("source", 0, SEED);
	let source_lookup = T::Lookup::unlookup(source.clone());
	endow::<T>(
		&source,
		held.checked_mul(&n.into())
			.unwrap()
			.checked_add(&BalancesPallet::<T>::minimum_balance())
			.unwrap(),
	)
	.unwrap();

	T::BlockNumberProvider::set_block_number(BlockNumberFor::<T>::zero());

	let mut total_locked: BalanceOf<T> = Zero::zero();
	for _ in 0..n {
		total_locked += held;

		let schedule = ScheduleOf::<T>::new(held, per_block, Some(starting_block.into()));
		assert_ok!(Pallet::<T>::do_vested_transfer(
			source_lookup.clone(),
			target.clone(),
			schedule
		));
	}

	Ok(total_locked)
}

#[benchmarks(where
	T: BalancesConfig<Balance = <T as Config>::Balance>,
	<T as BalancesConfig>::RuntimeHoldReason: From<crate::HoldReason>,
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn vest_locked(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES }>,
	) {
		let caller: T::AccountId = whitelisted_caller();
		let caller_lookup = T::Lookup::unlookup(caller.clone());
		endow::<T>(&caller, BalancesPallet::<T>::minimum_balance()).unwrap();

		add_holds::<T>(&caller, l as u8);
		let expected_balance = add_vesting_schedules::<T>(caller_lookup, s).unwrap();

		// At block zero, everything is vested.
		assert_eq!(SystemPallet::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			Pallet::<T>::vesting_balance(&caller),
			Some(expected_balance),
			"Vesting schedule not added",
		);

		let origin = RawOrigin::Signed(caller.clone());
		#[extrinsic_call]
		Pallet::<T>::vest(origin);

		// Nothing happened since everything is still vested.
		assert_eq!(
			Pallet::<T>::vesting_balance(&caller),
			Some(expected_balance),
			"Vesting schedule was removed",
		);
	}

	#[benchmark]
	fn vest_unlocked(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES }>,
	) {
		let caller: T::AccountId = whitelisted_caller();
		let caller_lookup = T::Lookup::unlookup(caller.clone());
		endow::<T>(&caller, BalancesPallet::<T>::minimum_balance()).unwrap();

		add_holds::<T>(&caller, l as u8);
		add_vesting_schedules::<T>(caller_lookup, s).unwrap();

		// At block 21, everything is unlocked.
		T::BlockNumberProvider::set_block_number(21u32.into());
		assert_eq!(
			Pallet::<T>::vesting_balance(&caller),
			Some(BalanceOf::<T>::zero()),
			"Vesting schedule still active",
		);

		let origin = RawOrigin::Signed(caller.clone());
		#[extrinsic_call]
		Pallet::<T>::vest(origin);

		// Vesting schedule is removed!
		assert_eq!(Pallet::<T>::vesting_balance(&caller), None, "Vesting schedule was not removed",);
	}

	#[benchmark]
	fn vest_other_locked(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES }>,
	) {
		let other: T::AccountId = account("other", 0, SEED);
		let other_lookup = T::Lookup::unlookup(other.clone());

		endow::<T>(&other, BalancesPallet::<T>::minimum_balance()).unwrap();
		add_holds::<T>(&other, l as u8);
		let expected_balance = add_vesting_schedules::<T>(other_lookup.clone(), s).unwrap();

		// At block zero, everything is vested.
		assert_eq!(SystemPallet::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			Pallet::<T>::vesting_balance(&other),
			Some(expected_balance),
			"Vesting schedule not added",
		);

		let caller: T::AccountId = whitelisted_caller();
		let origin = RawOrigin::Signed(caller.clone());
		#[extrinsic_call]
		Pallet::<T>::vest_other(origin, other_lookup);

		// Nothing happened since everything is still vested.
		assert_eq!(
			Pallet::<T>::vesting_balance(&other),
			Some(expected_balance),
			"Vesting schedule was removed",
		);
	}

	#[benchmark]
	fn vest_other_unlocked(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES }>,
	) {
		let other: T::AccountId = account("other", 0, SEED);
		let other_lookup = T::Lookup::unlookup(other.clone());

		endow::<T>(&other, BalancesPallet::<T>::minimum_balance()).unwrap();
		add_holds::<T>(&other, l as u8);
		add_vesting_schedules::<T>(other_lookup.clone(), s).unwrap();
		// At block 21 everything is unlocked.
		T::BlockNumberProvider::set_block_number(21u32.into());

		assert_eq!(
			Pallet::<T>::vesting_balance(&other),
			Some(BalanceOf::<T>::zero()),
			"Vesting schedule still active",
		);

		let caller: T::AccountId = whitelisted_caller();
		let origin = RawOrigin::Signed(caller.clone());
		#[extrinsic_call]
		Pallet::<T>::vest_other(origin, other_lookup);

		// Vesting schedule is removed.
		assert_eq!(Pallet::<T>::vesting_balance(&other), None, "Vesting schedule was not removed",);
	}

	#[benchmark]
	fn vested_transfer(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>,
	) {
		let transfer_amount = T::MinVestedTransfer::get();
		let caller: T::AccountId = whitelisted_caller();
		endow::<T>(
			&caller,
			transfer_amount.checked_add(&BalancesPallet::<T>::minimum_balance()).unwrap(),
		)
		.unwrap();

		let target: T::AccountId = account("target", 0, SEED);
		let target_lookup = T::Lookup::unlookup(target.clone());
		// Give target existing locks
		endow::<T>(&target, BalancesPallet::<T>::minimum_balance()).unwrap();
		add_holds::<T>(&target, l as u8);
		// Add some vesting schedules.
		let orig_balance = <BalancesPallet<T> as Inspect<_>>::total_balance(&target);
		let mut expected_balance = add_vesting_schedules::<T>(target_lookup.clone(), s).unwrap();

		// At block zero, everything is vested.
		assert_eq!(SystemPallet::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			Pallet::<T>::vesting_balance(&target),
			Some(expected_balance),
			"Vesting schedule not added",
		);

		let per_block = transfer_amount.checked_div(&20u32.into()).unwrap();
		expected_balance += transfer_amount;

		let vesting_schedule = Schedule::new(transfer_amount, per_block, Some(1u32.into()));
		let origin = RawOrigin::Signed(caller.clone());
		#[extrinsic_call]
		Pallet::<T>::vested_transfer(origin, target_lookup, vesting_schedule);

		assert_eq!(
			orig_balance + expected_balance,
			<BalancesPallet<T> as Inspect<_>>::total_balance(&target),
			"Transfer didn't happen",
		);
		assert_eq!(
			Pallet::<T>::vesting_balance(&target),
			Some(expected_balance),
			"Lock not correctly updated",
		);
	}

	#[benchmark]
	fn force_vested_transfer(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES - 1 }>,
	) {
		let source: T::AccountId = account("source", 0, SEED);
		let source_lookup = T::Lookup::unlookup(source.clone());
		let transfer_amount = T::MinVestedTransfer::get();
		endow::<T>(
			&source,
			transfer_amount.checked_add(&BalancesPallet::<T>::minimum_balance()).unwrap(),
		)
		.unwrap();

		let target: T::AccountId = account("target", 0, SEED);
		let target_lookup = T::Lookup::unlookup(target.clone());
		// Give target existing locks
		endow::<T>(&target, BalancesPallet::<T>::minimum_balance()).unwrap();
		add_holds::<T>(&target, l as u8);
		// Add one less than max vesting schedules
		let orig_balance = <BalancesPallet<T> as Inspect<_>>::total_balance(&target);
		let mut expected_balance = add_vesting_schedules::<T>(target_lookup.clone(), s).unwrap();

		let per_block = transfer_amount.checked_div(&20u32.into()).unwrap();
		expected_balance += transfer_amount;

		let vesting_schedule = Schedule::new(transfer_amount, per_block, Some(1u32.into()));

		#[extrinsic_call]
		Pallet::<T>::force_vested_transfer(
			RawOrigin::Root,
			source_lookup,
			target_lookup,
			vesting_schedule,
		);

		assert_eq!(
			orig_balance + expected_balance,
			<BalancesPallet<T> as Inspect<_>>::total_balance(&target),
			"Transfer didn't happen",
		);
		assert_eq!(
			Pallet::<T>::vesting_balance(&target),
			Some(expected_balance),
			"Lock not correctly updated",
		);
	}

	#[benchmark]
	fn not_unlocking_merge_schedules(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<2, { T::MAX_VESTING_SCHEDULES }>,
	) {
		let caller: T::AccountId = account("caller", 0, SEED);
		let caller_lookup = T::Lookup::unlookup(caller.clone());
		// Give target existing locks.
		endow::<T>(&caller, BalancesPallet::<T>::minimum_balance()).unwrap();
		add_holds::<T>(&caller, l as u8);
		// Add max vesting schedules.
		let expected_balance = add_vesting_schedules::<T>(caller_lookup, s).unwrap();

		// Schedules are not vesting at block 0.
		assert_eq!(SystemPallet::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			Pallet::<T>::vesting_balance(&caller),
			Some(expected_balance),
			"Vesting balance should equal sum locked of all schedules",
		);
		assert_eq!(
			VestingStorage::<T>::get(&caller).unwrap().len(),
			s as usize,
			"There should be exactly max vesting schedules"
		);

		#[extrinsic_call]
		Pallet::<T>::merge_schedules(RawOrigin::Signed(caller.clone()), 0, s - 1);

		let expected_schedule = Schedule::new(
			T::MinVestedTransfer::get() * 20u32.into() * 2u32.into(),
			T::MinVestedTransfer::get() * 2u32.into(),
			Some(1u32.into()),
		);
		let expected_index = (s - 2) as usize;
		assert_eq!(VestingStorage::<T>::get(&caller).unwrap()[expected_index], expected_schedule);
		assert_eq!(
			Pallet::<T>::vesting_balance(&caller),
			Some(expected_balance),
			"Vesting balance should equal total locked of all schedules",
		);
		assert_eq!(
			VestingStorage::<T>::get(&caller).unwrap().len(),
			(s - 1) as usize,
			"Schedule count should reduce by 1"
		);
	}
	#[benchmark]
	fn unlocking_merge_schedules(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<2, { T::MAX_VESTING_SCHEDULES }>,
	) {
		// Destination used just for currency transfers in asserts.
		let test_dest: T::AccountId = account("test_dest", 0, SEED);

		let caller: T::AccountId = account("caller", 0, SEED);
		let caller_lookup = T::Lookup::unlookup(caller.clone());
		// Give target other locks.
		endow::<T>(&caller, BalancesPallet::<T>::minimum_balance()).unwrap();
		add_holds::<T>(&caller, l as u8);
		// Add max vesting schedules.
		let total_transferred = add_vesting_schedules::<T>(caller_lookup, s).unwrap();

		// Go to about half way through all the schedules duration. (They all start at 1, and have a duration of 20 or 21).
		T::BlockNumberProvider::set_block_number(11u32.into());
		// We expect half the original locked balance (+ any remainder that vests on the last block).
		let expected_balance = total_transferred / 2u32.into();
		assert_eq!(
			Pallet::<T>::vesting_balance(&caller),
			Some(expected_balance),
			"Vesting balance should reflect that we are half way through all schedules duration",
		);
		assert_eq!(
			VestingStorage::<T>::get(&caller).unwrap().len(),
			s as usize,
			"There should be exactly max vesting schedules"
		);
		// The balance is not actually transferable because it has not been unlocked.
		assert!(T::Fungible::transfer(
			&caller,
			&test_dest,
			expected_balance,
			Preservation::Expendable
		)
		.is_err());

		#[extrinsic_call]
		Pallet::<T>::merge_schedules(RawOrigin::Signed(caller.clone()), 0, s - 1);

		let expected_schedule = Schedule::new(
			T::MinVestedTransfer::get() * 2u32.into() * 10u32.into(),
			T::MinVestedTransfer::get() * 2u32.into(),
			Some(11u32.into()),
		);
		let expected_index = (s - 2) as usize;
		assert_eq!(
			VestingStorage::<T>::get(&caller).unwrap()[expected_index],
			expected_schedule,
			"New schedule is properly created and placed"
		);

		assert_eq!(
			Pallet::<T>::vesting_balance(&caller),
			Some(expected_balance),
			"Vesting balance should equal half total locked of all schedules",
		);
		assert_eq!(
			VestingStorage::<T>::get(&caller).unwrap().len(),
			(s - 1) as usize,
			"Schedule count should reduce by 1"
		);
		// Since merge unlocks all schedules we can now transfer the balance.
		assert_ok!(T::Fungible::transfer(
			&caller,
			&test_dest,
			expected_balance,
			Preservation::Expendable
		));
	}

	#[benchmark]
	fn force_remove_vesting_schedule(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<2, { T::MAX_VESTING_SCHEDULES }>,
	) {
		let target: T::AccountId = account("target", 0, SEED);
		let target_lookup: <T::Lookup as StaticLookup>::Source =
			T::Lookup::unlookup(target.clone());
		endow::<T>(&target, BalancesPallet::<T>::minimum_balance()).unwrap();

		// Give target existing locks.
		add_holds::<T>(&target, l as u8);
		let _ = add_vesting_schedules::<T>(target_lookup.clone(), s).unwrap();

		// The last vesting schedule.
		let schedule_index = s - 1;

		#[extrinsic_call]
		Pallet::<T>::force_remove_vesting_schedule(RawOrigin::Root, target_lookup, schedule_index);

		assert_eq!(
			VestingStorage::<T>::get(&target).unwrap().len(),
			schedule_index as usize,
			"Schedule count should reduce by 1"
		);
	}

	#[benchmark]
	fn start_vesting_schedule_unlocked(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES }>,
	) {
		let target: T::AccountId = account("target", 0, SEED);
		let target_lookup = T::Lookup::unlookup(target.clone());
		endow::<T>(&target, BalancesPallet::<T>::minimum_balance()).unwrap();

		add_holds::<T>(&target, l as u8);
		add_vesting_schedules::<T>(target_lookup.clone(), s).unwrap();

		VestingStorage::<T>::mutate_extant(&target, |schedules| {
			schedules[0].starting_block = None;
		});

		// At block 21, everything but the unvested schedule is unvested.
		SystemPallet::<T>::set_block_number(21u32.into());
		assert_eq!(
			Pallet::<T>::vesting_balance(&target),
			Some(T::MinVestedTransfer::get().checked_mul(&20u32.into()).unwrap()),
			"Vesting schedule not unvested",
		);

		#[extrinsic_call]
		Pallet::<T>::start_vesting_schedule(RawOrigin::Root, target_lookup, 0, 0u32.into());

		// Even the last schedule is unvested by the call.
		assert_eq!(
			Pallet::<T>::vesting_balance(&target),
			None,
			"Vesting schedule started and unlocked",
		);
	}

	#[benchmark]
	fn start_vesting_schedule_locked(
		l: Linear<0, { crate::HoldReason::VARIANT_COUNT - 1 }>,
		s: Linear<1, { T::MAX_VESTING_SCHEDULES }>,
	) {
		let target: T::AccountId = account("target", 0, SEED);
		let target_lookup = T::Lookup::unlookup(target.clone());
		endow::<T>(&target, BalancesPallet::<T>::minimum_balance()).unwrap();

		add_holds::<T>(&target, l as u8);
		let expected_balance = add_vesting_schedules::<T>(target_lookup.clone(), s).unwrap();

		VestingStorage::<T>::mutate_extant(&target, |schedules| {
			schedules[0].starting_block = None;
		});

		// At block zero, everything is vested.
		assert_eq!(SystemPallet::<T>::block_number(), BlockNumberFor::<T>::zero());
		assert_eq!(
			Pallet::<T>::vesting_balance(&target),
			Some(expected_balance),
			"Vesting schedule not added",
		);

		#[extrinsic_call]
		Pallet::<T>::start_vesting_schedule(RawOrigin::Root, target_lookup, 0, 1u32.into());

		// Nothing happened since everything is still vested.
		assert_eq!(
			Pallet::<T>::vesting_balance(&target),
			Some(expected_balance),
			"Vesting schedule was not started",
		);
	}

	// impl_benchmark_test_suite!(
	// 	Vesting,
	// 	crate::mock::ExtBuilder::default().existential_deposit(256).build(),
	// 	crate::mock::Test,
	// );
}
