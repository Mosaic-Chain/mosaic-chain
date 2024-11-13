use super::*;

#[rstest]
fn set_minimum_staking_period_is_successful(
	mut ext: TestExternalities,
	#[values(MinimumStakingPeriod::get().into(), 500, 10000)] new_period: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let res = Staking::set_minimum_staking_period(validator.origin, new_period);
		assert_ok!(res, ());

		assert!(matches!(
			Validators::<Test>::get(validator.account_id),
			Some(ValidatorDetails::DPoS { min_staking_period, .. }) if min_staking_period == new_period
		));

		System::assert_last_event(
			Event::MinimumStakingPeriodChanged { validator: validator.account_id, new_period }
				.into(),
		);
	});
}

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(0);

		let res = Staking::set_minimum_staking_period(origin, MinimumStakingPeriod::get().into());
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[rstest]
fn chilled(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::set_minimum_staking_period(
			validator.origin,
			MinimumStakingPeriod::get().into(),
		);
		assert_noop!(res, Error::<Test>::CallerIsChilled);
	});
}

#[rstest]
fn not_dpos(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();

		let res = Staking::set_minimum_staking_period(
			validator.origin,
			MinimumStakingPeriod::get().into(),
		);
		assert_noop!(res, Error::<Test>::CallerNotDPoS);
	});
}

#[rstest]
fn too_small_staiking_period(
	mut ext: TestExternalities,
	#[values(0, u32::from(MinimumStakingPeriod::get()) - 1)] new_period: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let res = Staking::set_minimum_staking_period(validator.origin, new_period);

		assert_noop!(res, Error::<Test>::InvalidStakingPeriod);
	});
}
