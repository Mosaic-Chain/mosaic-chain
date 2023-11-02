use frame_support::{
	assert_err, assert_ok,
	traits::{OnFinalize, OnInitialize},
};
use frame_system::RawOrigin;
use pallet_nft_staking::NftDelegation as TNftDelegation;
use sp_runtime::Perbill;

use crate::{mock::*, Error, Event};

type AccountIdOf<Test> = <Test as frame_system::Config>::AccountId;

fn account(id: u8) -> AccountId {
	[id; 32].into()
}

#[test]
fn mint_delegator_token_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let expiration = 3;
		let nominal_value = 100;
		let item_id = <Test as utils::traits::Successor<u32>>::initial();

		assert_ok!(
			NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value),
			item_id
		);

		System::assert_last_event(
			Event::TokenCreated {
				account: owner,
				item_id: <Test as utils::traits::Successor<u32>>::initial(),
			}
			.into(),
		);

		assert_ok!(NftDelegation::expiration_of(&item_id), 3);
		assert_ok!(NftDelegation::nominal_value_of(&item_id), 100);
	});
}

#[test]
fn bind_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let validator = account(2);
		let expiration = 4;
		let nominal_value = 42;
		let item_id =
			NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value).unwrap();

		assert_err!(
			NftDelegation::bind(&account(2), &validator, &item_id),
			Error::<Test>::WrongOwner
		);
		assert_ok!(NftDelegation::bind(&owner, &validator, &item_id), (expiration, nominal_value));

		System::assert_last_event(Event::TokenBound { item_id }.into());

		assert_err!(
			NftDelegation::bind(&owner, &account(3), &item_id),
			Error::<Test>::AlreadyBound
		);

		Nfts::transfer(
			RawOrigin::Signed(owner).into(),
			NftDelegation::collection_id().unwrap(),
			item_id,
			account(4),
		)
		.unwrap_err();
	});
}

#[test]
fn unbind_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let validator = account(2);
		let expiration = 3;
		let nominal_value = 42;
		let item_id =
			NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value).unwrap();

		assert_err!(NftDelegation::unbind(&owner, &validator, &item_id), Error::<Test>::NotBound);
		assert_ok!(NftDelegation::bind(&owner, &validator, &item_id), (expiration, nominal_value));
		assert_err!(NftDelegation::unbind(&owner, &account(3), &item_id), Error::<Test>::NotBound);
		assert_ok!(NftDelegation::unbind(&owner, &validator, &item_id), nominal_value);

		System::assert_last_event(Event::TokenUnbound { item_id }.into());

		assert_ok!(Nfts::transfer(
			RawOrigin::Signed(owner).into(),
			NftDelegation::collection_id().unwrap(),
			item_id,
			account(4),
		));
	});
}

#[test]
fn slash_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let validator = account(2);
		let expiration = 4;
		let nominal_value = 100;
		let item1 =
			NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value).unwrap();
		let item2 =
			NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value).unwrap();

		assert_err!(
			NftDelegation::slash(&validator, &owner, Perbill::from_percent(16)),
			Error::<Test>::NotBound
		);
		assert_ok!(NftDelegation::bind(&owner, &validator, &item1), (expiration, nominal_value));
		assert_ok!(NftDelegation::bind(&owner, &validator, &item2), (expiration, nominal_value));
		assert_ok!(NftDelegation::slash(&validator, &owner, Perbill::from_percent(16)));

		System::assert_has_event(Event::TokenSlashed { item_id: item1, nominal_value: 84 }.into());
		System::assert_has_event(Event::TokenSlashed { item_id: item2, nominal_value: 84 }.into());
	});
}

#[test]
fn expiration_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let nominal_value = 100;

		for i in 1..11u32 {
			NftDelegation::do_mint_delegator_token(&owner, i, &nominal_value)
				.expect("could create token");
		}

		// Each block is a session in this case.
		run_to_block(11, |n| {
			System::assert_has_event(Event::TokensExpired { items: [n - 1].to_vec() }.into());
		});

		for i in 0..10u32 {
			let expired = ExpirationHandler::expired_on(i + 1);

			assert_eq!(expired, Some(vec![i]));
		}
	});
}

// Testing block production, for reference see:
// https://web.archive.org/web/20230129131011/https://docs.substrate.io/test/unit-testing/#block-production
fn run_to_block(n: u64, on_new: impl Fn(u32)) {
	while System::block_number() < n {
		if System::block_number() > 0 {
			Session::on_finalize(System::block_number());
			System::on_finalize(System::block_number());
		}

		System::reset_events();
		System::set_block_number(System::block_number() + 1);
		System::on_initialize(System::block_number());
		Session::on_initialize(System::block_number());

		on_new(Session::current_index());
	}
}
