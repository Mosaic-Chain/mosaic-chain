use super::*;

#[rstest]
fn set_minimum_staking_amount_is_successful(
	mut ext: TestExternalities,
	#[values(MinimumStakingAmount::get(), 500, 10000)] new_amount: u128,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let res = Staking::set_minimum_staking_amount(validator.origin, new_amount);
		assert_ok!(res, ());

		assert!(matches!(
			Validators::<Test>::get(validator.account_id),
			Some(ValidatorDetails::DPoSv2 { min_staking_amount, .. }) if min_staking_amount == new_amount
		));

		System::assert_last_event(
			Event::MinimumStakingAmountChanged { validator: validator.account_id, new_amount }
				.into(),
		);
	});
}

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(0);

		let res = Staking::set_minimum_staking_amount(origin, MinimumStakingAmount::get());
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[rstest]
fn chilled(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res =
			Staking::set_minimum_staking_amount(validator.origin, MinimumStakingAmount::get());
		assert_noop!(res, Error::<Test>::CallerIsChilled);
	});
}

#[rstest]
fn not_dpos(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();

		let res =
			Staking::set_minimum_staking_amount(validator.origin, MinimumStakingAmount::get());
		assert_noop!(res, Error::<Test>::CallerNotDPoS);
	});
}

#[rstest]
fn too_small_staking_amount(
	mut ext: TestExternalities,
	#[values(0, MinimumStakingAmount::get() - 1)] new_amount: u128,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let res = Staking::set_minimum_staking_amount(validator.origin, new_amount);

		assert_noop!(res, Error::<Test>::InvalidStakingAmount);
	});
}
