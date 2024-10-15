use super::*;
use sdk::{frame_benchmarking::v2::*, frame_system::RawOrigin};

#[benchmarks]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn change_subset_size() {
		let origin = RawOrigin::Root;
		let new_subset_size = 176;

		#[extrinsic_call]
		change_subset_size(origin, new_subset_size);
	}

	/*impl_benchmark_test_suite!(
		ValidatorSubsetSelection,
		crate::mock::new_test_ext(None),
		crate::mock::Test
	);*/
}
