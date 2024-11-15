use super::*;
use sdk::{frame_support, frame_system, sp_io, sp_runtime};

use frame_support::{
	derive_impl,
	traits::{Contains, EnsureOrigin, OriginTrait},
};
use sp_runtime::BuildStorage;

use crate as doas;

// Logger module to track execution.
#[frame_support::pallet]
pub mod logger {
	use sdk::frame_support::pallet_prelude::*;
	use sdk::frame_system::pallet_prelude::*;

	#[pallet::config]
	pub trait Config: sdk::frame_system::Config {
		type RuntimeEvent: From<Event<Self>>
			+ IsType<<Self as sdk::frame_system::Config>::RuntimeEvent>;
	}

	#[pallet::pallet]
	pub struct Pallet<T>(_);

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::call_index(0)]
		#[pallet::weight(*weight)]
		pub fn privileged_call(
			origin: OriginFor<T>,
			i: i32,
			weight: Weight,
		) -> DispatchResultWithPostInfo {
			// Ensure that the `origin` is `Root`.
			ensure_root(origin)?;
			<ValueLog<T>>::try_append(i).map_err(|_| "could not append")?;
			Self::deposit_event(Event::AppendValue { value: i, weight });
			Ok(().into())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(*weight)]
		pub fn non_privileged_call(
			origin: OriginFor<T>,
			i: i32,
			weight: Weight,
		) -> DispatchResultWithPostInfo {
			// Ensure that the `origin` is some signed account.
			let sender = ensure_signed(origin)?;
			<ValueLog<T>>::try_append(i).map_err(|_| "could not append")?;
			<AccountLog<T>>::try_append(sender.clone()).map_err(|_| "could not append")?;
			Self::deposit_event(Event::AppendValueAndAccount { sender, value: i, weight });
			Ok(().into())
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		AppendValue { value: i32, weight: Weight },
		AppendValueAndAccount { sender: T::AccountId, value: i32, weight: Weight },
	}

	#[pallet::storage]
	#[pallet::getter(fn account_log)]
	pub(super) type AccountLog<T: Config> =
		StorageValue<_, BoundedVec<T::AccountId, ConstU32<1_000>>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn value_log)]
	pub(super) type ValueLog<T> = StorageValue<_, BoundedVec<i32, ConstU32<1_000>>, ValueQuery>;
}

type Block = frame_system::mocking::MockBlock<Test>;
type AccountId = u64;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		DoAs: doas,
		Logger: logger,
	}
);

pub struct BlockEverything;
impl Contains<RuntimeCall> for BlockEverything {
	fn contains(_: &RuntimeCall) -> bool {
		false
	}
}

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type BaseCallFilter = BlockEverything;
	type Block = Block;
	type AccountId = AccountId;
}

impl logger::Config for Test {
	type RuntimeEvent = RuntimeEvent;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type EnsureOrigin = EnsurePrivileged;
	type WeightInfo = ();
}

pub const PRIVILEGED_ACCOUNT: AccountId = 42;

pub struct EnsurePrivileged;

impl EnsureOrigin<RuntimeOrigin> for EnsurePrivileged {
	type Success = ();
	fn try_origin(origin: RuntimeOrigin) -> Result<Self::Success, RuntimeOrigin> {
		if !origin.clone().into_signer().is_some_and(|s| s == PRIVILEGED_ACCOUNT) {
			return Err(origin);
		}

		Ok(())
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<RuntimeOrigin, ()> {
		Ok(RuntimeOrigin::signed(PRIVILEGED_ACCOUNT))
	}
}

pub type DoAsCall = doas::Call<Test>;
pub type LoggerCall = logger::Call<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut ext: sp_io::TestExternalities =
		frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}

// #[cfg(feature = "runtime-benchmarks")]
// pub fn new_bench_ext() -> sp_io::TestExternalities {
// 	frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into()
// }
