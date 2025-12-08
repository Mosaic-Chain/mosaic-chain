use sdk::{frame_support, sp_runtime, sp_std};

use frame_support::{
	traits::{fungible::Balanced, tokens::Precision, Imbalance, OnUnbalanced},
	weights::WeightMeter,
};
use sp_runtime::{
	helpers_128bit::multiply_by_rational_with_rounding,
	traits::{Get, Saturating, Zero},
	PerThing, Perbill, Rounding,
};
use sp_std::vec::Vec as SpVec;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use utils::traits::StakingHooks;

use crate::{
	types::{Contract, StorageLayer, ValidatorState, MAX_NFTS_PER_CONTRACT},
	weights::WeightInfo,
	Config, Contracts, Event, Pallet, TotalValidatorStakes, ValidatorStates,
};

use super::{Progress, Status};

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub struct SweepContext<T: Config> {
	pub remaining_validators: SpVec<T::AccountId>,
	pub session_reward: u128,
}

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub enum Sweep<T: Config> {
	CalculateTotalStake,
	IterValidators { reward_state: RewardValidator<T>, context: ValidatorContext<T> },
	DepositContribution { total_contribution: T::Balance },
}

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub struct ValidatorContext<T: Config> {
	pub validator: T::AccountId,
	pub total_committed_stake: u128,
	pub session_reward: u128,
	pub total_contribution: T::Balance,
}

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub enum RewardValidator<T: Config> {
	MaybeResetValidatorState,
	// Storing the `Vec<u8>` cursor would result in `MaxEncodedLen` not being implemented.
	IterContracts { prev_delegator: Option<T::AccountId>, total_v_imbalance: T::Balance },
	DepositValidatorReward { total_v_imbalance: T::Balance },
}

impl<T: Config> Sweep<T> {
	pub(crate) fn total_committed_stake() -> T::Balance {
		TotalValidatorStakes::<T>::iter_prefix((StorageLayer::Committed,))
			.map(|(_, tvs)| tvs.total_stake)
			.fold(T::Balance::zero(), Saturating::saturating_add)
	}

	pub(crate) fn deposit_contribution(total_contribution: T::Balance) {
		let imbalance = T::Fungible::issue(total_contribution);
		T::ContributionDestination::on_unbalanced(imbalance);
	}
}

impl<T: Config> Progress for Sweep<T> {
	type Context = SweepContext<T>;

	fn make_progress(
		self,
		context: &mut Self::Context,
		weight_meter: &mut WeightMeter,
	) -> Status<Self> {
		match self {
			Self::CalculateTotalStake => {
				let weight = <T as Config>::WeightInfo::total_committed_stake(
					T::MaximumBoundValidators::get(),
				);
				if !weight_meter.can_consume(weight) {
					return Status::Stalled(Self::CalculateTotalStake);
				};

				let Some(first_validator) = context.remaining_validators.pop() else {
					return Status::Completed;
				};

				weight_meter.consume(weight);

				let total_committed_stake = Self::total_committed_stake().into();

				if total_committed_stake.is_zero() {
					return Status::Completed;
				}

				Status::Resumable(Self::IterValidators {
					reward_state: RewardValidator::MaybeResetValidatorState,
					context: ValidatorContext {
						validator: first_validator,
						total_committed_stake,
						session_reward: context.session_reward,
						total_contribution: T::Balance::zero(),
					},
				})
			},
			Self::IterValidators { reward_state, context: mut validator_context } => {
				match reward_state.try_complete(&mut validator_context, weight_meter) {
					Ok(()) => {
						let Some(validator) = context.remaining_validators.pop() else {
							return Status::Resumable(Self::DepositContribution {
								total_contribution: validator_context.total_contribution,
							});
						};

						Status::Resumable(Self::IterValidators {
							reward_state: RewardValidator::MaybeResetValidatorState,
							context: ValidatorContext { validator, ..validator_context },
						})
					},
					Err(reward_state) => Status::Stalled(Self::IterValidators {
						reward_state,
						context: validator_context,
					}),
				}
			},
			Self::DepositContribution { total_contribution } => {
				if weight_meter
					.try_consume(<T as Config>::WeightInfo::deposit_contribution())
					.is_err()
				{
					return Status::Stalled(Self::DepositContribution { total_contribution });
				};

				Self::deposit_contribution(total_contribution);

				Status::Completed
			},
		}
	}
}

struct ContractReward<Balance> {
	pub validator_reward: Balance,
	pub delegator_reward: Balance,
	pub contribution: Balance,
}

impl<T: Config> RewardValidator<T> {
	/// Set validator state to `Normal` if it's now `Faulted`
	pub(crate) fn maybe_reset_validator_state(validator: &T::AccountId) {
		ValidatorStates::<T>::mutate_extant(validator, |vstate| {
			if let ValidatorState::Faulted = *vstate {
				*vstate = ValidatorState::Normal;
			}
		});
	}

	pub(crate) fn deposit_validator_reward(validator: &T::AccountId, commission: T::Balance) {
		let Ok(v_imbalance) = T::Fungible::deposit(validator, commission, Precision::BestEffort)
		else {
			log::error!("Staking Reward: validator commission could not be deposited.");
			return;
		};

		let Ok(v_locked) =
			Pallet::<T>::lock_currency(validator, v_imbalance.peek(), Precision::BestEffort)
		else {
			log::error!(
				"Staking Reward: validator commission should be lockable after being deposited, even if some of it goes to the existential deposit."
			);
			return;
		};

		T::Hooks::on_reward(validator, commission);

		if let Some(mut self_contract) = Pallet::<T>::current_contract(validator, validator) {
			self_contract.stake.currency.saturating_accrue(v_locked);
			Pallet::<T>::stage_contract(validator, validator, self_contract);
			Pallet::<T>::grow_total_validator_stake_by(validator, v_locked);
		} else {
			log::error!("Staking Reward: validator {validator:?} does not have self-contract");
		};
	}

	/// Fully process a single contract.
	/// Returns the new validator imbalance.
	pub(crate) fn reward_contract(
		delegator: T::AccountId,
		contract: &Contract<T::Balance, T::ItemId>,
		mut total_v_imbalance: T::Balance,
		context: &mut <Self as Progress>::Context,
	) -> T::Balance {
		let reward = Self::calculate_contract_reward(
			context.total_committed_stake,
			context.session_reward,
			contract,
		);

		context.total_contribution.saturating_accrue(reward.contribution);
		total_v_imbalance.saturating_accrue(reward.validator_reward);

		let s_imbalance =
			T::Fungible::deposit(&delegator, reward.delegator_reward, Precision::BestEffort)
				.expect("BestEffort deposit should not fail");

		let s_locked =
			Pallet::<T>::lock_currency(&delegator, s_imbalance.peek(), Precision::BestEffort)
				.expect("the reward is available as free balance");

		let mut new_contract = Pallet::<T>::current_staged_contract(&context.validator, &delegator)
			.unwrap_or_else(|| contract.clone());

		new_contract.stake.currency.saturating_accrue(s_locked);
		Pallet::<T>::stage_contract(&context.validator, &delegator, new_contract);

		Pallet::<T>::grow_total_validator_stake_by(&context.validator, s_locked);

		T::Hooks::on_reward(&delegator, reward.delegator_reward);

		Pallet::<T>::deposit_event(Event::<T>::ContractReward {
			validator: context.validator.clone(),
			staker: delegator,
			reward: reward.delegator_reward,
			commission: reward.validator_reward,
		});

		total_v_imbalance
	}

	#[inline]
	fn rmul(a: u128, b: u128, c: u128) -> Option<u128> {
		multiply_by_rational_with_rounding(a, b, c, Rounding::NearestPrefDown)
	}

	/// Calculates how much is rewarded in a session for a given staking contract to the validator, the delegator and the treasury.
	///
	/// ```rust,ignore
	/// contract_reward = session_reward * contract.stake.total / total_stake
	///
	/// contribution = ContributionPercentage * contract_reward
	/// validator_reward = contract.commission * (contract_reward - contribution)
	/// delegator_reward = contract_reward - contribution - validator_reward
	/// ```
	fn calculate_contract_reward(
		total_stake: u128,
		session_reward: u128,
		contract: &Contract<T::Balance, T::ItemId>,
	) -> ContractReward<T::Balance> {
		let contract_reward =
			Self::rmul(session_reward, contract.stake.total().into(), total_stake)
				.expect("contract.stake <= total_stake ==> contract_reward <= session_reward");

		let contribution = T::ContributionPercentage::get() * contract_reward;
		let contract_reward = contract_reward.saturating_sub(contribution);

		let (nominator, denominator) = (contract.commission.deconstruct(), Perbill::ACCURACY);

		let validator_reward = Self::rmul(contract_reward, nominator.into(), denominator.into())
			.expect("commission <= 1");
		let delegator_reward = contract_reward.saturating_sub(validator_reward);

		ContractReward {
			validator_reward: validator_reward.into(),
			delegator_reward: delegator_reward.into(),
			contribution: contribution.into(),
		}
	}
}

impl<T: Config> Progress for RewardValidator<T> {
	type Context = ValidatorContext<T>;

	fn make_progress(
		self,
		context: &mut Self::Context,
		weight_meter: &mut WeightMeter,
	) -> Status<Self> {
		match self {
			Self::MaybeResetValidatorState => {
				if weight_meter
					.try_consume(<T as Config>::WeightInfo::maybe_reset_validator_state())
					.is_err()
				{
					return Status::Stalled(Self::MaybeResetValidatorState);
				}

				Self::maybe_reset_validator_state(&context.validator);

				Status::Resumable(Self::IterContracts {
					prev_delegator: None,
					total_v_imbalance: T::Balance::zero(),
				})
			},
			Self::IterContracts { mut prev_delegator, mut total_v_imbalance } => {
				let mut iter = if let Some(cursor) = prev_delegator.as_ref() {
					let raw_key = Contracts::<T>::hashed_key_for((
						StorageLayer::Committed,
						&context.validator,
						cursor,
					));
					Contracts::<T>::iter_prefix_from(
						(StorageLayer::Committed, &context.validator),
						raw_key,
					)
				} else {
					Contracts::<T>::iter_prefix((StorageLayer::Committed, &context.validator))
				};

				loop {
					if weight_meter.try_consume(T::DbWeight::get().reads(1)).is_err() {
						return Status::Stalled(Self::IterContracts {
							prev_delegator,
							total_v_imbalance,
						});
					}

					let Some((delegator, contract)) = iter.next() else {
						return Status::Resumable(Self::DepositValidatorReward {
							total_v_imbalance,
						});
					};

					if weight_meter
						.try_consume(<T as Config>::WeightInfo::reward_contract(
							MAX_NFTS_PER_CONTRACT,
						))
						.is_err()
					{
						return Status::Stalled(Self::IterContracts {
							prev_delegator,
							total_v_imbalance,
						});
					}

					total_v_imbalance = Self::reward_contract(
						delegator.clone(),
						&contract,
						total_v_imbalance,
						context,
					);

					prev_delegator = Some(delegator);
				}
			},
			Self::DepositValidatorReward { total_v_imbalance } => {
				if weight_meter
					.try_consume(<T as Config>::WeightInfo::deposit_validator_reward())
					.is_err()
				{
					return Status::Stalled(Self::DepositValidatorReward { total_v_imbalance });
				}

				Self::deposit_validator_reward(&context.validator, total_v_imbalance);

				Status::Completed
			},
		}
	}
}
