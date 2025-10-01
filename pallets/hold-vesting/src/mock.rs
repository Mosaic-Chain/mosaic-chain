use super::*;
use sdk::{
	frame_support::{self, derive_impl, parameter_types, traits::WithdrawReasons},
	frame_system, pallet_balances, sp_io,
	sp_runtime::{traits::Identity, BuildStorage},
};

pub use crate as hold_vesting;

type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test
	{
		System: frame_system,
		Balances: pallet_balances,
		Vesting: hold_vesting,
	}
);

#[derive_impl(frame_system::config_preludes::TestDefaultConfig)]
impl frame_system::Config for Test {
	type AccountData = pallet_balances::AccountData<u64>;
	type Block = Block;
}

#[derive_impl(pallet_balances::config_preludes::TestDefaultConfig)]
impl pallet_balances::Config for Test {
	type AccountStore = System;
	type ExistentialDeposit = ExistentialDeposit;
}

parameter_types! {
	pub const MinVestedTransfer: u64 = 256 * 2;
	pub UnvestedFundsAllowedWithdrawReasons: WithdrawReasons =
		WithdrawReasons::except(WithdrawReasons::TRANSFER | WithdrawReasons::RESERVE);
	pub static ExistentialDeposit: u64 = 1;
}

impl Config for Test {
	type BlockNumberToBalance = Identity;
	type Balance = u64;
	type Fungible = Balances;
	const MAX_VESTING_SCHEDULES: u32 = 3;
	type MinVestedTransfer = MinVestedTransfer;
	type WeightInfo = ();
	type BlockNumberProvider = System;
	type RuntimeHoldReason = RuntimeHoldReason;
}

pub struct ExtBuilder {
	existential_deposit: u64,
	vesting_genesis_config: Option<Vec<(u64, Option<u64>, u64, u64)>>,
}

impl Default for ExtBuilder {
	fn default() -> Self {
		Self { existential_deposit: 1, vesting_genesis_config: None }
	}
}

impl ExtBuilder {
	pub fn existential_deposit(mut self, existential_deposit: u64) -> Self {
		self.existential_deposit = existential_deposit;
		self
	}

	pub fn vesting_genesis_config(mut self, config: Vec<(u64, Option<u64>, u64, u64)>) -> Self {
		self.vesting_genesis_config = Some(config);
		self
	}

	pub fn build(self) -> sp_io::TestExternalities {
		EXISTENTIAL_DEPOSIT.with(|v| *v.borrow_mut() = self.existential_deposit);
		let mut t = frame_system::GenesisConfig::<Test>::default().build_storage().unwrap();
		pallet_balances::GenesisConfig::<Test> {
			balances: vec![
				(1, 10 * self.existential_deposit),
				(2, 21 * self.existential_deposit),
				(3, 30 * self.existential_deposit),
				(4, 40 * self.existential_deposit),
				(12, 10 * self.existential_deposit),
				(13, 9999 * self.existential_deposit),
			],
			..Default::default()
		}
		.assimilate_storage(&mut t)
		.unwrap();

		let vesting = if let Some(vesting_config) = self.vesting_genesis_config {
			vesting_config
		} else {
			vec![
				(1, Some(0), 10, 5 * self.existential_deposit),
				(2, Some(10), 20, 20 * self.existential_deposit),
				(12, Some(10), 20, 5 * self.existential_deposit),
			]
		};

		hold_vesting::GenesisConfig::<Test> { vesting }
			.assimilate_storage(&mut t)
			.unwrap();
		let mut ext = sp_io::TestExternalities::new(t);
		ext.execute_with(|| System::set_block_number(1));
		ext
	}
}

parameter_types! {
	static ObservedEventsVesting: usize = 0;
}

pub(crate) fn vesting_events_since_last_call() -> Vec<hold_vesting::Event<Test>> {
	let events = System::read_events_for_pallet::<hold_vesting::Event<Test>>();
	let already_seen = ObservedEventsVesting::get();
	ObservedEventsVesting::set(events.len());
	events.into_iter().skip(already_seen).collect()
}
