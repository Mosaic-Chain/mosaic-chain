use super::*;

use sdk::frame_benchmarking::v2::*;
use utils::storage::ClearAll;

#[benchmarks]
mod benchmarks {
	use super::*;

	const MIN_SUPERSET: u32 = 10;
	const MAX_SUPERSET: u32 = 2000;

	#[benchmark(extra)]
	fn select_subset(n: Linear<{ MIN_SUPERSET }, { MAX_SUPERSET }>) -> Result<(), BenchmarkError> {
		let superset: Vec<<T as Config>::ValidatorId> =
			(0..n).map(|i| account("validator", i, 0)).collect();

		DoubleBucketMap::<T>::clear_all();

		#[block]
		{
			Pallet::<T>::select_subset(superset, n / 2);
		}

		Ok(())
	}

	#[benchmark(extra)]
	fn select_subset_all(
		n: Linear<{ MIN_SUPERSET }, { MAX_SUPERSET }>,
	) -> Result<(), BenchmarkError> {
		let superset: Vec<<T as Config>::ValidatorId> =
			(0..n).map(|i| account("validator", i, 0)).collect();

		#[block]
		{
			Pallet::<T>::select_subset(superset, n * 2);
		}

		Ok(())
	}

	#[benchmark(extra)]
	fn select_subset_buckets_exist(
		n: Linear<{ MIN_SUPERSET }, { MAX_SUPERSET }>,
	) -> Result<(), BenchmarkError> {
		let superset: Vec<<T as Config>::ValidatorId> =
			(0..n).map(|i| account("validator", i, 0)).collect();

		for _ in 0..16 {
			Pallet::<T>::select_subset(superset.clone(), n / 2);
		}

		#[block]
		{
			Pallet::<T>::select_subset(superset, n / 2);
		}

		Ok(())
	}
}
