use super::*;

#[rstest]
#[allow(clippy::redundant_pattern_matching)]
fn bind_is_successful(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let alice = 0;
		let origin = origin(alice);

		let nft1 = NftStakingHandler::mint(&alice, &PermissionType::DPoS, &NOMINAL_VALUE)
			.expect("could mint permission nft");

		let res = Staking::bind_validator(origin, nft1);

		assert_ok!(res);

		assert!(Validators::<Test>::get(alice).is_some());

		assert_validator_state!(&alice, Some(ValidatorState::Normal));

		assert_current_validator_stake!(
			&alice,
			Some(TotalValidatorStake { total_stake: NOMINAL_VALUE, contract_count: 1 })
		);

		assert_current_contract!(&alice, &alice, Some(_));

		System::assert_last_event(Event::ValidatorBound(alice).into());
	});
}

#[rstest]
fn nft_does_not_exist(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(0);
		let res = Staking::bind_validator(origin, 42);
		assert_noop!(res, NftStakingHandlerError::TokenDoesNotExist);
	});
}

#[rstest]
fn wrong_owner(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let alice = origin(0);
		let bob = 1;

		let nft = NftStakingHandler::mint(&bob, &PermissionType::DPoS, &NOMINAL_VALUE)
			.expect("could mint permission nft");

		let res = Staking::bind_validator(alice, nft);
		assert_noop!(res, NftStakingHandlerError::WrongOwner);
	});
}

#[rstest]
fn disqualified(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let alice = 0;
		let origin = origin(alice);

		let nft = NftStakingHandler::mint(&alice, &PermissionType::DPoS, &100)
			.expect("could mint permission nft");

		// Nft's nominal value is decreased below 80% of the original value. This percentage is described in the pallet's config.
		// For unit tests its defined as 80 in mock.rs
		NftStakingHandler::set_nominal_value(&nft, 79).expect("could set nominal value");

		let res = Staking::bind_validator(origin, nft);
		assert_noop!(res, Error::<Test>::ValidatorDisqualified);
	});
}

#[rstest]
fn already_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let alice = 0;
		let origin = origin(alice);

		let nft1 = NftStakingHandler::mint(&alice, &PermissionType::DPoS, &NOMINAL_VALUE)
			.expect("could mint permission nft");

		let nft2 = NftStakingHandler::mint(&alice, &PermissionType::DPoS, &NOMINAL_VALUE)
			.expect("could mint permission nft");

		let res = Staking::bind_validator(origin.clone(), nft1);
		assert_ok!(res);

		let res = Staking::bind_validator(origin.clone(), nft1);
		assert_noop!(res, Error::<Test>::AlreadyBound);

		let res = Staking::bind_validator(origin, nft2);
		assert_noop!(res, Error::<Test>::AlreadyBound);
	});
}
