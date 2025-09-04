use super::*;
use sdk::{frame_support, frame_system, pallet_balances, sp_io, sp_runtime};

use frame_support::{
	derive_impl,
	dispatch::DispatchResult,
	pallet_prelude::StorageMap,
	traits::{
		fungible::{Inspect, MutateHold},
		tokens::Precision,
		VariantCountOf,
	},
};
use sp_core::{ConstU128, ConstU32};
use sp_runtime::{traits::ConvertInto, BuildStorage};
use utils::vesting::{HoldVestingSchedule, Schedule};

pub use crate as vesting_to_freeze;

pub mod vesting;
pub use vesting::{MockVesting, HOLD_REASON, MAX_VESTING_SCHEDULES};

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		VestingToFreeze: vesting_to_freeze
	}
);

pub type AccountId = <Test as frame_system::Config>::AccountId;
pub type Balance = u128; // should be larger than a u64 to test this case

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
	type FreezeIdentifier = RuntimeFreezeReason;
	type RuntimeHoldReason = u8;
	type MaxReserves = ConstU32<1>;
	type MaxFreezes = VariantCountOf<RuntimeFreezeReason>;
	type Balance = Balance;
	type ExistentialDeposit = ConstU128<1>;
}

impl vesting_to_freeze::Config for Test {
	type RuntimeFreezeReason = RuntimeFreezeReason;
	type Balance = Balance;
	type Fungible = Balances;
	type VestingSchedule = MockVesting;
	type BlockNumberToBalance = ConvertInto;
	type BlockNumberProvider = System;
	type MaxFrozenSchedules = ConstU32<3>;
	type MaxFreezes = ConstU32<5>;
	type MaxVestingSchedules = ConstU32<{ MAX_VESTING_SCHEDULES }>;
	type WeightInfo = ();
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut ext: sp_io::TestExternalities =
		frame_system::GenesisConfig::<Test>::default().build_storage().unwrap().into();
	ext.execute_with(|| System::set_block_number(1));
	ext
}
