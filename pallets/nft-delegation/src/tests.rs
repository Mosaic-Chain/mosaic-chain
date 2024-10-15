use sdk::{
	frame_support::{
		assert_err, assert_ok,
		traits::{Incrementable, OnFinalize, OnInitialize},
	},
	frame_system::RawOrigin,
	pallet_nfts::Error as NftsError,
};

use crate::{mock::*, Error, Event, Status};
use utils::traits::NftDelegation as TNftDelegation; // This alias is needed to distingish between the runtime definition and the utils trait

type AccountIdOf<Test> = <Test as sdk::frame_system::Config>::AccountId;

fn account(id: u8) -> AccountId {
	[id; 32].into()
}

#[test]
fn mint_delegator_token_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let expiration = 3;
		let nominal_value = 100;
		let item_id =
			<<Test as sdk::pallet_nfts::Config>::ItemId as Incrementable>::initial_value().unwrap();

		assert_ok!(
			NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value),
			item_id
		);

		System::assert_last_event(Event::TokenCreated { account: owner, item_id }.into());

		assert_ok!(NftDelegation::status_of(&item_id), Status::Inactive { expiration });
		assert_ok!(NftDelegation::nominal_value_of(&item_id), nominal_value);
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
			NftDelegation::bind(&account(2), &item_id, validator.clone()),
			Error::<Test>::WrongOwner
		);
		assert_ok!(
			NftDelegation::bind(&owner, &item_id, validator.clone()),
			(expiration, nominal_value)
		);

		System::assert_last_event(Event::TokenBound { item_id }.into());

		assert_err!(NftDelegation::bind(&owner, &item_id, validator), Error::<Test>::AlreadyBound);
		assert_err!(NftDelegation::bind(&owner, &item_id, account(3)), Error::<Test>::AlreadyBound);
		assert_err!(
			Nfts::transfer(
				RawOrigin::Signed(owner.clone()).into(),
				NftDelegation::collection_id().unwrap(),
				item_id,
				account(4),
			),
			NftsError::<Test>::ItemLocked
		);
	});
}

#[test]
fn unbind_should_work() {
	new_test_ext().execute_with(|| {
		let owner1 = account(1);
		let owner2 = account(4);
		let validator = account(2);
		let expiration = 3;
		let nominal_value = 42;
		let item_id =
			NftDelegation::do_mint_delegator_token(&owner1, expiration, &nominal_value).unwrap();

		assert_err!(NftDelegation::unbind(&validator, &item_id), Error::<Test>::WrongOwner);

		assert_err!(NftDelegation::unbind(&owner1, &item_id), Error::<Test>::NotBound);

		NftDelegation::bind(&owner1, &item_id, validator.clone()).unwrap();

		assert_ok!(NftDelegation::unbind(&owner1, &item_id), (nominal_value, validator.clone()));

		System::assert_last_event(Event::TokenUnbound { item_id }.into());

		assert_ok!(
			Nfts::transfer(
				RawOrigin::Signed(owner1.clone()).into(),
				NftDelegation::collection_id().unwrap(),
				item_id,
				owner2.clone(),
			),
			()
		);

		// Check whether unbind cleaned up everything correctly for a rebind
		assert_ok!(NftDelegation::bind(&owner2, &item_id, validator), (expiration, nominal_value));
	});
}

#[test]
fn expiration_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let nominal_value = 100;

		for i in 1..11 {
			let item_id =
				NftDelegation::do_mint_delegator_token(&owner, i, &nominal_value).unwrap();

			NftDelegation::bind(&owner, &item_id, account(42)).unwrap();
		}

		// Each block is a session in this case.
		run_to_block(11, |n| {
			System::assert_has_event(Event::TokensExpired { items: vec![n - 1] }.into());
			assert_ok!(NftDelegation::status_of(&(n - 1)), Status::Expired { expired_on: n });
		});

		for i in 1..11 {
			let expired = ExpirationHandler::expired_on(i);
			assert_eq!(expired, Some(vec![i - 1]));
		}
	});
}

#[test]
fn status_change_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let nominal_value = 100;

		let item_id = NftDelegation::do_mint_delegator_token(&owner, 10, &nominal_value).unwrap();
		assert_ok!(NftDelegation::status_of(&item_id), Status::Inactive { expiration: 10 });

		// nth session = n+1 block
		run_to_block(11, |_| {});

		NftDelegation::bind(&owner, &item_id, account(42)).unwrap();
		assert_ok!(NftDelegation::status_of(&item_id), Status::Active { expires_on: 20 });

		run_to_block(21, |_| {});

		assert_ok!(NftDelegation::status_of(&item_id), Status::Expired { expired_on: 20 });
	});
}

#[test]
fn expires_even_if_unbound() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let nominal_value = 100;

		let item_id = NftDelegation::do_mint_delegator_token(&owner, 10, &nominal_value).unwrap();

		NftDelegation::bind(&owner, &item_id, account(42)).unwrap();
		assert_ok!(NftDelegation::status_of(&item_id), Status::Active { expires_on: 10 });

		NftDelegation::unbind(&owner, &item_id).unwrap();
		run_to_block(11, |_| {});
		assert_ok!(NftDelegation::status_of(&item_id), Status::Expired { expired_on: 10 });
	});
}

// Testing block production, for reference see:
// https://web.archive.org/web/20230129131011/https://docs.substrate.io/test/unit-testing/#block-production
fn run_to_block(n: u64, on_new: impl Fn(u32)) {
	let mut block_number = System::block_number();

	assert!(
		block_number < n,
		"Fix your test! It does not know that block {n} has already been created."
	);

	loop {
		block_number = System::block_number();

		if block_number >= n {
			break;
		}

		if block_number > 0 {
			Session::on_finalize(block_number);
			System::on_finalize(block_number);
		}

		System::reset_events();
		System::set_block_number(block_number + 1);

		block_number = System::block_number();

		System::on_initialize(block_number);
		Session::on_initialize(block_number);

		on_new(Session::current_index());
	}
}
