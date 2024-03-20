use frame_support::traits::{Currency, Get, Imbalance};
use sp_runtime::{traits::Zero, PerThing, Saturating};
use sp_std::vec::Vec as SpVec;

use utils::traits::{NftDelegation, NftStaking};

use super::{
	ChillReason, Config, Contracts, Event, InverseSlashes, Pallet, UnlockingCurrency,
	ValidatorState, ValidatorStates,
};

impl<T: Config> Pallet<T> {
	fn chill_if_faulted(validator: &T::AccountId) {
		// Autochill after two consecutive slashes
		ValidatorStates::<T>::mutate_extant(validator, |vstate| {
			*vstate = match *vstate {
				ValidatorState::Normal => ValidatorState::Faulted,
				ValidatorState::Faulted => {
					Self::chill_state(validator.clone(), ChillReason::DoubleFault)
				},
				s => s,
			}
		});
	}

	fn chill_if_disqualified(validator: &T::AccountId) {
		// If slashed below some% autochill
		if T::NftStakingHandler::nominal_factor_of(validator).expect("could get nominal factor")
			<= T::NominalValueThreshold::get()
		{
			ValidatorStates::<T>::mutate_extant(validator, |vstate| {
				*vstate = Self::chill_state(validator.clone(), ChillReason::Disqualified);
			});
		}
	}

	pub(crate) fn do_slash_participants() {
		for (validator, slash) in
			InverseSlashes::<T>::drain().map(|(v, is)| (v, is.left_from_one()))
		{
			Self::chill_if_faulted(&validator);

			for (delegator, contract) in Contracts::<T>::iter_prefix(&validator) {
				let Some(committed_contract) = contract.committed().cloned() else {
					// We can only slash commited contracts
					// There is a small chance the validators who just bound themselves this session are slashed (but their self-contract is not yet commited)
					// But this slash can be forgiven as rewards are not received either (they can't yet be active)
					return;
				};

				let Some(mut new_contract) = contract.current().cloned() else {
					// Theoretically impossible
					return;
				};

				let mut total_stake_slash = T::Balance::zero();

				// Currency slash
				let currency_slash = slash * committed_contract.stake.currency;
				let staged_value = new_contract.stake.currency;

				total_stake_slash = total_stake_slash.saturating_add(currency_slash);
				if currency_slash > staged_value {
					let rem = currency_slash - staged_value;
					// Prevent double-unlocking by this...
					UnlockingCurrency::<T>::mutate_extant(&delegator, |unlocking| {
						*unlocking = (*unlocking).saturating_sub(rem);
					});

					total_stake_slash = total_stake_slash.saturating_sub(rem);
				}

				Self::unlock_currency(&delegator, currency_slash);

				let (actual_slash, _) = <T as Config>::Currency::slash(&delegator, currency_slash);

				if actual_slash.peek() != currency_slash {
					log::debug!("actual slash did not match calculated currency slash");
				}

				new_contract.stake.currency =
					new_contract.stake.currency.saturating_sub(actual_slash.peek());

				// Slash delegator nfts
				// TODO: we iterate through delegated nfts quite a lot. Can we make it more efficient?
				let mut d_nft_slash = slash * committed_contract.stake.delegated_nft();

				let mut slashed_delegator_nfts = SpVec::new();
				for (nft, value) in &committed_contract.stake.delegated_nfts {
					if d_nft_slash.is_zero() {
						break;
					}

					let slashed_from_this = d_nft_slash.min(*value);
					d_nft_slash -= slashed_from_this;
					let new_value = *value - slashed_from_this;

					// find nft in new_contract and deduct the slash both from it and the total stake.
					// if it's not found it means it just being unbound in which case it's whole value is already deducted.

					for (s_nft, s_value) in &mut new_contract.stake.delegated_nfts {
						if *s_nft == *nft {
							*s_value = new_value;
							total_stake_slash = total_stake_slash.saturating_add(slashed_from_this);
							break;
						}
					}

					// set nft nominal value
					if let Err(err) = T::NftDelegationHandler::set_nominal_value(nft, new_value) {
						log::error!(
							"Could not set delegator nft nominal value during slashing: {err:?}"
						);
					}

					slashed_delegator_nfts.push((nft.clone(), slashed_from_this));
				}

				// If it's the self-contract let's slash the permission nft as well.
				let permission_nft_slash =
					if let Some(p_nominal_value) = committed_contract.stake.permission_nft {
						let p_slash = slash * p_nominal_value;
						total_stake_slash = total_stake_slash.saturating_add(p_slash);
						let new_value =
							new_contract.stake.permission_nft.unwrap().saturating_sub(p_slash);
						new_contract.stake.permission_nft = Some(new_value);
						T::NftStakingHandler::set_nominal_value_of_bound(&validator, new_value)
							.expect("could set nominal value");

						Self::chill_if_disqualified(&validator);

						Some(p_slash)
					} else {
						None
					};

				// End
				Contracts::<T>::mutate(&validator, &delegator, |s| s.stage(new_contract));

				Self::deposit_event(Event::<T>::Slash {
					offender: validator.clone(),
					staker: delegator,
					currency: currency_slash,
					delegator_nfts: slashed_delegator_nfts,
					permission_nft: permission_nft_slash,
				});

				Self::shrink_total_validator_stake_by(&validator, total_stake_slash);
			}
		}
	}
}
