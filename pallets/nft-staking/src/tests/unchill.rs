use super::*;

#[apply(permission_cases)]
fn unchill_is_successful(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::unchill_validator(validator.origin);
		assert_ok!(res, ());

		assert_validator_state!(&validator.account_id, Some(ValidatorState::Normal));
		System::assert_last_event(Event::ValidatorUnchilled(validator.account_id).into());
	});
}

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(account(0));

		let res = Staking::unchill_validator(origin);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(permission_cases)]
fn not_chilled(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		let res = Staking::unchill_validator(validator.origin);
		assert_noop!(res, Error::<Test>::CallerIsNotChilled);
	});
}

#[apply(permission_cases)]
fn disqualified(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default()
			.permission(permission)
			.nft_nominal_value(u128::from(Perbill::ACCURACY))
			.mint()
			.bind();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");
		NftStakingHandler::set_nominal_value_of_bound(
			&validator.account_id,
			Balance::from(NominalValueThreshold::get().deconstruct()),
		)
		.expect("could set nominal value");

		let res = Staking::unchill_validator(validator.origin);
		assert_noop!(res, Error::<Test>::ValidatorDisqualified);
	});
}
