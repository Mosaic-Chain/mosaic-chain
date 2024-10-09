use frame_support::traits::{fungible::Balanced, tokens::Precision, Imbalance, OnUnbalanced};
use sp_runtime::{
	helpers_128bit::multiply_by_rational_with_rounding, traits::Zero, PerThing, Perbill, Rounding,
	Saturating,
};

use super::{
	Config, Contract, Contracts, Event, Get, NegativeImbalanceOf, Pallet, PositiveImbalanceOf,
	TotalValidatorStakes, ValidatorState, ValidatorStates,
};

#[inline]
fn rmul(a: u128, b: u128, c: u128) -> Option<u128> {
	multiply_by_rational_with_rounding(a, b, c, Rounding::NearestPrefDown)
}

struct ContractReward<Balance> {
	pub validator_reward: Balance,
	pub staker_reward: Balance,
	pub contribution: Balance,
}

/// Calculates how much is rewarded in a session for a given staking contract to the validator, the delegator and the treasury.
///
/// ```
/// contract_reward = session_reward * contract.stake.total / total_stake
///
/// contribution = ContributionPercentage * contract_reward
/// validator_reward = contract.commission * (contract_reward - contribution)
/// staker_reward = contract_reward - contribution - validator_reward
/// ```
fn calculate_contract_reward<T: Config>(
	total_stake: u128,
	session_reward: u128,
	contract: &Contract<T::Balance, T::ItemId>,
) -> ContractReward<T::Balance> {
	let contract_reward = rmul(session_reward, contract.stake.total().into(), total_stake)
		.expect("contract.stake <= total_stake ==> contract_reward <= session_reward");

	let contribution = T::ContributionPercentage::get() * contract_reward;
	let contract_reward = contract_reward.saturating_sub(contribution);

	let (nominator, denominator) = (contract.commission.deconstruct(), Perbill::ACCURACY);

	let validator_reward =
		rmul(contract_reward, nominator.into(), denominator.into()).expect("commission <= 1");
	let staker_reward = contract_reward.saturating_sub(validator_reward);

	ContractReward {
		validator_reward: validator_reward.into(),
		staker_reward: staker_reward.into(),
		contribution: contribution.into(),
	}
}

fn total_committed_stake<T: Config>() -> T::Balance {
	TotalValidatorStakes::<T>::iter()
		.filter_map(|(_, s)| s.committed().map(|tvs| tvs.total_stake))
		.fold(T::Balance::zero(), Saturating::saturating_add)
}

impl<T: Config> Pallet<T> {
	pub(crate) fn do_reward_participants(
		rewarded_validators: impl Iterator<Item = T::AccountId>,
		session_reward: u128,
	) {
		let committed_total_stake: u128 = total_committed_stake::<T>().into();

		if committed_total_stake == 0 {
			return;
		}

		let mut total_rewarded = PositiveImbalanceOf::<T>::default();
		let mut total_contribution = NegativeImbalanceOf::<T>::default();

		for validator in rewarded_validators {
			ValidatorStates::<T>::mutate_extant(&validator, |vstate| {
				if let ValidatorState::Faulted = *vstate {
					*vstate = ValidatorState::Normal;
				}
			});

			let mut total_v_imbalance = PositiveImbalanceOf::<T>::default();
			for (staker, contract) in Contracts::<T>::iter_prefix(&validator)
				.filter_map(|(staker, s)| s.committed().cloned().map(|c| (staker, c)))
			{
				let reward = calculate_contract_reward::<T>(
					committed_total_stake,
					session_reward,
					&contract,
				);

				let contribution = <T as Config>::Fungible::issue(reward.contribution);
				total_contribution.subsume(contribution);

				let v_imbalance = <T as Config>::Fungible::deposit(
					&validator,
					reward.validator_reward,
					Precision::BestEffort,
				)
				.expect("BestEffort deposit should not fail");

				total_v_imbalance.subsume(v_imbalance);

				let s_imbalance = <T as Config>::Fungible::deposit(
					&staker,
					reward.staker_reward,
					Precision::BestEffort,
				)
				.expect("BestEffort deposit should not fail");

				Self::lock_currency(&staker, s_imbalance.peek())
					.expect("the reward is available as free balance");

				Contracts::<T>::mutate(&validator, &staker, |s| {
					if let Some(new) = s.ensure_staging_mut() {
						new.stake.currency = new.stake.currency.saturating_add(s_imbalance.peek());
					}
				});

				Self::grow_total_validator_stake_by(&validator, s_imbalance.peek());
				total_rewarded.subsume(s_imbalance);

				Self::deposit_event(Event::<T>::ContractReward {
					validator: validator.clone(),
					staker,
					reward: reward.staker_reward,
					commission: reward.validator_reward,
				});
			}

			Self::lock_currency(&validator, total_v_imbalance.peek())
				.expect("the reward is available as free balance");

			Contracts::<T>::mutate(&validator, &validator, |s| {
				if let Some(new) = s.ensure_staging_mut() {
					new.stake.currency =
						new.stake.currency.saturating_add(total_v_imbalance.peek());
				}
			});

			Self::grow_total_validator_stake_by(&validator, total_v_imbalance.peek());
			total_rewarded.subsume(total_v_imbalance);
		}

		T::OnReward::on_unbalanced(total_rewarded);
		T::ContributionDestination::on_unbalanced(total_contribution);
	}
}
