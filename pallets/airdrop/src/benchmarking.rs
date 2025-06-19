use super::*;

use sdk::{frame_benchmarking, frame_system, sp_application_crypto, sp_core};

use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::sp_std::prelude::*;

use pallet_nft_staking::{Config as NftStakingConfig, PermissionType};

pub mod mint_crypto {
	mod app_sr25519 {
		use sdk::sp_application_crypto::{app_crypto, sr25519, KeyTypeId};

		app_crypto!(sr25519, KeyTypeId(*b"mint"));
	}

	pub type MintId = app_sr25519::Public;
}

use mint_crypto::MintId;

const METADATA_LEN: usize = 256;

fn minting_authority_id<T>() -> T::AccountId
where
	T: Config,
	T::AccountId: From<sr25519::Public>,
{
	let pair = MintId::generate_pair(Some("//MintingAuthority".into()));
	let public: sr25519::Public = pair.into();
	public.into()
}

fn create_package<T>(delegator_nft_count: u32) -> PackageOf<T>
where
	T: Config,
	T::Balance: From<u128>,
	T::AccountId: From<sr25519::Public>,
	T::PermissionType: From<PermissionType>,
{
	let account_id = whitelisted_caller();
	let balance = Some(50u128.into());
	let vesting = Some(VestingInfo {
		amount: 50u128.into(),
		unlock_per_block: 1u128.into(),
		start_block: None,
	});
	let permission_nft = Some(PermissionNft {
		permission: PermissionType::DPoS.into(),
		nominal_value: 50u128.into(),
		metadata: Some(Vec::from([0; METADATA_LEN])),
	});
	let delegator_nfts = (0..delegator_nft_count)
		.map(|i| DelegatorNft { expiration: i, nominal_value: 50u128.into(), metadata: None })
		.collect::<Vec<_>>();
	let delegator_nfts = BoundedVec::truncate_from(delegator_nfts);

	PackageOf::<T> { account_id, balance, vesting, permission_nft, delegator_nfts }
}

#[expect(clippy::multiple_bound_locations)]
#[benchmarks(where
    T: NftStakingConfig<Balance = <T as Config>::Balance>,
	<T as NftStakingConfig>::Balance: From<u128>,
    T::AccountId: From<sr25519::Public>,
    T::PermissionType: From<PermissionType>
)]
mod benchmarks {

	use super::*;

	#[benchmark]
	fn rotate_key() {
		let account_id = minting_authority_id::<T>();

		#[extrinsic_call]
		_(RawOrigin::Root, account_id)
	}

	#[benchmark()]
	fn airdrop(d: Linear<0, { T::MAX_DELEGATOR_NFTS }>) {
		let package = create_package(d);

		let origin = RawOrigin::Signed(minting_authority_id::<T>());

		#[extrinsic_call]
		Pallet::<T>::airdrop(origin, package)
	}

	// impl_benchmark_test_suite!(Airdrop, crate::mock::new_test_ext(), crate::mock::Runtime);
}
