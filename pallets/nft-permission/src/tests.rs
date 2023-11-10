use frame_support::{assert_err, assert_ok, traits::Incrementable};
use frame_system::RawOrigin;
use sp_runtime::Perbill;

use crate::{mock::*, Error, Event};
use utils::traits::NftStaking;

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
				item_id: <<Test as pallet_nfts::Config>::ItemId as Incrementable>::initial_value()
					.unwrap(),
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

		assert_err!(NftPermission::bind(&account(2), &item1), Error::<Test>::WrongOwner);
		assert_ok!(NftPermission::bind(&owner, &item1), (permission, nominal_value));

		System::assert_last_event(Event::TokenBound { item_id: item1 }.into());

		assert_err!(NftPermission::bind(&owner, &item2), Error::<Test>::AlreadyBound);

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

		assert_err!(NftPermission::unbind(&owner), Error::<Test>::NotBound);
		assert_ok!(NftPermission::bind(&owner, &item), (permission, nominal_value));
		assert_err!(NftPermission::unbind(&owner), Error::<Test>::NotChilled);
		assert_ok!(NftPermission::chill(&owner));
		assert_ok!(NftPermission::unbind(&owner), nominal_value);

		System::assert_last_event(Event::TokenUnbound { item_id: item }.into());

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

		assert_err!(NftPermission::chill(&owner), Error::<Test>::NotBound);
		assert_ok!(NftPermission::bind(&owner, &item), (permission, nominal_value));
		assert_ok!(NftPermission::chill(&owner));

		System::assert_last_event(Event::TokenChilled { item_id: item }.into());

		assert_err!(NftPermission::chill(&owner), Error::<Test>::AlreadyChilled);
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

		assert_err!(NftPermission::unchill(&owner), Error::<Test>::NotBound);
		assert_ok!(NftPermission::bind(&owner, &item), (permission, nominal_value));
		assert_ok!(NftPermission::chill(&owner));
		assert_ok!(NftPermission::unchill(&owner));

		System::assert_last_event(Event::TokenUnchilled { item_id: item }.into());

		assert_err!(NftPermission::unchill(&owner), Error::<Test>::NotChilled);
	});
}

#[test]
fn slash_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let nominal_value = 100;
		let owner = account(1);
		let item =
			NftPermission::do_mint_permission_token(&owner, &permission, &nominal_value).unwrap();

		assert_err!(
			NftPermission::slash(&owner, Perbill::from_percent(16)),
			Error::<Test>::NotBound
		);
		assert_ok!(NftPermission::bind(&owner, &item), (permission, nominal_value));
		assert_ok!(NftPermission::slash(&owner, Perbill::from_percent(16)));

		System::assert_last_event(Event::TokenSlashed { item_id: item, nominal_value: 84 }.into());
	});
}
