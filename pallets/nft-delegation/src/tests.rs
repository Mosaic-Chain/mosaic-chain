use frame_support::{
	assert_err, assert_ok,
	traits::{Incrementable, OnFinalize, OnInitialize},
};
use frame_system::RawOrigin;
use pallet_nfts::Error as NftsError;
use sp_runtime::Perbill;

use crate::{mock::*, Error, Event};
use utils::traits::NftDelegation as TNftDelegation; // This alias is needed to distingish between the runtime definition and the utils trait

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
		let item_id =
			<<Test as pallet_nfts::Config>::ItemId as Incrementable>::initial_value().unwrap();

		assert_ok!(
			NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value),
			item_id
		);

		System::assert_last_event(Event::TokenCreated { account: owner, item_id }.into());

		assert_ok!(NftDelegation::expiration_of(&item_id), expiration);
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
			NftDelegation::bind(&account(2), &validator, &item_id),
			Error::<Test>::WrongOwner
		);
		assert_ok!(NftDelegation::bind(&owner, &validator, &item_id), (expiration, nominal_value));

		System::assert_last_event(Event::TokenBound { item_id }.into());

		assert_err!(NftDelegation::bind(&owner, &validator, &item_id), Error::<Test>::AlreadyBound);
		assert_err!(
			NftDelegation::bind(&owner, &account(3), &item_id),
			Error::<Test>::AlreadyBound
		);
		assert_err!(
			Nfts::transfer(
				RawOrigin::Signed(owner.clone()).into(),
				NftDelegation::collection_id().unwrap(),
				item_id,
				account(4),
			),
			NftsError::<Test>::ItemLocked
		);

		for _ in 0..255 {
			let item =
				NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value).unwrap();

			NftDelegation::bind(&owner, &validator, &item).unwrap();
		}

		let item =
			NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value).unwrap();

		assert_err!(
			NftDelegation::bind(&owner, &validator, &item),
			Error::<Test>::ExceededMaxCapacity
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

		assert_err!(
			NftDelegation::unbind(&validator, &validator, &item_id),
			Error::<Test>::WrongOwner
		);

		assert_err!(NftDelegation::unbind(&owner1, &validator, &item_id), Error::<Test>::NotBound);

		NftDelegation::bind(&owner1, &validator, &item_id).unwrap();

		// NOTE: this is semantically incorrect behavior, see FIXME at NftDelegation trait `unbind` implementation in lib.rs and I188
		assert_err!(NftDelegation::unbind(&owner1, &account(3), &item_id), Error::<Test>::NotBound);
		assert_ok!(NftDelegation::unbind(&owner1, &validator, &item_id), nominal_value);

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
		assert_ok!(NftDelegation::bind(&owner2, &validator, &item_id), (expiration, nominal_value));
	});
}

#[test]
fn slash_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let validator = account(2);
		let expiration = 4;
		let nominal_value = 100;
		let slash_proportion = Perbill::from_percent(16);
		let amount = 42;

		assert_err!(
			NftDelegation::slash(&validator, &owner, slash_proportion),
			Error::<Test>::NotBound
		);

		let items = (0..amount)
			.map(|_| {
				let item =
					NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value)
						.unwrap();

				NftDelegation::bind(&owner, &validator, &item).unwrap();

				item
			})
			.collect::<Vec<_>>();

		let slashed_value_per_nft = nominal_value - slash_proportion * nominal_value;

		assert_ok!(
			NftDelegation::slash(&validator, &owner, slash_proportion),
			amount * slashed_value_per_nft
		);

		for item in items {
			System::assert_has_event(
				Event::TokenSlashed { item_id: item, nominal_value: slashed_value_per_nft }.into(),
			);
		}
	});
}

#[test]
fn kick_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let validator = account(2);
		let expiration = 4;
		let nominal_value = 42;
		let item_id =
			NftDelegation::do_mint_delegator_token(&owner, expiration, &nominal_value).unwrap();

		assert_err!(NftDelegation::kick(&validator, &owner), Error::<Test>::NotBound);

		NftDelegation::bind(&owner, &validator, &item_id).unwrap();

		assert_ok!(NftDelegation::kick(&validator, &owner), nominal_value);
	});
}

#[test]
fn expiration_should_work() {
	new_test_ext().execute_with(|| {
		let owner = account(1);
		let nominal_value = 100;

		for i in 1..11 {
			NftDelegation::do_mint_delegator_token(&owner, i, &nominal_value).unwrap();
		}

		// Each block is a session in this case.
		run_to_block(11, |n| {
			System::assert_has_event(Event::TokensExpired { items: vec![n - 1] }.into());
		});

		for i in 1..11 {
			let expired = ExpirationHandler::expired_on(i);

			assert_eq!(expired, Some(vec![i - 1]));
		}
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
