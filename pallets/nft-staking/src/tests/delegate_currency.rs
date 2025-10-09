use super::*;

#[rstest]
fn delegate_currency_is_successful(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_ok!(res, ());

		System::assert_last_event(
			Event::CurrencyStaked {
				validator: validator.account_id,
				staker: delegator.account_id,
				amount: MinimumStakingAmount::get(),
			}
			.into(),
		);

		// pallet_balances does not have a Hold event.
		//
		// System::assert_has_event(
		// 	pallet_balances::Event::Locked {
		// 		who: delegator.account_id.clone(),
		// 		amount: MinimumStakingAmount::get(),
		// 	}
		// 	.into(),
		// );

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if total_stake == NOMINAL_VALUE + MinimumStakingAmount::get()
		);

		assert_current_contract!(&validator.account_id,  &delegator.account_id,
			Some(Contract {
				stake: Stake {
					currency,
					permission_nft: None,
					..
				},
				..
			}) if currency == MinimumStakingAmount::get());
	});
}

#[rstest]
fn staking_period_resets(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		run_until::<AllPalletsWithSystem, Test>(ToSession(50));

		Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		)
		.expect("could delegate currency");

		let new_period_end = u32::from(MinimumStakingPeriod::get()) + Session::current_index();

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract { min_staking_period_end, .. } ) if min_staking_period_end == new_period_end);
	});
}

#[rstest]
fn new_commission_applies(mut ext: TestExternalities) {
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

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract { commission, .. } ) if commission == MinimumCommission::get());

		let new_commission = Perbill::from_percent(42);

		Staking::set_commission(validator.origin, new_commission).expect("could set commission");

		Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			new_commission,
		)
		.expect("could delegate currency");

		assert_current_contract!(&validator.account_id, &delegator.account_id,
			Some(Contract { commission, .. } ) if commission == new_commission);
	});
}

#[rstest]
fn target_not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = 0;
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::NotBound);
	});
}

#[rstest]
fn target_chilled(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::chill_validator(validator.origin).expect("could chill validator");

		let res = Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::TargetIsChilled);
	});
}

#[rstest]
fn target_not_dpos(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::PoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::TargetNotDPoS);
	});
}

#[rstest]
fn target_denies_delegations(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		Staking::disable_delegations(validator.origin).expect("could deny delegations");

		let res = Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::TargetDeniesDelegations);
	});
}

#[rstest]
fn staking_period_slippage(
	mut ext: TestExternalities,
	#[values(u32::from(MinimumStakingPeriod::get()) - 1, u32::from(MinimumStakingPeriod::get()) + 1)]
	observed: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			observed,
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::SlippageExceeded);
	});
}

#[rstest]
fn commission_slippage(
	mut ext: TestExternalities,
	#[values(MinimumCommission::get().less_epsilon(), MinimumCommission::get().plus_epsilon())]
	observed: Perbill,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			observed,
		);

		assert_noop!(res, Error::<Test>::SlippageExceeded);
	});
}

#[rstest]
fn target_is_caller(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().account_id(validator.account_id).endow();

		let res = Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get(),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get().plus_epsilon(),
		);

		assert_noop!(res, Error::<Test>::InvalidTarget);
	});
}

#[rstest]
fn too_small_delegation(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_currency(
			delegator.origin,
			MinimumStakingAmount::get() - 1,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::TooSmallStake);
	});
}

#[rstest]
fn insufficient_funds(
	mut ext: TestExternalities,
	#[values(MinimumStakingAmount::get(), MinimumStakingAmount::get() + 1, 100_000_000_000)]
	amount: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().currency(amount - 1).endow();

		let res = Staking::delegate_currency(
			delegator.origin,
			amount,
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::InsufficientFunds);
	});
}

#[rstest]
fn target_would_be_overdominant(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(PermissionType::DPoS).mint().bind();
		let delegator = EndowParams::default().endow();

		let res = Staking::delegate_currency(
			delegator.origin,
			MaximumStakePercentage::get() * (Balances::total_issuance() - NOMINAL_VALUE),
			validator.account_id,
			MinimumStakingPeriod::get().into(),
			MinimumCommission::get(),
		);

		assert_noop!(res, Error::<Test>::OverdominantStake);
	});
}
