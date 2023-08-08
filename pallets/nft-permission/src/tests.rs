use crate::{mock::*, Event};
use frame_support::assert_ok;

type AccountIdOf<Test> = <Test as frame_system::Config>::AccountId;

fn account(id: u8) -> AccountId {
	[id; 32].into()
}

#[test]
fn mint_permission_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let account = account(1);

		assert_ok!(NftPermission::do_mint_permission_token(&account, &permission));
		System::assert_last_event(
			Event::TokenCreated {
				account: account.clone(),
				item_id: <Test as utils::traits::Successor<u32>>::initial(),
			}
			.into(),
		);

		assert_ok!(NftPermission::check_permission(&account, &"InvalidPermission".into()), false);
		assert_ok!(NftPermission::check_permission(&account, &permission), true);
	});
}

#[test]
fn permission_query_should_work() {
	new_test_ext().execute_with(|| {
		let even_permission = "EvenPermission".into();
		let odd_permission = "OddPermission".into();

		for acc in 1..10 {
			let permission = if acc % 2 == 0 { &even_permission } else { &odd_permission };
			assert_ok!(NftPermission::do_mint_permission_token(&account(acc), permission));
		}

		assert_ok!(NftPermission::check_permission(&account(1), &even_permission), false);
		assert_eq!(NftPermission::check_permission(&account(3), &odd_permission), Ok(true));

		assert_ok!(NftPermission::check_permission(&account(42), &even_permission), false);

		assert_ok!(
			NftPermission::permission_holders(&even_permission)
				.map(|v| v.into_iter().collect::<Vec<_>>()),
			vec![account(2), account(4), account(6), account(8)]
		);

		assert_ok!(
			NftPermission::permission_holders(&"InvalidPermission".into()).map(|v| v.is_empty()),
			true
		);
	});
}

#[test]
fn suspend_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let account = account(1);

		let item_id = NftPermission::do_mint_permission_token(&account, &permission).unwrap();

		assert_ok!(NftPermission::check_permission(&account, &permission), true);

		assert_ok!(NftPermission::do_set_suspend(&item_id, true));
		System::assert_last_event(Event::TokenSuspendSet { item_id, suspended: true }.into());
		assert_ok!(NftPermission::check_permission(&account, &permission), false);

		assert_ok!(NftPermission::do_set_suspend(&item_id, false));
		System::assert_last_event(Event::TokenSuspendSet { item_id, suspended: false }.into());
		assert_ok!(NftPermission::check_permission(&account, &permission), true);
	});
}

#[test]
fn suspend_all_should_work() {
	new_test_ext().execute_with(|| {
		let permission = "ValidPermission".into();
		let account = account(1);

		let item_ids: Vec<_> = (0..3)
			.map(|_| NftPermission::do_mint_permission_token(&account, &permission).unwrap())
			.collect();

		assert_ok!(
			NftPermission::permission_holders(&permission)
				.map(|v| v.into_iter().collect::<Vec<_>>()),
			vec![account.clone()]
		);

		assert_ok!(NftPermission::do_set_suspend_all(&account, &permission, true));

		for id in &item_ids {
			System::assert_has_event(
				Event::TokenSuspendSet { item_id: *id, suspended: true }.into(),
			);
		}

		assert_ok!(NftPermission::check_permission(&account, &permission), false);

		assert_ok!(NftPermission::do_set_suspend_all(&account, &permission, false));

		for id in &item_ids {
			System::assert_has_event(
				Event::TokenSuspendSet { item_id: *id, suspended: false }.into(),
			);
		}

		assert_ok!(NftPermission::check_permission(&account, &permission), true);
	});
}

#[test]
fn nft_operations_should_work() {
	new_test_ext().execute_with(|| {
		let alice = account(1);
		let bob = account(2);
		let permission = "ValidPermission".into();
		let collection = NftPermission::collection_id().unwrap();

		let item_id = NftPermission::do_mint_permission_token(&alice, &permission).unwrap();

		assert_ok!(NftPermission::check_permission(&alice, &permission), true);
		assert_ok!(NftPermission::check_permission(&bob, &permission), false);

		assert_ok!(Nfts::do_transfer(collection, item_id, bob.clone(), |_, _| { Ok(()) }));

		assert_ok!(NftPermission::check_permission(&alice, &permission), false);
		assert_ok!(NftPermission::check_permission(&bob, &permission), true);

		assert_ok!(Nfts::do_burn(collection, item_id, |_| { Ok(()) }));

		assert_ok!(NftPermission::permission_holders(&permission).map(|v| v.is_empty()), true);
	});
}
