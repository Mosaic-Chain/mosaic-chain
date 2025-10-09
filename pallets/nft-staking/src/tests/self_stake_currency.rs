use super::*;

#[apply(dpos)]
fn self_stake_currency_is_successful(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		let res = Staking::self_stake_currency(validator.origin, MinimumStakingAmount::get());
		assert_ok!(res, ());

		System::assert_last_event(
			Event::CurrencyStaked {
				validator: validator.account_id,
				staker: validator.account_id,
				amount: MinimumStakingAmount::get(),
			}
			.into(),
		);

		// pallet_balances does not have a Hold event.
		//
		// System::assert_has_event(
		// 	pallet_balances::Event::Locked {
		// 		who: validator.account_id.clone(),
		// 		amount: MinimumStakingAmount::get(),
		// 	}
		// 	.into(),
		// );

		assert_current_validator_stake!(
			&validator.account_id,
			Some(TotalValidatorStake { total_stake, .. }) if total_stake == NOMINAL_VALUE + MinimumStakingAmount::get()
		);

		assert_current_contract!(&validator.account_id, &validator.account_id,
			Some(Contract {
				stake: Stake {
					currency,
					permission_nft: Some(NOMINAL_VALUE),
					..
				},
				..
			}) if currency == MinimumStakingAmount::get());
	});
}

#[apply(pos)]
fn self_stake_currency_is_unsuccessful(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		assert_noop!(
			Staking::self_stake_currency(validator.origin, MinimumStakingAmount::get()),
			Error::<Test>::CallerNotDPoS
		);
	});
}

#[apply(dpos)]
fn minimum_staking_period_resets(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(1, MinimumStakingPeriod::get().get(), MinimumStakingPeriod::get().get(), 1000)]
	session: u32,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		run_until::<AllPalletsWithSystem, Test>(ToSession(session));

		let res = Staking::self_stake_currency(validator.origin, MinimumStakingAmount::get());
		assert_ok!(res, ());

		let new_period_end = u32::from(MinimumStakingPeriod::get()) + Session::current_index();

		assert_current_contract!(&validator.account_id, &validator.account_id,
			Some(Contract { min_staking_period_end, .. }) if min_staking_period_end == new_period_end
		);
	});
}

#[rstest]
fn not_bound(mut ext: TestExternalities) {
	ext.execute_with(|| {
		let validator = 0;
		let _ = EndowParams::default().account_id(validator).endow();

		let res = Staking::self_stake_currency(origin(validator), 100);
		assert_err!(res, Error::<Test>::NotBound);
	});
}

#[apply(dpos)]
fn chilled(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		Staking::chill_validator(validator.origin.clone()).expect("could chill validator");

		let res = Staking::self_stake_currency(validator.origin, 100);
		assert_err!(res, Error::<Test>::TargetIsChilled);
	});
}

#[apply(dpos)]
fn too_small_amount(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(0, 1, MinimumStakingAmount::get() - 1)] amount: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id);

		let res = Staking::self_stake_currency(validator.origin, amount);
		assert_err!(res, Error::<Test>::TooSmallStake);
	});
}

#[apply(dpos)]
fn not_enough_balance(
	mut ext: TestExternalities,
	permission: PermissionType,
	#[values(MinimumStakingAmount::get(), MinimumStakingAmount::get() + 1, 100_000_000_000)]
	amount: Balance,
) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().currency(amount - 1).endow();

		let res = Staking::self_stake_currency(validator.origin, amount);
		assert_err!(res, Error::<Test>::InsufficientFunds);
	});
}

#[apply(dpos)]
fn would_be_overdominant(mut ext: TestExternalities, permission: PermissionType) {
	ext.execute_with(|| {
		let validator = BindParams::default().permission(permission).mint().bind();
		let _ = EndowParams::default().account_id(validator.account_id).endow();

		let res = Staking::self_stake_currency(
			validator.origin,
			MaximumStakePercentage::get()
				* (Balances::total_issuance()
					- NftStakingHandler::nominal_value(&validator.permission_nft).unwrap()),
		);

		assert_err!(res, Error::<Test>::OverdominantStake);
	});
}
