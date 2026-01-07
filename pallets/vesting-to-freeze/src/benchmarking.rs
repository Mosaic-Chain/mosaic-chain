use super::*;

use sdk::{frame_benchmarking, frame_support, frame_system};

use frame_benchmarking::v2::*;
use frame_support::traits::{
	fungible::{Mutate, MutateFreeze},
	VariantCount,
};
use frame_system::RawOrigin;

use utils::vesting::Schedule;

fn add_freezes<T: Config>(who: &T::AccountId, n: u32)
where
	T::Fungible: MutateFreeze<T::AccountId> + Mutate<T::AccountId>,
{
	const ALL_TEST_REASONS: [FreezeReason; 4] =
		[FreezeReason::Test1, FreezeReason::Test2, FreezeReason::Test3, FreezeReason::Test4];

	T::Fungible::mint_into(who, T::Fungible::minimum_balance()).expect("Could mint ED");
	for id in 0..n {
		let reason = (ALL_TEST_REASONS[id as usize]).into();
		<T::Fungible as MutateFreeze<_>>::set_freeze(&reason, who, T::Fungible::minimum_balance())
			.expect("Could freeze ED");
	}
}

fn add_frozen_schedules<T: Config>(who: &T::AccountId, f: u32)
where
	T::Fungible: Mutate<T::AccountId>,
{
	let frozen_amount = (f * 20).into();

	T::Fungible::mint_into(who, T::Fungible::minimum_balance() + frozen_amount)
		.expect("Could mint ED + freeze");
	T::Fungible::increase_frozen(&FreezeReason::VestingToFreeze.into(), who, frozen_amount)
		.expect("Could increase freeze");

	FrozenSchedules::<T>::mutate(who, |schedules| {
		*schedules = BoundedVec::truncate_from(
			[(20u32.into(), 20u32.into())].into_iter().cycle().take(f as usize).collect(),
		);
	})
}

#[benchmarks(
	where T::Fungible: Mutate<T::AccountId> + MutateFreeze<T::AccountId>
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn convert_schedule(
		s: Linear<1, { T::MaxVestingSchedules::get() }>,
		r: Linear<0, { FreezeReason::VARIANT_COUNT - 1 }>,
		f: Linear<0, { T::MaxFrozenSchedules::get() - 1 }>,
	) {
		let caller = whitelisted_caller();

		T::Fungible::mint_into(&caller, T::Fungible::minimum_balance() + (s * 20).into()).unwrap();

		for _ in 0..s {
			T::VestingSchedule::add_vesting_schedule(
				&caller,
				Schedule {
					locked: 20u32.into(),
					per_block: 1u32.into(),
					starting_block: Some(1u32.into()),
				},
			)
			.unwrap();
		}

		add_freezes::<T>(&caller, r);
		add_frozen_schedules::<T>(&caller, f);

		#[extrinsic_call]
		Pallet::<T>::convert_schedule(RawOrigin::Signed(caller.clone()), s - 1);

		assert_eq!(FrozenSchedules::<T>::decode_len(&caller), Some(f as usize + 1));

		let vesting_balance = if s > 1 { Some(((s - 1) * 20).into()) } else { None };
		assert_eq!(T::VestingSchedule::vesting_balance(&caller), vesting_balance);
	}

	#[benchmark]
	fn thaw_expired(
		r: Linear<0, { FreezeReason::VARIANT_COUNT - 1 }>,
		f: Linear<1, { T::MaxFrozenSchedules::get() }>,
	) {
		let caller = whitelisted_caller();

		add_freezes::<T>(&caller, r);
		add_frozen_schedules::<T>(&caller, f);

		// Every frozen schedule will be expired
		T::BlockNumberProvider::set_block_number(21u32.into());

		#[extrinsic_call]
		Pallet::<T>::thaw_expired(RawOrigin::Signed(caller.clone()));

		assert_eq!(FrozenSchedules::<T>::decode_len(&caller), None)
	}

	#[benchmark]
	fn force_thaw(
		r: Linear<0, { FreezeReason::VARIANT_COUNT - 1 }>,
		f: Linear<1, { T::MaxFrozenSchedules::get() }>,
	) {
		let caller = whitelisted_caller();

		add_freezes::<T>(&caller, r);
		add_frozen_schedules::<T>(&caller, f);

		#[extrinsic_call]
		Pallet::<T>::force_thaw(RawOrigin::Root, caller.clone(), f / 2);

		let expected_schedules_len = (f > 1).then_some(f as usize - 1);
		assert_eq!(FrozenSchedules::<T>::decode_len(&caller), expected_schedules_len);
	}
}
