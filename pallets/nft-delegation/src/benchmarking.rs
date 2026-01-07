//! Benchmarking setup for the delegation NFT pallet

use super::*;
use crate::Pallet;

use sdk::frame_benchmarking::v2::*;
use sdk::frame_system::RawOrigin;

const MAX_EXPIRATION_PER_SESSION: u32 = 16;

#[benchmarks(
	where T::ItemId: Incrementable + From<u32>, T::Balance: From<u32>, T::BindMetadata: From<T::AccountId>
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn mint_delegator_token() {
		let origin = RawOrigin::<T::AccountId>::Root;
		let account_id: T::AccountId = whitelisted_caller();
		let expiration: SessionIndex = 4320;
		let nominal_value: T::Balance = 100u32.into();

		#[extrinsic_call]
		_(origin, account_id, expiration, nominal_value);
	}

	#[benchmark]
	fn force_stop_expiration() {
		let origin = RawOrigin::<T::AccountId>::Root;
		let account_id: T::AccountId = whitelisted_caller();
		let expiration: SessionIndex = 4320;
		let nominal_value: T::Balance = 100u32.into();

		for i in 0..MAX_EXPIRATION_PER_SESSION {
			<Pallet<T> as NftDelegation<_, _, _, _>>::mint(&account_id, expiration, &nominal_value)
				.expect("could mint delegator token");
			<Pallet<T> as NftDelegation<_, _, _, _>>::bind(
				&account_id,
				&i.into(),
				account_id.clone().into(),
			)
			.expect("could bind nft");
		}

		#[extrinsic_call]
		_(origin, (MAX_EXPIRATION_PER_SESSION - 1).into());
	}
}
