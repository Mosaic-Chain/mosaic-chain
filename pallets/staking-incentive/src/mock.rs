use super::*;

use sdk::{frame_support, frame_system, pallet_balances, sp_core, sp_io, sp_runtime};

use frame_support::{derive_impl, parameter_types};
use sp_core::ConstU128;
use sp_runtime::{traits::ConvertInto, BuildStorage};

pub use crate as staking_incentive;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		StakingIncentive: staking_incentive,
		Balances: pallet_balances,
		HoldVesting: pallet_hold_vesting,
	}
);

pub type AccountId = u64;
pub type Balance = u128;

pub const YEARS: u32 = 5_259_600;

pub const ALICE: AccountId = 0;
pub const BOB: AccountId = 1;
pub const CHARLIE: AccountId = 2;

type Block = frame_system::mocking::MockBlock<Test>;

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type Block = Block;
	type AccountId = AccountId;
	type AccountData = pallet_balances::AccountData<Balance>;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type Balance = Balance;
	type ExistentialDeposit = ConstU128<1>;
	type AccountStore = System;
}

impl pallet_hold_vesting::Config for Test {
	type RuntimeHoldReason = RuntimeHoldReason;
	type Balance = Balance;
	type Fungible = Balances;
	type BlockNumberToBalance = ConvertInto;
	type MinVestedTransfer = ConstU128<1>;
	type WeightInfo = ();
	type BlockNumberProvider = System;
	const MAX_VESTING_SCHEDULES: u32 = 42;
}

parameter_types! {
	pub static PerBlockMultiplier: FixedU128 = FixedU128::from_u32(2);
	pub const StakingIncentivePalletId: PalletId = PalletId(*b"stakince");
}

impl staking_incentive::Config for Test {
	type Balance = Balance;
	type Fungible = Balances;
	type VestingSchedule = HoldVesting;
	type ClaimVestingScheduleLength = ConstU32<500>;
	type PerBlockMultiplier = PerBlockMultiplier;
	type BlockNumberProvider = System;
	type BalanceToScore = ConvertInto;
	type PalletId = StakingIncentivePalletId;
	type MaxPayouts = ConstU32<16>;
	type WeightInfo = ();
}

pub fn new_test_ext(per_block_multiplier: Option<FixedU128>) -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();

	PerBlockMultiplier::set(per_block_multiplier.unwrap_or_else(|| FixedU128::from_u32(2)));

	pallet_balances::GenesisConfig::<Test> {
		balances: (0..15).map(|acc| (acc, 1000)).collect(),
		dev_accounts: None,
	}
	.assimilate_storage(&mut t)
	.unwrap();

	staking_incentive::GenesisConfig::<Test> { incentive_pool: 10000 }
		.assimilate_storage(&mut t)
		.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| System::set_block_number(1));
	ext
}

pub fn skip_blocks(n: u32) {
	utils::run_until::run_until::<AllPalletsWithSystem, Test>(utils::run_until::Blocks(n));
}

pub fn stake(account_id: AccountId, amount: Balance) {
	<StakingIncentive as StakingHooks<AccountId, Balance, ()>>::on_currency_stake(
		&account_id,
		&account_id,
		amount,
	);
}

pub fn unstake(account_id: AccountId, amount: Balance) {
	<StakingIncentive as StakingHooks<AccountId, Balance, ()>>::on_currency_unstake(
		&account_id,
		&account_id,
		amount,
	);
}

pub fn weight_of(acc: AccountId) -> FixedU128 {
	let acc_score = StakingIncentive::current_score_of(&acc).effective();
	let global_score = Global::<Test>::get().score.effective();

	acc_score / global_score
}

/// Check for equality between two `FixU128`s with `prec` decimal places worth of precision.
pub fn fixed_point_eq(a: FixedU128, b: FixedU128, prec: u32) -> bool {
	let (a, b) = if a < b { (a, b) } else { (b, a) };
	b - a < FixedU128::from_rational(1, 10u128.pow(prec))
}
