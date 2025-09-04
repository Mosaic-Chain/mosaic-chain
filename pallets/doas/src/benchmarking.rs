//! Benchmarks for the DoAs Pallet forked from the Polkadot-SDK provided Sudo Pallet

use super::*;
use crate::Pallet;

use sdk::{
	frame_benchmarking::v2::*,
	frame_support::traits::EnsureOrigin,
	frame_system::{Call as SystemCall, Pallet as SystemPallet},
	sp_std::prelude::*,
};

fn assert_last_event<T: Config>(generic_event: crate::Event<T>) {
	let re: T::RuntimeEvent = generic_event.into();
	SystemPallet::<T>::assert_last_event(re);
}

#[benchmarks(where <T as Config>::RuntimeCall: From<SystemCall<T>>)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn doas_root() {
		let origin = T::EnsureOrigin::try_successful_origin().expect("Should succeed");

		let call = SystemCall::remark { remark: vec![] }.into();

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, Box::new(call));

		assert_last_event::<T>(Event::DidAsRoot { doas_result: Ok(()) })
	}

	#[benchmark]
	fn doas() {
		let origin = T::EnsureOrigin::try_successful_origin().expect("Should succeed");

		let call = SystemCall::remark { remark: vec![] }.into();

		let who: T::AccountId = account("as", 0, 0);
		let who_lookup = T::Lookup::unlookup(who);

		#[extrinsic_call]
		_(origin as T::RuntimeOrigin, who_lookup, Box::new(call));

		assert_last_event::<T>(Event::DidAs { doas_result: Ok(()) })
	}

	// impl_benchmark_test_suite!(Pallet, crate::mock::new_bench_ext(), crate::mock::Test);
}
