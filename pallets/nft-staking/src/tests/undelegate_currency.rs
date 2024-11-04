use super::*;

#[rstest]
fn undelegate_currency_is_successful(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		skip_min_staking_period();

		let res = Staking::undelegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
		);
		assert_ok!(res, ());

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if *total_stake == NOMINAL_VALUE
		);

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract {
				stake: Stake {
					currency,
					..
				},
				..
			}) if *currency == 0);

		System::assert_last_event(
			Event::<Test>::CurrencyUnstaked {
				validator: validator.account_id,
				staker: delegator.account_id,
				amount: MinimumStakingAmount::get(),
			}
			.into(),
		);

		next_session();

		assert!(Contracts::<Test>::get(validator.account_id, delegator.account_id)
			.current()
			.is_none());

		// pallet_balances does not have a Unhold event.
		//
		// System::assert_has_event(
		// 	pallet_balances::Event::<Test>::Unlocked {
		// 		who: delegator.account_id,
		// 		amount: MinimumStakingAmount::get(),
		// 	}
		// 	.into(),
		// );
	});
}

#[rstest]
fn can_undelegate_if_target_is_slacking(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		ValidatorStates::<Test>::mutate_extant(validator.account_id, |s| {
			*s = ValidatorState::Chilled(Session::current_index());
		});

		// 1 block = 1 session
		// (n + 1)th block = nth session
		// is_slacking = current_index - chill_index > SlackingPeriod
		// => chill_imdex is 0
		// => slacking since session SlackingPeriod + 1
		// => slacking since block SlackingPeriod + 2
		run_to_block(u64::from(SlackingPeriod::get()) + 2, |_| {});

		let res = Staking::undelegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
		);
		assert_ok!(res, ());
	});
}

#[rstest]
fn target_not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = 0;
		let delegator = origin(1);

		let res = Staking::undelegate_currency(delegator, MinimumStakingAmount::get(), validator);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[rstest]
fn target_not_dpos(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::undelegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::TargetNotDPoS);
	});
}

#[rstest]
fn target_is_caller(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		let res = Staking::undelegate_currency(
			validator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::InvalidTarget);
	});
}

#[rstest]
fn no_contract(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::undelegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::NoContract);
	});
}

#[rstest]
fn binding_contract(
	mut ext: TestExternalities,
	#[values(2, u32::from(MinimumStakingPeriod::get()) / 2, u32::from(MinimumStakingPeriod::get()))]
	block: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		run_to_block(block.into(), |_| {});

		let res = Staking::undelegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::EarlyUnstake);
	});
}

#[rstest]
fn too_small_unstake(
	mut ext: TestExternalities,
	#[values(0, 1, MinimumStakingAmount::get() - 1)] amount: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		skip_min_staking_period();

		let res = Staking::undelegate_currency(delegator.origin, amount, validator.account_id);
		assert_noop!(res, Error::<Test>::TooSmallUnstake);
	});
}

#[rstest]
fn not_enough_currency_stake(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		skip_min_staking_period();

		let res = Staking::undelegate_currency(
			delegator.origin,
			MinimumStakingAmount::get() + 1,
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::InsufficientFunds);
	});
}

#[rstest]
fn unstake_would_dust_stake(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::delegate_currency(
			delegator.origin.clone(),
			MinimumStakingAmount::get() + 1,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		skip_min_staking_period();

		let res = Staking::undelegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
		);
		assert_noop!(res, Error::<Test>::WouldDust);
	});
}
