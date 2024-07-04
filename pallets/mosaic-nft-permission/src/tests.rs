use frame_support::{assert_err, assert_ok, traits::Incrementable};
use frame_system::RawOrigin;
use pallet_nfts::Error as NftsError;

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
			<<Test as pallet_nfts::Config>::ItemId as Incrementable>::initial_value().unwrap();

		assert_ok!(
			NftPermission::do_mint_permission_token(&owner, &permission, &nominal_value),
			item_id
		);

		System::assert_last_event(Event::TokenCreated { account: owner, item_id }.into());

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

		assert_err!(
			Nfts::transfer(
				RawOrigin::Signed(owner).into(),
				NftPermission::collection_id().unwrap(),
				item1,
				account(2),
			),
			NftsError::<Test>::ItemLocked
		);
	});
}

#[test]
fn unbind_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let nominal_value = 42;
		let owner1 = account(1);
		let owner2 = account(2);
		let item_id =
			NftPermission::do_mint_permission_token(&owner1, &permission, &nominal_value).unwrap();

		assert_err!(NftPermission::unbind(&owner1), Error::<Test>::NotBound);

		NftPermission::bind(&owner1, &item_id).unwrap();

		assert_ok!(NftPermission::unbind(&owner1), nominal_value);

		System::assert_last_event(Event::TokenUnbound { item_id }.into());

		assert_ok!(
			Nfts::transfer(
				RawOrigin::Signed(owner1).into(),
				NftPermission::collection_id().unwrap(),
				item_id,
				owner2.clone(),
			),
			()
		);

		// Check whether unbind cleaned up everything correctly for a rebind
		assert_ok!(NftPermission::bind(&owner2, &item_id), (permission, nominal_value));
	});
}
