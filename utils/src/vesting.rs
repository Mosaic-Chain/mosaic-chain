use codec::{Decode, DecodeWithMemTracking, Encode, MaxEncodedLen};
use scale_info::TypeInfo;
use sdk::frame_support::{
	dispatch::DispatchResult,
	sp_runtime::{
		traits::{AtLeast32BitUnsigned, Bounded, Convert, One, Zero},
		DispatchError, RuntimeDebug,
	},
	traits::fungible::Inspect,
};

#[derive(
	Encode,
	Decode,
	DecodeWithMemTracking,
	Copy,
	Clone,
	PartialEq,
	Eq,
	RuntimeDebug,
	MaxEncodedLen,
	TypeInfo,
)]
pub struct Schedule<Balance, BlockNumber> {
	pub locked: Balance,
	pub per_block: Balance,
	pub starting_block: Option<BlockNumber>,
}

impl<Balance, BlockNumber> Schedule<Balance, BlockNumber>
where
	Balance: AtLeast32BitUnsigned + Copy,
	BlockNumber: AtLeast32BitUnsigned + Copy + Bounded,
{
	/// Instantiate a new `Schedule`.
	pub fn new(locked: Balance, per_block: Balance, starting_block: Option<BlockNumber>) -> Self {
		Self { locked, per_block, starting_block }
	}

	/// Validate parameters for `Schedule`. Note that this does not check
	/// against `MinVestedTransfer`.
	pub fn is_valid(&self) -> bool {
		!self.locked.is_zero() && !self.raw_per_block().is_zero()
	}

	/// Locked amount at schedule creation.
	pub fn locked(&self) -> Balance {
		self.locked
	}

	/// Amount that gets unlocked every block after `starting_block`. Corrects for `per_block` of 0.
	/// We don't let `per_block` be less than 1, or else the vesting will never end.
	/// This should be used whenever accessing `per_block` unless explicitly checking for 0 values.
	pub fn per_block(&self) -> Balance {
		self.per_block.max(One::one())
	}

	/// Get the unmodified `per_block`. Generally should not be used, but is useful for
	/// validating `per_block`.
	pub(crate) fn raw_per_block(&self) -> Balance {
		self.per_block
	}

	/// Starting block for unlocking(vesting).
	pub fn starting_block(&self) -> Option<BlockNumber> {
		self.starting_block
	}

	/// Amount on hold at block `n`.
	pub fn locked_at<BlockNumberToBalance: Convert<BlockNumber, Balance>>(
		&self,
		n: BlockNumber,
	) -> Balance {
		let Some(starting_block) = self.starting_block else {
			return self.locked;
		};

		// Number of blocks that count toward vesting;
		// saturating to 0 when n < starting_block.
		let vested_block_count = n.saturating_sub(starting_block);
		let vested_block_count = BlockNumberToBalance::convert(vested_block_count);
		// Return amount that is still locked in vesting.
		vested_block_count
			.checked_mul(&self.per_block()) // `per_block` accessor guarantees at least 1.
			.map_or(Zero::zero(), |to_unlock| self.locked.saturating_sub(to_unlock))
	}

	/// Block number at which the schedule ends (as type `Balance`).
	pub fn ending_block_as_balance<BlockNumberToBalance: Convert<BlockNumber, Balance>>(
		&self,
	) -> Option<Balance> {
		let starting_block = BlockNumberToBalance::convert(self.starting_block?);
		let duration = if self.per_block() >= self.locked {
			// If `per_block` is bigger than `locked`, the schedule will end
			// the block after starting.
			One::one()
		} else {
			self.locked / self.per_block()
				+ if (self.locked % self.per_block()).is_zero() {
					Zero::zero()
				} else {
					// `per_block` does not perfectly divide `locked`, so we need an extra block to
					// unlock some amount less than `per_block`.
					One::one()
				}
		};

		Some(starting_block.saturating_add(duration))
	}
}

type BalanceOf<T, A> = <<T as HoldVestingSchedule<A>>::Fungible as Inspect<A>>::Balance;
/// A vesting schedule over a fungible. This allows a particular fungible asset to have vesting limits
/// applied to it.
pub trait HoldVestingSchedule<AccountId> {
	/// The quantity used to denote time; usually just a `BlockNumber`.
	type BlockNumber;

	/// The fungible that this schedule applies to.
	type Fungible: Inspect<AccountId>;

	/// Get the amount that is currently being vested and cannot be transferred out of this account.
	/// Returns `None` if the account has no vesting schedule.
	fn vesting_balance(who: &AccountId) -> Option<BalanceOf<Self, AccountId>>;

	/// Adds a vesting schedule to a given account.
	///
	/// If the account has `MaxVestingSchedules`, an Error is returned and nothing
	/// is updated.
	///
	/// Is a no-op if the amount to be vested is zero.
	fn add_vesting_schedule(
		who: &AccountId,
		schedule: Schedule<BalanceOf<Self, AccountId>, Self::BlockNumber>,
	) -> DispatchResult;

	/// Checks if `add_vesting_schedule` would work against `who`.
	fn can_add_vesting_schedule(
		who: &AccountId,
		schedule: &Schedule<BalanceOf<Self, AccountId>, Self::BlockNumber>,
	) -> DispatchResult;

	/// Get a vesting schedule of a given account
	fn get_vesting_schedule(
		who: &AccountId,
		schedule_index: u32,
	) -> Result<Schedule<BalanceOf<Self, AccountId>, Self::BlockNumber>, DispatchError>;

	/// Remove a vesting schedule for a given account.
	fn remove_vesting_schedule(who: &AccountId, schedule_index: u32) -> DispatchResult;
}
