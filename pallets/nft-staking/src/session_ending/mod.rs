use sdk::{frame_support, sp_runtime, sp_std};

use frame_support::weights::WeightMeter;
use sp_runtime::Perbill;
use sp_std::vec::Vec as SpVec;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use utils::SessionIndex;

pub mod commit;
pub mod reward;
pub mod slash;

use crate::Config;

pub enum Status<S> {
	/// Some progress have been made, can be resumed.
	Resumable(S),

	/// Some progress **might** have been made, but ran out of weight.
	Stalled(S),

	/// All steps are done, resuming progress is undefined behaviour.
	Completed,
}

pub trait Progress: Sized {
	type Context;

	/// Make some progress.
	///
	/// - If progress could be made and some weight might remain returns `Status::Resumable`
	/// - If ran out of weight before finishing returns `Status::Stalled`
	/// - If everything is completed returns `Status::Completed`,
	fn make_progress(
		self,
		context: &mut Self::Context,
		weight_meter: &mut WeightMeter,
	) -> Status<Self>;

	/// Makes progress in a greedy manner consuming as much weight as possible.
	fn try_complete(
		mut self,
		context: &mut Self::Context,
		weight_meter: &mut WeightMeter,
	) -> Result<(), Self> {
		loop {
			match self.make_progress(context, weight_meter) {
				Status::Completed => {
					return Ok(());
				},
				Status::Stalled(new) => return Err(new),
				Status::Resumable(new) => {
					self = new;
					continue;
				},
			}
		}
	}
}

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub struct SessionEnding<T: Config> {
	pub state: State<T>,
	pub context: Context<T>,
}

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub struct Context<T: Config> {
	pub session_index: SessionIndex,
	pub slash_context: slash::SweepContext<T>,
}

impl<T: Config> SessionEnding<T> {
	pub fn new(
		session_index: SessionIndex,
		session_reward: u128,
		rewarded: SpVec<T::AccountId>,
		slashes: SpVec<(T::AccountId, Perbill)>,
	) -> Self {
		Self {
			state: State::Reward {
				sweep: reward::Sweep::CalculateTotalStake,
				context: reward::SweepContext { remaining_validators: rewarded, session_reward },
			},
			context: Context {
				session_index,
				slash_context: slash::SweepContext { remaining_offenders: slashes },
			},
		}
	}
}

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub enum State<T: Config> {
	Reward { sweep: reward::Sweep<T>, context: reward::SweepContext<T> },
	Slash { sweep: slash::Sweep<T> },
	Commit { sweep: commit::Sweep<T> },
}

impl<T: Config> Progress for State<T> {
	type Context = Context<T>;

	fn make_progress(
		self,
		context: &mut Self::Context,
		weight_meter: &mut WeightMeter,
	) -> Status<Self> {
		match self {
			State::Reward { sweep, context: mut sweep_context } => {
				match sweep.try_complete(&mut sweep_context, weight_meter) {
					Ok(()) => Status::Resumable(Self::Slash { sweep: slash::Sweep::Init }),
					Err(sweep) => Status::Stalled(Self::Reward { sweep, context: sweep_context }),
				}
			},
			State::Slash { sweep } => {
				match sweep.try_complete(&mut context.slash_context, weight_meter) {
					Ok(()) => Status::Resumable(Self::Commit {
						sweep: commit::Sweep::RotateStagingLayers,
					}),
					Err(sweep) => Status::Stalled(Self::Slash { sweep }),
				}
			},
			State::Commit { sweep } => match sweep.try_complete(&mut (), weight_meter) {
				Ok(()) => Status::Completed,
				Err(sweep) => Status::Stalled(Self::Commit { sweep }),
			},
		}
	}
}
