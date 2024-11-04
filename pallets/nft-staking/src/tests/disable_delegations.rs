use super::*;

#[rstest]
fn disable_delegations_is_successful(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::disable_delegations(validator.origin.clone());
		assert_ok!(res, ());

		System::assert_last_event(Event::DelegationDisabled(validator.account_id).into());

		let res = Staking::delegate_currency(
			delegator.origin.clone(),
			100,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);
		assert_noop!(res, Error::<Test>::TargetDeniesDelegations);

		let res = Staking::delegate_nft(
			delegator.origin,
			delegator.delegator_nft,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);
		assert_noop!(res, Error::<Test>::TargetDeniesDelegations);
	});
}

#[rstest]
fn can_self_stake(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegation_details = EndowParams::default().account_id(validator.account_id).endow();

		Staking::disable_delegations(validator.origin.clone()).expect("could disable delegations");

		let res = Staking::self_stake_currency(validator.origin.clone(), 100);
		assert_ok!(res, ());

		let res = Staking::self_stake_nft(validator.origin, delegation_details.delegator_nft);
		assert_ok!(res, ());
	});
}

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let origin = origin(0);

		let res = Staking::disable_delegations(origin);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[rstest]
fn not_dpos(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();
		let res = Staking::disable_delegations(validator.origin);
		assert_noop!(res, Error::<Test>::CallerNotDPoS);
	});
}

#[rstest]
fn already_disabled(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();

		Staking::disable_delegations(validator.origin.clone()).expect("could disable delegations");

		let res = Staking::disable_delegations(validator.origin);
		assert_noop!(res, Error::<Test>::AlreadyDeniesDelegations);
	});
}
