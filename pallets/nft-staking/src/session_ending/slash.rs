use sdk::{frame_support, sp_runtime, sp_std};

use frame_support::{
	traits::{fungible::BalancedHold, Imbalance},
	weights::WeightMeter,
};
use sp_runtime::{
	traits::{Get, Saturating, Zero},
	Perbill,
};
use sp_std::vec::Vec as SpVec;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use utils::traits::{NftDelegation, NftStaking, StakingHooks};

use crate::{
	types::{ChillReason, Contract, StorageLayer, ValidatorState, MAX_NFTS_PER_CONTRACT},
	weights::WeightInfo,
	Config, Contracts, Event, HoldReason, Pallet, UnlockingCurrency, ValidatorStates,
};

use super::{Progress, Status};

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub struct SweepContext<T: Config> {
	pub remaining_offenders: SpVec<(T::AccountId, Perbill)>,
}

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub enum Sweep<T: Config> {
	Init,
	IterOffenders { slash_state: SlashOffender<T>, context: OffenderContext<T> },
}

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub struct OffenderContext<T: Config> {
	pub offender: T::AccountId,
	pub slash: Perbill,
}

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub enum SlashOffender<T: Config> {
	MaybeChill,
	IterContracts { prev_delegator: Option<T::AccountId> },
}

impl<T: Config> Progress for Sweep<T> {
	type Context = SweepContext<T>;

	fn make_progress(
		self,
		context: &mut Self::Context,
		weight_meter: &mut WeightMeter,
	) -> Status<Self> {
		match self {
			Self::Init => {
				let Some((first_offender, slash)) = context.remaining_offenders.pop() else {
					return Status::Completed;
				};

				Status::Resumable(Self::IterOffenders {
					slash_state: SlashOffender::MaybeChill,
					context: OffenderContext { offender: first_offender, slash },
				})
			},

			Self::IterOffenders { slash_state, context: mut offender_context } => match slash_state
				.try_complete(&mut offender_context, weight_meter)
			{
				Ok(()) => {
					let Some((offender, slash)) = context.remaining_offenders.pop() else {
						return Status::Completed;
					};

					Status::Resumable(Self::IterOffenders {
						slash_state: SlashOffender::MaybeChill,
						context: OffenderContext { offender, slash },
					})
				},
				Err(slash_state) => {
					Status::Stalled(Self::IterOffenders { slash_state, context: offender_context })
				},
			},
		}
	}
}

impl<T: Config> SlashOffender<T> {
	pub(crate) fn chill_if_faulted(offender: &T::AccountId) {
		// Autochill after two consecutive slashes
		ValidatorStates::<T>::mutate_extant(offender, |vstate| {
			*vstate = match *vstate {
				ValidatorState::Normal => ValidatorState::Faulted,
				ValidatorState::Faulted => {
					Pallet::<T>::chill_state(offender.clone(), ChillReason::DoubleFault)
				},
				s => s,
			}
		});
	}

	pub(crate) fn slash_contract(
		delegator: T::AccountId,
		committed_contract: &Contract<T::Balance, T::ItemId>,
		context: &mut <Self as Progress>::Context,
	) {
		let mut new_contract = Pallet::<T>::current_staged_contract(&context.offender, &delegator)
			.unwrap_or_else(|| committed_contract.clone());

		let mut total_stake_slash = T::Balance::zero();

		// Currency slash
		let currency_slash = context.slash * committed_contract.stake.currency;
		let staged_value = new_contract.stake.currency;

		total_stake_slash.saturating_accrue(currency_slash);
		if currency_slash > staged_value {
			let rem = currency_slash - staged_value;
			// Prevent double-unlocking by this...
			UnlockingCurrency::<T>::mutate_extant(&delegator, |unlocking| {
				unlocking.saturating_reduce(rem);
			});

			total_stake_slash.saturating_reduce(rem);
		}

		let (actual_slash, _) = <<T as Config>::Fungible as BalancedHold<T::AccountId>>::slash(
			&HoldReason::Staking.into(),
			&delegator,
			currency_slash,
		);

		if actual_slash.peek() != currency_slash {
			log::debug!("actual slash did not match calculated currency slash");
		}

		T::Hooks::on_currency_slash(&delegator, actual_slash.peek());

		new_contract.stake.currency.saturating_reduce(actual_slash.peek());

		// Slash delegator nfts
		// TODO: we iterate through delegated nfts quite a lot. Can we make it more efficient?
		let mut d_nft_slash = context.slash * committed_contract.stake.delegated_nft();

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
					total_stake_slash.saturating_accrue(slashed_from_this);
					break;
				}
			}

			// set nft nominal value
			if T::NftDelegationHandler::is_bound(nft) {
				if let Err(err) = T::NftDelegationHandler::set_nominal_value(nft, new_value) {
					log::error!(
						"Could not set delegator nft nominal value during slashing: {err:?}"
					);
				}
			}

			T::Hooks::on_nft_slash(&delegator, nft, slashed_from_this);
			slashed_delegator_nfts.push((nft.clone(), slashed_from_this));
		}

		// If it's the self-contract let's slash the permission nft as well.
		let permission_nft_slash =
			if let Some(p_nominal_value) = committed_contract.stake.permission_nft {
				let p_slash = context.slash * p_nominal_value;
				total_stake_slash = total_stake_slash.saturating_add(p_slash);
				let new_value = new_contract.stake.permission_nft.unwrap().saturating_sub(p_slash);
				new_contract.stake.permission_nft = Some(new_value);
				T::NftStakingHandler::set_nominal_value_of_bound(&context.offender, new_value)
					.expect("could set nominal value");

				Self::chill_if_disqualified(&context.offender);

				T::Hooks::on_permission_nft_slash(&context.offender, p_slash);

				Some(p_slash)
			} else {
				None
			};

		// End

		Pallet::<T>::stage_contract(&context.offender, &delegator, new_contract);

		Pallet::<T>::deposit_event(Event::<T>::Slash {
			offender: context.offender.clone(),
			staker: delegator,
			currency: currency_slash,
			delegator_nfts: slashed_delegator_nfts,
			permission_nft: permission_nft_slash,
		});

		Pallet::<T>::shrink_total_validator_stake_by(&context.offender, total_stake_slash);
	}

	pub(crate) fn chill_if_disqualified(validator: &T::AccountId) {
		// If slashed below some% autochill
		if T::NftStakingHandler::nominal_factor_of_bound(validator)
			.expect("could get nominal factor")
			<= T::NominalValueThreshold::get()
		{
			ValidatorStates::<T>::mutate_extant(validator, |vstate| {
				*vstate = Pallet::<T>::chill_state(validator.clone(), ChillReason::Disqualified);
			});
		}
	}
}

impl<T: Config> Progress for SlashOffender<T> {
	type Context = OffenderContext<T>;

	fn make_progress(
		self,
		context: &mut Self::Context,
		weight_meter: &mut WeightMeter,
	) -> Status<Self> {
		match self {
			Self::MaybeChill => {
				if weight_meter.try_consume(<T as Config>::WeightInfo::chill_if_faulted()).is_err()
				{
					return Status::Stalled(Self::MaybeChill);
				}

				Self::chill_if_faulted(&context.offender);

				Status::Resumable(Self::IterContracts { prev_delegator: None })
			},
			Self::IterContracts { mut prev_delegator } => {
				let mut iter = if let Some(cursor) = prev_delegator.as_ref() {
					let raw_key = Contracts::<T>::hashed_key_for((
						StorageLayer::Committed,
						&context.offender,
						cursor,
					));
					Contracts::<T>::iter_prefix_from(
						(StorageLayer::Committed, &context.offender),
						raw_key,
					)
				} else {
					Contracts::<T>::iter_prefix((StorageLayer::Committed, &context.offender))
				};

				loop {
					if weight_meter.try_consume(T::DbWeight::get().reads(1)).is_err() {
						return Status::Stalled(Self::IterContracts { prev_delegator });
					}

					let Some((delegator, contract)) = iter.next() else {
						return Status::Completed;
					};

					if weight_meter
						.try_consume(
							<T as Config>::WeightInfo::slash_contract(MAX_NFTS_PER_CONTRACT)
								+ <T as Config>::WeightInfo::chill_if_disqualified(),
						)
						.is_err()
					{
						return Status::Stalled(Self::IterContracts { prev_delegator });
					}

					Self::slash_contract(delegator.clone(), &contract, context);

					prev_delegator = Some(delegator);
				}
			},
		}
	}
}
