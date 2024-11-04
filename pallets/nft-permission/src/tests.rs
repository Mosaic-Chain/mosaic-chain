use sdk::{frame_support, frame_system, pallet_nfts, sp_runtime};

use frame_support::{assert_noop, assert_ok};
use frame_system::RawOrigin;
use sp_runtime::{DispatchError, Perbill};

use pallet_nfts::Error as NftsError;

use crate::{mock::*, Error, Event};
use utils::traits::NftStaking;

type AccountIdOf<Test> = <Test as frame_system::Config>::AccountId;

#[test]
fn mint_permission_works() {
	new_test_ext().execute_with(|| {
		let permission = "permission".into();
		let nominal_value = 42;
		let owner = account(1);
		let item_id = 0;

		assert_ok!(NftPermission::mint(&owner, &permission, &nominal_value), item_id);

		assert_ok!(NftPermission::permission_of(&item_id), permission);
		assert_ok!(NftPermission::nominal_value(&item_id), nominal_value);
		assert_ok!(NftPermission::issued_nominal_value(&item_id), nominal_value);
		assert_ok!(NftPermission::owner(&item_id), owner.clone());

		System::assert_last_event(Event::TokenCreated { account: owner, item_id }.into());
	});
}

#[test]
fn mint_id_increments() {
	new_test_ext().execute_with(|| {
		let item1 =
			NftPermission::mint(&account(1), &"permission".into(), &42).expect("could mint nft");
		let item2 =
			NftPermission::mint(&account(2), &"permission".into(), &420).expect("could mint nft");
		assert!(item2 > item1);
	});
}

#[test]
fn mint_ext_requires_privileged_origin() {
	new_test_ext().execute_with(|| {
		// A simple signed origin is not privileged in the mock configuration
		let res = NftPermission::mint_permission_token(
			RuntimeOrigin::signed(account(1)),
			account(2),
			"permission".into(),
			42,
		);

		assert_noop!(res, DispatchError::BadOrigin);

		// The root origin is privileged
		let res = NftPermission::mint_permission_token(
			RuntimeOrigin::root(),
			account(2),
			"permission".into(),
			42,
		);

		assert_ok!(res);
	});
}

#[test]
fn set_nominal_value_works() {
	new_test_ext().execute_with(|| {
		let item =
			NftPermission::mint(&account(1), &"permission".into(), &100).expect("could mint nft");
		NftPermission::bind(&account(1), &item).expect("could bind nft");

		// `set_nominal_value_of_bound` internally calls `set_nominal_value`
		assert_ok!(NftPermission::set_nominal_value_of_bound(&account(1), 90));

		assert_ok!(NftPermission::issued_nominal_value(&item), 100);
		assert_ok!(NftPermission::nominal_value_of(&item), 90);
		assert_ok!(NftPermission::nominal_factor_of_bound(&account(1)), Perbill::from_percent(90));
	});
}

#[test]
fn set_nominal_value_higher_than_issued() {
	new_test_ext().execute_with(|| {
		let item =
			NftPermission::mint(&account(1), &"permission".into(), &100).expect("could mint nft");

		assert_noop!(
			NftPermission::set_nominal_value(&item, 101),
			Error::<Test>::InvalidNominalValue
		);
	});
}

#[test]
fn set_nominal_value_back_to_issued() {
	new_test_ext().execute_with(|| {
		let item =
			NftPermission::mint(&account(1), &"permission".into(), &100).expect("could mint nft");

		NftPermission::set_nominal_value(&item, 90).expect("could set nominal value");

		assert_ok!(NftPermission::set_nominal_value(&item, 100));
	});
}

#[test]
fn set_nominal_value_of_bound_not_bound() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		NftPermission::mint(&owner, &"permission".into(), &100).expect("could mint nft");

		assert_noop!(
			NftPermission::set_nominal_value_of_bound(&owner, 90),
			Error::<Test>::NotBound
		);
	});
}

#[test]
fn bind_works() {
	new_test_ext().execute_with(|| {
		let permission = "permission".into();
		let nominal_value = 42;
		let owner = account(1);
		let item = NftPermission::mint(&owner, &permission, &nominal_value).unwrap();

		assert_ok!(NftPermission::bind(&owner, &item), (permission, nominal_value));

		System::assert_last_event(Event::TokenBound { item_id: item }.into());
	});
}

#[test]
fn bind_wrong_owner() {
	new_test_ext().execute_with(|| {
		let item =
			NftPermission::mint(&account(1), &"permission".into(), &42).expect("could mint nft");
		assert_noop!(NftPermission::bind(&account(2), &item), Error::<Test>::WrongOwner);
	});
}

#[test]
fn bind_item_does_not_exist() {
	new_test_ext().execute_with(|| {
		assert_noop!(NftPermission::bind(&account(2), &42), Error::<Test>::WrongOwner);
	});
}

#[test]
fn bind_item_already_bound() {
	new_test_ext().execute_with(|| {
		let item =
			NftPermission::mint(&account(1), &"permission".into(), &42).expect("could mint nft");
		assert_ok!(NftPermission::bind(&account(1), &item));
		assert_noop!(NftPermission::bind(&account(1), &item), Error::<Test>::AlreadyBound);
	});
}

#[test]
fn bind_an_other_item_is_bound() {
	new_test_ext().execute_with(|| {
		let item1 =
			NftPermission::mint(&account(1), &"permission".into(), &42).expect("could mint nft");
		let item2 =
			NftPermission::mint(&account(1), &"permission".into(), &42).expect("could mint nft");

		assert_ok!(NftPermission::bind(&account(1), &item1));
		assert_noop!(NftPermission::bind(&account(1), &item2), Error::<Test>::AlreadyBound);
	});
}

#[test]
fn bind_transfer_is_locked() {
	new_test_ext().execute_with(|| {
		let owner = account(1);

		let item = NftPermission::mint(&owner, &"permission".into(), &42).expect("could mint nft");
		assert_ok!(NftPermission::bind(&owner, &item));

		assert_noop!(
			Nfts::transfer(
				RawOrigin::Signed(owner).into(),
				NftPermission::collection_id().unwrap(),
				item,
				account(2),
			),
			NftsError::<Test>::ItemLocked
		);
	});
}

#[test]
fn unbind_works() {
	new_test_ext().execute_with(|| {
		let nominal_value = 42;
		let owner = account(1);
		let item = NftPermission::mint(&owner, &"permission".into(), &nominal_value)
			.expect("could mint nft");

		NftPermission::bind(&owner, &item).expect("could bind nft");

		assert_ok!(NftPermission::unbind(&owner), nominal_value);

		System::assert_last_event(Event::TokenUnbound { item_id: item }.into());
	});
}

#[test]
fn unbind_not_bound() {
	new_test_ext().execute_with(|| {
		assert_noop!(NftPermission::unbind(&account(1)), Error::<Test>::NotBound);
	});
}

#[test]
fn unbind_transfer_is_unlocked() {
	new_test_ext().execute_with(|| {
		let owner1 = account(1);
		let owner2 = account(2);

		let item = NftPermission::mint(&owner1, &"permission".into(), &42).expect("could mint nft");
		NftPermission::bind(&owner1, &item).expect("could bind nft");
		NftPermission::unbind(&owner1).expect("could unbind nft");

		assert_ok!(Nfts::transfer(
			RawOrigin::Signed(owner1).into(),
			NftPermission::collection_id().unwrap(),
			item,
			owner2.clone(),
		));

		// Check whether unbind cleaned up everything correctly for a rebind
		assert_ok!(NftPermission::bind(&owner2, &item));
	});
}
