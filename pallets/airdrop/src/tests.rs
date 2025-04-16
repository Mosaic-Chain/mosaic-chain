use sdk::{
	frame_support::{assert_noop, assert_ok},
	frame_system::Config as SystemConfig,
	sp_application_crypto::Pair,
	sp_runtime::DispatchError,
};

use super::*;
use mock::*;

fn empty_package() -> PackageOf<Test> {
	Package {
		account_id: account(0),
		balance: None,
		vesting: None,
		permission_nft: None,
		delegator_nfts: BoundedVec::truncate_from(vec![]),
	}
}

fn prefund_account_with_ed(account: &AccountId) {
	MintLog::mint_into(account, MintLog::minimum_balance()).expect("could mint tokens");
}

fn signed_origin_of(pair: sr25519::Pair) -> <Test as SystemConfig>::RuntimeOrigin {
	let public_key: sr25519::Public = pair.public();
	let account_id: <Test as SystemConfig>::AccountId = public_key.into();
	RuntimeOrigin::signed(account_id)
}

fn signed_origin_of_minting_authority() -> <Test as SystemConfig>::RuntimeOrigin {
	signed_origin_of(mock::minting_authority())
}

#[test]
fn cannot_be_called_by_root() {
	new_test_ext().execute_with(|| {
		let package = empty_package();

		assert_noop!(Airdrop::airdrop(RuntimeOrigin::root(), package), DispatchError::BadOrigin);
	});
}

#[test]
fn deposits_event() {
	new_test_ext().execute_with(|| {
		let package = empty_package();
		let account_id = package.account_id.clone();

		Airdrop::airdrop(signed_origin_of_minting_authority(), package)
			.expect("could airdrop package");
		System::assert_last_event(Event::<Test>::MintedPackage { account_id }.into());
	});
}

#[test]
fn mints_tokens() {
	new_test_ext().execute_with(|| {
		let mut package = empty_package();
		package.balance = Some(42);

		Airdrop::airdrop(signed_origin_of_minting_authority(), package)
			.expect("could airdrop package");

		MintLog::assert_last_entry(&Entry::TokensMinted { account: account(0), amount: 42 });
		MintLog::assert_height(1);
	});
}

#[test]
fn mints_at_least_ed_on_empty_account() {
	new_test_ext().execute_with(|| {
		let package = empty_package();
		let account_id = package.account_id.clone();

		assert_eq!(MintLog::total_balance(&account_id), 0);
		assert_eq!(package.balance, None);

		Airdrop::airdrop(signed_origin_of_minting_authority(), package)
			.expect("could airdrop package");

		MintLog::assert_last_entry(&Entry::TokensMinted {
			account: account_id,
			amount: MintLog::minimum_balance(),
		});
		MintLog::assert_height(1);
	});
}

#[test]
fn does_not_mint_extra_if_account_not_empty() {
	new_test_ext().execute_with(|| {
		let package = empty_package();
		let account_id = package.account_id.clone();

		prefund_account_with_ed(&account_id);
		let log_height = MintLog::height();

		Airdrop::airdrop(signed_origin_of_minting_authority(), package)
			.expect("could airdrop package");
		MintLog::assert_height(log_height); // No assets were minted
	});
}

#[test]
fn mints_delegator_nfts() {
	new_test_ext().execute_with(|| {
		let mut package = empty_package();
		package.delegator_nfts = BoundedVec::truncate_from(vec![
			DelegatorNft { expiration: 1, nominal_value: 100, metadata: None },
			DelegatorNft { expiration: 2, nominal_value: 200, metadata: None },
		]);
		let account = package.account_id.clone();

		prefund_account_with_ed(&account);
		let log_height = MintLog::height();

		Airdrop::airdrop(signed_origin_of_minting_authority(), package)
			.expect("could airdrop package");

		MintLog::assert_has_entry(&Entry::DelegatorNftMinted {
			account: account.clone(),
			expiration: 1,
			nominal_value: 100,
		});

		MintLog::assert_has_entry(&Entry::DelegatorNftMinted {
			account: account.clone(),
			expiration: 2,
			nominal_value: 200,
		});

		MintLog::assert_height(log_height + 2);
	});
}

#[test]
fn sets_delegator_nft_metadata() {
	new_test_ext().execute_with(|| {
		let mut package = empty_package();
		package.delegator_nfts = BoundedVec::truncate_from(vec![DelegatorNft {
			expiration: 1,
			nominal_value: 100,
			metadata: Some(b"nft #1".to_vec()),
		}]);

		Airdrop::airdrop(signed_origin_of_minting_authority(), package)
			.expect("could airdrop package");

		MintLog::assert_has_entry(&Entry::NftMetadataSet {
			item_id: 0,
			metadata: b"nft #1".to_vec(),
		});
	});
}

#[test]
fn mints_permission_nfts() {
	new_test_ext().execute_with(|| {
		let mut package = empty_package();
		package.permission_nft = Some(PermissionNft {
			permission: Permission::DPoS,
			nominal_value: 100,
			metadata: None,
		});
		let account = package.account_id.clone();

		prefund_account_with_ed(&account);
		let log_height = MintLog::height();

		Airdrop::airdrop(signed_origin_of_minting_authority(), package)
			.expect("could airdrop package");

		MintLog::assert_has_entry(&Entry::PermissionNftMinted {
			account: account.clone(),
			nominal_value: 100,
			permission: Permission::DPoS,
		});

		MintLog::assert_height(log_height + 1);
	});
}

#[test]
fn sets_permission_nft_metadata() {
	new_test_ext().execute_with(|| {
		let mut package = empty_package();
		package.permission_nft = Some(PermissionNft {
			permission: Permission::DPoS,
			nominal_value: 100,
			metadata: Some(b"validator #1".to_vec()),
		});

		Airdrop::airdrop(signed_origin_of_minting_authority(), package)
			.expect("could airdrop package");

		MintLog::assert_has_entry(&Entry::NftMetadataSet {
			item_id: 0,
			metadata: b"validator #1".to_vec(),
		});
	});
}

#[test]
fn adds_vesting_schedule() {
	new_test_ext().execute_with(|| {
		let mut package = empty_package();
		package.vesting = Some(VestingInfo { amount: 101, unlock_per_block: 1, start_block: None });
		let account = package.account_id.clone();

		prefund_account_with_ed(&account);
		let log_height = MintLog::height();

		Airdrop::airdrop(signed_origin_of_minting_authority(), package)
			.expect("could airdrop package");

		// Mints tokens to vest
		MintLog::assert_has_entry(&Entry::TokensMinted { account: account.clone(), amount: 101 });

		// Vests tokens
		MintLog::assert_has_entry(&Entry::VestingScheduleAdded {
			account: account.clone(),
			schedule: VestingSchedule { locked: 101, per_block: 1, starting_block: None },
		});

		MintLog::assert_height(log_height + 2);
	});
}

#[test]
fn refuses_invalid_vesting_schedule() {
	new_test_ext().execute_with(|| {
		let mut package = empty_package();
		package.vesting = Some(VestingInfo { amount: 101, unlock_per_block: 0, start_block: None });

		assert_noop!(
			Airdrop::airdrop(signed_origin_of_minting_authority(), package),
			mint_log::INVALID_SCHEDULE_ERROR
		);
	});
}

#[test]
fn refuses_signature_not_by_authority() {
	new_test_ext().execute_with(|| {
		let pair = sr25519::Pair::from_string("//EvilGenius", None).expect("valid seed");
		let package = empty_package();

		assert_noop!(
			Airdrop::airdrop(signed_origin_of(pair), package),
			Error::<Test>::InvalidSignature
		);
	});
}

#[test]
fn rotate_key_works() {
	new_test_ext().execute_with(|| {
		let account_id: AccountId =
			sr25519::Pair::from_string("//GoodGuy", None).unwrap().public().into();
		assert_ok!(Airdrop::rotate_key(RuntimeOrigin::root(), account_id.clone()));

		System::assert_last_event(Event::<Test>::KeyRotated { new_account_id: account_id }.into());
	});
}

#[test]
fn rotate_key_ensures_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::rotate_key(
				RuntimeOrigin::signed(account(0)),
				sr25519::Pair::from_string("//BadGuy", None).unwrap().public().into()
			),
			DispatchError::BadOrigin
		);
	});
}
