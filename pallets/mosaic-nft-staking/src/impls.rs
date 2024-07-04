use frame_support::{
	dispatch::DispatchResult,
	traits::{Get, ValidatorSet, ValidatorSetWithIdentification},
};
use sp_runtime::{
	traits::{Convert, ConvertInto},
	PerThing, Perbill,
};
use sp_staking::SessionIndex;
use sp_std::{marker::PhantomData, vec::Vec as SpVec};

use super::{
	Config, Contracts, Event, InverseSlashes, Pallet, SessionPallet, ValidatorState,
	ValidatorStates,
};

pub struct SelectableValidators<T>(sp_std::marker::PhantomData<T>);
pub struct SlashableValidators<T>(sp_std::marker::PhantomData<T>);

impl<T: Config> ValidatorSet<T::AccountId> for SelectableValidators<T> {
	type ValidatorId = T::AccountId;
	type ValidatorIdOf = ConvertInto;

	fn session_index() -> SessionIndex {
		SessionPallet::<T>::current_index()
	}

	fn validators() -> SpVec<Self::ValidatorId> {
		ValidatorStates::<T>::iter()
			.filter_map(|(validator_id, vstate)| match vstate {
				ValidatorState::Normal | ValidatorState::Faulted => Some(validator_id),
				ValidatorState::Chilled(_) => None,
			})
			.collect()
	}
}

impl<T: Config> ValidatorSet<T::AccountId> for SlashableValidators<T> {
	type ValidatorId = T::AccountId;
	type ValidatorIdOf = ConvertInto;

	fn session_index() -> SessionIndex {
		SessionPallet::<T>::current_index()
	}

	// A validator can be slashed if:
	// - has committed self-contract
	// - currently in active set or is selected to be in the next one (even if chilled)
	// - currently not selected and not chilled
	fn validators() -> SpVec<Self::ValidatorId> {
		let drafted_validators: SpVec<_> = Pallet::<T>::drafted_validators().collect(); // I'd like to use some sort of set so much...

		ValidatorStates::<T>::iter()
			.filter_map(|(validator_id, vstate)| {
				let chilled = matches!(vstate, ValidatorState::Chilled(_));

				let converted_id = T::ValidatorIdOf::convert(validator_id.clone())
					.expect("caller address can be converted to validator id");

				let drafted = drafted_validators.iter().any(|id| converted_id == *id);

				let has_commited_self_contract =
					Contracts::<T>::get(&validator_id, &validator_id).exists_committed();

				if (drafted || !chilled) && has_commited_self_contract {
					Some(validator_id)
				} else {
					None
				}
			})
			.collect()
	}
}
pub struct ValidatorOf<T>(PhantomData<T>);

impl<T: Config> Convert<T::AccountId, Option<T::AccountId>> for ValidatorOf<T> {
	fn convert(account: T::AccountId) -> Option<T::AccountId> {
		Some(account)
	}
}

impl<T: Config> ValidatorSetWithIdentification<T::AccountId> for SelectableValidators<T> {
	type Identification = T::AccountId;
	type IdentificationOf = ValidatorOf<T>;
}

impl<T: Config> ValidatorSetWithIdentification<T::AccountId> for SlashableValidators<T> {
	type Identification = T::AccountId;
	type IdentificationOf = ValidatorOf<T>;
}

impl<T: Config>
	utils::traits::OnDelegationNftExpire<T::AccountId, T::ItemId, T::Balance, T::AccountId>
	for Pallet<T>
{
	fn on_expire(
		owner: &T::AccountId,
		validator: Option<T::AccountId>,
		item_id: &T::ItemId,
		nominal_value: &T::Balance,
	) {
		let Some(validator) = validator else {
			return;
		};

		// Note: the nft is still considered in the session it expires, the delegator may still receive reward
		// We have ensured that this nominal value was staked for at least the minimum staking period when staking.
		Contracts::<T>::mutate(&validator, owner, |s| {
			let Some(contract) = s.ensure_staging_mut() else {
				return;
			};

			// If we don't find the index in the current contract, that means the delegator just manually undelegated the item in the current session.
			// In which case the nominal value is already deducted from the total stake.
			if let Some(index) =
				contract.stake.delegated_nfts.iter().position(|(x, _)| x == item_id)
			{
				contract.stake.delegated_nfts.remove(index);

				Self::shrink_total_validator_stake_by(&validator, *nominal_value);

				Self::deposit_event(Event::<T>::NftUndelegated {
					validator: validator.clone(),
					staker: owner.clone(),
					item_id: item_id.clone(),
				});
			}
		});
	}
}

impl<T: Config> utils::traits::SessionHook for Pallet<T>
where
	T::AccountId: From<<T as pallet_session::Config>::ValidatorId>,
{
	fn session_ended(_: u32) -> DispatchResult {
		let active_validators = SessionPallet::<T>::validators();
		let session_reward = T::SessionReward::get();

		let rewarded = active_validators.into_iter().filter_map(|v| {
			let v = T::AccountId::from(v);
			(!InverseSlashes::<T>::contains_key(&v)).then_some(v)
		});

		Self::do_reward_participants(rewarded, session_reward);
		Self::do_slash_participants();

		Self::commit_storage();

		Ok(())
	}
}

// TODO: define weights
impl<T: Config>
	sp_staking::offence::OnOffenceHandler<
		<T as frame_system::Config>::AccountId,
		<T as pallet_offences::Config>::IdentificationTuple,
		frame_support::weights::Weight,
	> for Pallet<T>
{
	fn on_offence(
		offenders: &[sp_staking::offence::OffenceDetails<
			<T as frame_system::Config>::AccountId,
			<T as pallet_offences::Config>::IdentificationTuple,
		>],
		slash_fraction: &[Perbill],
		_session: sp_staking::SessionIndex,
		_disable_strategy: sp_staking::offence::DisableStrategy,
	) -> frame_support::weights::Weight {
		for (o, slash) in offenders.iter().zip(slash_fraction.iter()) {
			InverseSlashes::<T>::mutate_exists(
				T::OffenderToValidatorId::convert(o.offender.clone()),
				|s| match *s {
					None => *s = Some(slash.left_from_one()),
					Some(ref mut acc) => *acc = *acc * slash.left_from_one(),
				},
			);
		}

		frame_support::weights::Weight::default()
	}
}
