//! Benchmarking setup for the delegation NFT pallet

use super::*;
//use crate::Pallet as NftDelegation;

use sdk::frame_benchmarking::v2::*;
use sdk::frame_system::RawOrigin;

#[benchmarks(
	where T::ItemId: Incrementable, T::Balance: From<u32>
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
}
