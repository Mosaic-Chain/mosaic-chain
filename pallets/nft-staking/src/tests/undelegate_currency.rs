use super::*;

fn undelegate_maybe_force(
	validator: AccountId,
	delegator: AccountId,
	amount: Balance,
	force: bool,
) -> DispatchResult {
	if force {
		Staking::force_undelegate_currency(RuntimeOrigin::root(), validator, delegator, amount)
	} else {
		Staking::undelegate_currency(origin(delegator), amount, validator)
	}
}

#[apply(force_cases)]
fn undelegate_currency_is_successful(mut ext: TestExternalities, force: bool) {
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

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			MinimumStakingAmount::get(),
			force,
		);
		assert_ok!(res, ());

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if total_stake == NOMINAL_VALUE
		);

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract {
				stake: Stake {
					currency,
					..
				},
				..
			}) if currency == 0);

		System::assert_last_event(
			Event::<Test>::CurrencyUnstaked {
				validator: validator.account_id,
				staker: delegator.account_id,
				amount: MinimumStakingAmount::get(),
			}
			.into(),
		);

		next_session();

		assert!(Pallet::<Test>::current_contract(&validator.account_id, &delegator.account_id)
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

#[apply(force_cases)]
fn can_undelegate_if_target_is_slacking(mut ext: TestExternalities, force: bool) {
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

		// is_slacking = current_index - chill_index > SlackingPeriod
		// => slacking since session `current` + `SlackingPeriod` + 1
		run_until::<AllPalletsWithSystem, Test>(ToSession::current_plus::<Test>(
			SlackingPeriod::get() + 1,
		));

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			MinimumStakingAmount::get(),
			force,
		);
		assert_ok!(res, ());
	});
}

#[apply(force_cases)]
fn target_not_bound(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = 0;
		let delegator = 1;

		let res = undelegate_maybe_force(validator, delegator, MinimumStakingAmount::get(), force);
		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[apply(force_cases)]
fn target_not_dpos(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			MinimumStakingAmount::get(),
			force,
		);
		assert_noop!(res, Error::<Test>::TargetNotDPoS);
	});
}

#[rstest]
fn target_can_be_caller_for_pos_forced(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		ValidatorSet::set(vec![validator.account_id]);
		SessionReward::set(1250); // 1250 * (1 - contribution) = 1000

		next_session();
		next_session();

		Staking::force_undelegate_currency(
			RuntimeOrigin::root(),
			validator.account_id,
			validator.account_id,
			1000,
		)
		.expect("could forcefully undelegate currency");

		System::assert_last_event(
			Event::<Test>::CurrencyUnstaked {
				validator: validator.account_id,
				staker: validator.account_id,
				amount: 1000,
			}
			.into(),
		);
	});
}

#[apply(force_cases)]
fn target_is_caller(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		Staking::self_stake_currency(validator.origin.clone(), MinimumStakingAmount::get())
			.expect("could delegate currency");

		if force {
			Staking::force_undelegate_currency(
				RuntimeOrigin::root(),
				validator.account_id,
				validator.account_id,
				MinimumStakingAmount::get(),
			)
			.expect("could forcefully undelegate currency");
		} else {
			let res = Staking::undelegate_currency(
				validator.origin,
				MinimumStakingAmount::get(),
				validator.account_id,
			);
			assert_noop!(res, Error::<Test>::InvalidTarget);
		}
	});
}

#[apply(force_cases)]
fn no_contract(mut ext: TestExternalities, force: bool) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			MinimumStakingAmount::get(),
			force,
		);
		assert_noop!(res, Error::<Test>::NoContract);
	});
}

#[apply(force_cases)]
fn binding_contract(
	mut ext: TestExternalities,
	#[values(1, MinimumStakingPeriod::get().get() / 2, MinimumStakingPeriod::get().get()  - 1)]
	session: u32,
	force: bool,
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

		run_until::<AllPalletsWithSystem, Test>(ToSession(session));

		if force {
			Staking::force_undelegate_currency(
				RuntimeOrigin::root(),
				validator.account_id,
				delegator.account_id,
				MinimumStakingAmount::get(),
			)
			.expect("could undelegate currency with force");

			System::assert_last_event(
				Event::<Test>::CurrencyUnstaked {
					validator: validator.account_id,
					staker: delegator.account_id,
					amount: MinimumStakingAmount::get(),
				}
				.into(),
			);
		} else {
			let res = Staking::undelegate_currency(
				delegator.origin,
				MinimumStakingAmount::get(),
				validator.account_id,
			);

			assert_noop!(res, Error::<Test>::EarlyUnstake);
		}
	});
}

#[apply(force_cases)]
fn too_small_unstake(
	mut ext: TestExternalities,
	#[values(0, 1, MinimumStakingAmount::get() - 1)] amount: Balance,
	force: bool,
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

		if force {
			Staking::force_undelegate_currency(
				RuntimeOrigin::root(),
				validator.account_id,
				delegator.account_id,
				amount,
			)
			.expect("could undelegate currency with force");

			System::assert_last_event(
				Event::<Test>::CurrencyUnstaked {
					validator: validator.account_id,
					staker: delegator.account_id,
					amount,
				}
				.into(),
			);
		} else {
			let res = Staking::undelegate_currency(delegator.origin, amount, validator.account_id);

			assert_noop!(res, Error::<Test>::TooSmallUnstake);
		}
	});
}

#[apply(force_cases)]
fn not_enough_currency_stake(mut ext: TestExternalities, force: bool) {
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

		let res = undelegate_maybe_force(
			validator.account_id,
			delegator.account_id,
			MinimumStakingAmount::get() + 1,
			force,
		);
		assert_noop!(res, Error::<Test>::InsufficientFunds);
	});
}

#[apply(force_cases)]
fn unstake_would_dust_stake(mut ext: TestExternalities, force: bool) {
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

		if force {
			Staking::force_undelegate_currency(
				RuntimeOrigin::root(),
				validator.account_id,
				delegator.account_id,
				MinimumStakingAmount::get(),
			)
			.expect("could undelegate currency with force");

			System::assert_last_event(
				Event::<Test>::CurrencyUnstaked {
					validator: validator.account_id,
					staker: delegator.account_id,
					amount: MinimumStakingAmount::get(),
				}
				.into(),
			);
		} else {
			let res = Staking::undelegate_currency(
				delegator.origin,
				MinimumStakingAmount::get(),
				validator.account_id,
			);

			assert_noop!(res, Error::<Test>::WouldDust);
		}
	});
}
