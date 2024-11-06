use super::*;

pub const MAX_VESTING_SCHEDULES: u32 = 8;
pub const HOLD_REASON: u8 = 0;

pub struct MockVesting;

impl frame_support::traits::StorageInstance for MockVesting {
	fn pallet_prefix() -> &'static str {
		"NoPallet"
	}

	const STORAGE_PREFIX: &'static str = "MockVesting";
}

pub type MockVestingStorage = StorageMap<
	MockVesting,
	Twox64Concat,
	AccountId,
	BoundedVec<Schedule<Balance, BlockNumberFor<Test>>, ConstU32<MAX_VESTING_SCHEDULES>>,
>;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Error {
	NotVesting,
	NoSuchSchedule,
	InvalidSchedule,
	MaxSchedulesReached,
}

impl From<Error> for DispatchError {
	fn from(value: Error) -> Self {
		let str_repr = match value {
			Error::NotVesting => "NotVesting",
			Error::NoSuchSchedule => "NoSuchSchedule",
			Error::InvalidSchedule => "InvalidSchedule",
			Error::MaxSchedulesReached => "MaxSchedulesReached",
		};
		DispatchError::Other(str_repr)
	}
}

impl HoldVestingSchedule<AccountId> for MockVesting {
	type BlockNumber = BlockNumberFor<Test>;
	type Fungible = Balances;

	fn vesting_balance(who: &AccountId) -> Option<Balance> {
		let schedules = MockVestingStorage::get(who)?;

		Some(
			schedules
				.into_iter()
				.fold(0, |a, s| a + s.locked_at::<ConvertInto>(System::block_number()))
				.min(Balances::balance(who)),
		)
	}

	fn add_vesting_schedule(
		who: &AccountId,
		schedule: Schedule<Balance, Self::BlockNumber>,
	) -> DispatchResult {
		if schedule.locked.is_zero() {
			return Ok(());
		}

		if !schedule.is_valid() {
			return Err(Error::InvalidSchedule.into());
		};

		Balances::hold(&HOLD_REASON, who, schedule.locked)?;

		MockVestingStorage::mutate_exists(who, |schedules| {
			match schedules {
				Some(s) => s.try_push(schedule).map_err(|_| Error::MaxSchedulesReached)?,
				None => *schedules = Some(BoundedVec::truncate_from(vec![schedule])),
			}

			Ok(())
		})
	}

	fn can_add_vesting_schedule(
		who: &AccountId,
		schedule: &Schedule<Balance, Self::BlockNumber>,
	) -> DispatchResult {
		if !schedule.is_valid() {
			return Err(Error::InvalidSchedule.into());
		}

		if MockVestingStorage::decode_len(who).unwrap_or_default() >= MAX_VESTING_SCHEDULES as usize
		{
			return Err(Error::MaxSchedulesReached.into());
		}

		Ok(())
	}

	fn get_vesting_schedule(
		who: &AccountId,
		schedule_index: u32,
	) -> Result<Schedule<Balance, Self::BlockNumber>, DispatchError> {
		let schedule = MockVestingStorage::get(who)
			.ok_or(Error::NotVesting)?
			.get(schedule_index as usize)
			.copied()
			.ok_or(Error::NoSuchSchedule)?;

		Ok(schedule)
	}

	fn remove_vesting_schedule(who: &AccountId, schedule_index: u32) -> DispatchResult {
		MockVestingStorage::mutate_exists(who, |schedules| {
			if let Some(s) = schedules {
				if schedule_index as usize >= s.len() {
					return Err(Error::NoSuchSchedule.into());
				}

				let schedule = s.remove(schedule_index as usize);

				// In this model the user never manually unlocks their vesting.
				// That case is handled in `pallet-hold-vesting` though.
				Balances::release(&HOLD_REASON, who, schedule.locked, Precision::Exact)?;

				if s.is_empty() {
					*schedules = None;
				}

				Ok(())
			} else {
				Err(Error::NotVesting.into())
			}
		})
	}
}
