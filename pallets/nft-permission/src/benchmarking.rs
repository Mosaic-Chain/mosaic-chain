//! Benchmarking setup for the permission NFT pallet

use super::*;
//use crate::Pallet as NftPermission;

use sdk::frame_benchmarking::v2::*;
use sdk::frame_system::RawOrigin;

use pallet_nft_staking::PermissionType;

#[benchmarks(
	where T::ItemId: Incrementable, T::Permission: From<PermissionType>, T::Balance: From<u32>
)]
mod benchmarks {
	use super::*;

	#[benchmark]
	fn mint_permission_token() {
		let origin = RawOrigin::<T::AccountId>::Root;
		let account_id: T::AccountId = whitelisted_caller();
		let permission: T::Permission = PermissionType::DPoS.into();
		let nominal_value: T::Balance = 100u32.into();

		#[extrinsic_call]
		_(origin, account_id, permission, nominal_value);
	}
}
