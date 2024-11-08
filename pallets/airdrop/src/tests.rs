use frame_support::{assert_err, assert_noop, assert_ok};
use sp_runtime::DispatchError;

use super::*;
use mock::*;

fn sign_package(package: &PackageOf<Test>) -> sr25519::Signature {
	minting_authority().sign(package.encode().as_slice())
}

fn dummy_package() -> (PackageOf<Test>, sr25519::Signature) {
	let package = Package {
		nonce: 0,
		account_id: account(0),
		balance: None,
		vesting: None,
		permission_nft: None,
		delegator_nfts: None,
	};

	let signature = sign_package(&package);

	(package, signature)
}

fn prefund_account_with_ed(account: &AccountId) {
	MintLog::mint_into(account, MintLog::minimum_balance()).expect("could mint tokens");
}

#[test]
fn cannot_be_called_by_root() {
	new_test_ext().execute_with(|| {
		let (package, signature) = dummy_package();

		assert_noop!(
			Airdrop::airdrop(RuntimeOrigin::root(), package, signature),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn deposits_event() {
	new_test_ext().execute_with(|| {
		let (package, signature) = dummy_package();
		let account_id = package.account_id.clone();

		Airdrop::airdrop(RuntimeOrigin::none(), package, signature).expect("could airdrop package");
		System::assert_last_event(Event::<Test>::MintedPackage { account_id }.into());
	});
}

#[test]
fn mints_tokens() {
	new_test_ext().execute_with(|| {
		let package = Package {
			nonce: 0,
			account_id: account(0),
			balance: Some(42),
			vesting: None,
			permission_nft: None,
			delegator_nfts: None,
		};
		let signature = sign_package(&package);

		Airdrop::airdrop(RuntimeOrigin::none(), package, signature).expect("could airdrop package");
		MintLog::assert_last_entry(&Entry {
			account: account(0),
			event: MintEvent::TokensMinted(42),
		});
		MintLog::assert_height(1);
	});
}

#[test]
fn mints_at_least_ed_on_empty_account() {
	new_test_ext().execute_with(|| {
		let (package, signature) = dummy_package();
		let account_id = package.account_id.clone();

		assert_eq!(MintLog::total_balance(&account_id), 0);
		assert_eq!(package.balance, None);

		Airdrop::airdrop(RuntimeOrigin::none(), package, signature).expect("could airdrop package");
		MintLog::assert_last_entry(&Entry {
			account: account_id,
			event: MintEvent::TokensMinted(MintLog::minimum_balance()),
		});
		MintLog::assert_height(1);
	});
}

#[test]
fn does_not_mint_extra_if_account_not_empty() {
	new_test_ext().execute_with(|| {
		let (package, signature) = dummy_package();
		let account_id = package.account_id.clone();

		prefund_account_with_ed(&account_id);
		let log_height = MintLog::height();

		Airdrop::airdrop(RuntimeOrigin::none(), package, signature).expect("could airdrop package");
		MintLog::assert_height(log_height); // No assets were minted
	});
}

#[test]
fn mints_delegator_nfts() {
	new_test_ext().execute_with(|| {
		let account = account(0);

		let package = Package {
			nonce: 0,
			account_id: account.clone(),
			balance: None,
			vesting: None,
			permission_nft: None,
			delegator_nfts: Some(BoundedVec::truncate_from(vec![
				DelegatorNft { expiration: 1, nominal_value: 100 },
				DelegatorNft { expiration: 2, nominal_value: 200 },
			])),
		};

		let signature = sign_package(&package);

		prefund_account_with_ed(&account);
		let log_height = MintLog::height();

		Airdrop::airdrop(RuntimeOrigin::none(), package, signature).expect("could airdrop package");

		MintLog::assert_has_entry(&Entry {
			account: account.clone(),
			event: MintEvent::DelegatorNftMinted { expiration: 1, nominal_value: 100 },
		});
		MintLog::assert_has_entry(&Entry {
			account: account.clone(),
			event: MintEvent::DelegatorNftMinted { expiration: 2, nominal_value: 200 },
		});

		MintLog::assert_height(log_height + 2);
	});
}

#[test]
fn mints_permission_nfts() {
	new_test_ext().execute_with(|| {
		let account = account(0);

		let package = Package {
			nonce: 0,
			account_id: account.clone(),
			balance: None,
			vesting: None,
			permission_nft: Some(PermissionNft {
				permission: Permission::DPoS,
				nominal_value: 100,
			}),
			delegator_nfts: None,
		};

		let signature = sign_package(&package);

		prefund_account_with_ed(&account);
		let log_height = MintLog::height();

		Airdrop::airdrop(RuntimeOrigin::none(), package, signature).expect("could airdrop package");

		MintLog::assert_has_entry(&Entry {
			account: account.clone(),
			event: MintEvent::PermissionNftMinted {
				nominal_value: 100,
				permission: Permission::DPoS,
			},
		});

		MintLog::assert_height(log_height + 1);
	});
}

#[test]
fn adds_vesting_schedule() {
	new_test_ext().execute_with(|| {
		let account = account(0);

		let package = Package {
			nonce: 0,
			account_id: account.clone(),
			balance: None,
			vesting: Some(VestingInfo { amount: 101, unlock_per_block: 1, start_block: None }),
			permission_nft: None,
			delegator_nfts: None,
		};

		let signature = sign_package(&package);

		prefund_account_with_ed(&account);
		let log_height = MintLog::height();

		Airdrop::airdrop(RuntimeOrigin::none(), package, signature).expect("could airdrop package");

		// Mints tokens to vest
		MintLog::assert_has_entry(&Entry {
			account: account.clone(),
			event: MintEvent::TokensMinted(101),
		});

		// Vests tokens
		MintLog::assert_has_entry(&Entry {
			account: account.clone(),
			event: MintEvent::VestingScheduleAdded(VestingSchedule {
				locked: 101,
				per_block: 1,
				starting_block: None,
			}),
		});

		MintLog::assert_height(log_height + 2);
	});
}

#[test]
fn refuses_invalid_vesting_schedule() {
	new_test_ext().execute_with(|| {
		let account = account(0);

		let package = Package {
			nonce: 0,
			account_id: account.clone(),
			balance: None,
			// `per_block` = 0 => schedule is invalid
			vesting: Some(VestingInfo { amount: 101, unlock_per_block: 0, start_block: None }),
			permission_nft: None,
			delegator_nfts: None,
		};

		let signature = sign_package(&package);
		assert_noop!(
			Airdrop::airdrop(RuntimeOrigin::none(), package, signature),
			mint_log::INVALID_SCHEDULE_ERROR
		);
	});
}

#[test]
fn refuses_signature_not_by_authority() {
	new_test_ext().execute_with(|| {
		let pair = sr25519::Pair::from_string("//EvilGenius", None).expect("valid seed");
		let (package, _) = dummy_package();
		let signature = pair.sign(package.encode().as_slice());

		assert_noop!(
			Airdrop::airdrop(RuntimeOrigin::none(), package, signature),
			Error::<Test>::InvalidSignature
		);
	});
}

#[test]
fn refuses_signature_of_wrong_package() {
	new_test_ext().execute_with(|| {
		let (mut package, signature) = dummy_package();
		package.account_id = account(42);

		assert_noop!(
			Airdrop::airdrop(RuntimeOrigin::none(), package, signature),
			Error::<Test>::InvalidSignature
		);
	});
}

#[test]
fn refuses_bad_nonce() {
	new_test_ext().execute_with(|| {
		Nonce::<Test>::set(2);

		let (mut package, _) = dummy_package();

		// Nonce too low
		package.nonce = 1;
		let signature = sign_package(&package);

		assert_noop!(
			Airdrop::airdrop(RuntimeOrigin::none(), package.clone(), signature),
			Error::<Test>::BadNonce
		);

		// Nonce too high
		package.nonce = 3;
		let signature = sign_package(&package);

		assert_noop!(
			Airdrop::airdrop(RuntimeOrigin::none(), package, signature),
			Error::<Test>::BadNonce
		);
	});
}

#[test]
fn increments_nonce() {
	new_test_ext().execute_with(|| {
		assert_eq!(Nonce::<Test>::get(), 0);

		let (package, signature) = dummy_package();

		Airdrop::airdrop(RuntimeOrigin::none(), package, signature).expect("could airdrop package");

		assert_eq!(Nonce::<Test>::get(), 1);
	});
}

#[test]
fn rotate_key_works() {
	new_test_ext().execute_with(|| {
		let key = sr25519::Pair::from_string("//GoodGuy", None).unwrap().public();
		assert_ok!(Airdrop::rotate_key(RuntimeOrigin::root(), key,));

		System::assert_last_event(Event::<Test>::KeyRotated { new_key: key }.into());
	});
}

#[test]
fn rotate_key_ensures_root() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Airdrop::rotate_key(
				RuntimeOrigin::signed(account(0)),
				sr25519::Pair::from_string("//BadGuy", None).unwrap().public()
			),
			DispatchError::BadOrigin
		);
	});
}

#[test]
fn validate_unsigned_checks_signature() {
	new_test_ext().execute_with(|| {
		let (mut package, signature) = dummy_package();
		package.balance = Some(42); // this invalidates the signature

		assert_err!(
			Airdrop::validate_unsigned(
				TransactionSource::External,
				&Call::<Test>::airdrop { package, signature }
			),
			InvalidTransaction::BadSigner
		);
	});
}

#[test]
fn validate_unsigned_checks_nonce() {
	new_test_ext().execute_with(|| {
		Nonce::<Test>::set(2);
		let (mut package, _) = dummy_package();

		// future nonce
		package.nonce = 4;
		let signature = sign_package(&package);

		assert_err!(
			Airdrop::validate_unsigned(
				TransactionSource::External,
				&Call::<Test>::airdrop { package: package.clone(), signature }
			),
			InvalidTransaction::Future
		);

		// old nonce
		package.nonce = 1;
		let signature = sign_package(&package);

		assert_err!(
			Airdrop::validate_unsigned(
				TransactionSource::External,
				&Call::<Test>::airdrop { package: package.clone(), signature }
			),
			InvalidTransaction::Stale
		);

		// not current, but acceptable nonce (eg.: next)
		package.nonce = 3;
		let signature = sign_package(&package);

		assert_ok!(Airdrop::validate_unsigned(
			TransactionSource::External,
			&Call::<Test>::airdrop { package, signature }
		));
	});
}

#[test]
fn validate_unsigned_current_nonce_metadata() {
	new_test_ext().execute_with(|| {
		let (package, signature) = dummy_package();

		assert_ok!(
			Airdrop::validate_unsigned(
				TransactionSource::External,
				&Call::<Test>::airdrop { package, signature }
			),
			ValidTransaction::with_tag_prefix("Airdrop")
				.longevity(1)
				.and_provides(0u64)
				.priority(2) // base(0) + (nonce_limit(0 + 2) - nonce(0))
				.build()
				.unwrap()
		);
	});
}

#[test]
fn validate_unsigned_upcoming_nonce_metadata() {
	new_test_ext().execute_with(|| {
		let (mut package, _) = dummy_package();

		package.nonce = 1;
		let signature = sign_package(&package);

		assert_ok!(
			Airdrop::validate_unsigned(
				TransactionSource::External,
				&Call::<Test>::airdrop { package, signature }
			),
			ValidTransaction::with_tag_prefix("Airdrop")
				.longevity(1)
				.and_provides(1u64)
				.and_requires(0u64)
				.priority(1) // base(0) + (nonce_limit(0 + 2) - nonce(1))
				.build()
				.unwrap()
		);
	});
}
