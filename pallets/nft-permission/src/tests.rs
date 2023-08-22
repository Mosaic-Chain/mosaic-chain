use crate::{mock::*, Error, Event};
use frame_support::{assert_err, assert_ok};
use frame_system::RawOrigin;

type AccountIdOf<Test> = <Test as frame_system::Config>::AccountId;

fn account(id: u8) -> AccountId {
	[id; 32].into()
}

#[test]
fn mint_permission_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let nominal_value = 42;
		let owner = account(1);

		let item_id =
			NftPermission::do_mint_permission_token(&owner, &permission, &nominal_value).unwrap();

		System::assert_last_event(
			Event::TokenCreated {
				account: owner,
				item_id: <Test as utils::traits::Successor<u32>>::initial(),
			}
			.into(),
		);

		assert_ok!(NftPermission::permission_of(&item_id), permission);
		assert_ok!(NftPermission::nominal_value_of(&item_id), nominal_value);
	});
}

#[test]
fn bind_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let nominal_value = 42;
		let owner = account(1);

		let item1 =
			NftPermission::do_mint_permission_token(&owner, &permission, &nominal_value).unwrap();
		let item2 =
			NftPermission::do_mint_permission_token(&owner, &permission, &nominal_value).unwrap();

		assert_err!(NftPermission::do_bind(&account(2), &item1), Error::<Test>::WrongOwner);

		assert_ok!(NftPermission::do_bind(&owner, &item1), (permission, nominal_value));

		System::assert_last_event(Event::TokenBound { item_id: item1 }.into());

		assert_err!(NftPermission::do_bind(&owner, &item2), Error::<Test>::AlreadyBound);

		Nfts::transfer(
			RawOrigin::Signed(owner).into(),
			NftPermission::collection_id().unwrap(),
			item1,
			account(2),
		)
		.unwrap_err();
	});
}

#[test]
fn unbind_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let nominal_value = 42;
		let owner = account(1);

		let item =
			NftPermission::do_mint_permission_token(&owner, &permission, &nominal_value).unwrap();

		assert_err!(NftPermission::do_unbind(&owner, &0), Error::<Test>::NotBound);

		assert_ok!(NftPermission::do_bind(&owner, &item), (permission, nominal_value));

		assert_err!(NftPermission::do_unbind(&owner, &0), Error::<Test>::NotChilled);

		assert_ok!(NftPermission::do_chill(&owner));

		assert_ok!(NftPermission::do_unbind(&owner, &0));

		System::assert_last_event(Event::TokenUnbound { item_id: item }.into());

		assert_ok!(NftPermission::nominal_value_of(&item), 0);

		assert_ok!(Nfts::transfer(
			RawOrigin::Signed(owner).into(),
			NftPermission::collection_id().unwrap(),
			item,
			account(2),
		));
	});
}

#[test]
fn chill_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let nominal_value = 42;
		let owner = account(1);

		let item =
			NftPermission::do_mint_permission_token(&owner, &permission, &nominal_value).unwrap();

		assert_err!(NftPermission::do_chill(&owner), Error::<Test>::NotBound);

		assert_ok!(NftPermission::do_bind(&owner, &item), (permission, nominal_value));

		assert_ok!(NftPermission::do_chill(&owner));

		System::assert_last_event(Event::TokenChilled { item_id: item }.into());

		assert_err!(NftPermission::do_chill(&owner), Error::<Test>::AlreadyChilled);
	});
}

#[test]
fn unchill_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let nominal_value = 42;
		let owner = account(1);

		let item =
			NftPermission::do_mint_permission_token(&owner, &permission, &nominal_value).unwrap();

		assert_err!(NftPermission::do_unchill(&owner), Error::<Test>::NotBound);

		assert_ok!(NftPermission::do_bind(&owner, &item), (permission, nominal_value));

		assert_ok!(NftPermission::do_chill(&owner));

		assert_ok!(NftPermission::do_unchill(&owner));

		System::assert_last_event(Event::TokenUnchilled { item_id: item }.into());

		assert_err!(NftPermission::do_unchill(&owner), Error::<Test>::NotChilled);
	});
}
