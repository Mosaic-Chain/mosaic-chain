use super::*;

#[rstest]
fn enable_delegations_is_successful(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		Staking::disable_delegations(validator.origin.clone()).expect("could disable delegations");

		let res = Staking::enable_delegations(validator.origin.clone());
		assert_ok!(res, ());

		System::assert_last_event(Event::DelegationEnabled(validator.account_id.clone()).into());

		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_currency(
			delegator.origin.clone(),
			100,
			validator.account_id.clone(),
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);
		assert_ok!(res, ());

		let res = Staking::delegate_nft(
			delegator.origin.clone(),
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);
		assert_ok!(res, ());
	});
}

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(account(0));

		let res = Staking::enable_delegations(origin);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[rstest]
fn not_dpos(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();

		let res = Staking::enable_delegations(validator.origin);
		assert_noop!(res, Error::<Test>::CallerNotDPoS);
	});
}

#[rstest]
fn already_enabled(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		let res = Staking::enable_delegations(validator.origin);
		assert_noop!(res, Error::<Test>::AlreadyAcceptsDelegations);
	});
}
