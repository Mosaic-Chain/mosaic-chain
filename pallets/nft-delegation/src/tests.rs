use sdk::{
	frame_support::{assert_noop, assert_ok},
	frame_system::RawOrigin,
	pallet_nfts::Error as NftsError,
	sp_runtime::{BoundedVec, DispatchError},
	sp_std::vec,
};

use utils::{
	run_until::{run_until, Blocks, ToBlock},
	traits::NftDelegation as TNftDelegation,
};

use crate::{mock::*, Error, Event, Status};

#[test]
fn mint_delegator_token_works() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let expiration = 3;
		let nominal_value = 100;
		let item = 0;

		assert_ok!(NftDelegation::mint(&owner, expiration, &nominal_value), item);

		System::assert_last_event(Event::TokenCreated { account: owner, item_id: item }.into());

		assert_ok!(NftDelegation::status_of(&item), Status::Inactive { expiration });
		assert_ok!(NftDelegation::nominal_value_of(&item), nominal_value);
	});
}

#[test]
fn mint_id_increments() {
	new_test_ext().execute_with(|| {
		let item1 = NftDelegation::mint(&account(1), 3, &42).expect("could mint nft");
		let item2 = NftDelegation::mint(&account(2), 3, &42).expect("could mint nft");
		assert!(item2 > item1);
	});
}

#[test]
fn mint_ext_requires_privileged_origin() {
	new_test_ext().execute_with(|| {
		// A simple signed origin is not privileged in the mock configuration
		let res = NftDelegation::mint_delegator_token(
			RuntimeOrigin::signed(account(1)),
			account(2),
			3,
			42,
		);

		assert_noop!(res, DispatchError::BadOrigin);

		// The root origin is privileged
		let res = NftDelegation::mint_delegator_token(RuntimeOrigin::root(), account(2), 3, 42);

		assert_ok!(res);
	});
}

#[test]
fn set_metadata_of_bound_works() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let item = NftDelegation::mint(&owner, 3, &42).expect("could mint nft");
		NftDelegation::bind(&owner, &item, account(2)).expect("could bind nft");

		assert_ok!(NftDelegation::metadata_of_bound(&item), account(2));

		NftDelegation::set_metadata_of_bound(&item, account(3)).expect("could set metadata");

		assert_ok!(NftDelegation::metadata_of_bound(&item), account(3));
	});
}

#[test]
fn set_metadata_of_bound_not_bound() {
	new_test_ext().execute_with(|| {
		let item = NftDelegation::mint(&account(0), 3, &42).expect("could mint nft");
		assert_noop!(
			NftDelegation::set_metadata_of_bound(&item, account(2)),
			Error::<Test>::NotBound
		);
	});
}

#[test]
fn set_nominal_value_works() {
	new_test_ext().execute_with(|| {
		let item = NftDelegation::mint(&account(0), 3, &42).expect("could mint nft");
		NftDelegation::set_nominal_value(&item, 420).expect("could set nominal value");

		assert_ok!(NftDelegation::nominal_value_of(&item), 420);
	});
}

#[test]
fn bind_works() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let validator = account(2);
		let expiration = 4;
		let nominal_value = 42;
		let item = NftDelegation::mint(&owner, expiration, &nominal_value).unwrap();

		assert_ok!(
			NftDelegation::bind(&owner, &item, validator.clone()),
			(expiration, nominal_value)
		);

		assert_ok!(NftDelegation::metadata_of_bound(&item), validator);

		System::assert_last_event(Event::TokenBound { item_id: item }.into());
	});
}

#[test]
fn bind_wrong_owner() {
	new_test_ext().execute_with(|| {
		let item = NftDelegation::mint(&account(1), 3, &42).expect("could mint nft");
		assert_noop!(
			NftDelegation::bind(&account(2), &item, account(2)),
			Error::<Test>::WrongOwner
		);
	});
}

#[test]
fn bind_item_does_not_exist() {
	new_test_ext().execute_with(|| {
		assert_noop!(NftDelegation::bind(&account(1), &42, account(2)), Error::<Test>::WrongOwner);
	});
}

#[test]
fn bind_already_bound() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let item = NftDelegation::mint(&owner, 3, &42).expect("could mint nft");
		NftDelegation::bind(&owner, &item, account(2)).expect("could bind nft");

		assert_noop!(
			NftDelegation::bind(&account(1), &item, account(2)),
			Error::<Test>::AlreadyBound
		);
	});
}

#[test]
fn bind_transfer_is_locked() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let item = NftDelegation::mint(&owner, 3, &42).expect("could mint nft");

		NftDelegation::bind(&owner, &item, account(2)).expect("could bind nft");

		assert_noop!(
			Nfts::transfer(
				RuntimeOrigin::signed(owner.clone()),
				NftDelegation::collection_id().unwrap(),
				item,
				account(3),
			),
			NftsError::<Test>::ItemLocked
		);
	});
}

#[test]
fn unbind_works() {
	new_test_ext().execute_with(|| {
		let owner1 = account(1);
		let owner2 = account(4);
		let validator = account(2);
		let expiration = 3;
		let nominal_value = 42;
		let item = NftDelegation::mint(&owner1, expiration, &nominal_value).unwrap();

		assert_noop!(NftDelegation::unbind(&validator, &item), Error::<Test>::WrongOwner);

		assert_noop!(NftDelegation::unbind(&owner1, &item), Error::<Test>::NotBound);

		NftDelegation::bind(&owner1, &item, validator.clone()).unwrap();

		assert_ok!(NftDelegation::unbind(&owner1, &item), (nominal_value, validator.clone()));

		System::assert_last_event(Event::TokenUnbound { item_id: item }.into());

		assert_ok!(
			Nfts::transfer(
				RawOrigin::Signed(owner1.clone()).into(),
				NftDelegation::collection_id().unwrap(),
				item,
				owner2.clone(),
			),
			()
		);

		// Check whether unbind cleaned up everything correctly for a rebind
		assert_ok!(NftDelegation::bind(&owner2, &item, validator), (expiration, nominal_value));
	});
}

#[test]
fn unbind_not_bound() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let item = NftDelegation::mint(&owner, 3, &42).expect("could mint nft");

		assert_noop!(NftDelegation::unbind(&owner, &item), Error::<Test>::NotBound);
	});
}

#[test]
fn unbind_wrong_owner() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let item = NftDelegation::mint(&owner, 3, &42).expect("could mint nft");
		NftDelegation::bind(&owner, &item, account(2)).expect("could bind nft");

		assert_noop!(NftDelegation::unbind(&account(3), &item), Error::<Test>::WrongOwner);
	});
}

#[test]
fn unbind_transfer_is_unlocked() {
	new_test_ext().execute_with(|| {
		let owner1 = account(1);
		let owner2 = account(2);

		let item = NftDelegation::mint(&owner1, 3, &42).expect("could mint nft");
		NftDelegation::bind(&owner1, &item, account(3)).expect("could bind nft");
		NftDelegation::unbind(&owner1, &item).expect("could unbind nft");

		assert_ok!(Nfts::transfer(
			RawOrigin::Signed(owner1).into(),
			NftDelegation::collection_id().unwrap(),
			item,
			owner2.clone(),
		));

		// Check whether unbind cleaned up everything correctly for a rebind
		assert_ok!(NftDelegation::bind(&owner2, &item, account(4)));
	});
}

#[test]
fn expiration_works() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let nominal_value = 100;

		for i in 1..11 {
			let item = NftDelegation::mint(&owner, i, &nominal_value).unwrap();

			NftDelegation::bind(&owner, &item, account(42)).unwrap();
		}

		run_until::<AllPalletsWithoutSystem, Test>(|| {
			let session = Session::current_index();
			if session >= 11 {
				return false;
			}

			if session > 0 {
				System::assert_has_event(
					Event::TokensExpired { items: BoundedVec::truncate_from(vec![session - 1]) }
						.into(),
				);
				assert_ok!(
					NftDelegation::status_of(&(session - 1)),
					Status::Expired { expired_on: session }
				);
			}

			true
		});

		for i in 1..11 {
			let expired = ExpirationHandler::expired_on(i);
			assert_eq!(expired, Some(vec![i - 1]));
		}
	});
}

#[test]
fn status_change_works() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let nominal_value = 100;

		let item = NftDelegation::mint(&owner, 10, &nominal_value).unwrap();
		assert_ok!(NftDelegation::status_of(&item), Status::Inactive { expiration: 10 });

		// nth session = n+1 block
		run_until::<AllPalletsWithoutSystem, Test>(ToBlock(11));

		NftDelegation::bind(&owner, &item, account(42)).unwrap();
		assert_ok!(NftDelegation::status_of(&item), Status::Active { expires_on: 20 });

		run_until::<AllPalletsWithoutSystem, Test>(Blocks(10u32));

		assert_ok!(NftDelegation::status_of(&item), Status::Expired { expired_on: 20 });
	});
}

#[test]
fn expires_even_if_unbound() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let nominal_value = 100;

		let item = NftDelegation::mint(&owner, 10, &nominal_value).unwrap();

		NftDelegation::bind(&owner, &item, account(42)).unwrap();
		assert_ok!(NftDelegation::status_of(&item), Status::Active { expires_on: 10 });

		NftDelegation::unbind(&owner, &item).unwrap();
		run_until::<AllPalletsWithoutSystem, Test>(Blocks(11u32));
		assert_ok!(NftDelegation::status_of(&item), Status::Expired { expired_on: 10 });
	});
}
