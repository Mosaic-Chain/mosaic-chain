use super::*;

use sdk::{frame_benchmarking, frame_system, sp_application_crypto, sp_core};

use frame_benchmarking::v2::*;
use frame_system::RawOrigin;
use sp_application_crypto::RuntimeAppPublic;
use sp_core::crypto::DEV_PHRASE;
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

fn create_key() -> mint_crypto::MintId {
	MintId::generate_pair(Some(DEV_PHRASE.into()))
}

fn create_package<T>(delegator_nft_count: u32) -> (PackageOf<T>, sr25519::Signature)
where
	T: Config,
	T::Balance: From<u128>,
	T::AccountId: From<sr25519::Public>,
	T::PermissionType: From<PermissionType>,
{
	let authority_key = create_key();
	MintingAuthority::<T>::put(sr25519::Public::from(authority_key.clone()));

	let nonce = 0u64;
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
	});
	let delegator_nfts = (0..delegator_nft_count)
		.map(|i| DelegatorNft { expiration: i, nominal_value: 50u128.into() })
		.collect::<Vec<_>>();
	let delegator_nfts = BoundedVec::truncate_from(delegator_nfts);

	let package =
		PackageOf::<T> { nonce, account_id, balance, vesting, permission_nft, delegator_nfts };
	let package_bytes = package.encode();

	let signature = authority_key.sign(&package_bytes).expect("Should be able to sign it");
	(package, signature.into())
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
		let key = create_key();

		#[extrinsic_call]
		_(RawOrigin::Root, key.into())
	}

	#[benchmark()]
	fn airdrop(d: Linear<0, { T::MAX_DELEGATOR_NFTS }>) {
		let (package, signature) = create_package(d);

		let origin = RawOrigin::None;

		#[extrinsic_call]
		Pallet::<T>::airdrop(origin, package, signature)
	}

	#[benchmark()]
	fn validate_unsigned(d: Linear<0, { T::MAX_DELEGATOR_NFTS }>) {
		let (package, signature) = create_package(d);

		let call = Call::airdrop { package, signature };

		#[block]
		{
			Pallet::<T>::validate_unsigned(TransactionSource::InBlock, &call)
				.map_err(<&str>::from)
				.unwrap();
		}
	}
	// impl_benchmark_test_suite!(Airdrop, crate::mock::new_test_ext(), crate::mock::Runtime);
}
