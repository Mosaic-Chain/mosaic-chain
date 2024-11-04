use super::*;

#[apply(permission_cases)]
fn chill_is_successful(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		let res = Staking::chill_validator(validator.origin);
		assert_ok!(res, ());

		assert_validator_state!(&validator.account_id, Some(ValidatorState::Chilled(0)));

		System::assert_last_event(
			Event::ValidatorChilled {
				validator: validator.account_id,
				reason: ChillReason::Manual,
			}
			.into(),
		);
	});
}

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(0);

		let res = Staking::chill_validator(origin);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(permission_cases)]
fn already_chilled(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::chill_validator(validator.origin);
		assert_noop!(res, Error::<Test>::CallerIsChilled);
	});
}
