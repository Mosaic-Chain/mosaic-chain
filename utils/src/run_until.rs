use sdk::{
	frame_support::traits::{OnFinalize, OnInitialize},
	frame_system::{self, pallet_prelude::*, Pallet as System},
	sp_runtime::traits::{AtLeast32BitUnsigned, One, Zero},
};

pub trait Until<T> {
	fn should_step(&mut self) -> bool;
}

pub struct Blocks<B>(pub B);

impl<T, B: AtLeast32BitUnsigned> Until<T> for Blocks<B> {
	fn should_step(&mut self) -> bool {
		if self.0.is_zero() {
			return false;
		}

		self.0 -= One::one();
		true
	}
}

pub struct ToBlock<B>(pub B);

impl<T> Until<T> for ToBlock<BlockNumberFor<T>>
where
	T: frame_system::Config,
{
	fn should_step(&mut self) -> bool {
		System::<T>::block_number() < self.0
	}
}

impl<T, F: Fn() -> bool> Until<T> for F {
	fn should_step(&mut self) -> bool {
		(self)()
	}
}

// Testing block production, for reference see:
// https://web.archive.org/web/20230129131011/https://docs.substrate.io/test/unit-testing/#block-production

#[cfg(any(feature = "std", feature = "runtime-benchmarks"))]
pub fn run_until<Hooks, T>(mut until: impl Until<T>)
where
	T: frame_system::Config,
	Hooks: OnInitialize<BlockNumberFor<T>> + OnFinalize<BlockNumberFor<T>>,
{
	while until.should_step() {
		let block_number = System::<T>::block_number();

		if block_number > Zero::zero() {
			System::<T>::on_finalize(block_number);
			Hooks::on_finalize(block_number);
		}

		let block_number = block_number + One::one();

		System::<T>::reset_events();
		System::<T>::set_block_number(block_number);
		System::<T>::on_initialize(block_number);
		Hooks::on_initialize(block_number);
	}
}
