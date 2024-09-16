use super::*;

#[rstest]
fn set_commission_is_successful(
	mut ext: TestExternalities,
	#[values(MinimumCommission::get(), Perbill::from_percent(50), Perbill::from_percent(100))]
	new_commission: Perbill,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let res = Staking::set_commission(validator.origin, new_commission);
		assert_ok!(res, ());

		assert!(matches!(
			Validators::<Test>::get(&validator.account_id),
			Some(ValidatorDetails::DPoS { commission, .. }) if commission == new_commission
		));

		System::assert_last_event(
			Event::CommissionChanged { validator: validator.account_id, new_commission }.into(),
		);
	});
}

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(account(0));

		let res = Staking::set_commission(origin, MinimumCommission::get());
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[rstest]
fn chilled(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::set_commission(validator.origin, MinimumCommission::get());
		assert_noop!(res, Error::<Test>::CallerIsChilled);
	});
}

#[rstest]
fn not_dpos(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();

		let res = Staking::set_commission(validator.origin, MinimumCommission::get());
		assert_noop!(res, Error::<Test>::CallerNotDPoS);
	});
}

#[rstest]
fn too_small_commission(
	mut ext: TestExternalities,
	#[values(Perbill::zero(), MinimumCommission::get().less_epsilon())] new_commission: Perbill,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let res = Staking::set_commission(validator.origin, new_commission);
		assert_noop!(res, Error::<Test>::InvalidCommission);
	});
}
