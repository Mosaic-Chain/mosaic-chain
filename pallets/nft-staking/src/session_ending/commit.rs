use sdk::{frame_support, sp_runtime};

use frame_support::weights::{Weight, WeightMeter};
use sp_runtime::traits::Get;

use codec::{Decode, Encode, MaxEncodedLen};
use scale_info::TypeInfo;

use utils::traits::NftDelegation;

use crate::{
	types::{Contract, StorageLayer, TotalValidatorStake},
	weights::WeightInfo,
	Config, Contracts, Pallet, TotalValidatorStakes, UnlockingCurrency, UnlockingDelegatorNfts,
};

use super::{Progress, Status};

#[derive(TypeInfo, MaxEncodedLen, Encode, Decode)]
#[scale_info(skip_type_params(T))]
pub enum Sweep<T: Config> {
	RotateStagingLayers,
	CommitContracts,
	CommitTotalValidatorStakes,
	UnlockCurrency,
	UnlockDelegatorNfts,
	_Unreachable(core::marker::PhantomData<T>),
}

impl<T: Config> Sweep<T> {
	pub(crate) fn commit_contract(
		validator: T::AccountId,
		delegator: T::AccountId,
		contract: Contract<T::Balance, T::ItemId>,
	) {
		let maybe_contract = if contract.stake.is_empty() {
			Pallet::<T>::relinquish_contract(&validator);
			None
		} else {
			Some(contract)
		};

		Contracts::<T>::set((StorageLayer::Committed, validator, delegator), maybe_contract);
	}

	pub(crate) fn commit_total_validator_stakes(
		validator: &T::AccountId,
		total_validator_stake: TotalValidatorStake<T::Balance>,
	) {
		TotalValidatorStakes::<T>::insert(
			(StorageLayer::Committed, &validator),
			total_validator_stake,
		);
	}

	#[inline]
	fn drain_template<I, V>(
		weight_meter: &mut WeightMeter,
		drain_iter_f: fn(StorageLayer) -> I,
		stall_status: Status<Self>,
		next_status: Status<Self>,
		op_weight: Weight,
		op: fn(V),
	) -> Status<Self>
	where
		I: Iterator<Item = V>,
	{
		if weight_meter.try_consume(T::DbWeight::get().reads_writes(1, 0)).is_err() {
			return stall_status;
		}
		let layer = Pallet::<T>::current_staging_layer().other();
		let mut drain_iter = drain_iter_f(StorageLayer::Staged(layer));

		let drain_weight = T::DbWeight::get().reads_writes(1, 1);
		let full_weight = drain_weight.saturating_add(op_weight);
		loop {
			if !weight_meter.can_consume(full_weight) {
				return stall_status;
			}

			let Some(v) = drain_iter.next() else {
				weight_meter.consume(drain_weight);
				return next_status;
			};

			weight_meter.consume(full_weight);
			op(v);
		}
	}
}

impl<T: Config> Progress for Sweep<T> {
	type Context = ();

	fn make_progress(
		self,
		_context: &mut Self::Context,
		weight_meter: &mut WeightMeter,
	) -> Status<Self> {
		match self {
			Self::RotateStagingLayers => {
				if weight_meter
					.try_consume(<T as Config>::WeightInfo::rotate_staging_layer())
					.is_err()
				{
					return Status::Stalled(Self::RotateStagingLayers);
				}

				Pallet::<T>::rotate_staging_layer();
				Status::Resumable(Self::CommitContracts)
			},
			Self::CommitContracts => Self::drain_template(
				weight_meter,
				|l| Contracts::<T>::drain_prefix((l,)),
				Status::Stalled(Self::CommitContracts),
				Status::Resumable(Self::CommitTotalValidatorStakes),
				<T as Config>::WeightInfo::commit_empty_contract()
					.max(<T as Config>::WeightInfo::commit_full_contract()),
				|((validator, delegator), contract)| {
					Self::commit_contract(validator, delegator, contract)
				},
			),
			Self::CommitTotalValidatorStakes => Self::drain_template(
				weight_meter,
				|l| TotalValidatorStakes::<T>::drain_prefix((l,)),
				Status::Stalled(Self::CommitTotalValidatorStakes),
				Status::Resumable(Self::UnlockCurrency),
				<T as Config>::WeightInfo::commit_total_validator_stakes(),
				|(validator, total_validator_stake)| {
					Self::commit_total_validator_stakes(&validator, total_validator_stake);
				},
			),
			Self::UnlockCurrency => Self::drain_template(
				weight_meter,
				|_l| UnlockingCurrency::<T>::drain(),
				Status::Stalled(Self::UnlockCurrency),
				Status::Resumable(Self::UnlockDelegatorNfts),
				<T as Config>::WeightInfo::unlock_currency(),
				|(delegator, amount)| {
					Pallet::<T>::unlock_currency(&delegator, amount);
				},
			),
			Self::UnlockDelegatorNfts => Self::drain_template(
				weight_meter,
				|_l| UnlockingDelegatorNfts::<T>::drain(),
				Status::Stalled(Self::UnlockDelegatorNfts),
				Status::Completed,
				<T as Config>::WeightInfo::unlock_delegator_nft(),
				|(item_id, delegator)| {
					if let Err(e) = T::NftDelegationHandler::unbind(&delegator, &item_id) {
						log::error!(
							"failed to unbind delegator nft({item_id:?}) of {delegator:?}: {e:?}"
						);
					}
				},
			),
			Self::_Unreachable(_) => {
				log::error!("Nft staking commit storage: unreachable state reached");
				Status::Completed
			},
		}
	}
}
